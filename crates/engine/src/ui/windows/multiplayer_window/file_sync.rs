//! File synchronization functionality

use gpui::*;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::state::MultiplayerWindow;
use super::types::*;
use crate::ui::multiuser_client::{ClientMessage, MultiuserClient};

impl MultiplayerWindow {
    pub(super) fn approve_file_sync(&mut self, cx: &mut Context<Self>) {
        if let Some((diff, host_peer_id)) = self.pending_file_sync.take() {
            tracing::info!("File sync approved - requesting {} files from {}",
                diff.added.len() + diff.modified.len(), host_peer_id);

            self.file_sync_in_progress = true;

            if let (Some(client), Some(session), Some(peer_id)) = (&self.client, &self.active_session, &self.current_peer_id) {
                let client = client.clone();
                let session_id = session.session_id.clone();
                let peer_id = peer_id.clone();

                // Request all files that need to be downloaded/updated
                let mut files_to_request = Vec::new();
                files_to_request.extend(diff.added.iter().map(|f| f.path.clone()));
                files_to_request.extend(diff.modified.iter().map(|f| f.path.clone()));

                cx.spawn(async move |this, mut cx| {
                    let client_guard = client.read().await;

                    for file_path in files_to_request {
                        let path_str = file_path.to_string_lossy().to_string();
                        tracing::info!("Requesting file: {}", path_str);

                        let _ = client_guard.send(ClientMessage::RequestFile {
                            session_id: session_id.clone(),
                            peer_id: peer_id.clone(),
                            file_path: path_str,
                        }).await;
                    }

                    // Mark sync as complete after requests sent
                    cx.update(|cx| {
                        this.update(cx, |this, cx| {
                            this.file_sync_in_progress = false;
                            cx.notify();
                        }).ok();
                    }).ok();
                }).detach();
            }

            self.pending_file_sync = None;
            cx.notify();
        }
    }


    pub(super) fn cancel_file_sync(&mut self, cx: &mut Context<Self>) {
        tracing::info!("File sync cancelled");
        self.pending_file_sync = None;
        cx.notify();
    }

}
