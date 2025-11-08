//! Pulsar MultiEdit Service - Main Entry Point
//!
//! This binary starts the complete Pulsar MultiEdit service including:
//! - HTTP admin server (health, metrics, session management)
//! - QUIC relay server
//! - UDP hole punching coordinator
//! - WebSocket rendezvous signaling
//! - Session garbage collection
//! - Telemetry and metrics collection

use anyhow::{Context, Result};
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tokio::sync::mpsc;
use tracing::info;

use pulsar_multiedit::*;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Load configuration
    let config = Arc::new(Config::from_env().context("Failed to load configuration")?);

    // 2. Initialize logging and tracing
    logging::init(&config).context("Failed to initialize logging")?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Pulsar MultiEdit service starting"
    );

    // Log configuration (sanitized)
    logging::log_config(&config);

    // 3. Initialize telemetry (OpenTelemetry)
    telemetry::init_telemetry(&config).context("Failed to initialize telemetry")?;

    // 4. Initialize metrics registry
    let _metrics = metrics::init(&config).context("Failed to initialize metrics")?;

    // 5. Initialize persistence (DB + S3)
    let _persistence = PersistenceLayer::new((*config).clone())
        .await
        .context("Failed to initialize persistence layer")?;

    // 6. Create shutdown channels
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
    let (http_shutdown_tx, http_shutdown_rx) = mpsc::channel::<()>(1);
    let (quic_shutdown_tx, quic_shutdown_rx) = mpsc::channel::<()>(1);

    // 7. Start HTTP admin server
    let config_http = config.clone();
    let http_handle = tokio::spawn(async move {
        if let Err(e) = http_server::run_server(config_http, http_shutdown_rx).await {
            tracing::error!(error = %e, "HTTP server failed");
        }
    });

    // 8. Start QUIC relay server
    let config_quic = config.clone();
    let quic_handle = tokio::spawn(async move {
        match QuicServer::new(config_quic).await {
            Ok(server) => {
                let server = Arc::new(server);
                if let Err(e) = server.run(quic_shutdown_rx).await {
                    tracing::error!(error = %e, "QUIC server failed");
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "Failed to create QUIC server");
            }
        }
    });

    // 9. Start session garbage collector
    let config_gc = config.clone();
    let sessions = Arc::new(SessionStore::new(config_gc.clone()));
    let sessions_gc = sessions.clone();
    let gc_handle = tokio::spawn(async move {
        session::garbage_collector_loop(sessions_gc, Duration::from_secs(60)).await;
    });

    info!("All services started successfully");

    // 10. Wait for shutdown signal
    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("Received Ctrl+C, initiating graceful shutdown");
        }
        _ = shutdown_rx.recv() => {
            info!("Received shutdown signal");
        }
    }

    // 11. Initiate graceful shutdown
    info!("Shutting down services...");

    // Signal all services to stop
    let _ = http_shutdown_tx.send(()).await;
    let _ = quic_shutdown_tx.send(()).await;

    // Wait for services with timeout
    let shutdown_timeout = Duration::from_secs(10);

    tokio::select! {
        _ = http_handle => info!("HTTP server stopped"),
        _ = tokio::time::sleep(shutdown_timeout) => {
            tracing::warn!("HTTP server shutdown timeout");
        }
    }

    tokio::select! {
        _ = quic_handle => info!("QUIC server stopped"),
        _ = tokio::time::sleep(shutdown_timeout) => {
            tracing::warn!("QUIC server shutdown timeout");
        }
    }

    // Stop GC task (it runs indefinitely)
    gc_handle.abort();

    // 12. Shutdown telemetry
    telemetry::shutdown().await;

    info!("Pulsar MultiEdit service stopped");

    Ok(())
}
