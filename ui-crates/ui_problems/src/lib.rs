//! Problems UI
//!
//! Diagnostics and error display

mod problems_drawer;
pub mod window;

// Re-export main types
pub use problems_drawer::{ProblemsDrawer, Diagnostic, DiagnosticSeverity, NavigateToDiagnostic};
pub use window::ProblemsWindow;
