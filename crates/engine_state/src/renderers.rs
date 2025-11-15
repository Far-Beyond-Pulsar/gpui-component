//! GPU Renderer Registry
//!
//! Thread-safe registry of GPU renderers indexed by window ID

use dashmap::DashMap;
use std::sync::Arc;

/// Opaque renderer handle (avoid direct dependency on backend)
pub type RendererHandle = Arc<dyn std::any::Any + Send + Sync>;

#[derive(Clone)]
pub struct RendererRegistry {
    renderers: Arc<DashMap<u64, RendererHandle>>,
}

impl RendererRegistry {
    pub fn new() -> Self {
        Self {
            renderers: Arc::new(DashMap::new()),
        }
    }

    pub fn register(&self, window_id: u64, renderer: RendererHandle) {
        self.renderers.insert(window_id, renderer);
    }

    pub fn unregister(&self, window_id: u64) -> Option<RendererHandle> {
        self.renderers.remove(&window_id).map(|(_, v)| v)
    }

    pub fn get(&self, window_id: u64) -> Option<RendererHandle> {
        self.renderers.get(&window_id).map(|v| v.clone())
    }
}

impl Default for RendererRegistry {
    fn default() -> Self {
        Self::new()
    }
}
