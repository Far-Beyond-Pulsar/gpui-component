//! Settings UI
//!
//! Application and project settings

pub mod settings;
pub mod window;

// Re-export main types
pub use window::SettingsWindow;
pub use settings::{SettingsScreen, SettingsScreenProps};
