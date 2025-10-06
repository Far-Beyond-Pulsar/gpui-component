/// LSP Completion Provider that connects to the global rust-analyzer manager
/// This provides real-time code completions from rust-analyzer

use anyhow::Result;
use gpui::{App, Context, Entity, Task, Window};
use gpui_component::input::{CompletionProvider, InputState, RopeExt};
use lsp_types::{CompletionContext, CompletionResponse, CompletionParams, TextDocumentPositionParams, TextDocumentIdentifier, WorkDoneProgressParams};
use ropey::Rope;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

use super::rust_analyzer_manager::RustAnalyzerManager;

/// Completion provider that uses the global rust-analyzer instance
pub struct GlobalRustAnalyzerCompletionProvider {
    /// Reference to the global rust-analyzer manager
    analyzer: Entity<RustAnalyzerManager>,
    /// Current file path
    file_path: PathBuf,
    /// Workspace root
    workspace_root: PathBuf,
}

impl GlobalRustAnalyzerCompletionProvider {
    pub fn new(
        analyzer: Entity<RustAnalyzerManager>,
        file_path: PathBuf,
        workspace_root: PathBuf,
    ) -> Self {
        Self {
            analyzer,
            file_path,
            workspace_root,
        }
    }

    /// Convert file path to LSP URI
    fn path_to_uri(&self) -> String {
        let path_str = self.file_path.to_string_lossy().replace("\\", "/");
        if path_str.starts_with("C:/") || path_str.starts_with("c:/") {
            format!("file:///{}", path_str)
        } else {
            format!("file://{}", path_str)
        }
    }
}

impl CompletionProvider for GlobalRustAnalyzerCompletionProvider {
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        _trigger: CompletionContext,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>> {
        // Check if analyzer is ready
        let is_ready = self.analyzer.read(cx).is_running();
        if !is_ready {
            return Task::ready(Ok(CompletionResponse::Array(vec![])));
        }

        // Convert offset to LSP position
        let position = text.offset_to_position(offset);

        // Create completion params
        let uri = self.path_to_uri();
        
        // Request completions from rust-analyzer
        let analyzer = self.analyzer.clone();
        let file_path = self.file_path.clone();
        
        cx.spawn_in(window, async move |_, cx| {
            // Send completion request
            let result = analyzer.update(&cx, |analyzer, _cx| {
                let params = json!({
                    "textDocument": {
                        "uri": uri
                    },
                    "position": position,
                    "context": {
                        "triggerKind": 1
                    }
                });

                analyzer.send_request("textDocument/completion", params)
            });

            match result {
                Ok(response_result) => {
                    match response_result {
                        Ok(response) => {
                            // Parse the response
                            if let Some(result) = response.get("result") {
                                if let Ok(items) = serde_json::from_value::<Vec<lsp_types::CompletionItem>>(result.clone()) {
                                    return Ok(CompletionResponse::Array(items));
                                } else if let Ok(list) = serde_json::from_value::<lsp_types::CompletionList>(result.clone()) {
                                    return Ok(CompletionResponse::List(list));
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Completion request failed: {}", e);
                        }
                    }
                }
                Err(_) => {}
            }

            // Return empty on error
            Ok(CompletionResponse::Array(vec![]))
        })
    }

    fn is_completion_trigger(
        &self,
        _offset: usize,
        new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        // Trigger on dot, double colon, or alphanumeric
        new_text.contains('.') 
            || new_text.contains("::")
            || new_text.chars().any(|c| c.is_alphanumeric())
    }
}
