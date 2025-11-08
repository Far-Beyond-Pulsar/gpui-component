//! Graceful shutdown handler
//!
//! This module handles graceful shutdown of the service, including cleanup
//! of resources, flushing telemetry, and coordinated shutdown of tasks.

use anyhow::Result;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::time::timeout;
use tracing::{error, info, warn};

use crate::config::Config;

/// Shutdown coordinator
#[derive(Clone)]
pub struct ShutdownCoordinator {
    config: Config,
    shutdown_tx: broadcast::Sender<()>,
    is_shutting_down: Arc<AtomicBool>,
}

impl ShutdownCoordinator {
    /// Create a new shutdown coordinator
    pub fn new(config: Config) -> Self {
        let (shutdown_tx, _) = broadcast::channel(16);

        Self {
            config,
            shutdown_tx,
            is_shutting_down: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if shutdown is in progress
    pub fn is_shutting_down(&self) -> bool {
        self.is_shutting_down.load(Ordering::Relaxed)
    }

    /// Get a shutdown receiver
    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.shutdown_tx.subscribe()
    }

    /// Initiate graceful shutdown
    pub async fn shutdown(&self) -> Result<()> {
        if self
            .is_shutting_down
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            warn!("Shutdown already in progress");
            return Ok(());
        }

        info!("Initiating graceful shutdown");

        // Broadcast shutdown signal
        if let Err(e) = self.shutdown_tx.send(()) {
            warn!(error = %e, "Failed to broadcast shutdown signal");
        }

        // Wait for components to shut down
        tokio::time::sleep(Duration::from_millis(100)).await;

        info!("Shutdown complete");

        Ok(())
    }

    /// Wait for shutdown signal
    pub async fn wait_for_signal(&self) {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {
                info!("Received Ctrl+C signal");
            }
            _ = terminate => {
                info!("Received SIGTERM signal");
            }
        }
    }

    /// Run shutdown with timeout
    pub async fn shutdown_with_timeout(&self, shutdown_timeout: Duration) -> Result<()> {
        match timeout(shutdown_timeout, self.shutdown()).await {
            Ok(result) => result,
            Err(_) => {
                error!("Shutdown timeout exceeded");
                anyhow::bail!("Shutdown timeout exceeded");
            }
        }
    }
}

/// Graceful shutdown handler with telemetry cleanup
pub async fn graceful_shutdown() -> Result<()> {
    info!("Starting graceful shutdown");

    // Flush OpenTelemetry traces
    info!("Flushing OpenTelemetry traces");
    if let Err(e) = flush_telemetry().await {
        warn!(error = %e, "Failed to flush telemetry");
    }

    // Final log message
    info!("Graceful shutdown complete");

    Ok(())
}

/// Flush telemetry data
async fn flush_telemetry() -> Result<()> {
    // Give telemetry exporters time to flush
    tokio::time::sleep(Duration::from_millis(500)).await;

    // In a real implementation, this would call:
    // - opentelemetry::global::shutdown_tracer_provider()
    // - Flush Prometheus metrics if needed
    // - Close any other telemetry connections

    info!("Telemetry flushed");

    Ok(())
}

/// Shutdown task handle
pub struct ShutdownHandle {
    task: tokio::task::JoinHandle<()>,
    coordinator: ShutdownCoordinator,
}

impl ShutdownHandle {
    /// Create a new shutdown handle
    pub fn new(config: Config) -> Self {
        let coordinator = ShutdownCoordinator::new(config);
        let coord_clone = coordinator.clone();

        let task = tokio::spawn(async move {
            coord_clone.wait_for_signal().await;
            if let Err(e) = coord_clone.shutdown().await {
                error!(error = %e, "Shutdown failed");
            }
        });

        Self { task, coordinator }
    }

    /// Get the shutdown coordinator
    pub fn coordinator(&self) -> &ShutdownCoordinator {
        &self.coordinator
    }

    /// Wait for shutdown to complete
    pub async fn wait(self) -> Result<()> {
        self.task.await?;
        Ok(())
    }
}

/// Task runner with graceful shutdown support
pub struct GracefulTask {
    name: String,
    shutdown_rx: broadcast::Receiver<()>,
}

impl GracefulTask {
    /// Create a new graceful task
    pub fn new(name: String, shutdown_rx: broadcast::Receiver<()>) -> Self {
        Self { name, shutdown_rx }
    }

    /// Run a task with shutdown support
    pub async fn run<F, Fut>(mut self, f: F) -> Result<()>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        info!(task = %self.name, "Starting task");

        tokio::select! {
            result = f() => {
                match result {
                    Ok(_) => {
                        info!(task = %self.name, "Task completed");
                        Ok(())
                    }
                    Err(e) => {
                        error!(task = %self.name, error = %e, "Task failed");
                        Err(e)
                    }
                }
            }
            _ = self.shutdown_rx.recv() => {
                info!(task = %self.name, "Task received shutdown signal");
                Ok(())
            }
        }
    }

    /// Run a task loop with shutdown support
    pub async fn run_loop<F, Fut>(mut self, mut f: F) -> Result<()>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<()>>,
    {
        info!(task = %self.name, "Starting task loop");

        loop {
            tokio::select! {
                result = f() => {
                    if let Err(e) = result {
                        error!(task = %self.name, error = %e, "Task loop iteration failed");
                        // Continue running unless it's a fatal error
                        if is_fatal_error(&e) {
                            return Err(e);
                        }
                    }
                }
                _ = self.shutdown_rx.recv() => {
                    info!(task = %self.name, "Task loop received shutdown signal");
                    break;
                }
            }
        }

        Ok(())
    }
}

