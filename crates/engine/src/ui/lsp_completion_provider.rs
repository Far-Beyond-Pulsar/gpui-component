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
        // Check if analyzer is ready
        let is_ready = self.analyzer.read(cx).is_running();
        if !is_ready {
            return Task::ready(Ok(lsp_types::CompletionResponse::Array(vec![])));
        }

        // Convert offset to LSP position
        let position = text.offset_to_position(offset);

        // Create completion params
        let uri = self.path_to_uri();
        
        // Request completions from rust-analyzer
        let analyzer = self.analyzer.clone();
        let trigger_kind = match trigger.trigger_kind {
            lsp_types::CompletionTriggerKind::INVOKED => 1,
            lsp_types::CompletionTriggerKind::TRIGGER_CHARACTER => 2,
            lsp_types::CompletionTriggerKind::TRIGGER_FOR_INCOMPLETE_COMPLETIONS => 3,
            _ => 1,
        };
        
        cx.spawn_in(window, async move |_, cx| {
            // Send completion request - use ok() to handle Result
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
                // Parse the response
                if let Some(result) = response.get("result") {
                    // Try as array first
                    if let Ok(items) = serde_json::from_value::<Vec<lsp_types::CompletionItem>>(result.clone()) {
                        return Ok(lsp_types::CompletionResponse::Array(items));
                    } 
                    // Try as completion list
                    else if let Ok(list) = serde_json::from_value::<lsp_types::CompletionList>(result.clone()) {
                        return Ok(lsp_types::CompletionResponse::List(list));
                    }
                }
            }

            // Return empty on error
            Ok(lsp_types::CompletionResponse::Array(vec![]))
        })
    }

    fn is_completion_trigger(
        &self,
        _offset: usize,
        new_text: &str,
        _cx: &mut Context<InputState>,
    ) -> bool {
        // Trigger on dot, double colon, or after alphanumeric characters
        if new_text.is_empty() {
            return false;
        }
        
        // Trigger on member access
        if new_text.ends_with('.') || new_text.ends_with("::") {
            return true;
        }
        
        // Trigger after typing identifiers (but not on every keystroke)
        let last_char = new_text.chars().last().unwrap();
        last_char.is_alphanumeric() || last_char == '_'
    }
}
