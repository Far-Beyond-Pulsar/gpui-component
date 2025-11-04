//! Window Components
//!
//! This module contains all window types in the engine's UI:
//!
//! ## Main Windows
//!
//! - `editor/` - Main editor window with tabs and drawers
//!   - `tabs/` - All editor tabs including specialized editors
//!   - `drawers/` - File manager, terminal, and problems drawers
//! - `terminal/` - Integrated terminal emulator
//! - `entry_screen/` - Project selection and startup screens
//!
//! ## Utility Windows
//!
//! - `entry_window` - Entry window wrapper
//! - `loading_window` - Loading screen
//! - `settings/` - Settings screens
//! - `settings_window` - Settings window wrapper
//! - `file_manager_window` - File system browser
//! - `problems_window` - Error and warning display
//! - `terminal_window` - Standalone terminal window wrapper

pub mod editor;
pub mod entry_screen;
pub mod entry_window;
pub mod file_manager_window;
pub mod loading_window;
pub mod problems_window;
pub mod settings;
pub mod settings_window;
pub mod terminal;
pub mod terminal_window;

// Re-export main editor tabs
pub use editor::{
    BlueprintEditorPanel, DawEditorPanel, FileManagerDrawer, LevelEditorPanel, ProblemsDrawer,
    ScriptEditorPanel, TerminalDrawer, TextEditorEvent,
};

// Re-export window types
pub use entry_window::EntryWindow;
pub use file_manager_window::FileManagerWindow;
pub use loading_window::{LoadingComplete, LoadingWindow};
pub use problems_window::ProblemsWindow;
pub use settings_window::SettingsWindow;
pub use terminal_window::TerminalWindow;
