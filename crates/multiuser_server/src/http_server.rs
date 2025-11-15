//! HTTP admin server with Axum
//!
//! Provides REST API endpoints for session management, health checks,
//! metrics, and WebSocket signaling.

use anyhow::Result;
use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    response::{IntoResponse, Json, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing::{error, info, warn};

use crate::auth::{AuthService, Role};
use crate::config::Config;
use crate::health::{HealthChecker, HealthStatus};
use crate::metrics::METRICS;
use crate::session::SessionStore;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub auth: Arc<AuthService>,
    pub sessions: Arc<SessionStore>,
    pub health: Arc<HealthChecker>,
    pub session_broadcasts: Arc<dashmap::DashMap<String, broadcast::Sender<ServerMessageWs>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub host_id: String,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateSessionResponse {
    pub session_id: String,
    pub join_token: String,
    pub expires_at: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinSessionRequest {
    pub join_token: String,
    pub peer_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JoinSessionResponse {
    pub session_id: String,
    pub peer_id: String,
    pub role: Role,
    pub participant_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

impl IntoResponse for ErrorResponse {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(self)).into_response()
    }
}

pub async fn run_server(
    config: Arc<Config>,
    shutdown: mpsc::Receiver<()>,
) -> Result<()> {
    let bind_addr = config.http_bind;

    // Initialize services
    let auth = Arc::new(AuthService::new(&config)?);
    let sessions = Arc::new(SessionStore::new(config.clone()));
    let health = Arc::new(HealthChecker::new(config.clone()));

    let state = AppState {
        config,
        auth,
        sessions,
        health,
        session_broadcasts: Arc::new(dashmap::DashMap::new()),
    };

    let app = create_router(state);

    info!("Starting HTTP server on {}", bind_addr);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown))
        .await?;

    Ok(())
}

fn create_router(state: AppState) -> Router {
    Router::new()
        // Health and metrics
        .route("/health", get(health_check))
        .route("/health/liveness", get(liveness_check))
        .route("/health/readiness", get(readiness_check))
        .route("/metrics", get(metrics_handler))
        .route("/metrics/json", get(metrics_json_handler))
        // Session management
        .route("/v1/sessions", post(create_session))
        .route("/v1/sessions/:id/join", post(join_session))
        .route("/v1/sessions/:id/close", post(close_session))
        .route("/v1/sessions/:id", get(get_session))
        // WebSocket signaling
        .route("/v1/signaling", get(websocket_handler))
        .route("/ws", get(websocket_handler))  // Simple /ws endpoint for client compatibility
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

async fn health_check(State(state): State<AppState>) -> impl IntoResponse {
    let status = state.health.check_health().await;
    Json(status)
}

async fn liveness_check(State(state): State<AppState>) -> impl IntoResponse {
    let status = state.health.liveness_check().await;
    (StatusCode::OK, Json(status))
}

async fn readiness_check(State(state): State<AppState>) -> impl IntoResponse {
    let status = state.health.check_health().await;
    match status.status.as_str() {
        "healthy" => (StatusCode::OK, Json(status)),
        "degraded" => (StatusCode::OK, Json(status)),
        _ => (StatusCode::SERVICE_UNAVAILABLE, Json(status)),
    }
}

async fn metrics_handler() -> impl IntoResponse {
    match METRICS.encode() {
        Ok(metrics) => (StatusCode::OK, metrics),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to encode metrics: {}", e),
        ),
    }
}

async fn metrics_json_handler() -> impl IntoResponse {
    match METRICS.as_json() {
        Ok(metrics) => (StatusCode::OK, Json(metrics)),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Failed to encode metrics: {}", e)})),
        ),
    }
}

async fn create_session(
    State(state): State<AppState>,
    Json(req): Json<CreateSessionRequest>,
) -> Result<Json<CreateSessionResponse>, ErrorResponse> {
    let metadata = req.metadata.unwrap_or_else(|| serde_json::json!({}));

    let session = state
        .sessions
        .create_session(req.host_id.clone(), metadata)
        .map_err(|e| ErrorResponse {
            error: "session_creation_failed".to_string(),
            message: e.to_string(),
        })?;

    // Generate join token
    let join_token = state
        .auth
        .create_join_token(
            session.id.clone(),
            Role::Host,
            Duration::from_secs(3600),
        )
        .map_err(|e| ErrorResponse {
            error: "token_generation_failed".to_string(),
            message: e.to_string(),
        })?;

    Ok(Json(CreateSessionResponse {
        session_id: session.id,
        join_token,
        expires_at: session.expires_at,
    }))
}

