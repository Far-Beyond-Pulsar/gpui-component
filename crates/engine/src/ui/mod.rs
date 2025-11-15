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

//! UI Module
//! 
//! This module re-exports all UI components from the ui-crates.
//! UI implementations are in separate crates under ui-crates/ for modularity.

// Re-export all UI crates
pub use ui_common as common;
pub use ui_core as core;

// Re-export core types for convenience
pub use ui_common::{GpuRenderer, GlobalRustAnalyzerCompletionProvider, RustAnalyzerManager, CommandPalette, Menu, AppTitleBar};
pub use ui_core::{PulsarApp, PulsarRoot, ToggleCommandPalette, ToggleFileManager, ToggleProblems, ToggleTerminal};

// Re-export windows
pub use ui_entry::{EntryWindow, LoadingWindow, LoadingComplete};
pub use ui_editor::{
    ScriptEditorPanel, LevelEditorPanel, BlueprintEditorPanel, DawEditorPanel,
    FileManagerDrawer, TerminalDrawer, ProblemsDrawer,
    TextEditorEvent,
};
pub use ui_settings::SettingsWindow;
pub use ui_terminal::TerminalWindow;
pub use ui_multiplayer::MultiplayerWindow;
pub use ui_problems::ProblemsWindow;
pub use ui_file_manager::FileManagerWindow;

// Backend networking (moved to engine_backend - re-exported for compatibility)
pub mod multiuser_client {
    pub use engine_backend::subsystems::networking::multiuser::*;
}
pub mod p2p_connection {
    pub use engine_backend::subsystems::networking::p2p::*;
}
pub mod git_sync {
    pub use engine_backend::subsystems::networking::git_sync::*;
}
pub mod simple_sync {
    pub use engine_backend::subsystems::networking::simple_sync::*;
}

// Re-export editor stuff that other code expects
pub mod windows {
    pub use ui_entry::{EntryWindow, LoadingWindow, LoadingComplete};
    pub use ui_editor as editor;
    pub use ui_editor::{
        ScriptEditorPanel, LevelEditorPanel, BlueprintEditorPanel, DawEditorPanel,
        FileManagerDrawer, TerminalDrawer, ProblemsDrawer,
    };
    pub use ui_settings::SettingsWindow;
    pub use ui_terminal::TerminalWindow;
    pub use ui_multiplayer::MultiplayerWindow;
    pub use ui_problems::ProblemsWindow;
    pub use ui_file_manager::FileManagerWindow;
    
    // Re-export entry_screen for compatibility
    pub mod entry_screen {
        pub use ui_entry::entry_screen::*;
    }
}

pub use windows::{
    BlueprintEditorPanel, DawEditorPanel, EntryWindow, FileManagerDrawer, FileManagerWindow,
    LevelEditorPanel, LoadingComplete, LoadingWindow, ProblemsDrawer, ProblemsWindow,
    ScriptEditorPanel, SettingsWindow, TerminalDrawer, TerminalWindow, TextEditorEvent,
};

