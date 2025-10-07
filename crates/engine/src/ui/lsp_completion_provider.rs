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
        // Check if analyzer is ready (fast check)
        let is_ready = self.analyzer.read(cx).is_running();
        if !is_ready {
            return Task::ready(Ok(lsp_types::CompletionResponse::Array(vec![])));
        }

        // Clone only what we need - DO NOT convert rope to string here (blocks UI!)
        let uri = self.path_to_uri();
        let file_path = self.file_path.clone();
        let analyzer = self.analyzer.clone();
        let text_clone = text.clone(); // Rope clone is cheap (it's a rope, not a copy)
        
        let trigger_kind = match trigger.trigger_kind {
            lsp_types::CompletionTriggerKind::INVOKED => 1,
            lsp_types::CompletionTriggerKind::TRIGGER_CHARACTER => 2,
            lsp_types::CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS => 3,
            _ => 1,
        };
        
        let trigger_char = trigger.trigger_character.clone();
        
        // Spawn immediately - do ALL potentially slow work in the async block
        cx.spawn_in(window, async move |_, cx| {
            // Convert to position in background (can be slow for large files)
            let position = text_clone.offset_to_position(offset);
            
            // Convert rope to string in background (can be slow for large files)
            let content = text_clone.to_string();
            
            // Sync file content with rust-analyzer
            let sync_result = analyzer.update(cx, |analyzer, _| {
                analyzer.did_change_file(&file_path, &content, 999999)
            }).ok().and_then(|r| r.ok());
            
            if sync_result.is_none() {
                eprintln!("⚠️  Failed to sync file content with rust-analyzer");
                return Ok(lsp_types::CompletionResponse::Array(vec![]));
            }
            
            // NO delay - request immediately! rust-analyzer is fast enough
            // Any delay just makes the UI feel sluggish
            
            // Send completion request (async, non-blocking!)
            let response_rx = match analyzer.update(cx, |analyzer, _| {
                let mut context = json!({
                    "triggerKind": trigger_kind
                });
                
                // Include trigger character if present
                if let Some(ref ch) = trigger_char {
                    context["triggerCharacter"] = json!(ch);
                }
                
                let params = json!({
                    "textDocument": {
                        "uri": uri
                    },
                    "position": {
                        "line": position.line,
                        "character": position.character
                    },
                    "context": context
                });

                analyzer.send_request_async("textDocument/completion", params)
            }).ok().and_then(|r| r.ok()) {
                Some(rx) => rx,
                None => {
                    eprintln!("⚠️  Failed to send completion request");
                    return Ok(lsp_types::CompletionResponse::Array(vec![]));
                }
            };

            // Wait for response asynchronously (non-blocking!)
            let response = match response_rx.recv_async().await {
                Ok(resp) => resp,
                Err(e) => {
                    eprintln!("⚠️  Failed to receive completion response: {}", e);
                    return Ok(lsp_types::CompletionResponse::Array(vec![]));
                }
            };

            // Check for error in response
            if let Some(error) = response.get("error") {
                eprintln!("❌ rust-analyzer completion error: {}", error);
                return Ok(lsp_types::CompletionResponse::Array(vec![]));
            }
            
            // Parse the response
            if let Some(result) = response.get("result") {
                // Check if result is null
                if result.is_null() {
                    return Ok(lsp_types::CompletionResponse::Array(vec![]));
                }
                
                // Try as array first
                if let Ok(items) = serde_json::from_value::<Vec<lsp_types::CompletionItem>>(result.clone()) {
                    return Ok(lsp_types::CompletionResponse::Array(items));
                }
                
                // Try as completion list
                if let Ok(list) = serde_json::from_value::<lsp_types::CompletionList>(result.clone()) {
                    return Ok(lsp_types::CompletionResponse::List(list));
                }
                
                // If we get here, parsing failed
                eprintln!("⚠️  Failed to parse completion response: {:?}", result);
            } else {
                eprintln!("⚠️  No 'result' field in response");
            }

            // Return empty on error or no response
            Ok(lsp_types::CompletionResponse::Array(vec![]))
        })
    }

    fn is_completion_trigger(
        &self,
        _offset: usize,
        new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        // VSCode behavior: Trigger on almost every keystroke to let rust-analyzer decide
        // rust-analyzer is smart enough to return empty results when appropriate
        
        if new_text.is_empty() {
            return false;
        }
        
        let last_char = new_text.chars().last().unwrap();
        
        // ALWAYS trigger on:
        // 1. Identifier characters (alphanumeric or underscore)
        // 2. rust-analyzer trigger characters (., :, <)
        // 3. Additional useful characters ((, ), >, ,, [, space)
        
        // Trigger on identifier characters - this is the most important
        if last_char.is_alphanumeric() || last_char == '_' {
            return true;
        }
        
        // rust-analyzer registered trigger characters
        if matches!(last_char, '.' | ':' | '<') {
            return true;
        }
        
        // Additional useful triggers
        if matches!(last_char, '(' | ')' | '>' | ',' | '[' | ' ') {
            return true;
        }
        
        // Don't trigger on other special characters
        false
    }
}