async fn join_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
    Json(req): Json<JoinSessionRequest>,
) -> Result<Json<JoinSessionResponse>, ErrorResponse> {
    // Verify join token
    let (verified_session_id, role) = state
        .auth
        .verify_join_token(&req.join_token)
        .map_err(|e| ErrorResponse {
            error: "invalid_token".to_string(),
            message: e.to_string(),
        })?;

    if verified_session_id != session_id {
        return Err(ErrorResponse {
            error: "session_mismatch".to_string(),
            message: "Token session ID does not match".to_string(),
        });
    }

    // Join session
    let session = state
        .sessions
        .join_session(&session_id, req.peer_id.clone(), role.clone())
        .map_err(|e| ErrorResponse {
            error: "join_failed".to_string(),
            message: e.to_string(),
        })?;

    Ok(Json(JoinSessionResponse {
        session_id: session.id,
        peer_id: req.peer_id,
        role,
        participant_count: session.participants.len(),
    }))
}

async fn close_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<StatusCode, ErrorResponse> {
    state
        .sessions
        .close_session(&session_id, "user_requested")
        .map_err(|e| ErrorResponse {
            error: "close_failed".to_string(),
            message: e.to_string(),
        })?;

    Ok(StatusCode::NO_CONTENT)
}

async fn get_session(
    State(state): State<AppState>,
    Path(session_id): Path<String>,
) -> Result<Json<serde_json::Value>, ErrorResponse> {
    let session = state
        .sessions
        .get_session(&session_id)
        .ok_or_else(|| ErrorResponse {
            error: "not_found".to_string(),
            message: "Session not found".to_string(),
        })?;

    Ok(Json(serde_json::to_value(session).unwrap()))
}

async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_websocket_connection(socket, state))
}

