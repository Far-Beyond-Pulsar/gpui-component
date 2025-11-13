//! Connection management for multiplayer sessions

use gpui::*;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::state::MultiplayerWindow;
use super::types::*;
use crate::ui::file_sync::ProjectTree;
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

                                            // Scan project tree (host only)
                                            if let Some(project_root) = &this.project_root {
                                                tracing::info!("CREATE_SESSION: Scanning project tree at {:?}", project_root);
                                                match ProjectTree::from_directory(project_root) {
                                                    Ok(tree) => {
                                                        tracing::info!("CREATE_SESSION: Scanned {} files", tree.files.len());
                                                        this.local_tree = Some(tree);
                                                    }
                                                    Err(e) => {
                                                        tracing::error!("CREATE_SESSION: Failed to scan project tree: {}", e);
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

                                                // Send our tree to the requesting peer
                                                let tree_result = cx.update(|cx| {
                                                    this.update(cx, |this, _cx| {
                                                        this.local_tree.clone()
                                                    }).ok()
                                                }).ok().flatten();

                                                if let Some(tree) = tree_result.flatten() {
                                                    if let Ok(tree_json) = serde_json::to_string(&tree) {
                                                        let our_peer_id_result = cx.update(|cx| {
                                                            this.update(cx, |this, _cx| {
                                                                this.current_peer_id.clone()
                                                            }).ok()
                                                        }).ok().flatten().flatten();

                                                        if let Some(our_peer_id) = our_peer_id_result {
                                                            let client_guard = client.read().await;
                                                            let _ = client_guard.send(ClientMessage::ProjectTreeResponse {
                                                                session_id: req_session_id,
                                                                peer_id: our_peer_id,
                                                                tree_json,
                                                            }).await;
                                                            tracing::info!("CREATE_SESSION: Sent ProjectTreeResponse");
                                                        }
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

                                    // Scan local project tree
                                    if let Some(project_root) = &this.project_root {
                                        tracing::info!("JOIN_SESSION: Scanning local project tree at {:?}", project_root);
                                        match ProjectTree::from_directory(project_root) {
                                            Ok(tree) => {
                                                tracing::info!("JOIN_SESSION: Scanned {} local files", tree.files.len());
                                                this.local_tree = Some(tree);
                                            }
                                            Err(e) => {
                                                tracing::error!("JOIN_SESSION: Failed to scan local tree: {}", e);
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
                                        tracing::info!("JOIN_SESSION: Received ProjectTreeResponse from {}", from_peer_id);

                                        // Parse remote tree
                                        if let Ok(remote_tree) = serde_json::from_str::<ProjectTree>(&tree_json) {
                                            tracing::info!("JOIN_SESSION: Remote tree has {} files", remote_tree.files.len());

                                            // Compute diff with local tree
                                            let diff_result = cx.update(|cx| {
                                                this.update(cx, |this, _cx| {
                                                    if let Some(local_tree) = &this.local_tree {
                                                        let diff = local_tree.diff(&remote_tree);
                                                        tracing::info!(
                                                            "JOIN_SESSION: Diff computed - {} added, {} modified, {} deleted",
                                                            diff.added.len(),
                                                            diff.modified.len(),
                                                            diff.deleted.len()
                                                        );
                                                        Some((diff, from_peer_id.clone()))
                                                    } else {
                                                        None
                                                    }
                                                }).ok()
                                            }).ok().flatten().flatten();

                                            if let Some((diff, host_id)) = diff_result {
                                                // Store pending file sync
                                                cx.update(|cx| {
                                                    this.update(cx, |this, cx| {
                                                        this.pending_file_sync = Some((diff, host_id));
                                                        this.current_tab = SessionTab::FileSync;
                                                        cx.notify();
                                                    }).ok();
                                                }).ok();
                                            }
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
                                    ServerMessage::RequestProjectTree { from_peer_id, session_id: req_session_id, .. } => {
                                        tracing::info!("JOIN_SESSION: Received RequestProjectTree from {}", from_peer_id);

                                        // Send our tree to the requesting peer
                                        let tree_result = cx.update(|cx| {
                                            this.update(cx, |this, _cx| {
                                                this.local_tree.clone()
                                            }).ok()
                                        }).ok().flatten();

                                        if let Some(tree) = tree_result.flatten() {
                                            if let Ok(tree_json) = serde_json::to_string(&tree) {
                                                let our_peer_id_result = cx.update(|cx| {
                                                    this.update(cx, |this, _cx| {
                                                        this.current_peer_id.clone()
                                                    }).ok()
                                                }).ok().flatten().flatten();

                                                if let Some(our_peer_id) = our_peer_id_result {
                                                    let client_guard = client.read().await;
                                                    let _ = client_guard.send(ClientMessage::ProjectTreeResponse {
                                                        session_id: req_session_id,
                                                        peer_id: our_peer_id,
                                                        tree_json,
                                                    }).await;
                                                }
                                            }
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
