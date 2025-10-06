/// LSP Completion Provider that connects to the global rust-analyzer manager
/// This provides real-time code completions from rust-analyzer

use anyhow::Result;
use gpui::{Context, Task, Window};
use gpui_component::input::{CompletionProvider, InputState, RopeExt};
use serde_json::json;
use std::path::PathBuf;

use super::rust_analyzer_manager::RustAnalyzerManager;
use gpui::Entity;

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
        text: &ropey::Rope,
        offset: usize,
        trigger: lsp_types::CompletionContext,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<lsp_types::CompletionResponse>> {
        println!("🔍 GlobalRustAnalyzerCompletionProvider::completions called for {:?} at offset {}", 
            self.file_path.file_name(), offset);
        
        // Check if analyzer is ready
        let is_ready = self.analyzer.read(cx).is_running();
        println!("   rust-analyzer running: {}", is_ready);
        if !is_ready {
            println!("   ⚠️  rust-analyzer not ready, returning empty completions");
            return Task::ready(Ok(lsp_types::CompletionResponse::Array(vec![])));
        }

        // Convert offset to LSP position
        let position = text.offset_to_position(offset);
        println!("   LSP position: line {}, character {}", position.line, position.character);

        // Create completion params
        let uri = self.path_to_uri();
        println!("   File URI: {}", uri);
        
        // Get the full current content to ensure synchronization
        let content = text.to_string();
        let file_path = self.file_path.clone();
        
        // Request completions from rust-analyzer
        let analyzer = self.analyzer.clone();
        let trigger_kind = match trigger.trigger_kind {
            lsp_types::CompletionTriggerKind::INVOKED => 1,
            lsp_types::CompletionTriggerKind::TRIGGER_CHARACTER => 2,
            lsp_types::CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS => 3,
            _ => 1,
        };
        
        println!("   Trigger kind: {}", trigger_kind);
        
        cx.spawn_in(window, async move |_, cx| {
            println!("🔄 Syncing file content with rust-analyzer before completion...");
            // First, ensure the file is synchronized with rust-analyzer
            // Send a didChange with the current content to ensure we have the latest state
            let sync_result = analyzer.update(cx, |analyzer, _| {
                // Use a high version number to ensure this is the latest
                analyzer.did_change_file(&file_path, &content, 999999)
            }).ok().and_then(|r| r.ok());
            
            if sync_result.is_none() {
                // If sync failed, still try to get completions but they might be stale
                eprintln!("⚠️  Failed to sync file content with rust-analyzer before completion");
            } else {
                println!("✓ File content synced");
            }
            
            // Small delay to allow rust-analyzer to process the change
            // This is important for accurate completions
            println!("⏱️  Waiting 50ms for rust-analyzer to process...");
            gpui::Timer::after(std::time::Duration::from_millis(50)).await;
            
            println!("📡 Sending completion request to rust-analyzer...");
            // Send completion request
            let response_result = analyzer.update(cx, |analyzer, _| {
                let params = json!({
                    "textDocument": {
                        "uri": uri
                    },
                    "position": {
                        "line": position.line,
                        "character": position.character
                    },
                    "context": {
                        "triggerKind": trigger_kind
                    }
                });

                analyzer.send_request("textDocument/completion", params)
            }).ok().and_then(|r| r.ok());

            if let Some(response) = response_result {
                println!("✓ Received response from rust-analyzer");
                
                // Check for error in response
                if let Some(error) = response.get("error") {
                    eprintln!("❌ rust-analyzer returned error: {}", error);
                    return Ok(lsp_types::CompletionResponse::Array(vec![]));
                }
                
                // Parse the response
                if let Some(result) = response.get("result") {
                    // Check if result is null
                    if result.is_null() {
                        println!("⚠️  rust-analyzer returned null result (no completions available)");
                        return Ok(lsp_types::CompletionResponse::Array(vec![]));
                    }
                    
                    println!("📦 Result field type: {}", match result {
                        serde_json::Value::Null => "null",
                        serde_json::Value::Bool(_) => "bool",
                        serde_json::Value::Number(_) => "number",
                        serde_json::Value::String(_) => "string",
                        serde_json::Value::Array(_) => "array",
                        serde_json::Value::Object(_) => "object",
                    });
                    
                    // For debugging, print first 500 chars of result
                    let result_str = serde_json::to_string(&result).unwrap_or_else(|_| "Invalid JSON".to_string());
                    if result_str.len() > 500 {
                        println!("📦 Result preview (first 500 chars): {}...", &result_str[..500]);
                    } else {
                        println!("📦 Result: {}", result_str);
                    }
                    
                    // Try as array first
                    match serde_json::from_value::<Vec<lsp_types::CompletionItem>>(result.clone()) {
                        Ok(items) => {
                            println!("✓ Parsed {} completion items as array", items.len());
                            return Ok(lsp_types::CompletionResponse::Array(items));
                        }
                        Err(e) => {
                            println!("   Array parse failed: {}", e);
                        }
                    }
                    
                    // Try as completion list
                    match serde_json::from_value::<lsp_types::CompletionList>(result.clone()) {
                        Ok(list) => {
                            println!("✓ Parsed completion list with {} items", list.items.len());
                            return Ok(lsp_types::CompletionResponse::List(list));
                        }
                        Err(e) => {
                            println!("   List parse failed: {}", e);
                        }
                    }
                    
                    eprintln!("⚠️  Failed to parse completion response - unknown format");
                } else {
                    eprintln!("⚠️  No 'result' field in response");
                    eprintln!("   Response keys: {:?}", response.as_object().map(|o| o.keys().collect::<Vec<_>>()));
                }
            } else {
                eprintln!("⚠️  No response from rust-analyzer");
            }

            // Return empty on error
            println!("❌ Returning empty completions");
            Ok(lsp_types::CompletionResponse::Array(vec![]))
        })
    }

    fn is_completion_trigger(
        &self,
        _offset: usize,
        new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        // Rust-analyzer's official trigger characters: '.', ':', '<'
        // These are the characters rust-analyzer declares in its CompletionOptions
        
        if new_text.is_empty() {
            return false;
        }
        
        let last_char = new_text.chars().last().unwrap();
        
        // Only auto-trigger on rust-analyzer's declared trigger characters
        matches!(last_char, '.' | ':' | '<')
    }
}
