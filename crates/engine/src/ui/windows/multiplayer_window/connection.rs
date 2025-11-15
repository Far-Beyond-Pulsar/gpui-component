//! Connection management for multiplayer sessions

use gpui::*;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::state::MultiplayerWindow;
use super::types::*;
use crate::ui::git_sync;
use crate::ui::multiuser_client::{ClientMessage, MultiuserClient, ServerMessage};

impl MultiplayerWindow {
    pub(super) fn create_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let server_address = self.server_address_input.read(cx).text().to_string();

        if server_address.is_empty() {
            self.connection_status = ConnectionStatus::Error("Server address is required".to_string());
            cx.notify();
            return;
        }

        self.connection_status = ConnectionStatus::Connecting;
        cx.notify();

        // Create client
        let client = Arc::new(RwLock::new(MultiuserClient::new(server_address.clone())));
        self.client = Some(client.clone());

        let session_id_input = self.session_id_input.clone();
        let session_password_input = self.session_password_input.clone();

        cx.spawn(async move |this, mut cx| {
            // Call create_session on the client
            let result = {
                let client_guard = client.read().await;
                client_guard.create_session().await
            };

            match result {
                Ok((session_id, join_token)) => {
                    // Store credentials for later display
                    let session_id_for_display = session_id.clone();
                    let join_token_for_display = join_token.clone();

                    // Update UI state
                    cx.update(|cx| {
                        this.update(cx, |this, cx| {
                            this.connection_status = ConnectionStatus::Connected;
                            this.active_session = Some(ActiveSession {
                                session_id: session_id_for_display.clone(),
                                join_token: join_token_for_display.clone(),
                                server_address: server_address.clone(),
                                connected_users: vec!["You (Host)".to_string()],
                            });
                            cx.notify();
                        }).ok();
                    }).ok();

                    // Now connect via WebSocket
                    let event_rx_result = {
                        let mut client_guard = client.write().await;
                        client_guard.connect(session_id.clone(), join_token).await
                    }; // client_guard dropped here, releasing the lock

                    match event_rx_result {
                        Ok(mut event_rx) => {
                            // Wait for the initial response (Joined or Error)
                            match event_rx.recv().await {
                                Some(ServerMessage::Joined { peer_id, participants, .. }) => {
                                    cx.update(|cx| {
                                        this.update(cx, |this, cx| {
                                            // Store our peer_id first
                                            this.current_peer_id = Some(peer_id.clone());

                                            tracing::info!(
                                                "CREATE_SESSION: Received Joined - our peer_id: {}, participants: {:?}",
                                                peer_id,
                                                participants
                                            );

                                            if let Some(session) = &mut this.active_session {
                                                // Store raw participant list
                                                session.connected_users = participants.clone();
                                            }

                                            // Initialize git and commit current state (host only)
                                            if let Some(project_root) = &this.project_root {
                                                tracing::info!("CREATE_SESSION: Initializing git at {:?}", project_root);
                                                match git_sync::ensure_git_repo(project_root) {
                                                    Ok(repo) => {
                                                        // Commit current state
                                                        match git_sync::commit_current_state(&repo, "Multiplayer session start") {
                                                            Ok(commit_id) => {
                                                                let commit_hash = commit_id.to_string();
                                                                tracing::info!("CREATE_SESSION: Created commit {}", commit_hash);
                                                                this.local_commit = Some(commit_hash);
                                                            }
                                                            Err(e) => {
                                                                tracing::error!("CREATE_SESSION: Failed to create commit: {}", e);
                                                            }
                                                        }
                                                    }
                                                    Err(e) => {
                                                        tracing::error!("CREATE_SESSION: Failed to init git: {}", e);
                                                    }
                                                }
                                            }

                                            cx.notify();
                                        }).ok();
                                    }).ok();

                                    // Continue listening for PeerJoined/PeerLeft events
                                    while let Some(msg) = event_rx.recv().await {
                                        // Check if we're still connected - break if disconnected
                                        let still_connected = cx.update(|cx| {
                                            this.update(cx, |this, _cx| {
                                                this.active_session.is_some()
                                            }).unwrap_or(false)
                                        }).unwrap_or(false);

                                        if !still_connected {
                                            tracing::info!("CREATE_SESSION: Session disconnected, stopping event loop");
                                            break;
                                        }

                                        match msg {
                                            ServerMessage::PeerJoined { peer_id: joined_peer_id, .. } => {
                                                cx.update(|cx| {
                                                    this.update(cx, |this, cx| {
                                                        tracing::info!(
                                                            "CREATE_SESSION: Received PeerJoined - joined_peer_id: {}, our peer_id: {:?}",
                                                            joined_peer_id,
                                                            this.current_peer_id
                                                        );

                                                        // Ignore PeerJoined about ourselves
                                                        if this.current_peer_id.as_ref() == Some(&joined_peer_id) {
                                                            tracing::info!("CREATE_SESSION: Ignoring PeerJoined about ourselves");
                                                            return;
                                                        }

                                                        if let Some(session) = &mut this.active_session {
                                                            // Add raw peer_id if not already present
                                                            if !session.connected_users.contains(&joined_peer_id) {
                                                                tracing::info!(
                                                                    "CREATE_SESSION: Adding peer {} to list. List before: {:?}",
                                                                    joined_peer_id,
                                                                    session.connected_users
                                                                );
                                                                session.connected_users.push(joined_peer_id.clone());
                                                                cx.notify();
                                                            } else {
                                                                tracing::info!("CREATE_SESSION: Peer {} already in list", joined_peer_id);
                                                            }
                                                        }
                                                    }).ok();
                                                }).ok();
                                            }
                                            ServerMessage::PeerLeft { peer_id: left_peer_id, .. } => {
                                                cx.update(|cx| {
                                                    this.update(cx, |this, cx| {
                                                        if let Some(session) = &mut this.active_session {
                                                            session.connected_users.retain(|p| p != &left_peer_id);
                                                            cx.notify();
                                                        }
                                                    }).ok();
                                                }).ok();
                                            }
                                            ServerMessage::ChatMessage { peer_id: sender_peer_id, message, timestamp, .. } => {
                                                tracing::info!(
                                                    "CREATE_SESSION: Received ChatMessage from {} at {}: {}",
                                                    sender_peer_id, timestamp, message
                                                );
                                                cx.update(|cx| {
                                                    this.update(cx, |this, cx| {
                                                        let is_self = this.current_peer_id.as_ref() == Some(&sender_peer_id);
                                                        tracing::info!(
                                                            "CREATE_SESSION: Adding chat message. is_self: {}, current chat count: {}",
                                                            is_self, this.chat_messages.len()
                                                        );
                                                        this.chat_messages.push(ChatMessage {
                                                            peer_id: sender_peer_id,
                                                            message,
                                                            timestamp,
                                                            is_self,
                                                        });
                                                        tracing::info!("CREATE_SESSION: Chat messages now: {}", this.chat_messages.len());
                                                        cx.notify();
                                                    }).ok();
                                                }).ok();
                                            }
                                            ServerMessage::Error { message } => {
                                                cx.update(|cx| {
                                                    this.update(cx, |this, cx| {
                                                        this.connection_status = ConnectionStatus::Error(format!("Server error: {}", message));
                                                        cx.notify();
                                                    }).ok();
                                                }).ok();
                                            }
                                            ServerMessage::RequestProjectTree { from_peer_id, session_id: req_session_id, .. } => {
                                                tracing::info!("CREATE_SESSION: Received RequestProjectTree from {}", from_peer_id);

                                                // Send our commit hash to the requesting peer
                                                let commit_result = cx.update(|cx| {
                                                    this.update(cx, |this, _cx| {
                                                        this.local_commit.clone()
                                                    }).ok()
                                                }).ok().flatten();

                                                if let Some(commit_hash) = commit_result.flatten() {
                                                    let our_peer_id_result = cx.update(|cx| {
                                                        this.update(cx, |this, _cx| {
                                                            this.current_peer_id.clone()
                                                        }).ok()
                                                    }).ok().flatten().flatten();

                                                    if let Some(our_peer_id) = our_peer_id_result {
                                                        let client_guard = client.read().await;
                                                        // Send commit hash as "tree_json" for backwards compatibility
                                                        let _ = client_guard.send(ClientMessage::ProjectTreeResponse {
                                                            session_id: req_session_id,
                                                            peer_id: our_peer_id,
                                                            tree_json: commit_hash,
                                                        }).await;
                                                        tracing::info!("CREATE_SESSION: Sent commit hash");
                                                    }
                                                }
                                            }
                                            ServerMessage::RequestFile { from_peer_id, file_path, session_id: req_session_id, .. } => {
                                                tracing::info!("CREATE_SESSION: Received RequestFile for {} from {}", file_path, from_peer_id);

                                                // Read and send file in chunks
                                                let project_root_result = cx.update(|cx| {
                                                    this.update(cx, |this, _cx| {
                                                        this.project_root.clone()
                                                    }).ok()
                                                }).ok().flatten().flatten();

                                                if let Some(project_root) = project_root_result {
                                                    let full_path = project_root.join(&file_path);

                                                    if let Ok(data) = std::fs::read(&full_path) {
                                                        const CHUNK_SIZE: usize = 8192; // 8KB chunks

                                                        let our_peer_id_result = cx.update(|cx| {
                                                            this.update(cx, |this, _cx| {
                                                                this.current_peer_id.clone()
                                                            }).ok()
                                                        }).ok().flatten().flatten();

                                                        if let Some(our_peer_id) = our_peer_id_result {
                                                            for (i, chunk) in data.chunks(CHUNK_SIZE).enumerate() {
                                                                let offset = i * CHUNK_SIZE;
                                                                let is_last = offset + chunk.len() >= data.len();

                                                                let client_guard = client.read().await;
                                                                let _ = client_guard.send(ClientMessage::FileChunk {
                                                                    session_id: req_session_id.clone(),
                                                                    peer_id: our_peer_id.clone(),
                                                                    file_path: file_path.clone(),
                                                                    offset: offset as u64,
                                                                    data: chunk.to_vec(),
                                                                    is_last,
                                                                }).await;
                                                            }
                                                            tracing::info!("CREATE_SESSION: Sent file {}", file_path);
                                                        }
                                                    }
                                                }
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                                Some(ServerMessage::Error { message }) => {
                                    cx.update(|cx| {
                                        this.update(cx, |this, cx| {
                                            this.connection_status = ConnectionStatus::Error(format!("Server error: {}", message));
                                            this.active_session = None;
                                            cx.notify();
                                        }).ok();
                                    }).ok();
                                }
                                Some(_) => {
                                    cx.update(|cx| {
                                        this.update(cx, |this, cx| {
                                            this.connection_status = ConnectionStatus::Error("Unexpected server response".to_string());
                                            this.active_session = None;
                                            cx.notify();
                                        }).ok();
                                    }).ok();
                                }
                                None => {
                                    cx.update(|cx| {
                                        this.update(cx, |this, cx| {
                                            this.connection_status = ConnectionStatus::Error("Connection closed before response".to_string());
                                            this.active_session = None;
                                            cx.notify();
                                        }).ok();
                                    }).ok();
                                }
                            }
                        }
                        Err(e) => {
                            cx.update(|cx| {
                                this.update(cx, |this, cx| {
                                    this.connection_status = ConnectionStatus::Error(format!("Connection failed: {}", e));
                                    cx.notify();
                                }).ok();
                            }).ok();
                        }
                    }
                }
                Err(e) => {
                    cx.update(|cx| {
                        this.update(cx, |this, cx| {
                            this.connection_status = ConnectionStatus::Error(format!("Failed to create session: {}", e));
                            cx.notify();
                        }).ok();
                    }).ok();
                }
            }
        }).detach();
    }


    pub(super) fn join_session(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let server_address = self.server_address_input.read(cx).text().to_string();
        let session_id = self.session_id_input.read(cx).text().to_string();
        let join_token = self.session_password_input.read(cx).text().to_string();

        if server_address.is_empty() {
            self.connection_status = ConnectionStatus::Error("Server address is required".to_string());
            cx.notify();
            return;
        }

        if session_id.is_empty() || join_token.is_empty() {
            self.connection_status = ConnectionStatus::Error("Session ID and password are required".to_string());
            cx.notify();
            return;
        }

        self.connection_status = ConnectionStatus::Connecting;
        cx.notify();

        // Create client
        let client = Arc::new(RwLock::new(MultiuserClient::new(server_address.clone())));
        self.client = Some(client.clone());

        cx.spawn(async move |this, mut cx| {
            let join_token_clone = join_token.clone();

            let event_rx_result = {
                let mut client_guard = client.write().await;
                client_guard.connect(session_id.clone(), join_token_clone).await
            }; // client_guard dropped here, releasing the lock

            match event_rx_result {
                Ok(mut event_rx) => {
                    // Wait for the initial response (Joined or Error)
                    match event_rx.recv().await {
                        Some(ServerMessage::Joined { peer_id, participants, .. }) => {
                            cx.update(|cx| {
                                this.update(cx, |this, cx| {
                                    this.connection_status = ConnectionStatus::Connected;
                                    // Store our peer_id first
                                    this.current_peer_id = Some(peer_id.clone());

                                    tracing::info!(
                                        "JOIN_SESSION: Received Joined - our peer_id: {}, participants: {:?}",
                                        peer_id,
                                        participants
                                    );

                                    this.active_session = Some(ActiveSession {
                                        session_id: session_id.clone(),
                                        join_token: join_token.clone(),
                                        server_address: server_address.clone(),
                                        // Store raw participant list
                                        connected_users: participants.clone(),
                                    });

                                    // Initialize git and commit local state
                                    if let Some(project_root) = &this.project_root {
                                        tracing::info!("JOIN_SESSION: Initializing git at {:?}", project_root);
                                        match git_sync::ensure_git_repo(project_root) {
                                            Ok(repo) => {
                                                // Commit current state before syncing
                                                match git_sync::commit_current_state(&repo, "Before multiplayer sync") {
                                                    Ok(commit_id) => {
                                                        let commit_hash = commit_id.to_string();
                                                        tracing::info!("JOIN_SESSION: Created local commit {}", commit_hash);
                                                        this.local_commit = Some(commit_hash);
                                                    }
                                                    Err(e) => {
                                                        tracing::error!("JOIN_SESSION: Failed to create commit: {}", e);
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                tracing::error!("JOIN_SESSION: Failed to init git: {}", e);
                                            }
                                        }
                                    }

                                    cx.notify();
                                }).ok();
                            }).ok();

                            // Request project tree from host (first participant)
                            if let Some(host_peer_id) = participants.first() {
                                tracing::info!("JOIN_SESSION: Requesting project tree from host {}", host_peer_id);

                                let our_peer_id_result = cx.update(|cx| {
                                    this.update(cx, |this, _cx| {
                                        this.current_peer_id.clone()
                                    }).ok()
                                }).ok().flatten().flatten();

                                if let Some(our_peer_id) = our_peer_id_result {
                                    let client_guard = client.read().await;
                                    let _ = client_guard.send(ClientMessage::RequestProjectTree {
                                        session_id: session_id.clone(),
                                        peer_id: our_peer_id,
                                    }).await;
                                    tracing::info!("JOIN_SESSION: Sent RequestProjectTree");
                                }
                            }

                            // Continue listening for PeerJoined/PeerLeft events
                            while let Some(msg) = event_rx.recv().await {
                                // Check if we're still connected - break if disconnected
                                let still_connected = cx.update(|cx| {
                                    this.update(cx, |this, _cx| {
                                        this.active_session.is_some()
                                    }).unwrap_or(false)
                                }).unwrap_or(false);

                                if !still_connected {
                                    tracing::info!("JOIN_SESSION: Session disconnected, stopping event loop");
                                    break;
                                }

                                match msg {
                                    ServerMessage::PeerJoined { peer_id: joined_peer_id, .. } => {
                                        cx.update(|cx| {
                                            this.update(cx, |this, cx| {
                                                tracing::info!(
                                                    "JOIN_SESSION: Received PeerJoined - joined_peer_id: {}, our peer_id: {:?}",
                                                    joined_peer_id,
                                                    this.current_peer_id
                                                );

                                                // Ignore PeerJoined about ourselves
                                                if this.current_peer_id.as_ref() == Some(&joined_peer_id) {
                                                    tracing::info!("JOIN_SESSION: Ignoring PeerJoined about ourselves");
                                                    return;
                                                }

                                                if let Some(session) = &mut this.active_session {
                                                    // Add raw peer_id if not already present
                                                    if !session.connected_users.contains(&joined_peer_id) {
                                                        tracing::info!(
                                                            "JOIN_SESSION: Adding peer {} to list. List before: {:?}",
                                                            joined_peer_id,
                                                            session.connected_users
                                                        );
                                                        session.connected_users.push(joined_peer_id);
                                                        cx.notify();
                                                    } else {
                                                        tracing::info!("JOIN_SESSION: Peer {} already in list", joined_peer_id);
                                                    }
                                                }
                                            }).ok();
                                        }).ok();
                                    }
                                    ServerMessage::PeerLeft { peer_id: left_peer_id, .. } => {
                                        cx.update(|cx| {
                                            this.update(cx, |this, cx| {
                                                if let Some(session) = &mut this.active_session {
                                                    session.connected_users.retain(|p| p != &left_peer_id);
                                                    cx.notify();
                                                }
                                            }).ok();
                                        }).ok();
                                    }
                                    ServerMessage::ChatMessage { peer_id: sender_peer_id, message, timestamp, .. } => {
                                        tracing::info!(
                                            "JOIN_SESSION: Received ChatMessage from {} at {}: {}",
                                            sender_peer_id, timestamp, message
                                        );
                                        cx.update(|cx| {
                                            this.update(cx, |this, cx| {
                                                let is_self = this.current_peer_id.as_ref() == Some(&sender_peer_id);
                                                tracing::info!(
                                                    "JOIN_SESSION: Adding chat message. is_self: {}, current chat count: {}",
                                                    is_self, this.chat_messages.len()
                                                );
                                                this.chat_messages.push(ChatMessage {
                                                    peer_id: sender_peer_id,
                                                    message,
                                                    timestamp,
                                                    is_self,
                                                });
                                                tracing::info!("JOIN_SESSION: Chat messages now: {}", this.chat_messages.len());
                                                cx.notify();
                                            }).ok();
                                        }).ok();
                                    }
                                    ServerMessage::Error { message } => {
                                        cx.update(|cx| {
                                            this.update(cx, |this, cx| {
                                                this.connection_status = ConnectionStatus::Error(format!("Server error: {}", message));
                                                cx.notify();
                                            }).ok();
                                        }).ok();
                                    }
                                    ServerMessage::ProjectTreeResponse { from_peer_id, tree_json, .. } => {
                                        tracing::info!("JOIN_SESSION: Received commit hash from {}: {}", from_peer_id, tree_json);

                                        // tree_json now contains remote commit hash
                                        let remote_commit_hash = tree_json;

                                        // Check if we need to sync
                                        let needs_sync = cx.update(|cx| {
                                            this.update(cx, |this, _cx| {
                                                if let Some(local_commit) = &this.local_commit {
                                                    let differs = local_commit != &remote_commit_hash;
                                                    tracing::info!("JOIN_SESSION: Local commit: {}, Remote commit: {}, Differs: {}",
                                                        local_commit, remote_commit_hash, differs);
                                                    differs
                                                } else {
                                                    tracing::info!("JOIN_SESSION: No local commit, sync needed");
                                                    true
                                                }
                                            }).unwrap_or(true)
                                        }).unwrap_or(true);

                                        if needs_sync {
                                            // Create a git diff indicating sync is needed
                                            // Add placeholder to indicate we need to fetch the commit
                                            let diff = git_sync::GitDiff {
                                                changed_files: vec![
                                                    git_sync::ChangedFile {
                                                        path: format!("Syncing to commit {}", &remote_commit_hash[..8]),
                                                        size: 0,
                                                        status: git_sync::FileStatus::Modified,
                                                    }
                                                ],
                                                deleted_files: vec![],
                                                target_commit: remote_commit_hash.clone(),
                                            };

                                            tracing::info!("JOIN_SESSION: Commits differ, setting pending_file_sync");
                                            cx.update(|cx| {
                                                this.update(cx, |this, cx| {
                                                    this.pending_file_sync = Some((diff, from_peer_id.clone()));
                                                    this.current_tab = SessionTab::FileSync;
                                                    tracing::info!("JOIN_SESSION: pending_file_sync set, notifying");
                                                    cx.notify();
                                                }).ok();
                                            }).ok();
                                        } else {
                                            tracing::info!("JOIN_SESSION: Commits match, already in sync");
                                        }
                                    }
                                    ServerMessage::FileChunk { from_peer_id, file_path, offset, data, is_last, .. } => {
                                        tracing::info!("JOIN_SESSION: Received FileChunk for {} from {} (offset: {}, is_last: {})",
                                            file_path, from_peer_id, offset, is_last);

                                        // Write chunk to file
                                        let project_root_result = cx.update(|cx| {
                                            this.update(cx, |this, _cx| {
                                                this.project_root.clone()
                                            }).ok()
                                        }).ok().flatten().flatten();

                                        if let Some(project_root) = project_root_result {
                                            let full_path = project_root.join(&file_path);

                                            // Create parent directories if needed
                                            if let Some(parent) = full_path.parent() {
                                                let _ = std::fs::create_dir_all(parent);
                                            }

                                            // Write chunk
                                            use std::io::{Write, Seek, SeekFrom};
                                            if let Ok(mut file) = std::fs::OpenOptions::new()
                                                .create(true)
                                                .write(true)
                                                .open(&full_path)
                                            {
                                                let _ = file.seek(SeekFrom::Start(offset));
                                                let _ = file.write_all(&data);
                                            }

                                            if is_last {
                                                tracing::info!("JOIN_SESSION: Completed download of {}", file_path);
                                            }
                                        }
                                    }
                                    ServerMessage::GitObjectsChunk { from_peer_id, objects_json, chunk_index, total_chunks, .. } => {
                                        tracing::info!("JOIN_SESSION: Received GitObjectsChunk from {} (chunk {}/{})",
                                            from_peer_id, chunk_index + 1, total_chunks);

                                        // Set progress indicator - receiving data
                                        cx.update(|cx| {
                                            this.update(cx, |this, cx| {
                                                this.sync_progress_message = Some("Receiving files...".to_string());
                                                this.sync_progress_percent = Some(0.1);
                                                cx.notify();
                                            }).ok();
                                        }).ok();

                                        // For now, handle single chunk only
                                        // TODO: Handle multi-chunk transfers by buffering
                                        if chunk_index == 0 && total_chunks == 1 {
                                            let project_root_result = cx.update(|cx| {
                                                this.update(cx, |this, _cx| {
                                                    this.project_root.clone()
                                                }).ok()
                                            }).ok().flatten().flatten();

                                            if let Some(project_root) = project_root_result {
                                                // Update progress - starting sync
                                                cx.update(|cx| {
                                                    this.update(cx, |this, cx| {
                                                        this.sync_progress_message = Some("Processing files...".to_string());
                                                        this.sync_progress_percent = Some(0.2);
                                                        cx.notify();
                                                    }).ok();
                                                }).ok();

                                                // Clone data for blocking task
                                                let objects_json_clone = objects_json.clone();
                                                let project_root_clone = project_root.clone();

                                                // Update progress - extracting
                                                cx.update(|cx| {
                                                    this.update(cx, |this, cx| {
                                                        this.sync_progress_message = Some("Extracting files...".to_string());
                                                        this.sync_progress_percent = Some(0.5);
                                                        cx.notify();
                                                    }).ok();
                                                }).ok();

                                                // Do git operations in blocking task
                                                let result = tokio::task::spawn_blocking(move || -> Result<(String, usize), String> {
                                                    tracing::info!("SYNC_TASK: Starting git object processing in blocking thread");

                                                    match git_sync::ensure_git_repo(&project_root_clone) {
                                                        Ok(repo) => {
                                                            tracing::info!("SYNC_TASK: Opened git repository at {:?}", project_root_clone);

                                                            // Deserialize git objects
                                                            tracing::info!("SYNC_TASK: Deserializing git objects from JSON ({} bytes)", objects_json_clone.len());
                                                            match serde_json::from_str::<Vec<git_sync::GitObject>>(&objects_json_clone) {
                                                                Ok(git_objects) => {
                                                                    tracing::info!("SYNC_TASK: Successfully deserialized {} git objects", git_objects.len());

                                                                    // Find the commit hash
                                                                    let commit_hash = git_objects.iter()
                                                                        .find(|obj| obj.object_type == git_sync::GitObjectType::Commit)
                                                                        .map(|obj| obj.oid.clone());

                                                                    if let Some(commit_hash) = commit_hash {
                                                                        tracing::info!("SYNC_TASK: Found commit hash: {}", commit_hash);

                                                                        // Reconstruct git objects in the ODB
                                                                        tracing::info!("SYNC_TASK: Reconstructing git objects in ODB...");
                                                                        if let Err(e) = git_sync::reconstruct_objects(&repo, git_objects) {
                                                                            tracing::error!("SYNC_TASK: Failed to reconstruct git objects: {}", e);
                                                                        } else {
                                                                            tracing::info!("SYNC_TASK: Git objects reconstructed successfully");
                                                                        }

                                                                        // Extract files from the commit
                                                                        tracing::info!("SYNC_TASK: Extracting files from commit {}", commit_hash);
                                                                        match git_sync::extract_files_from_commit(&repo, &commit_hash) {
                                                                            Ok(files) => {
                                                                                tracing::info!("SYNC_TASK: Extracted {} files from commit", files.len());

                                                                                // Write files to working directory
                                                                                let mut written_count = 0;
                                                                                for (path, data) in &files {
                                                                                    let full_path = project_root_clone.join(path);
                                                                                    tracing::debug!("SYNC_TASK: Writing file {:?} ({} bytes)", full_path, data.len());

                                                                                    if let Some(parent) = full_path.parent() {
                                                                                        if let Err(e) = std::fs::create_dir_all(parent) {
                                                                                            tracing::error!("SYNC_TASK: Failed to create directory {:?}: {}", parent, e);
                                                                                            continue;
                                                                                        }
                                                                                    }

                                                                                    match std::fs::write(&full_path, data) {
                                                                                        Ok(_) => {
                                                                                            written_count += 1;
                                                                                            tracing::debug!("SYNC_TASK: Successfully wrote {:?}", full_path);
                                                                                        }
                                                                                        Err(e) => {
                                                                                            tracing::error!("SYNC_TASK: Failed to write file {:?}: {}", full_path, e);
                                                                                        }
                                                                                    }
                                                                                }

                                                                                tracing::info!("SYNC_TASK: Successfully wrote {}/{} files to disk", written_count, files.len());

                                                                                // Create a commit with the synced files
                                                                                tracing::info!("SYNC_TASK: Creating commit for synced files...");
                                                                                match git_sync::commit_current_state(&repo, "Synced from host") {
                                                                                    Ok(commit_id) => {
                                                                                        tracing::info!("SYNC_TASK: Created commit {} successfully", commit_id);
                                                                                        Ok((commit_id.to_string(), files.len()))
                                                                                    }
                                                                                    Err(e) => {
                                                                                        tracing::error!("Failed to create commit: {}", e);
                                                                                        Err(format!("Failed to create commit: {}", e))
                                                                                    }
                                                                                }
                                                                            }
                                                                            Err(e) => {
                                                                                tracing::error!("Failed to extract files: {}", e);
                                                                                Err(format!("Failed to extract files: {}", e))
                                                                            }
                                                                        }
                                                                    } else {
                                                                        tracing::error!("No commit found in git objects");
                                                                        Err("No commit found in git objects".to_string())
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    tracing::error!("Failed to deserialize git objects: {}", e);
                                                                    Err(format!("Failed to deserialize: {}", e))
                                                                }
                                                            }
                                                        }
                                                        Err(e) => {
                                                            tracing::error!("Failed to open git repo: {}", e);
                                                            Err(format!("Failed to open repo: {}", e))
                                                        }
                                                    }
                                                }).await;

                                                // Update progress - finalizing
                                                cx.update(|cx| {
                                                    this.update(cx, |this, cx| {
                                                        this.sync_progress_message = Some("Finalizing...".to_string());
                                                        this.sync_progress_percent = Some(0.9);
                                                        cx.notify();
                                                    }).ok();
                                                }).ok();

                                                // Update UI based on result
                                                match result {
                                                    Ok(Ok((new_commit, file_count))) => {
                                                        tracing::info!("SYNC_SUCCESS: Synced {} files, new commit: {}", file_count, new_commit);

                                                        // Clear sync state and show success
                                                        cx.update(|cx| {
                                                            this.update(cx, |this, cx| {
                                                                tracing::info!("SYNC_SUCCESS: Updating UI state - clearing progress and pending sync");
                                                                this.local_commit = Some(new_commit.clone());
                                                                this.file_sync_in_progress = false;
                                                                this.pending_file_sync = None;
                                                                this.sync_progress_message = None;
                                                                this.sync_progress_percent = None;

                                                                tracing::info!("SYNC_SUCCESS: State updated - file_sync_in_progress={}, pending_file_sync={:?}",
                                                                    this.file_sync_in_progress, this.pending_file_sync.is_some());
                                                                cx.notify();
                                                            }).ok();
                                                        }).ok();

                                                        tracing::info!("SYNC_SUCCESS: File synchronization complete!");
                                                    }
                                                    Ok(Err(err)) => {
                                                        tracing::error!("SYNC_ERROR: Sync failed with error: {}", err);
                                                        cx.update(|cx| {
                                                            this.update(cx, |this, cx| {
                                                                this.file_sync_in_progress = false;
                                                                this.pending_file_sync = None;
                                                                this.sync_progress_message = Some(format!("Sync failed: {}", err));
                                                                this.sync_progress_percent = None;
                                                                cx.notify();
                                                            }).ok();
                                                        }).ok();
                                                    }
                                                    Err(e) => {
                                                        tracing::error!("SYNC_ERROR: Sync task panicked: {}", e);
                                                        cx.update(|cx| {
                                                            this.update(cx, |this, cx| {
                                                                this.file_sync_in_progress = false;
                                                                this.pending_file_sync = None;
                                                                this.sync_progress_message = Some("Sync task failed".to_string());
                                                                this.sync_progress_percent = None;
                                                                cx.notify();
                                                            }).ok();
                                                        }).ok();
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    ServerMessage::RequestProjectTree { from_peer_id, session_id: req_session_id, .. } => {
                                        tracing::info!("JOIN_SESSION: Received RequestProjectTree from {}", from_peer_id);

                                        // Send our commit hash to the requesting peer
                                        let commit_result = cx.update(|cx| {
                                            this.update(cx, |this, _cx| {
                                                this.local_commit.clone()
                                            }).ok()
                                        }).ok().flatten();

                                        if let Some(commit_hash) = commit_result.flatten() {
                                            let our_peer_id_result = cx.update(|cx| {
                                                this.update(cx, |this, _cx| {
                                                    this.current_peer_id.clone()
                                                }).ok()
                                            }).ok().flatten().flatten();

                                            if let Some(our_peer_id) = our_peer_id_result {
                                                let client_guard = client.read().await;
                                                // Send commit hash as "tree_json" for backwards compatibility
                                                let _ = client_guard.send(ClientMessage::ProjectTreeResponse {
                                                    session_id: req_session_id,
                                                    peer_id: our_peer_id,
                                                    tree_json: commit_hash,
                                                }).await;
                                                tracing::info!("JOIN_SESSION: Sent commit hash");
                                            }
                                        }
                                    }
                                    ServerMessage::RequestGitObjects { from_peer_id, commit_hash, session_id: req_session_id, .. } => {
                                        tracing::info!("JOIN_SESSION: Received RequestGitObjects for commit {} from {}", commit_hash, from_peer_id);

                                        // Serialize the requested commit and send git objects
                                        let project_root_result = cx.update(|cx| {
                                            this.update(cx, |this, _cx| {
                                                this.project_root.clone()
                                            }).ok()
                                        }).ok().flatten().flatten();

                                        if let Some(project_root) = project_root_result {
                                            let commit_hash_for_blocking = commit_hash.clone();
                                            let commit_hash_for_log = commit_hash.clone();
                                            let session_id_clone = req_session_id.clone();
                                            let client_clone = client.clone();

                                            // Get peer_id before spawn
                                            let our_peer_id = cx.update(|cx| {
                                                this.update(cx, |this, _cx| {
                                                    this.current_peer_id.clone()
                                                }).ok()
                                            }).ok().flatten().flatten();

                                            // Do git operations in spawn_blocking to avoid freezing
                                            tokio::spawn(async move {
                                                let result = tokio::task::spawn_blocking(move || {
                                                    tracing::info!("HOST: Serializing commit in spawn_blocking");
                                                    match git_sync::ensure_git_repo(&project_root) {
                                                        Ok(repo) => {
                                                            git_sync::serialize_commit(&repo, &commit_hash_for_blocking)
                                                                .map_err(|e| format!("Serialize error: {}", e))
                                                        }
                                                        Err(e) => Err(format!("Git repo error: {}", e)),
                                                    }
                                                }).await;

                                                match result {
                                                    Ok(Ok(git_objects)) => {
                                                        tracing::info!("HOST: Serialized {} objects, converting to JSON", git_objects.len());
                                                        match serde_json::to_string(&git_objects) {
                                                            Ok(objects_json) => {
                                                                if let Some(peer_id) = our_peer_id {
                                                                    let client_guard = client_clone.read().await;
                                                                    let _ = client_guard.send(ClientMessage::GitObjectsChunk {
                                                                        session_id: session_id_clone,
                                                                        peer_id,
                                                                        objects_json,
                                                                        chunk_index: 0,
                                                                        total_chunks: 1,
                                                                    }).await;
                                                                    tracing::info!("HOST: Sent git objects for commit {}", commit_hash_for_log);
                                                                }
                                                            }
                                                            Err(e) => tracing::error!("HOST: Failed to serialize to JSON: {}", e),
                                                        }
                                                    }
                                                    Ok(Err(e)) => tracing::error!("HOST: Git operation failed: {}", e),
                                                    Err(e) => tracing::error!("HOST: Spawn blocking panicked: {}", e),
                                                }
                                            });
                                        }
                                    }
                                    ServerMessage::RequestFile { from_peer_id, file_path, session_id: req_session_id, .. } => {
                                        tracing::info!("JOIN_SESSION: Received RequestFile for {} from {}", file_path, from_peer_id);

                                        // Read and send file in chunks
                                        let project_root_result = cx.update(|cx| {
                                            this.update(cx, |this, _cx| {
                                                this.project_root.clone()
                                            }).ok()
                                        }).ok().flatten().flatten();

                                        if let Some(project_root) = project_root_result {
                                            let full_path = project_root.join(&file_path);

                                            if let Ok(data) = std::fs::read(&full_path) {
                                                const CHUNK_SIZE: usize = 8192;

                                                let our_peer_id_result = cx.update(|cx| {
                                                    this.update(cx, |this, _cx| {
                                                        this.current_peer_id.clone()
                                                    }).ok()
                                                }).ok().flatten().flatten();

                                                if let Some(our_peer_id) = our_peer_id_result {
                                                    for (i, chunk) in data.chunks(CHUNK_SIZE).enumerate() {
                                                        let offset = i * CHUNK_SIZE;
                                                        let is_last = offset + chunk.len() >= data.len();

                                                        let client_guard = client.read().await;
                                                        let _ = client_guard.send(ClientMessage::FileChunk {
                                                            session_id: req_session_id.clone(),
                                                            peer_id: our_peer_id.clone(),
                                                            file_path: file_path.clone(),
                                                            offset: offset as u64,
                                                            data: chunk.to_vec(),
                                                            is_last,
                                                        }).await;
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Some(ServerMessage::Error { message }) => {
                            cx.update(|cx| {
                                this.update(cx, |this, cx| {
                                    this.connection_status = ConnectionStatus::Error(format!("Server error: {}", message));
                                    cx.notify();
                                }).ok();
                            }).ok();
                        }
                        Some(_) => {
                            cx.update(|cx| {
                                this.update(cx, |this, cx| {
                                    this.connection_status = ConnectionStatus::Error("Unexpected server response".to_string());
                                    cx.notify();
                                }).ok();
                            }).ok();
                        }
                        None => {
                            cx.update(|cx| {
                                this.update(cx, |this, cx| {
                                    this.connection_status = ConnectionStatus::Error("Connection closed before response".to_string());
                                    cx.notify();
                                }).ok();
                            }).ok();
                        }
                    }
                }
                Err(e) => {
                    cx.update(|cx| {
                        this.update(cx, |this, cx| {
                            this.connection_status = ConnectionStatus::Error(format!("Connection failed: {}", e));
                            cx.notify();
                        }).ok();
                    }).ok();
                }
            }
        }).detach();
    }


    pub(super) fn disconnect(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let (Some(client), Some(session)) = (&self.client, &self.active_session) {
            let client = client.clone();
            let session_id = session.session_id.clone();
            let peer_id = self.current_peer_id.clone().unwrap_or_default();

            cx.spawn(async move |this, mut cx| {
                let mut client_guard = client.write().await;
                let _ = client_guard.disconnect(session_id, peer_id).await;

                cx.update(|cx| {
                    this.update(cx, |this, cx| {
                        this.connection_status = ConnectionStatus::Disconnected;
                        this.active_session = None;
                        this.client = None;
                        this.current_peer_id = None;
                        this.chat_messages.clear();
                        this.current_tab = SessionTab::Info;
                        cx.notify();
                    }).ok();
                }).ok();
            }).detach();
        } else {
            self.connection_status = ConnectionStatus::Disconnected;
            self.active_session = None;
            self.client = None;
            self.current_peer_id = None;
            self.chat_messages.clear();
            self.current_tab = SessionTab::Info;
            cx.notify();
        }
    }

}
