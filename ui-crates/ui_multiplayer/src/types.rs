//! Type definitions for the multiplayer window

#[derive(Clone, Debug, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum SessionTab {
    Info,
    Chat,
    FileSync,
    Presence, // Who's editing what - VSCode LiveShare style
}

#[derive(Clone, Debug)]
pub struct ActiveSession {
    pub session_id: String,
    pub join_token: String,
    pub server_address: String,
    pub connected_users: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub peer_id: String,
    pub message: String,
    pub timestamp: u64,
    pub is_self: bool,
}

#[derive(Clone, Debug)]
pub struct FileAsset {
    pub path: String,
    pub hash: String, // SHA-256 hash for verification
    pub size: u64,
    pub last_modified: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FileSyncStatus {
    Synced,
    OutOfSync,
    Missing,
    Checking,
}

#[derive(Clone, Debug)]
pub struct FileAssetStatus {
    pub asset: FileAsset,
    pub status: FileSyncStatus,
    pub peers_with_file: Vec<String>, // Which peers have this file
}

#[derive(Clone, Debug)]
pub struct UserPresence {
    pub peer_id: String,
    pub editing_file: Option<String>, // What file they're editing
    pub selected_object: Option<String>, // What object they have selected in scene
    pub cursor_position: Option<(f32, f32, f32)>, // 3D cursor position in scene
    pub color: [f32; 3], // RGB color to identify this user
}
