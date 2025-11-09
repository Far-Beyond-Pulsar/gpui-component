use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::Button,
    h_flex, v_flex,
    input::{InputState, TextInput},
    ActiveTheme as _, Disableable as _, Icon, IconName, StyledExt,
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

    fn render_tab_buttons(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_1()
            .p_1()
            .rounded(px(6.))
            .bg(cx.theme().muted.opacity(0.3))
            .child(
                div()
                    .px_3()
                    .py_2()
                    .rounded(px(4.))
                    .when(self.current_tab == SessionTab::Info, |this| {
                        this.bg(cx.theme().background)
                    })
                    .cursor_pointer()
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _window, cx| {
                        this.current_tab = SessionTab::Info;
                        cx.notify();
                    }))
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .text_color(cx.theme().foreground)
                            .child("Session Info")
                    )
            )
            .child(
                div()
                    .px_3()
                    .py_2()
                    .rounded(px(4.))
                    .when(self.current_tab == SessionTab::Chat, |this| {
                        this.bg(cx.theme().background)
                    })
                    .cursor_pointer()
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _window, cx| {
                        this.current_tab = SessionTab::Chat;
                        cx.notify();
                    }))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child("Chat")
                            )
                            .when(!self.chat_messages.is_empty(), |this| {
                                this.child(
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .rounded(px(10.))
                                        .bg(cx.theme().accent)
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_bold()
                                                .text_color(cx.theme().accent_foreground)
                                                .child(self.chat_messages.len().to_string())
                                        )
                                )
                            })
                    )
            )
    }

    fn render_session_info_tab(&self, session: &ActiveSession, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .child(
                // Session Info Box
                v_flex()
                    .gap_2()
                    .p_3()
                    .rounded(px(6.))
                    .bg(cx.theme().accent.opacity(0.1))
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_xs()
                            .font_bold()
                            .text_color(cx.theme().muted_foreground)
                            .child("SESSION CREDENTIALS")
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().foreground)
                                    .child(format!("ID: {}", session.session_id))
                            )
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().foreground)
                                    .child(format!("Password: {}", session.join_token))
                            )
                    )
            )
            .child(
                // Connected Users
                v_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_sm()
                            .font_bold()
                            .text_color(cx.theme().foreground)
                            .child(format!("Connected Users ({})", session.connected_users.len()))
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .children(
                                self.format_participants(&session.connected_users).iter().map(|user| {
                                    h_flex()
                                        .gap_2()
                                        .p_2()
                                        .rounded(px(4.))
                                        .child(
                                            Icon::new(IconName::User)
                                                .size(px(16.))
                                                .text_color(cx.theme().muted_foreground)
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().foreground)
                                                .child(user.clone())
                                        )
                                        .into_any_element()
                                })
                            )
                    )
            )
    }

    fn render_chat_tab(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .flex_1()
            .child(
                // Chat messages area
                div()
                    .flex_1()
                    .p_2()
                    .rounded(px(6.))
                    .bg(cx.theme().muted.opacity(0.2))
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        v_flex()
                            .gap_2()
                            .when(self.chat_messages.is_empty(), |this| {
                                this.child(
                                    div()
                                        .p_4()
                                        .text_center()
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("No messages yet. Start the conversation!")
                                        )
                                )
                            })
                            .children(
                                self.chat_messages.iter().map(|msg| {
                                    let display_name = if msg.is_self {
                                        "You".to_string()
                                    } else {
                                        msg.peer_id.clone()
                                    };

                                    v_flex()
                                        .gap_1()
                                        .when(msg.is_self, |this| this.items_end())
                                        .child(
                                            h_flex()
                                                .gap_2()
                                                .items_baseline()
                                                .when(msg.is_self, |this| this.flex_row_reverse())
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .font_bold()
                                                        .text_color(if msg.is_self {
                                                            cx.theme().accent
                                                        } else {
                                                            cx.theme().muted_foreground
                                                        })
                                                        .child(display_name)
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(cx.theme().muted_foreground)
                                                        .child(format_timestamp(msg.timestamp))
                                                )
                                        )
                                        .child(
                                            div()
                                                .px_3()
                                                .py_2()
                                                .rounded(px(8.))
                                                .max_w(px(300.))
                                                .bg(if msg.is_self {
                                                    cx.theme().accent
                                                } else {
                                                    cx.theme().muted
                                                })
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .text_color(if msg.is_self {
                                                            cx.theme().accent_foreground
                                                        } else {
                                                            cx.theme().foreground
                                                        })
                                                        .child(msg.message.clone())
                                                )
                                        )
                                        .into_any_element()
                                })
                            )
                    )
            )
            .child(
                // Chat input
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .flex_1()
                            .child(TextInput::new(&self.chat_input))
                    )
                    .child(
                        Button::new("send-chat")
                            .label("Send")
                            .icon(IconName::Send)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.send_chat_message(window, cx);
                            }))
                    )
            )
    }

    fn render_active_session(&self, session: &ActiveSession, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .p_4()
            .child(
                // Session Header
                v_flex()
                    .gap_2()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .w(px(8.))
                                    .h(px(8.))
                                    .rounded(px(4.))
                                    .bg(cx.theme().success)
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_bold()
                                    .text_color(cx.theme().success)
                                    .child("CONNECTED")
                            )
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_bold()
                            .text_color(cx.theme().foreground)
                            .child(format!("Session: {}", session.session_id))
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Server: {}", session.server_address))
                    )
            )
            .child(
                // Tab buttons
                self.render_tab_buttons(cx)
            )
            .child(
                // Tab content
                match self.current_tab {
                    SessionTab::Info => self.render_session_info_tab(session, cx).into_any_element(),
                    SessionTab::Chat => self.render_chat_tab(cx).into_any_element(),
                }
            )
            .child(
                // Disconnect Button
                Button::new("disconnect")
                    .label("Disconnect")
                    .icon(IconName::LogOut)
                    .w_full()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.disconnect(window, cx);
                    }))
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
        div()
            .size_full()
            .bg(cx.theme().background)
            .child(
                v_flex()
                    .size_full()
                    .child(
                        if let Some(ref session) = self.active_session {
                            self.render_active_session(session, cx).into_any_element()
                        } else {
                            self.render_connection_form(cx).into_any_element()
                        }
                    )
            )
    }
}
