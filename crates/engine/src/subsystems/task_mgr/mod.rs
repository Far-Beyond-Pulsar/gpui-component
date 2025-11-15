//! Task Management Subsystem
//!
//! Coordinates async task spawning and execution.
//! Provides a unified interface for spawning tasks across the engine.

use std::future::Future;
use std::sync::{Arc, Once};

static INIT: Once = Once::new();
static mut RUNTIME: Option<Arc<tokio::runtime::Runtime>> = None;

fn get_runtime() -> Arc<tokio::runtime::Runtime> {
    unsafe {
        INIT.call_once(|| {
            RUNTIME = Some(Arc::new(
                tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime")
            ));
        });
        RUNTIME.as_ref().unwrap().clone()
    }
}

/// Task manager for spawning and coordinating async operations
pub struct TaskManager {
    runtime: Arc<tokio::runtime::Runtime>,
}

impl TaskManager {
    /// Create a new task manager with shared runtime
    pub fn new() -> Self {
        Self {
            runtime: get_runtime(),
        }
    }

    /// Spawn an async task
    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime.spawn(future)
    }

    /// Block on a future (use sparingly, prefer spawn)
    pub fn block_on<F: Future>(&self, future: F) -> F::Output {
        self.runtime.block_on(future)
    }

    /// Get the runtime handle for advanced use
    pub fn runtime(&self) -> &tokio::runtime::Runtime {
        &self.runtime
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}
