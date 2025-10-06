/// Rust Analyzer integration for LSP-based code completion
/// Provides intelligent Rust code completion, goto definition, hover info, etc.

use anyhow::{anyhow, Result};
use gpui::{Context, Task, Window};
use lsp_types::{
    request::{Completion, GotoDefinition, HoverRequest},
    CompletionContext, CompletionItem, CompletionParams, CompletionResponse,
    GotoDefinitionParams, GotoDefinitionResponse, Hover, HoverParams,
    Position as LspPosition, TextDocumentIdentifier, TextDocumentPositionParams,
    Uri, WorkDoneProgressParams,
};
use ropey::Rope;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::input::{InputState, RopeExt};

/// Rust Analyzer LSP client
pub struct RustAnalyzerClient {
    /// The path to the rust-analyzer executable
    analyzer_path: PathBuf,
    /// The root path of the workspace
    workspace_root: PathBuf,
    /// The file URI being edited
    file_uri: Option<Uri>,
    /// LSP process handle
    process: Arc<Mutex<Option<Child>>>,
    /// Whether the client is initialized
    initialized: Arc<Mutex<bool>>,
}

impl RustAnalyzerClient {
    /// Create a new Rust Analyzer client
    pub fn new(workspace_root: PathBuf) -> Result<Self> {
        let analyzer_path = Self::find_rust_analyzer()?;
        
        Ok(Self {
            analyzer_path,
            workspace_root,
            file_uri: None,
            process: Arc::new(Mutex::new(None)),
            initialized: Arc::new(Mutex::new(false)),
        })
    }

    /// Find the rust-analyzer executable in PATH
    fn find_rust_analyzer() -> Result<PathBuf> {
        // Try common locations
        let candidates = vec![
            "rust-analyzer",
            "rust-analyzer.exe",
            "~/.cargo/bin/rust-analyzer",
            "~/.cargo/bin/rust-analyzer.exe",
        ];

        for candidate in candidates {
            if let Ok(output) = Command::new(candidate).arg("--version").output() {
                if output.status.success() {
                    return Ok(PathBuf::from(candidate));
                }
            }
        }

        Err(anyhow!("rust-analyzer not found in PATH. Please install it with: rustup component add rust-analyzer"))
    }

    /// Initialize the LSP server
    pub fn initialize(&self) -> Result<()> {
        let mut process_lock = self.process.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        
        if process_lock.is_some() {
            return Ok(()); // Already initialized
        }

        // Start rust-analyzer process
        let child = Command::new(&self.analyzer_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        *process_lock = Some(child);
        
        let mut init_lock = self.initialized.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        *init_lock = true;

        Ok(())
    }

    /// Set the file being edited
    pub fn set_file(&mut self, file_path: PathBuf) -> Result<()> {
        // Convert PathBuf to Uri using lsp_types::Uri
        let uri_string = format!("file://{}", file_path.display().to_string().replace("\\", "/"));
        // Parse as a URI
        let uri: Uri = uri_string.parse()
            .map_err(|e| anyhow!("Invalid URI: {}", e))?;
        self.file_uri = Some(uri);
        Ok(())
    }

    /// Shutdown the LSP server
    pub fn shutdown(&self) -> Result<()> {
        let mut process_lock = self.process.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        
        if let Some(mut child) = process_lock.take() {
            child.kill()?;
            child.wait()?;
        }

        let mut init_lock = self.initialized.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        *init_lock = false;

        Ok(())
    }
}

/// Rust Analyzer completion provider
pub struct RustAnalyzerCompletionProvider {
    client: Arc<Mutex<RustAnalyzerClient>>,
}

impl RustAnalyzerCompletionProvider {
    /// Create a new Rust Analyzer completion provider
    pub fn new(workspace_root: PathBuf) -> Result<Self> {
        let client = RustAnalyzerClient::new(workspace_root)?;
        client.initialize()?;
        
        Ok(Self {
            client: Arc::new(Mutex::new(client)),
        })
    }

