//! UI Module
//!
//! The UI module contains all user interface components, organized by purpose:
//!
//! ## Structure
//!
//! ### Core (`core/`)
//!
//! Core application state and fundamental UI utilities:
//!
//! - Main application state (`PulsarApp`, `PulsarRoot`)
//! - File system utilities for UI operations
//! - Shared UI utilities and helpers
//!
//! ### Editors (`editors/`)
//!
//! Base editor components and shared editor functionality:
//!
//! - Base editor traits and types
//! - Common editor utilities
//! - Editor state management
//!
//! ### Windows (`windows/`)
//!
//! Complete window components that represent different parts of the application:
//!
//! - `editor/` - Main editor window with tabs and drawers
//!   - `tabs/` - Editor tabs (script, level, blueprint, DAW, specialized editors)
//!   - `drawers/` - Collapsible panels (file manager, terminal, problems)
//! - `terminal/` - Integrated terminal emulator
//! - `entry_screen/` - Project selection and startup with project selector
//! - `entry_window` - Entry window wrapper
//! - `loading_window` - Application loading screen
//! - `settings/` - Settings configuration screens
//! - `settings_window` - Settings window wrapper
//! - `file_manager_window` - File system browser window
//! - `problems_window` - Error and warning display window
//! - `terminal_window` - Standalone terminal wrapper
//!
//! ### Common (`common/`)
//!
//! Truly shared components used across multiple windows:
//!
//! - `command_palette/` - Quick command access (Ctrl+Shift+P)
//! - `menu/` - Application menu system
//! - `services/` - Background services (GPU rendering, LSP, etc.)
//!
//! ## Design Philosophy
//!
//! - **Core**: Fundamental application state and utilities
//! - **Editors**: Base editor functionality shared across editor types
//! - **Windows**: Self-contained window components (own their specific UI)
//! - **Common**: Cross-cutting concerns used by multiple windows
//! - **Clear hierarchy**: Windows use common/core/editors, not vice versa

pub mod common;
pub mod core;
pub mod editors;
pub mod git_sync;
pub mod multiuser_client;
pub mod p2p_connection;
pub mod simple_sync;
pub mod windows;

// Re-export core types for backward compatibility and convenience
pub use common::{GpuRenderer, GlobalRustAnalyzerCompletionProvider, RustAnalyzerManager};

pub use core::{PulsarApp, PulsarRoot, ToggleCommandPalette};

pub use windows::{
    BlueprintEditorPanel, DawEditorPanel, EntryWindow, FileManagerDrawer, FileManagerWindow,
    LevelEditorPanel, LoadingComplete, LoadingWindow, ProblemsDrawer, ProblemsWindow,
    ScriptEditorPanel, SettingsWindow, TerminalDrawer, TerminalWindow, TextEditorEvent,
};

