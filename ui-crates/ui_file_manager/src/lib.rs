//! File Manager UI
//!
//! File browser and management

mod file_manager_drawer;
pub mod window;

// Re-export main types
pub use file_manager_drawer::{FileManagerDrawer, FileSelected};
pub use window::FileManagerWindow;