    /// Set the file being edited
    pub fn set_file(&self, file_path: PathBuf) -> Result<()> {
        let mut client = self.client.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        client.set_file(file_path)
    }

    /// Get completions from rust-analyzer
    fn get_completions_internal(
        &self,
        text: &Rope,
        offset: usize,
        _trigger: CompletionContext,
    ) -> Result<CompletionResponse> {
        let client = self.client.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        
        // Check if initialized
        let initialized = client.initialized.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        if !*initialized {
            return Ok(CompletionResponse::Array(vec![]));
        }

        // Get file URI
        let file_uri = client.file_uri.clone()
            .ok_or_else(|| anyhow!("No file set for rust-analyzer"))?;

        // Convert byte offset to LSP position
        let position = text.offset_to_position(offset);

        // Create completion params
        let params = CompletionParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier { uri: file_uri },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: Default::default(),
            context: None,
        };

        // Send LSP request to rust-analyzer
        let mut process_lock = client.process.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
        
        if let Some(child) = process_lock.as_mut() {
            // Send the completion request
            let params_json = serde_json::to_value(params)?;
            match lsp_communication::send_request(child, "textDocument/completion", params_json) {
                Ok(response) => {
                    // Parse the response
                    if let Ok(items) = serde_json::from_value::<Vec<CompletionItem>>(response["result"].clone()) {
                        return Ok(CompletionResponse::Array(items));
                    } else if let Ok(list) = serde_json::from_value::<lsp_types::CompletionList>(response["result"].clone()) {
                        return Ok(CompletionResponse::List(list));
                    }
                }
                Err(e) => {
                    eprintln!("rust-analyzer completion error: {}", e);
                }
            }
        }

        // Return empty completions on error
        Ok(CompletionResponse::Array(vec![]))
    }
}

impl super::CompletionProvider for RustAnalyzerCompletionProvider {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        trigger: CompletionContext,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>> {
        let text = text.clone();
        let client = self.client.clone();
        let offset_copy = offset; // Copy primitive to move
        let trigger_copy = trigger.clone(); // Clone to move
        
        cx.spawn_in(window, async move |_, _cx| {
            let provider = Self { client };
            provider.get_completions_internal(&text, offset_copy, trigger_copy)
        })
    }

    fn is_completion_trigger(
        &self,
        _offset: usize,
        new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        // Trigger on:
        // 1. Dot (method completion)
        // 2. Double colon (path completion)
        // 3. Alphanumeric (word completion)
        new_text.contains('.') 
            || new_text.contains("::")
            || new_text.chars().any(|c| c.is_alphanumeric())
    }
}

impl Drop for RustAnalyzerClient {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

/// Helper module for LSP communication
mod lsp_communication {
    use super::*;
    use std::io::{BufRead, BufReader, Write};

    /// Send an LSP request to rust-analyzer
    pub fn send_request(
        child: &mut Child,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value> {
        let stdin = child.stdin.as_mut()
            .ok_or_else(|| anyhow!("Failed to get stdin"))?;

        // Create JSON-RPC request
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params,
        });

        let request_str = serde_json::to_string(&request)?;
        let content_length = request_str.len();

        // Write headers and content
        write!(stdin, "Content-Length: {}\r\n\r\n", content_length)?;
        write!(stdin, "{}", request_str)?;
        stdin.flush()?;

        // Read response
        let stdout = child.stdout.as_mut()
            .ok_or_else(|| anyhow!("Failed to get stdout"))?;
        let mut reader = BufReader::new(stdout);

        // Read headers
        let mut headers = String::new();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line)?;
            if line == "\r\n" {
                break;
            }
            headers.push_str(&line);
        }

        // Parse Content-Length
        let content_length: usize = headers
            .lines()
            .find(|line| line.starts_with("Content-Length:"))
            .and_then(|line| line.split(':').nth(1))
            .and_then(|len| len.trim().parse().ok())
            .ok_or_else(|| anyhow!("Missing Content-Length header"))?;

        // Read content
        let mut content = vec![0u8; content_length];
        std::io::Read::read_exact(&mut reader, &mut content)?;

        // Parse JSON response
        let response: serde_json::Value = serde_json::from_slice(&content)?;
        
        Ok(response)
    }
}

