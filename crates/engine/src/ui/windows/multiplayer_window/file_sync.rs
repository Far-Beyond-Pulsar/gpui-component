//! Git-based file synchronization functionality

use gpui::*;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::state::MultiplayerWindow;
use super::types::*;
use crate::ui::git_sync;
use crate::ui::multiuser_client::{ClientMessage, MultiuserClient};

impl MultiplayerWindow {
    pub(super) fn approve_file_sync(&mut self, cx: &mut Context<Self>) {
        if let Some((diff, host_peer_id)) = self.pending_file_sync.take() {
            tracing::info!(
                "Git sync approved - requesting objects for commit {} from {}",
                diff.target_commit,
                host_peer_id
            );

            self.file_sync_in_progress = true;

            // Request git objects from host
            if let (Some(client), Some(session), Some(peer_id)) =
                (&self.client, &self.active_session, &self.current_peer_id)
            {
                let client = client.clone();
                let session_id = session.session_id.clone();
                let peer_id = peer_id.clone();
                let commit_hash = diff.target_commit.clone();

                cx.spawn(async move |this, mut cx| {
                    tracing::info!("Sending RequestGitObjects for commit {}", commit_hash);

                    let client_guard = client.read().await;
                    let _ = client_guard.send(ClientMessage::RequestGitObjects {
                        session_id,
                        peer_id,
                        commit_hash: commit_hash.clone(),
                    }).await;

                    tracing::info!("Git objects request sent, waiting for response...");
                }).detach();
            }

            self.pending_file_sync = None;
            cx.notify();
        }
    }

    pub(super) fn cancel_file_sync(&mut self, cx: &mut Context<Self>) {
        tracing::info!("Git sync cancelled");
        self.pending_file_sync = None;
        cx.notify();
    }
}
