//! Multiplayer UI
//!
//! Real-time collaboration and multiplayer features

mod connection;
mod mod_file;
mod session;
mod state;
mod traits;
mod types;
mod ui;
mod utils;

// Re-export main types
pub use mod_file::MultiplayerWindow;
pub use state::MultiplayerWindowState;
pub use types::*;
pub use traits::*;
