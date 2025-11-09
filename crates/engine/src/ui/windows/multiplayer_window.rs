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

/// Multiplayer collaboration window for connecting to multiuser servers
pub struct MultiplayerWindow {
    server_address_input: Entity<InputState>,
    session_id_input: Entity<InputState>,
    session_password_input: Entity<InputState>,
    connection_status: ConnectionStatus,
    active_session: Option<ActiveSession>,
    client: Option<Arc<RwLock<MultiuserClient>>>,
    current_peer_id: Option<String>,
    focus_handle: FocusHandle,
}

#[derive(Clone, Debug, PartialEq)]
enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

#[derive(Clone, Debug)]
struct ActiveSession {
    session_id: String,
    join_token: String,
    server_address: String,
    connected_users: Vec<String>,
}

impl MultiplayerWindow {
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

        Self {
            server_address_input,
            session_id_input,
            session_password_input,
            connection_status: ConnectionStatus::Disconnected,
            active_session: None,
            client: None,
            current_peer_id: None,
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
                    let mut client_guard = client.write().await;
                    match client_guard.connect(session_id.clone(), join_token).await {
                        Ok(mut event_rx) => {
                            // Wait for the initial response (Joined or Error)
                            match event_rx.recv().await {
                                Some(ServerMessage::Joined { peer_id, participants, .. }) => {
                                    cx.update(|cx| {
                                        this.update(cx, |this, cx| {
                                            if let Some(session) = &mut this.active_session {
                                                // Update with real participant data
                                                session.connected_users = participants.iter().map(|p| {
                                                    if p == &peer_id {
                                                        if participants.len() == 1 {
                                                            "You (Host)".to_string()
                                                        } else {
                                                            "You".to_string()
                                                        }
                                                    } else {
                                                        p.clone()
                                                    }
                                                }).collect();
                                                this.current_peer_id = Some(peer_id);
                                            }
                                            cx.notify();
                                        }).ok();
                                    }).ok();
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
            let mut client_guard = client.write().await;
            let join_token_clone = join_token.clone();

            match client_guard.connect(session_id.clone(), join_token_clone).await {
                Ok(mut event_rx) => {
                    // Wait for the initial response (Joined or Error)
                    match event_rx.recv().await {
                        Some(ServerMessage::Joined { peer_id, participants, .. }) => {
                            cx.update(|cx| {
                                this.update(cx, |this, cx| {
                                    this.connection_status = ConnectionStatus::Connected;
                                    this.active_session = Some(ActiveSession {
                                        session_id: session_id.clone(),
                                        join_token: join_token.clone(),
                                        server_address: server_address.clone(),
                                        connected_users: participants.iter().map(|p| {
                                            if p == &peer_id {
                                                if participants.len() == 1 {
                                                    "You (Host)".to_string()
                                                } else {
                                                    "You".to_string()
                                                }
                                            } else {
                                                p.clone()
                                            }
                                        }).collect(),
                                    });
                                    this.current_peer_id = Some(peer_id);
                                    cx.notify();
                                }).ok();
                            }).ok();
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
                        cx.notify();
                    }).ok();
                }).ok();
            }).detach();
        } else {
            self.connection_status = ConnectionStatus::Disconnected;
            self.active_session = None;
            self.client = None;
            self.current_peer_id = None;
            cx.notify();
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
                                session.connected_users.iter().map(|user| {
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
