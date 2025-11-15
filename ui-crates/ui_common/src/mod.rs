//! Common UI Components Module
//!
//! This module contains truly shared UI components and utilities used throughout
//! the application. These components provide cross-cutting functionality that is
//! used by multiple windows and editors.
//!
//! ## Structure
//!
//! - `command_palette/` - Quick command access system (Ctrl+Shift+P)
//! - `menu/` - Application menu system
//! - `services/` - Background services (GPU, LSP, etc.)

pub mod command_palette;
pub mod menu;
pub mod services;

// Re-export commonly used services
pub use services::{GpuRenderer, GlobalRustAnalyzerCompletionProvider, RustAnalyzerManager};