/// Check if an error is fatal
fn is_fatal_error(_error: &anyhow::Error) -> bool {
    // In a real implementation, classify errors
    // For now, treat all errors as non-fatal
    false
}

/// Resource cleanup tracker
pub struct ResourceCleanup {
    cleanups: Vec<Box<dyn FnOnce() + Send>>,
}

impl ResourceCleanup {
    /// Create a new resource cleanup tracker
    pub fn new() -> Self {
        Self {
            cleanups: Vec::new(),
        }
    }

    /// Register a cleanup function
    pub fn register<F>(&mut self, cleanup: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.cleanups.push(Box::new(cleanup));
    }

    /// Run all cleanup functions
    pub fn cleanup(self) {
        info!(count = self.cleanups.len(), "Running cleanup functions");

        for cleanup in self.cleanups {
            cleanup();
        }

        info!("All cleanup functions executed");
    }
}

impl Default for ResourceCleanup {
    fn default() -> Self {
        Self::new()
    }
}

/// Shutdown manager for coordinating multiple components
pub struct ShutdownManager {
    coordinator: ShutdownCoordinator,
    tasks: Vec<tokio::task::JoinHandle<Result<()>>>,
    cleanup: ResourceCleanup,
}

impl ShutdownManager {
    /// Create a new shutdown manager
    pub fn new(config: Config) -> Self {
        Self {
            coordinator: ShutdownCoordinator::new(config),
            tasks: Vec::new(),
            cleanup: ResourceCleanup::new(),
        }
    }

    /// Get the shutdown coordinator
    pub fn coordinator(&self) -> &ShutdownCoordinator {
        &self.coordinator
    }

    /// Spawn a task with shutdown support
    pub fn spawn_task<F, Fut>(&mut self, name: String, f: F)
    where
        F: FnOnce(broadcast::Receiver<()>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let shutdown_rx = self.coordinator.subscribe();
        let task = tokio::spawn(async move {
            let result = f(shutdown_rx).await;
            if let Err(ref e) = result {
                error!(task = %name, error = %e, "Task failed");
            }
            result
        });

        self.tasks.push(task);
    }

    /// Register a cleanup function
    pub fn register_cleanup<F>(&mut self, cleanup: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.cleanup.register(cleanup);
    }

    /// Wait for shutdown signal and coordinate shutdown
    pub async fn wait_and_shutdown(self) -> Result<()> {
        // Wait for signal
        self.coordinator.wait_for_signal().await;

        // Initiate shutdown
        self.coordinator.shutdown().await?;

        // Wait for all tasks with timeout
        let shutdown_timeout = Duration::from_secs(30);
        let wait_tasks = async {
            for task in self.tasks {
                if let Err(e) = task.await {
                    error!(error = %e, "Task join failed");
                }
            }
        };

        match timeout(shutdown_timeout, wait_tasks).await {
            Ok(_) => {
                info!("All tasks shut down successfully");
            }
            Err(_) => {
                warn!("Shutdown timeout exceeded, some tasks may not have completed");
            }
        }

        // Run cleanup functions
        self.cleanup.cleanup();

        // Final telemetry flush
        graceful_shutdown().await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shutdown_coordinator_creation() {
        let config = Config::default();
        let coordinator = ShutdownCoordinator::new(config);
        assert!(!coordinator.is_shutting_down());
    }

    #[tokio::test]
    async fn test_shutdown_signal() {
        let config = Config::default();
        let coordinator = ShutdownCoordinator::new(config);

        let mut rx = coordinator.subscribe();

        // Trigger shutdown
        coordinator.shutdown().await.unwrap();

        // Should receive signal
        assert!(rx.recv().await.is_ok());
        assert!(coordinator.is_shutting_down());
    }

    #[tokio::test]
    async fn test_shutdown_with_timeout() {
        let config = Config::default();
        let coordinator = ShutdownCoordinator::new(config);

        let timeout = Duration::from_secs(1);
        let result = coordinator.shutdown_with_timeout(timeout).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_resource_cleanup() {
        let mut cleanup = ResourceCleanup::new();
        let counter = Arc::new(AtomicBool::new(false));
        let counter_clone = counter.clone();

        cleanup.register(move || {
            counter_clone.store(true, Ordering::SeqCst);
        });

        cleanup.cleanup();

        assert!(counter.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_graceful_task() {
        let config = Config::default();
        let coordinator = ShutdownCoordinator::new(config);
        let shutdown_rx = coordinator.subscribe();

        let task = GracefulTask::new("test-task".to_string(), shutdown_rx);

        let result = task
            .run(|| async {
                tokio::time::sleep(Duration::from_millis(10)).await;
                Ok(())
            })
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_shutdown_manager() {
        let config = Config::default();
        let mut manager = ShutdownManager::new(config);

        manager.spawn_task("test-task".to_string(), |mut shutdown_rx| async move {
            tokio::select! {
                _ = tokio::time::sleep(Duration::from_secs(10)) => {
                    Ok(())
                }
                _ = shutdown_rx.recv() => {
                    Ok(())
                }
            }
        });

        let counter = Arc::new(AtomicBool::new(false));
        let counter_clone = counter.clone();
        manager.register_cleanup(move || {
            counter_clone.store(true, Ordering::SeqCst);
        });

        // Trigger immediate shutdown
        manager.coordinator().shutdown().await.unwrap();

        // Give tasks time to shut down
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
