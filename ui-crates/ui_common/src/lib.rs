//! Common UI Utilities
//!
//! Shared services, helpers, and utilities used across all UI components

pub mod command_palette;
pub mod diagnostics;
pub mod file_utils;
pub mod helpers;
pub mod menu;
pub mod services;

// Re-export commonly used types
pub use command_palette::CommandPalette;
pub use menu::AppTitleBar;
pub use services::{GpuRenderer, GlobalRustAnalyzerCompletionProvider, RustAnalyzerManager};
pub use file_utils::{FileInfo, FileType, find_openable_files};
pub use diagnostics::{Diagnostic, DiagnosticSeverity};