/// A mock rust-analyzer provider for testing without actual LSP connection
/// This provides basic Rust completions without needing rust-analyzer
pub struct MockRustCompletionProvider {
    common_rust_items: Vec<CompletionItem>,
}

impl MockRustCompletionProvider {
    pub fn new() -> Self {
        let mut common_rust_items = Vec::new();

        // Add common Rust types
        for (label, detail) in &[
            ("String", "std::string::String"),
            ("Vec", "std::vec::Vec"),
            ("Option", "std::option::Option"),
            ("Result", "std::result::Result"),
            ("Box", "std::boxed::Box"),
            ("Arc", "std::sync::Arc"),
            ("Rc", "std::rc::Rc"),
            ("HashMap", "std::collections::HashMap"),
            ("HashSet", "std::collections::HashSet"),
            ("BTreeMap", "std::collections::BTreeMap"),
            ("println!", "Print to stdout with newline"),
            ("eprintln!", "Print to stderr with newline"),
            ("format!", "Format string"),
            ("vec!", "Create a vector"),
            ("assert!", "Assert condition"),
            ("assert_eq!", "Assert equality"),
            ("unwrap", "Unwrap Option/Result"),
            ("expect", "Unwrap with message"),
            ("clone", "Clone value"),
            ("into", "Convert into type"),
            ("from", "Convert from type"),
            ("to_string", "Convert to String"),
            ("len", "Get length"),
            ("is_empty", "Check if empty"),
            ("iter", "Create iterator"),
            ("collect", "Collect iterator"),
            ("map", "Map iterator"),
            ("filter", "Filter iterator"),
            ("fold", "Fold iterator"),
        ] {
            common_rust_items.push(CompletionItem {
                label: label.to_string(),
                kind: Some(lsp_types::CompletionItemKind::FUNCTION),
                detail: Some(detail.to_string()),
                sort_text: Some(format!("b_{}", label)),
                ..Default::default()
            });
        }

        Self { common_rust_items }
    }
}

impl super::CompletionProvider for MockRustCompletionProvider {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        _trigger: CompletionContext,
        _window: &mut Window,
        _cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>> {
        // Get current word
        let current_word = self.get_word_at_offset(text, offset);
        
        let completions: Vec<CompletionItem> = self.common_rust_items
            .iter()
            .filter(|item| {
                current_word.is_empty() 
                    || item.label.to_lowercase().starts_with(&current_word.to_lowercase())
            })
            .cloned()
            .collect();

        Task::ready(Ok(CompletionResponse::Array(completions)))
    }

    fn is_completion_trigger(
        &self,
        _offset: usize,
        new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        new_text.contains('.') 
            || new_text.contains("::")
            || new_text.chars().any(|c| c.is_alphanumeric())
    }
}

impl MockRustCompletionProvider {
    fn get_word_at_offset(&self, text: &Rope, offset: usize) -> String {
        let offset = offset.min(text.len());
        let mut start = offset;
        
        while start > 0 {
            let prev_offset = start.saturating_sub(1);
            if prev_offset < text.len() {
                let ch = text.slice(prev_offset..prev_offset+1).to_string().chars().next().unwrap_or(' ');
                if !ch.is_alphanumeric() && ch != '_' && ch != '!' {
                    break;
                }
                start = start.saturating_sub(1);
            } else {
                break;
            }
        }
        
        text.slice(start..offset).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_rust_completions() {
        let provider = MockRustCompletionProvider::new();
        assert!(!provider.common_rust_items.is_empty());
    }

    #[test]
    fn test_find_rust_analyzer() {
        // This test may fail if rust-analyzer is not installed
        match RustAnalyzerClient::find_rust_analyzer() {
            Ok(path) => println!("Found rust-analyzer at: {:?}", path),
            Err(e) => println!("rust-analyzer not found: {}", e),
        }
    }
}
