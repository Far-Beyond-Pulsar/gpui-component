//! Editor Tabs Module
//!
//! This module contains all the specialized editor tabs that can be opened
//! in the main editor window. Each tab provides a complete editing experience
//! for a specific type of content.
//!
//! ## Main Editor Tabs
//!
//! - `script_editor` - Code and text editing with LSP support
//! - `level_editor` - 3D scene editing and level design
//! - `blueprint_editor` - Visual scripting editor
//! - `daw_editor` - Digital Audio Workstation for sound design
//!
//! ## Specialized Editors
//!
//! - `specialized_editors` - Asset-specific editors (materials, animations, particles, etc.)

pub mod blueprint_editor;
pub mod daw_editor;
pub mod level_editor;
pub mod script_editor;
pub mod specialized_editors;

// Re-exports for convenience
pub use blueprint_editor::BlueprintEditorPanel;
pub use daw_editor::DawEditorPanel;
pub use level_editor::LevelEditorPanel;
pub use script_editor::{ScriptEditor as ScriptEditorPanel, TextEditorEvent};
