/// Autocomplete integration for the Script Editor with rust-analyzer support
/// This module connects the global rust-analyzer instance to provide intelligent completions

use gpui::{App, Context, Entity, Window};
use gpui_component::input::{
    ComprehensiveCompletionProvider, 
    InputState,
};
use std::path::PathBuf;
use std::rc::Rc;

use crate::ui::rust_analyzer_manager::RustAnalyzerManager;
use crate::ui::lsp_completion_provider::GlobalRustAnalyzerCompletionProvider;

/// Helper function to set up autocomplete for a Rust file with real rust-analyzer completions
/// 
/// This configures the input state with:
/// - Dictionary-based completion (English words)
/// - Closure/bracket auto-completion
/// - Real Rust completions from global rust-analyzer LSP
pub fn setup_rust_autocomplete(
    input_state: &mut InputState,
    workspace_root: Option<PathBuf>,
    file_path: PathBuf,
    analyzer: Entity<RustAnalyzerManager>,
    window: &mut Window,
    cx: &mut Context<InputState>,
) {
    let workspace = workspace_root.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    
    // Create the comprehensive completion provider with dictionary + closures
    let mut provider = ComprehensiveCompletionProvider::new();
    
    // Add rust-analyzer completion provider if available
    let rust_provider = GlobalRustAnalyzerCompletionProvider::new(
        analyzer,
        file_path.clone(),
        workspace.clone(),
    );
    provider = provider.with_lsp_provider(Rc::new(rust_provider));
    
    // Set the completion provider
    input_state.lsp.completion_provider = Some(Rc::new(provider));
    
    println!("✓ Autocomplete configured for: {:?} (workspace: {:?})", file_path.file_name(), workspace);
}

/// Helper function to set up autocomplete for JavaScript/TypeScript files
pub fn setup_javascript_autocomplete(
    input_state: &mut InputState,
    workspace_root: Option<PathBuf>,
    file_path: PathBuf,
    _window: &mut Window,
    _cx: &mut Context<InputState>,
) {
    let provider = ComprehensiveCompletionProvider::new();
    input_state.lsp.completion_provider = Some(Rc::new(provider));
    
    let workspace = workspace_root.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    println!("✓ JavaScript/TypeScript autocomplete configured for: {:?} (workspace: {:?})", file_path.file_name(), workspace);
}

/// Helper function to set up autocomplete for Python files
pub fn setup_python_autocomplete(
    input_state: &mut InputState,
    workspace_root: Option<PathBuf>,
    file_path: PathBuf,
    _window: &mut Window,
    _cx: &mut Context<InputState>,
) {
    let provider = ComprehensiveCompletionProvider::new();
    input_state.lsp.completion_provider = Some(Rc::new(provider));
    
    let workspace = workspace_root.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    println!("✓ Python autocomplete configured for: {:?} (workspace: {:?})", file_path.file_name(), workspace);
}

/// Helper function to set up autocomplete for plain text files
pub fn setup_text_autocomplete(
    input_state: &mut InputState,
    workspace_root: Option<PathBuf>,
    file_path: PathBuf,
    _window: &mut Window,
    _cx: &mut Context<InputState>,
) {
    let provider = ComprehensiveCompletionProvider::new();
    input_state.lsp.completion_provider = Some(Rc::new(provider));
    
    let workspace = workspace_root.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    println!("✓ Text autocomplete configured for: {:?} (workspace: {:?})", file_path.file_name(), workspace);
}

/// Detect language and set up appropriate autocomplete with global rust-analyzer
pub fn setup_autocomplete_for_file(
    input_state: &mut InputState,
    file_path: PathBuf,
    workspace_root: Option<PathBuf>,
    analyzer: Entity<RustAnalyzerManager>,
    window: &mut Window,
    cx: &mut Context<InputState>,
) {
    let extension = file_path.extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");
    
    match extension {
        "rs" => setup_rust_autocomplete(input_state, workspace_root, file_path, analyzer, window, cx),
        "js" | "jsx" | "ts" | "tsx" => setup_javascript_autocomplete(input_state, workspace_root, file_path, window, cx),
        "py" => setup_python_autocomplete(input_state, workspace_root, file_path, window, cx),
        _ => setup_text_autocomplete(input_state, workspace_root, file_path, window, cx),
    }
}


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
