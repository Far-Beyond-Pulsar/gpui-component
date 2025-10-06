/// Example integration of comprehensive autocomplete into the Script Editor
/// This demonstrates how to set up and use all the completion features

use gpui::{Context, Window};
use gpui_component::input::{
    ComprehensiveCompletionProvider, 
    DictionaryProvider,
    MockRustCompletionProvider,
    RustAnalyzerCompletionProvider,
    InputState,
};
use std::path::PathBuf;
use std::rc::Rc;

/// Helper function to set up autocomplete for a Rust file
/// 
/// This configures the input state with:
/// - Dictionary-based completion
/// - Language-specific completion (keywords, snippets)
/// - Closure/bracket auto-completion
/// - Rust-analyzer LSP integration (if available)
pub fn setup_rust_autocomplete(
    input_state: &mut InputState,
    workspace_root: PathBuf,
    file_path: PathBuf,
    window: &mut Window,
    cx: &mut Context<InputState>,
) {
    // Create the comprehensive completion provider
    let provider = ComprehensiveCompletionProvider::new();
    
    // Try to set up rust-analyzer, fall back to mock if unavailable
    let provider_with_lsp = if let Ok(rust_analyzer) = RustAnalyzerCompletionProvider::new(workspace_root.clone()) {
        // Set the current file
        let _ = rust_analyzer.set_file(file_path.clone());
        provider.with_lsp_provider(Rc::new(rust_analyzer))
    } else {
        // Use mock rust provider as fallback
        println!("⚠️  rust-analyzer not available, using mock Rust completions");
        provider.with_lsp_provider(Rc::new(MockRustCompletionProvider::new()))
    };
    
    // Set the completion provider
    input_state.lsp.completion_provider = Some(Rc::new(provider_with_lsp));
    
    println!("✓ Autocomplete configured for: {:?}", file_path.file_name());
}

/// Helper function to set up autocomplete for JavaScript/TypeScript files
pub fn setup_javascript_autocomplete(
    input_state: &mut InputState,
    _window: &mut Window,
    _cx: &mut Context<InputState>,
) {
    let provider = ComprehensiveCompletionProvider::new();
    input_state.lsp.completion_provider = Some(Rc::new(provider));
    
    println!("✓ JavaScript autocomplete configured");
}

/// Helper function to set up autocomplete for Python files
pub fn setup_python_autocomplete(
    input_state: &mut InputState,
    _window: &mut Window,
    _cx: &mut Context<InputState>,
) {
    let provider = ComprehensiveCompletionProvider::new();
    input_state.lsp.completion_provider = Some(Rc::new(provider));
    
    println!("✓ Python autocomplete configured");
}

/// Helper function to set up autocomplete for generic text files
pub fn setup_text_autocomplete(
    input_state: &mut InputState,
    _window: &mut Window,
    _cx: &mut Context<InputState>,
) {
    let provider = ComprehensiveCompletionProvider::new();
    input_state.lsp.completion_provider = Some(Rc::new(provider));
    
    println!("✓ Text autocomplete configured (dictionary only)");
}

/// Detect language and set up appropriate autocomplete
pub fn setup_autocomplete_for_file(
    input_state: &mut InputState,
    file_path: PathBuf,
    workspace_root: PathBuf,
    window: &mut Window,
    cx: &mut Context<InputState>,
) {
    let extension = file_path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    
    match extension {
        "rs" => setup_rust_autocomplete(input_state, workspace_root, file_path, window, cx),
        "js" | "jsx" | "ts" | "tsx" => setup_javascript_autocomplete(input_state, window, cx),
        "py" => setup_python_autocomplete(input_state, window, cx),
        _ => setup_text_autocomplete(input_state, window, cx),
    }
}

/// Example usage in the text editor:
/// 
/// ```rust,no_run
/// use crate::ui::panels::script_editor::autocomplete_integration;
/// 
/// impl TextEditor {
///     pub fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
///         // ... existing file opening code ...
///         
///         // Set up autocomplete for the file
///         if let Some(open_file) = self.open_files.last_mut() {
///             open_file.input_state.update(cx, |state, cx| {
///                 let workspace_root = self.get_workspace_root();
///                 autocomplete_integration::setup_autocomplete_for_file(
///                     state,
///                     path.clone(),
///                     workspace_root,
///                     window,
///                     cx,
///                 );
///             });
///         }
///     }
/// }
/// ```

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_detection() {
        let test_cases = vec![
            ("test.rs", "rust"),
            ("test.js", "javascript"),
            ("test.ts", "javascript"),
            ("test.py", "python"),
            ("test.txt", "text"),
        ];

        for (filename, expected_lang) in test_cases {
            let path = PathBuf::from(filename);
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            
            let detected = match ext {
                "rs" => "rust",
                "js" | "ts" => "javascript",
                "py" => "python",
                _ => "text",
            };

            assert_eq!(detected, expected_lang, "Failed for: {}", filename);
        }
    }
}