async fn handle_websocket_connection(socket: axum::extract::ws::WebSocket, state: AppState) {
    use axum::extract::ws::Message;
    use futures::{SinkExt, StreamExt};

    let (mut sender, mut receiver) = socket.split();
    let mut current_session_id: Option<String> = None;
    let mut broadcast_rx: Option<broadcast::Receiver<ServerMessageWs>> = None;

    info!("WebSocket connection established");

    // Handle incoming messages and broadcasts
    loop {
        tokio::select! {
            // Handle incoming WebSocket messages
            Some(msg) = receiver.next() => {
                match msg {
                    Ok(Message::Text(text)) => {
                        // Parse incoming message
                        match serde_json::from_str::<ClientMessageWs>(&text) {
                            Ok(client_msg) => {
                                info!("Received client message: {:?}", client_msg);

                                // Handle different message types
                                match client_msg {
                                    ClientMessageWs::Join { session_id, peer_id, join_token } => {
                                        // For stateless sessions, we accept the client-generated session ID and token
                                        // Create session if it doesn't exist (first peer becomes host)
                                        let session_result = if state.sessions.get_session(&session_id).is_none() {
                                            // Session doesn't exist - create it with the peer as host using client-provided ID
                                            info!("Creating new session {} for host {}", session_id, peer_id);
                                            state.sessions.create_session_with_id(
                                                session_id.clone(),
                                                peer_id.clone(),
                                                serde_json::json!({
                                                    "join_token": join_token.clone()
                                                })
                                            )
                                        } else {
                                            // Session exists - verify join token matches
                                            state.sessions.get_session(&session_id).ok_or(anyhow::anyhow!("Session not found"))
                                        };

                                        match session_result {
                                            Ok(session) => {
                                                // Verify the join token from metadata
                                                let stored_token = session.metadata.get("join_token")
                                                    .and_then(|v| v.as_str())
                                                    .unwrap_or("");

                                                if stored_token == join_token || session.host_id == peer_id {
                                                    // Token matches or this is the host - allow join
                                                    let role = if session.host_id == peer_id {
                                                        Role::Host
                                                    } else {
                                                        Role::Editor
                                                    };

                                                    // Check if peer is already in the session
                                                    let already_joined = session.participants.iter().any(|p| p.peer_id == peer_id);

                                                    info!(
                                                        "Join check - session: {}, peer: {}, already_joined: {}, participant_count: {}",
                                                        session_id, peer_id, already_joined, session.participants.len()
                                                    );

                                                    let final_session = if already_joined {
                                                        // Already in session, just return current state
                                                        info!("Peer {} already in session {}, returning current state", peer_id, session_id);
                                                        Ok(session)
                                                    } else {
                                                        // Not in session yet, add them
                                                        info!("Peer {} not in session {}, calling join_session", peer_id, session_id);
                                                        state.sessions.join_session(&session_id, peer_id.clone(), role)
                                                    };

                                                    match final_session {
                                                        Ok(session) => {
                                                            // Send Joined message to the connecting peer
                                                            let response = ServerMessageWs::Joined {
                                                                session_id: session.id.clone(),
                                                                peer_id: peer_id.clone(),
                                                                participants: session.participants.iter().map(|p| p.peer_id.clone()).collect(),
                                                            };

                                                            if let Ok(json) = serde_json::to_string(&response) {
                                                                let _ = sender.send(Message::Text(json)).await;
                                                            }

                                                            // Get or create broadcast channel for this session
                                                            let broadcast_tx = state.session_broadcasts
                                                                .entry(session_id.clone())
                                                                .or_insert_with(|| {
                                                                    let (tx, _rx) = broadcast::channel(100);
                                                                    tx
                                                                })
                                                                .clone();

                                                            // Broadcast PeerJoined to existing session members FIRST (if not already in session)
                                                            // This ensures the new peer doesn't receive their own join notification
                                                            if !already_joined && session.participants.len() > 1 {
                                                                let peer_joined_msg = ServerMessageWs::PeerJoined {
                                                                    session_id: session_id.clone(),
                                                                    peer_id: peer_id.clone(),
                                                                };
                                                                let _ = broadcast_tx.send(peer_joined_msg);
                                                            }

                                                            // Subscribe to broadcasts for this session AFTER sending our join notification
                                                            current_session_id = Some(session_id.clone());
                                                            broadcast_rx = Some(broadcast_tx.subscribe());

                                                            info!("Peer {} joined session {}", peer_id, session_id);
                                                        }
                                                        Err(e) => {
                                                            let error_msg = ServerMessageWs::Error {
                                                                message: format!("Failed to join session: {}", e),
                                                            };
                                                            if let Ok(json) = serde_json::to_string(&error_msg) {
                                                                let _ = sender.send(Message::Text(json)).await;
                                                            }
                                                        }
                                                    }
                                                } else {
                                                    let error_msg = ServerMessageWs::Error {
                                                        message: "Invalid join token".to_string(),
                                                    };
                                                    if let Ok(json) = serde_json::to_string(&error_msg) {
                                                        let _ = sender.send(Message::Text(json)).await;
                                                    }
                                                }
                                            }
                                            Err(e) => {
                                                let error_msg = ServerMessageWs::Error {
                                                    message: format!("Failed to create/get session: {}", e),
                                                };
                                                if let Ok(json) = serde_json::to_string(&error_msg) {
                                                    let _ = sender.send(Message::Text(json)).await;
                                                }
                                            }
                                        }
                                    }
                                    ClientMessageWs::Leave { session_id, peer_id } => {
                                        // Handle leave - use session store's leave_session method
                                        if let Ok(()) = state.sessions.leave_session(&session_id, &peer_id) {
                                            info!("Peer {} left session {}", peer_id, session_id);

                                            // Broadcast PeerLeft to remaining participants
                                            if let Some(broadcast_tx) = state.session_broadcasts.get(&session_id) {
                                                let peer_left_msg = ServerMessageWs::PeerLeft {
                                                    session_id: session_id.clone(),
                                                    peer_id: peer_id.clone(),
                                                };
                                                let _ = broadcast_tx.send(peer_left_msg);
                                            }
                                        }
                                    }
                                    ClientMessageWs::ChatMessage { session_id, peer_id, message } => {
                                        // Broadcast chat message to all participants
                                        info!("Received chat message from {}: {}", peer_id, message);
                                        if let Some(broadcast_tx) = state.session_broadcasts.get(&session_id) {
                                            let now = std::time::SystemTime::now()
                                                .duration_since(std::time::UNIX_EPOCH)
                                                .unwrap_or_default()
                                                .as_secs();

                                            let chat_msg = ServerMessageWs::ChatMessage {
                                                session_id: session_id.clone(),
                                                peer_id: peer_id.clone(),
                                                message: message.clone(),
                                                timestamp: now,
                                            };

                                            match broadcast_tx.send(chat_msg) {
                                                Ok(receivers) => {
                                                    info!("Broadcasted chat message to {} receivers", receivers);
                                                }
                                                Err(e) => {
                                                    error!("Failed to broadcast chat message: {:?}", e);
                                                }
                                            }
                                        } else {
                                            warn!("No broadcast channel found for session {}", session_id);
                                        }
                                    }
                                    // Git sync message handlers
                                    ClientMessageWs::RequestProjectTree { session_id, peer_id } => {
                                        info!("Relaying RequestProjectTree from {}", peer_id);
                                        if let Some(broadcast_tx) = state.session_broadcasts.get(&session_id) {
                                            let relay_msg = ServerMessageWs::RequestProjectTree {
                                                session_id: session_id.clone(),
                                                from_peer_id: peer_id.clone(),
                                            };
                                            let _ = broadcast_tx.send(relay_msg);
                                        }
                                    }
                                    ClientMessageWs::ProjectTreeResponse { session_id, peer_id, tree_json } => {
                                        info!("Relaying ProjectTreeResponse from {}", peer_id);
                                        if let Some(broadcast_tx) = state.session_broadcasts.get(&session_id) {
                                            let relay_msg = ServerMessageWs::ProjectTreeResponse {
                                                session_id: session_id.clone(),
                                                from_peer_id: peer_id.clone(),
                                                tree_json: tree_json.clone(),
                                            };
                                            let _ = broadcast_tx.send(relay_msg);
                                        }
                                    }
                                    ClientMessageWs::RequestGitObjects { session_id, peer_id, commit_hash } => {
                                        info!("Relaying RequestGitObjects from {} for commit {}", peer_id, commit_hash);
                                        if let Some(broadcast_tx) = state.session_broadcasts.get(&session_id) {
                                            let relay_msg = ServerMessageWs::RequestGitObjects {
                                                session_id: session_id.clone(),
                                                from_peer_id: peer_id.clone(),
                                                commit_hash: commit_hash.clone(),
                                            };
                                            let _ = broadcast_tx.send(relay_msg);
                                        }
                                    }
                                    ClientMessageWs::GitObjectsChunk { session_id, peer_id, objects_json, chunk_index, total_chunks } => {
                                        info!("Relaying GitObjectsChunk from {} (chunk {}/{})", peer_id, chunk_index + 1, total_chunks);
                                        if let Some(broadcast_tx) = state.session_broadcasts.get(&session_id) {
                                            let relay_msg = ServerMessageWs::GitObjectsChunk {
                                                session_id: session_id.clone(),
                                                from_peer_id: peer_id.clone(),
                                                objects_json: objects_json.clone(),
                                                chunk_index,
                                                total_chunks,
                                            };
                                            let _ = broadcast_tx.send(relay_msg);
                                        }
                                    }
                                    // Legacy file transfer handlers
                                    ClientMessageWs::RequestFile { session_id, peer_id, file_path } => {
                                        info!("Relaying RequestFile from {} for {}", peer_id, file_path);
                                        if let Some(broadcast_tx) = state.session_broadcasts.get(&session_id) {
                                            let relay_msg = ServerMessageWs::RequestFile {
                                                session_id: session_id.clone(),
                                                from_peer_id: peer_id.clone(),
                                                file_path: file_path.clone(),
                                            };
                                            let _ = broadcast_tx.send(relay_msg);
                                        }
                                    }
                                    ClientMessageWs::FileChunk { session_id, peer_id, file_path, offset, data, is_last } => {
                                        info!("Relaying FileChunk from {} for {} (offset: {}, last: {})", peer_id, file_path, offset, is_last);
                                        if let Some(broadcast_tx) = state.session_broadcasts.get(&session_id) {
                                            let relay_msg = ServerMessageWs::FileChunk {
                                                session_id: session_id.clone(),
                                                from_peer_id: peer_id.clone(),
                                                file_path: file_path.clone(),
                                                offset,
                                                data: data.clone(),
                                                is_last,
                                            };
                                            let _ = broadcast_tx.send(relay_msg);
                                        }
                                    }
                                    ClientMessageWs::Ping => {
                                        // Send pong
                                        let pong = ServerMessageWs::Pong;
                                        if let Ok(json) = serde_json::to_string(&pong) {
                                            let _ = sender.send(Message::Text(json)).await;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("Failed to parse client message: {}", e);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("WebSocket closed by client");
                        break;
                    }
                    Ok(Message::Ping(_)) => {
                        // Pings are handled automatically
                    }
                    Ok(_) => {}
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                }
            }
            // Handle broadcast messages
            Ok(broadcast_msg) = async {
                match &mut broadcast_rx {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await,
                }
            } => {
                // Forward broadcast to this WebSocket connection
                info!("Received broadcast message: {:?}", broadcast_msg);
                if let Ok(json) = serde_json::to_string(&broadcast_msg) {
                    info!("Sending broadcast to WebSocket client");
                    if let Err(e) = sender.send(Message::Text(json)).await {
                        error!("Failed to send broadcast to client: {}", e);
                    }
                }
            }
        }
    }

    info!("WebSocket connection closed");
}

// Client messages (from engine)
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessageWs {
    Join {
        session_id: String,
        peer_id: String,
        join_token: String,
    },
    Leave {
        session_id: String,
        peer_id: String,
    },
    ChatMessage {
        session_id: String,
        peer_id: String,
        message: String,
    },
    // Git sync messages
    RequestProjectTree {
        session_id: String,
        peer_id: String,
    },
    ProjectTreeResponse {
        session_id: String,
        peer_id: String,
        tree_json: String,
    },
    RequestGitObjects {
        session_id: String,
        peer_id: String,
        commit_hash: String,
    },
    GitObjectsChunk {
        session_id: String,
        peer_id: String,
        objects_json: String,
        chunk_index: usize,
        total_chunks: usize,
    },
    // Legacy file transfer
    RequestFile {
        session_id: String,
        peer_id: String,
        file_path: String,
    },
    FileChunk {
        session_id: String,
        peer_id: String,
        file_path: String,
        offset: u64,
        data: Vec<u8>,
        is_last: bool,
    },
    Ping,
}

// Server messages (to engine)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ServerMessageWs {
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
    ChatMessage {
        session_id: String,
        peer_id: String,
        message: String,
        timestamp: u64,
    },
    // Git sync messages (relayed from other peer)
    RequestProjectTree {
        session_id: String,
        from_peer_id: String,
    },
    ProjectTreeResponse {
        session_id: String,
        from_peer_id: String,
        tree_json: String,
    },
    RequestGitObjects {
        session_id: String,
        from_peer_id: String,
        commit_hash: String,
    },
    GitObjectsChunk {
        session_id: String,
        from_peer_id: String,
        objects_json: String,
        chunk_index: usize,
        total_chunks: usize,
    },
    // Legacy file transfer
    RequestFile {
        session_id: String,
        from_peer_id: String,
        file_path: String,
    },
    FileChunk {
        session_id: String,
        from_peer_id: String,
        file_path: String,
        offset: u64,
        data: Vec<u8>,
        is_last: bool,
    },
    Pong,
    Error {
        message: String,
    },
}

async fn shutdown_signal(mut shutdown: mpsc::Receiver<()>) {
    shutdown.recv().await;
    info!("HTTP server shutdown signal received");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_creation() {
        let _state = test_state();
    }

    #[test]
    fn test_error_response() {
        let err = ErrorResponse {
            error: "test_error".to_string(),
            message: "Test message".to_string(),
        };

        let json = serde_json::to_string(&err).unwrap();
        assert!(json.contains("test_error"));
    }
}
