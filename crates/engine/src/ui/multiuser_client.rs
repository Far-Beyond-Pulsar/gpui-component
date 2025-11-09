use anyhow::{Context, Result};
use async_tungstenite::{tokio::{connect_async, TokioAdapter}, tungstenite::Message};
use futures::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, OnceLock};
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

// Global Tokio runtime for WebSocket operations
fn tokio_runtime() -> &'static tokio::runtime::Runtime {
    static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .thread_name("multiuser-ws")
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime for multiuser client")
    })
}

/// Messages sent from client to server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Join {
        session_id: String,
        peer_id: String,
        join_token: String,
    },
    Leave {
        session_id: String,
        peer_id: String,
    },
    Ping,
}

/// Messages received from server
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Joined {
        session_id: String,
        peer_id: String,
        participants: Vec<String>,
    },
    PeerJoined {
        session_id: String,
        peer_id: String,
    },
    PeerLeft {
        session_id: String,
        peer_id: String,
    },
    Pong,
    Error {
        message: String,
    },
}

/// Connection status
#[derive(Debug, Clone, PartialEq)]
pub enum ConnectionStatus {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

/// Multiuser client for connecting to collaboration server
pub struct MultiuserClient {
    server_url: String,
    status: Arc<RwLock<ConnectionStatus>>,
    message_tx: Option<mpsc::UnboundedSender<ClientMessage>>,
    event_rx: Option<mpsc::UnboundedReceiver<ServerMessage>>,
}

impl MultiuserClient {
    pub fn new(server_url: String) -> Self {
        Self {
            server_url,
            status: Arc::new(RwLock::new(ConnectionStatus::Disconnected)),
            message_tx: None,
            event_rx: None,
        }
    }

    /// Get current connection status
    pub async fn status(&self) -> ConnectionStatus {
        self.status.read().await.clone()
    }

    /// Create a new session (generates local credentials)
    /// The server will create the actual session on WebSocket connect
    pub async fn create_session(&self) -> Result<(String, String)> {
        // Generate session credentials locally
        let session_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let join_token = uuid::Uuid::new_v4().to_string()[..16].to_string();

        info!("Generated session credentials: {}", session_id);
        Ok((session_id, join_token))
    }

    /// Connect to a session via WebSocket
    pub async fn connect(
        &mut self,
        session_id: String,
        join_token: String,
    ) -> Result<mpsc::UnboundedReceiver<ServerMessage>> {
        *self.status.write().await = ConnectionStatus::Connecting;

        let peer_id = uuid::Uuid::new_v4().to_string();

        // Build WebSocket URL
        let ws_url = format!("{}/ws", self.server_url);

        info!("Connecting to WebSocket: {}", ws_url);

        // Connect to WebSocket
        let (ws_stream, _) = connect_async(&ws_url)
            .await
            .context("Failed to connect to WebSocket")?;

        let (mut write, mut read) = ws_stream.split();

        // Create channels for bidirectional communication
        let (message_tx, mut message_rx) = mpsc::unbounded_channel::<ClientMessage>();
        let (event_tx, event_rx) = mpsc::unbounded_channel::<ServerMessage>();

        // Send initial Join message
        let join_msg = ClientMessage::Join {
            session_id: session_id.clone(),
            peer_id: peer_id.clone(),
            join_token,
        };

        let join_json = serde_json::to_string(&join_msg)?;
        write.send(Message::Text(join_json)).await?;

        // Spawn task to handle outgoing messages on Tokio runtime
        let status_clone = self.status.clone();
        tokio_runtime().spawn(async move {
            while let Some(msg) = message_rx.recv().await {
                if let Ok(json) = serde_json::to_string(&msg) {
                    if let Err(e) = write.send(Message::Text(json)).await {
                        error!("Failed to send message: {}", e);
                        *status_clone.write().await = ConnectionStatus::Error(e.to_string());
                        break;
                    }
                }
            }
        });

        // Spawn task to handle incoming messages on Tokio runtime
        let status_clone = self.status.clone();
        tokio_runtime().spawn(async move {
            while let Some(result) = read.next().await {
                match result {
                    Ok(Message::Text(text)) => {
                        match serde_json::from_str::<ServerMessage>(&text) {
                            Ok(msg) => {
                                debug!("Received message: {:?}", msg);

                                // Update status on successful join
                                if matches!(msg, ServerMessage::Joined { .. }) {
                                    *status_clone.write().await = ConnectionStatus::Connected;
                                }

                                if event_tx.send(msg).is_err() {
                                    warn!("Event receiver dropped");
                                    break;
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse server message: {}", e);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket closed by server");
                        *status_clone.write().await = ConnectionStatus::Disconnected;
                        break;
                    }
                    Ok(Message::Ping(_)) => {
                        // Pings are handled automatically by tungstenite
                    }
                    Ok(_) => {}
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        *status_clone.write().await = ConnectionStatus::Error(e.to_string());
                        break;
                    }
                }
            }
        });

        self.message_tx = Some(message_tx);
        *self.status.write().await = ConnectionStatus::Connected;

        Ok(event_rx)
    }

    /// Send a message to the server
    pub async fn send(&self, message: ClientMessage) -> Result<()> {
        if let Some(tx) = &self.message_tx {
            tx.send(message).context("Failed to send message")?;
            Ok(())
        } else {
            anyhow::bail!("Not connected")
        }
    }

    /// Disconnect from the session
    pub async fn disconnect(&mut self, session_id: String, peer_id: String) -> Result<()> {
        if let Some(tx) = &self.message_tx {
            let leave_msg = ClientMessage::Leave { session_id, peer_id };
            tx.send(leave_msg)?;
        }

        self.message_tx = None;
        *self.status.write().await = ConnectionStatus::Disconnected;

        Ok(())
    }
}
