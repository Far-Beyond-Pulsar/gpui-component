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
use tokio::sync::mpsc;
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

    info!("WebSocket connection established");

    // Handle incoming messages
    while let Some(msg) = receiver.next().await {
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
                                    Ok(mut session) => {
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

                                            match state.sessions.join_session(&session_id, peer_id.clone(), role) {
                                                Ok(session) => {
                                                    let response = ServerMessageWs::Joined {
                                                        session_id: session.id.clone(),
                                                        peer_id: peer_id.clone(),
                                                        participants: session.participants.iter().map(|p| p.peer_id.clone()).collect(),
                                                    };

                                                    if let Ok(json) = serde_json::to_string(&response) {
                                                        let _ = sender.send(Message::Text(json)).await;
                                                    }

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
                                // Handle leave
                                if let Some(mut session) = state.sessions.get_session(&session_id) {
                                    session.participants.retain(|p| p.peer_id != peer_id);
                                    info!("Peer {} left session {}", peer_id, session_id);
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
    Ping,
}

// Server messages (to engine)
#[derive(Debug, Serialize, Deserialize)]
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

    fn test_state() -> AppState {
        let config = Arc::new(Config::default());
        let auth = Arc::new(AuthService::new(&config).unwrap());
        let sessions = Arc::new(SessionStore::new(config.clone()));
        let health = Arc::new(HealthChecker::new(config.clone()));

        AppState {
            config,
            auth,
            sessions,
            health,
        }
    }

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
