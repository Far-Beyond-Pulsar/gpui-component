//! Main Editor UI
//!
//! Main editor window with tabs and drawers

pub mod drawers;
pub mod editors;
pub mod tabs;

// Re-export editor window (from mod.rs)
mod editor_mod;
pub use editor_mod::*;

// Re-export commonly used editor types
pub use tabs::{
    ScriptEditorPanel, LevelEditorPanel, BlueprintEditorPanel, DawEditorPanel,
    TextEditorEvent,
};
pub use drawers::{
    FileManagerDrawer, TerminalDrawer, ProblemsDrawer,
    file_manager_drawer::{FileSelected, FileType as DrawerFileType, PopoutFileManagerEvent},
};
