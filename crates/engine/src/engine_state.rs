/// Shared engine state accessible across all windows
/// This stores metadata and global state that needs to be shared between windows

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::sync::mpsc::Sender;

// Re-export WindowRequest from main
#[derive(Debug, Clone)]
pub enum WindowRequest {
    Settings,
    ProjectEditor,
    ProjectSplash { project_path: String },
    CloseWindow { window_id: u64 },
}

#[derive(Clone)]
pub struct EngineState {
    inner: Arc<RwLock<EngineStateInner>>,
    window_request_tx: Option<Sender<WindowRequest>>,
}

struct EngineStateInner {
    /// Metadata key-value store
    metadata: HashMap<String, String>,
    /// Number of open windows
    window_count: usize,
}

impl EngineState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(EngineStateInner {
                metadata: HashMap::new(),
                window_count: 0,
            })),
            window_request_tx: None,
        }
    }

    pub fn with_window_sender(mut self, sender: Sender<WindowRequest>) -> Self {
        self.window_request_tx = Some(sender);
        self
    }

    pub fn request_window(&self, request: WindowRequest) {
        if let Some(tx) = &self.window_request_tx {
            let _ = tx.send(request);
        }
    }

    pub fn increment_window_count(&self) {
        if let Ok(mut state) = self.inner.write() {
            state.window_count += 1;
        }
    }

    pub fn decrement_window_count(&self) {
        if let Ok(mut state) = self.inner.write() {
            state.window_count = state.window_count.saturating_sub(1);
        }
    }

    pub fn window_count(&self) -> usize {
        self.inner.read().map(|s| s.window_count).unwrap_or(0)
    }

    pub fn set_metadata(&self, key: impl Into<String>, value: impl Into<String>) {
        if let Ok(mut state) = self.inner.write() {
            state.metadata.insert(key.into(), value.into());
        }
    }

    pub fn get_metadata(&self, key: &str) -> Option<String> {
        self.inner.read().ok()?.metadata.get(key).cloned()
    }

    pub fn remove_metadata(&self, key: &str) -> Option<String> {
        self.inner.write().ok()?.metadata.remove(key)
    }
}

impl Default for EngineState {
    fn default() -> Self {
        Self::new()
    }
}

// Global instance for easy access from GPUI views
use std::sync::OnceLock;
static GLOBAL_ENGINE_STATE: OnceLock<EngineState> = OnceLock::new();

impl EngineState {
    pub fn set_global(self) {
        let _ = GLOBAL_ENGINE_STATE.set(self);
    }

    pub fn global() -> Option<&'static EngineState> {
        GLOBAL_ENGINE_STATE.get()
    }
}
