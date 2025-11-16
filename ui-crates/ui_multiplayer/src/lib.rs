//! Multiplayer UI
//!
//! Real-time collaboration and multiplayer features

mod chat;
mod connection;
mod file_sync;
mod presence;
mod session;
mod state;
mod traits;
mod types;
mod ui;
mod utils;

// Re-export main types
pub use state::MultiplayerWindow;
pub use types::*;
