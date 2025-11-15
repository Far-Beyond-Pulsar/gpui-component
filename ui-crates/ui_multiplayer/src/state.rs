//! State management for the multiplayer window

use gpui::*;
use ui::input::InputState;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::*;
use pulsar_engine::ui::simple_sync::SyncDiff;
use pulsar_engine::ui::multiuser_client::MultiuserClient;

/// Multiplayer collaboration window for connecting to multiuser servers
pub struct MultiplayerWindow {
    pub(super) server_address_input: Entity<InputState>,
    pub(super) session_id_input: Entity<InputState>,
    pub(super) session_password_input: Entity<InputState>,
    pub(super) chat_input: Entity<InputState>,
    pub(super) connection_status: ConnectionStatus,
    pub(super) active_session: Option<ActiveSession>,
    pub(super) client: Option<Arc<RwLock<MultiuserClient>>>,
    pub(super) current_peer_id: Option<String>,
    pub(super) current_tab: SessionTab,
    pub(super) chat_messages: Vec<ChatMessage>,
    pub(super) file_assets: Vec<FileAssetStatus>, // Project assets with sync status
    pub(super) user_presences: Vec<UserPresence>, // Real-time user presence data
    pub(super) focus_handle: FocusHandle,
    // File sync state
    pub(super) project_root: Option<PathBuf>,
    pub(super) pending_file_sync: Option<(SyncDiff, String)>, // (diff, host_peer_id)
    pub(super) file_sync_in_progress: bool,
    pub(super) sync_progress_message: Option<String>,
    pub(super) sync_progress_percent: Option<f32>,
}

impl MultiplayerWindow {
    /// Create a new multiplayer window
        pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let server_address_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("ws://localhost:8080", window, cx);
            state
        });

        let session_id_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("Enter session ID", window, cx);
            state
        });

        let session_password_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("Enter session password", window, cx);
            state
        });

        let chat_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("Type a message...", window, cx);
            state
        });

        // Try to get project root from current directory
        let project_root = std::env::current_dir().ok();

        Self {
            server_address_input,
            session_id_input,
            session_password_input,
            chat_input,
            connection_status: ConnectionStatus::Disconnected,
            active_session: None,
            client: None,
            current_peer_id: None,
            current_tab: SessionTab::Info,
            chat_messages: Vec::new(),
            file_assets: Vec::new(),
            user_presences: Vec::new(),
            focus_handle: cx.focus_handle(),
            project_root,
            pending_file_sync: None,
            file_sync_in_progress: false,
            sync_progress_message: None,
            sync_progress_percent: None,
        }
    }

}
