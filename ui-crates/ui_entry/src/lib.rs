//! Entry Screen UI
//!
//! Project selection and startup screens

pub mod entry_screen;
pub mod loading_window;
pub mod window;

// Re-export main types
pub use window::EntryWindow;
pub use loading_window::{LoadingWindow, LoadingComplete};
pub use entry_screen::{EntryScreen, project_selector::ProjectSelected};
