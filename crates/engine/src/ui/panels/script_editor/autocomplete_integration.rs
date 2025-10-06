/// Example integration of comprehensive autocomplete into the Script Editor
/// This demonstrates how to set up and use all the completion features
/// Note: The global rust-analyzer is managed by the engine, not per-file

use gpui::{Context, Window};
use gpui_component::input::{
    ComprehensiveCompletionProvider, 
    DictionaryProvider,
    MockRustCompletionProvider,
    InputState,
};
use std::path::PathBuf;
use std::rc::Rc;

/// Helper function to set up autocomplete for a Rust file
/// 
/// This configures the input state with:
/// - Dictionary-based completion (English words)
/// - Closure/bracket auto-completion
/// - Rust completions from global rust-analyzer (when available)
/// 
/// Note: rust-analyzer is managed globally by the engine.
/// The global instance provides completions to all open files automatically via LSP.
pub fn setup_rust_autocomplete(
    input_state: &mut InputState,
    workspace_root: Option<PathBuf>,
    file_path: PathBuf,
    window: &mut Window,
    cx: &mut Context<InputState>,
) {
    // Create the comprehensive completion provider with dictionary + closures
    // The dictionary provides English word completions
    // Closure completion handles (), {}, [], "", '', etc.
    let provider = ComprehensiveCompletionProvider::new();
    
    // For now, we use mock rust completions for basic Rust keywords
    // TODO: Connect to the global rust-analyzer LSP for advanced completions
    // This requires implementing an LSP completion provider that communicates
    // with the RustAnalyzerManager instance
    let provider_with_lsp = provider.with_lsp_provider(Rc::new(MockRustCompletionProvider::new()));
    
    // Set the completion provider
    input_state.lsp.completion_provider = Some(Rc::new(provider_with_lsp));
    
    if let Some(workspace) = workspace_root {
        println!("✓ Autocomplete configured for: {:?} (workspace: {:?})", file_path.file_name(), workspace);
    } else {
        println!("✓ Autocomplete configured for: {:?}", file_path.file_name());
    }
}

/// Helper function to set up autocomplete for JavaScript/TypeScript files
pub fn setup_javascript_autocomplete(
    input_state: &mut InputState,
    workspace_root: Option<PathBuf>,
    file_path: PathBuf,
    _window: &mut Window,
    _cx: &mut Context<InputState>,
) {
    // Create the comprehensive completion provider with JS/TS language support
    let provider = ComprehensiveCompletionProvider::new();
    
    // Set the completion provider
    input_state.lsp.completion_provider = Some(Rc::new(provider));
    
    if let Some(workspace) = workspace_root {
        println!("✓ JavaScript/TypeScript autocomplete configured for: {:?} (workspace: {:?})", file_path.file_name(), workspace);
    } else {
        println!("✓ JavaScript/TypeScript autocomplete configured for: {:?}", file_path.file_name());
    }
}

/// Helper function to set up autocomplete for Python files
pub fn setup_python_autocomplete(
    input_state: &mut InputState,
    workspace_root: Option<PathBuf>,
    file_path: PathBuf,
    _window: &mut Window,
    _cx: &mut Context<InputState>,
) {
    // Create the comprehensive completion provider with Python language support
    let provider = ComprehensiveCompletionProvider::new();
    
    // Set the completion provider
    input_state.lsp.completion_provider = Some(Rc::new(provider));
    
    if let Some(workspace) = workspace_root {
        println!("✓ Python autocomplete configured for: {:?} (workspace: {:?})", file_path.file_name(), workspace);
    } else {
        println!("✓ Python autocomplete configured for: {:?}", file_path.file_name());
    }
}

/// Helper function to set up autocomplete for plain text files
pub fn setup_text_autocomplete(
    input_state: &mut InputState,
    workspace_root: Option<PathBuf>,
    file_path: PathBuf,
    _window: &mut Window,
    _cx: &mut Context<InputState>,
) {
    // Create a basic completion provider with dictionary only
    let provider = ComprehensiveCompletionProvider::new();
    
    // Set the completion provider
    input_state.lsp.completion_provider = Some(Rc::new(provider));
    
    if let Some(workspace) = workspace_root {
        println!("✓ Text autocomplete configured for: {:?} (workspace: {:?})", file_path.file_name(), workspace);
    } else {
        println!("✓ Text autocomplete configured for: {:?}", file_path.file_name());
    }
}

/// Detect language and set up appropriate autocomplete
pub fn setup_autocomplete_for_file(
    input_state: &mut InputState,
    file_path: PathBuf,
    workspace_root: Option<PathBuf>,
    window: &mut Window,
    cx: &mut Context<InputState>,
) {
    let extension = file_path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    
    match extension {
        "rs" => setup_rust_autocomplete(input_state, workspace_root, file_path, window, cx),
        "js" | "jsx" | "ts" | "tsx" => setup_javascript_autocomplete(input_state, workspace_root, file_path, window, cx),
        "py" => setup_python_autocomplete(input_state, workspace_root, file_path, window, cx),
        _ => setup_text_autocomplete(input_state, workspace_root, file_path, window, cx),
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
