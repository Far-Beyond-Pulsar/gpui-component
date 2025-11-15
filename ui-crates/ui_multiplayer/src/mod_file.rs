//! Multiplayer Window Module
//!
//! A well-organized module for managing multiplayer collaboration sessions.
//! This module provides functionality for:
//! - Creating and joining multiplayer sessions
//! - Real-time chat communication
//! - File synchronization across peers
//! - User presence tracking
//!
//! ## Module Structure
//!
//! - `types` - Type definitions (structs, enums)
//! - `state` - Main MultiplayerWindow state and constructor
//! - `connection` - Connection management (create/join/disconnect)
//! - `session` - Session management utilities
//! - `chat` - Chat messaging functionality
//! - `file_sync` - File synchronization logic
//! - `presence` - User presence tracking
//! - `ui` - UI rendering methods
//! - `traits` - Trait implementations (Focusable, Render)
//! - `utils` - Utility functions

// Module declarations
mod types;
mod state;
mod connection;
mod session;
mod chat;
mod file_sync;
mod presence;
mod ui;
mod traits;
mod utils;

// Re-export the main struct and public types
pub use state::MultiplayerWindow;
pub use types::{
    ActiveSession,
    ChatMessage,
    ConnectionStatus,
    FileAsset,
    FileAssetStatus,
    FileSyncStatus,
    SessionTab,
    UserPresence,
};
