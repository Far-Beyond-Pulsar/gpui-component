use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::Button,
    h_flex, v_flex,
    input::{InputState, TextInput},
    tab::{Tab, TabBar},
    ActiveTheme as _, Disableable as _, Icon, IconName, Sizable as _, StyledExt, TitleBar,
};
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::ui::multiuser_client::{MultiuserClient, ServerMessage};

/// Format a Unix timestamp as HH:MM
fn format_timestamp(timestamp: u64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let diff = if now > timestamp { now - timestamp } else { 0 };

    if diff < 60 {
        "Just now".to_string()
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

/// Multiplayer collaboration window for connecting to multiuser servers
pub struct MultiplayerWindow {
    server_address_input: Entity<InputState>,
    session_id_input: Entity<InputState>,
    session_password_input: Entity<InputState>,
    chat_input: Entity<InputState>,
    connection_status: ConnectionStatus,
    active_session: Option<ActiveSession>,
    client: Option<Arc<RwLock<MultiuserClient>>>,
    current_peer_id: Option<String>,
    current_tab: SessionTab,
    chat_messages: Vec<ChatMessage>,
    file_assets: Vec<FileAssetStatus>, // Project assets with sync status
    user_presences: Vec<UserPresence>, // Real-time user presence data
    focus_handle: FocusHandle,
}

#[derive(Clone, Debug, PartialEq)]
enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
enum SessionTab {
    Info,
    Chat,
    FileSync,
    Presence, // Who's editing what - VSCode LiveShare style
}

#[derive(Clone, Debug)]
struct ActiveSession {
    session_id: String,
    join_token: String,
    server_address: String,
    connected_users: Vec<String>,
}

#[derive(Clone, Debug)]
struct ChatMessage {
    peer_id: String,
    message: String,
    timestamp: u64,
    is_self: bool,
}

#[derive(Clone, Debug)]
struct FileAsset {
    path: String,
    hash: String, // SHA-256 hash for verification
    size: u64,
    last_modified: u64,
}

#[derive(Clone, Debug, PartialEq)]
enum FileSyncStatus {
    Synced,
    OutOfSync,
    Missing,
    Checking,
}

#[derive(Clone, Debug)]
struct FileAssetStatus {
    asset: FileAsset,
    status: FileSyncStatus,
    peers_with_file: Vec<String>, // Which peers have this file
}

#[derive(Clone, Debug)]
struct UserPresence {
    peer_id: String,
    editing_file: Option<String>, // What file they're editing
    selected_object: Option<String>, // What object they have selected in scene
    cursor_position: Option<(f32, f32, f32)>, // 3D cursor position in scene
    color: [f32; 3], // RGB color to identify this user
}

impl MultiplayerWindow {
    /// Format participant list for display, replacing our peer_id with "You" or "You (Host)"
    fn format_participants(&self, participants: &[String]) -> Vec<String> {
        let our_peer_id = match &self.current_peer_id {
            Some(id) => id,
            None => return participants.to_vec(),
        };

        let is_host = self.active_session.as_ref()
            .map(|s| participants.first() == Some(our_peer_id))
            .unwrap_or(false);

        participants.iter().map(|p| {
            if p == our_peer_id {
                if is_host {
                    "You (Host)".to_string()
                } else {
                    "You".to_string()
                }
            } else {
                p.clone()
            }
        }).collect()
    }

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
        }
    }

    fn create_session(&mut self, window: &mut Window, cx: &mut Context<Self>) {
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

    fn join_session(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
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

    fn disconnect(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
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

    fn send_chat_message(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let message = self.chat_input.read(cx).text().to_string();
        if message.trim().is_empty() {
            tracing::warn!("Attempted to send empty chat message");
            return;
        }

        tracing::info!("Preparing to send chat message: {}", message);

        if let (Some(client), Some(session), Some(peer_id)) = (&self.client, &self.active_session, &self.current_peer_id) {
            let client = client.clone();
            let session_id = session.session_id.clone();
            let peer_id = peer_id.clone();
            let message_to_send = message.clone();

            tracing::info!("Sending chat message from peer {} in session {}", peer_id, session_id);

            // Clear input immediately
            self.chat_input.update(cx, |state, cx| {
                state.set_value("".to_string(), window, cx);
            });

            cx.spawn(async move |_this, _cx| {
                use crate::ui::multiuser_client::ClientMessage;

                let client_guard = client.read().await;
                match client_guard.send(ClientMessage::ChatMessage {
                    session_id: session_id.clone(),
                    peer_id: peer_id.clone(),
                    message: message_to_send.clone(),
                }).await {
                    Ok(_) => {
                        tracing::info!("Successfully sent chat message to server");
                    }
                    Err(e) => {
                        tracing::error!("Failed to send chat message: {}", e);
                    }
                }
            }).detach();
        } else {
            tracing::error!("Cannot send chat message - client, session, or peer_id is None");
        }
    }

    fn render_connection_form(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .p_4()
            .child(
                // Header
                h_flex()
                    .items_center()
                    .gap_2()
                    .child(
                        Icon::new(IconName::User)
                            .size(px(24.))
                            .text_color(cx.theme().primary)
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_bold()
                            .text_color(cx.theme().foreground)
                            .child("Multiplayer Collaboration")
                    )
            )
            .child(
                // Server Address Input
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .text_color(cx.theme().foreground)
                            .child("Server Address")
                    )
                    .child(
                        TextInput::new(&self.server_address_input)
                    )
            )
            .child(
                // Create or Join Section
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .font_bold()
                            .text_color(cx.theme().muted_foreground)
                            .child("CREATE NEW SESSION")
                    )
                    .child(
                        Button::new("create-session")
                            .label("Create New Session")
                            .icon(IconName::Plus)
                            .w_full()
                            .disabled(self.server_address_input.read(cx).text().to_string().is_empty())
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.create_session(window, cx);
                            }))
                    )
            )
            .child(
                // Divider
                h_flex()
                    .items_center()
                    .gap_3()
                    .child(div().flex_1().h(px(1.)).bg(cx.theme().border))
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("OR")
                    )
                    .child(div().flex_1().h(px(1.)).bg(cx.theme().border))
            )
            .child(
                // Join Session Section
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .text_sm()
                            .font_bold()
                            .text_color(cx.theme().muted_foreground)
                            .child("JOIN EXISTING SESSION")
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child("Session ID")
                            )
                            .child(
                                TextInput::new(&self.session_id_input)
                            )
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child("Password")
                            )
                            .child(
                                TextInput::new(&self.session_password_input)
                            )
                    )
                    .child(
                        Button::new("join-session")
                            .label("Join Session")
                            .icon(IconName::LogIn)
                            .w_full()
                            .disabled(
                                self.server_address_input.read(cx).text().to_string().is_empty() ||
                                self.session_id_input.read(cx).text().to_string().is_empty() ||
                                self.session_password_input.read(cx).text().to_string().is_empty()
                            )
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.join_session(window, cx);
                            }))
                    )
            )
            .when_some(
                match &self.connection_status {
                    ConnectionStatus::Error(msg) => Some(msg.clone()),
                    _ => None,
                },
                |this, error_msg| {
                    this.child(
                        div()
                            .p_3()
                            .rounded(px(6.))
                            .bg(cx.theme().danger.opacity(0.1))
                            .border_1()
                            .border_color(cx.theme().danger)
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        Icon::new(IconName::TriangleAlert)
                                            .size(px(16.))
                                            .text_color(cx.theme().danger)
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().danger)
                                            .child(error_msg)
                                    )
                            )
                    )
                }
            )
    }

    fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_index = match self.current_tab {
            SessionTab::Info => 0,
            SessionTab::Presence => 1,
            SessionTab::FileSync => 2,
            SessionTab::Chat => 3,
        };

        let out_of_sync_count = self.file_assets.iter()
            .filter(|a| a.status != FileSyncStatus::Synced)
            .count();

        TabBar::new("multiplayer-tabs")
            .w_full()
            .bg(cx.theme().secondary)
            .border_b_1()
            .border_color(cx.theme().border)
            .selected_index(selected_index)
            .on_click(cx.listener(|this, ix: &usize, _window, cx| {
                this.current_tab = match ix {
                    0 => SessionTab::Info,
                    1 => SessionTab::Presence,
                    2 => SessionTab::FileSync,
                    3 => SessionTab::Chat,
                    _ => SessionTab::Info,
                };
                cx.notify();
            }))
            .child(Tab::new("Info"))
            .child(Tab::new("Presence"))
            .child(Tab::new(if out_of_sync_count > 0 {
                format!("Files ({})", out_of_sync_count)
            } else {
                "Files".to_string()
            }))
            .child(Tab::new(if !self.chat_messages.is_empty() {
                format!("Chat ({})", self.chat_messages.len())
            } else {
                "Chat".to_string()
            }))
    }

    fn render_session_info_tab(&self, session: &ActiveSession, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .p_4()
            .child(
                // Credentials
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("SESSION ID")
                    )
                    .child(
                        div()
                            .text_sm()
                            
                            .text_color(cx.theme().foreground)
                            .child(session.session_id.clone())
                    )
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("PASSWORD")
                    )
                    .child(
                        div()
                            .text_sm()
                            
                            .text_color(cx.theme().foreground)
                            .child(session.join_token.clone())
                    )
            )
            .child(
                div()
                    .h(px(1.))
                    .w_full()
                    .bg(cx.theme().border)
            )
            .child(
                // Users list
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{} CONNECTED", session.connected_users.len()))
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .children(
                                self.format_participants(&session.connected_users).iter().map(|user| {
                                    div()
                                        .px_2()
                                        .py_1()
                                        .text_sm()
                                        .text_color(cx.theme().foreground)
                                        .child(user.clone())
                                        .into_any_element()
                                })
                            )
                    )
            )
    }

    fn render_chat_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                // Messages
                div()
                    .flex_1()
                    .p_4()
                    .child(
                        v_flex()
                            .gap_3()
                            .when(self.chat_messages.is_empty(), |this| {
                                this.child(
                                    div()
                                        .text_sm()
                                        .text_center()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("No messages")
                                )
                            })
                            .children(
                                self.chat_messages.iter().map(|msg| {
                                    let peer_name = if msg.is_self { "You".to_string() } else { msg.peer_id.clone() };
                                    v_flex()
                                        .gap_0p5()
                                        .when(msg.is_self, |this| this.items_end())
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(peer_name)
                                        )
                                        .child(
                                            div()
                                                .px_3()
                                                .py_2()
                                                .rounded(px(6.))
                                                .bg(if msg.is_self {
                                                    cx.theme().accent
                                                } else {
                                                    cx.theme().secondary
                                                })
                                                .text_sm()
                                                .text_color(if msg.is_self {
                                                    cx.theme().accent_foreground
                                                } else {
                                                    cx.theme().foreground
                                                })
                                                .child(msg.message.clone())
                                        )
                                        .into_any_element()
                                })
                            )
                    )
            )
            .child(
                // Input
                h_flex()
                    .gap_2()
                    .p_4()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(TextInput::new(&self.chat_input).flex_1())
                    .child(
                        Button::new("send")
                            .label("Send")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.send_chat_message(window, cx);
                            }))
                    )
            )
    }

    fn render_file_sync_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                // Toolbar
                h_flex()
                    .px_4()
                    .py_2()
                    .items_center()
                    .justify_between()
                    .bg(cx.theme().secondary)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{} assets", self.file_assets.len()))
                    )
                    .child(
                        Button::new("scan")
                            .label("Scan")
                            .xsmall()
                            .on_click(cx.listener(|_this, _, _window, cx| {
                                println!("Scanning...");
                                cx.notify();
                            }))
                    )
            )
            .child(
                // Content
                div()
                    .flex_1()
                    .p_4()
                    .child(
                        v_flex()
                            .gap_1()
                            .when(self.file_assets.is_empty(), |this| {
                                this.child(
                                    div()
                                        .text_sm()
                                        .text_center()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("No assets scanned")
                                )
                            })
                            .children(
                                self.file_assets.iter().map(|asset| {
                                    h_flex()
                                        .gap_2()
                                        .px_2()
                                        .py_1()
                                        .items_center()
                                        .child(
                                            div()
                                                .w(px(4.))
                                                .h(px(4.))
                                                .rounded(px(2.))
                                                .bg(match asset.status {
                                                    FileSyncStatus::Synced => cx.theme().success,
                                                    FileSyncStatus::OutOfSync => cx.theme().warning,
                                                    FileSyncStatus::Missing => cx.theme().danger,
                                                    FileSyncStatus::Checking => cx.theme().muted_foreground,
                                                })
                                        )
                                        .child(
                                            div()
                                                .flex_1()
                                                .text_sm()
                                                .text_color(cx.theme().foreground)
                                                .child(asset.asset.path.clone())
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().muted_foreground)
                                                .child(format!("{}/{}",
                                                    asset.peers_with_file.len(),
                                                    self.active_session.as_ref().map(|s| s.connected_users.len()).unwrap_or(0)
                                                ))
                                        )
                                        .into_any_element()
                                })
                            )
                    )
            )
    }

    fn render_presence_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .p_4()
            .child(
                v_flex()
                    .gap_2()
                    .when(self.user_presences.is_empty(), |this| {
                        this.child(
                            div()
                                .text_sm()
                                .text_center()
                                .text_color(cx.theme().muted_foreground)
                                .child("No active users")
                        )
                    })
                    .children(
                        self.user_presences.iter().map(|presence| {
                            v_flex()
                                .gap_1()
                                .px_3()
                                .py_2()
                                .rounded(px(4.))
                                .bg(cx.theme().secondary)
                                .border_l(px(2.))
                                .border_color(cx.theme().accent)
                                .child(
                                    div()
                                        .text_sm()
                                        .font_medium()
                                        .text_color(cx.theme().foreground)
                                        .child(presence.peer_id.clone())
                                )
                                .when_some(presence.editing_file.as_ref(), |this, file| {
                                    this.child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!("Editing: {}", file))
                                    )
                                })
                                .when_some(presence.selected_object.as_ref(), |this, obj| {
                                    this.child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!("Selected: {}", obj))
                                    )
                                })
                                .into_any_element()
                        })
                    )
            )
    }

    fn render_active_session(&self, session: &ActiveSession, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(self.render_tab_bar(cx))
            .child(
                match self.current_tab {
                    SessionTab::Info => self.render_session_info_tab(session, cx).into_any_element(),
                    SessionTab::Presence => self.render_presence_tab(cx).into_any_element(),
                    SessionTab::FileSync => self.render_file_sync_tab(cx).into_any_element(),
                    SessionTab::Chat => self.render_chat_tab(cx).into_any_element(),
                }
            )
    }
}

impl Focusable for MultiplayerWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MultiplayerWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                TitleBar::new()
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .child(div().text_sm().child("Multiplayer"))
                            .when_some(self.active_session.as_ref(), |this, session| {
                                this.child(
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .child(
                                            div()
                                                .w(px(6.))
                                                .h(px(6.))
                                                .rounded(px(3.))
                                                .bg(cx.theme().success)
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().foreground)
                                                .child(format!("{} users", session.connected_users.len()))
                                        )
                                )
                            })
                    )
            )
            .child(
                if let Some(ref session) = self.active_session {
                    self.render_active_session(session, cx).into_any_element()
                } else {
                    self.render_connection_form(cx).into_any_element()
                }
            )
    }
}
