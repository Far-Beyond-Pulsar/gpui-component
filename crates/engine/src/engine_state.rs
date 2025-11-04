//! Engine State Management
//!
//! This module provides the global engine state that is shared across all windows and systems.
//! The `EngineState` struct acts as a central hub for:
//! - Window management and creation requests
//! - GPU renderer storage (per-window Bevy renderers)
//! - Metadata storage (key-value configuration)
//! - Thread-safe access via Arc/RwLock
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │         EngineState                 │
//! │  (Arc<RwLock<EngineStateInner>>)    │
//! ├─────────────────────────────────────┤
//! │ - window_count: usize               │
//! │ - window_sender: WindowRequest chan │
//! │ - window_renderers: HashMap         │
//! │ - metadata: HashMap<String, String> │
//! └─────────────────────────────────────┘
//!          ↓          ↓          ↓
//!      Window1    Window2    Window3
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! // Create global engine state
//! let engine_state = EngineState::new()
//!     .with_window_sender(window_tx);
//! engine_state.clone().set_global();
//!
//! // Request a new window
//! engine_state.request_window(WindowRequest::Settings);
//!
//! // Store metadata
//! engine_state.set_metadata("key".to_string(), "value".to_string());
//!
//! // Access from anywhere
//! let state = EngineState::global();
//! ```
//!
//! # Thread Safety
//!
//! The engine state is thread-safe and can be cloned cheaply (Arc-based).
//! All operations use RwLock for interior mutability.

/// Shared engine state accessible across all windows
/// This stores metadata and global state that needs to be shared between windows

use std::collections::HashMap;
use std::sync::{Arc, RwLock, Mutex};
use std::sync::mpsc::Sender;

// Import GpuRenderer
use crate::ui::common::services::gpu_renderer::GpuRenderer;

/// Window creation/management request
///
/// This enum represents different types of windows that can be requested
/// through the window management system.
#[derive(Debug, Clone)]
pub enum WindowRequest {
    /// Open the engine settings window
    Settings,
    /// Open the project editor for a specific project
    ProjectEditor {
        /// Path to the project directory
        project_path: String
    },
    /// Open the project loading splash screen
    ProjectSplash {
        /// Path to the project being loaded
        project_path: String
    },
    /// Close a specific window by ID
    CloseWindow {
        /// The window ID to close
        window_id: u64
    },
}

/// Engine state handle (cheap to clone)
///
/// This is the main interface to the global engine state.
/// It can be cloned freely as it only contains an Arc.
#[derive(Clone)]
pub struct EngineState {
    /// Inner state protected by RwLock for thread-safe access
    inner: Arc<RwLock<EngineStateInner>>,
    /// Optional channel for sending window creation requests
    window_request_tx: Option<Sender<WindowRequest>>,
}

/// Internal engine state
///
/// This struct contains the actual mutable state. It's protected by
/// an RwLock in the EngineState wrapper.
struct EngineStateInner {
    /// Metadata key-value store
    metadata: HashMap<String, String>,
    /// Number of open windows
    window_count: usize,
    /// Map window IDs to their GPU renderers (for windows with 3D viewports)
    window_renderers: HashMap<u64, Arc<Mutex<GpuRenderer>>>,
}

impl EngineState {
    /// Create a new engine state
    ///
    /// Returns an EngineState with empty metadata and no window sender.
    /// Use `with_window_sender()` to add window management capability.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(EngineStateInner {
                metadata: HashMap::new(),
                window_count: 0,
                window_renderers: HashMap::new(),
            })),
            window_request_tx: None,
        }
    }

    /// Add a window request sender (builder pattern)
    ///
    /// # Arguments
    /// * `sender` - Channel sender for WindowRequest messages
    ///
    /// # Returns
    /// Self with the sender attached
    pub fn with_window_sender(mut self, sender: Sender<WindowRequest>) -> Self {
        self.window_request_tx = Some(sender);
        self
    }

    /// Request creation of a new window
    ///
    /// Sends a window request through the channel if one is configured.
    /// The request will be processed asynchronously by the window manager.
    ///
    /// # Arguments
    /// * `request` - The type of window to create
    pub fn request_window(&self, request: WindowRequest) {
        if let Some(tx) = &self.window_request_tx {
            let _ = tx.send(request);
        }
    }

    /// Increment the window count
    ///
    /// Called when a new window is created.
    pub fn increment_window_count(&self) {
        if let Ok(mut state) = self.inner.write() {
            state.window_count += 1;
        }
    }

    /// Decrement the window count
    ///
    /// Called when a window is closed. Uses saturating subtraction to prevent underflow.
    pub fn decrement_window_count(&self) {
        if let Ok(mut state) = self.inner.write() {
            state.window_count = state.window_count.saturating_sub(1);
        }
    }

    /// Get the current number of open windows
    ///
    /// # Returns
    /// Number of windows, or 0 if lock acquisition fails
    pub fn window_count(&self) -> usize {
        self.inner.read().map(|s| s.window_count).unwrap_or(0)
    }

    /// Set a metadata value
    ///
    /// Stores a key-value pair in the metadata dictionary.
    /// Useful for cross-window communication and configuration.
    ///
    /// # Arguments
    /// * `key` - Metadata key
    /// * `value` - Metadata value
    pub fn set_metadata(&self, key: impl Into<String>, value: impl Into<String>) {
        if let Ok(mut state) = self.inner.write() {
            state.metadata.insert(key.into(), value.into());
        }
    }

    /// Get a metadata value
    ///
    /// # Arguments
    /// * `key` - Metadata key to look up
    ///
    /// # Returns
    /// The value if found, None otherwise
    pub fn get_metadata(&self, key: &str) -> Option<String> {
        self.inner.read().ok()?.metadata.get(key).cloned()
    }

    /// Remove a metadata value
    ///
    /// # Arguments
    /// * `key` - Metadata key to remove
    ///
    /// # Returns
    /// The removed value if it existed, None otherwise
    pub fn remove_metadata(&self, key: &str) -> Option<String> {
        self.inner.write().ok()?.metadata.remove(key)
    }

    /// Set the GPU renderer for a specific window
    ///
    /// Stores a reference to the Bevy GPU renderer for a window.
    /// This allows the main render loop to access Bevy's textures for composition.
    ///
    /// # Arguments
    /// * `window_id` - Window identifier (from Winit WindowId)
    /// * `renderer` - Arc to the GPU renderer
    pub fn set_window_gpu_renderer(&self, window_id: u64, renderer: Arc<Mutex<GpuRenderer>>) {
        if let Ok(mut state) = self.inner.write() {
            state.window_renderers.insert(window_id, renderer);
        }
    }

    /// Get the GPU renderer for a specific window
    ///
    /// # Arguments
    /// * `window_id` - Window identifier
    ///
    /// # Returns
    /// Arc to the renderer if found
    pub fn get_window_gpu_renderer(&self, window_id: u64) -> Option<Arc<Mutex<GpuRenderer>>> {
        self.inner.read().ok()?.window_renderers.get(&window_id).cloned()
    }

    /// Remove the GPU renderer for a specific window
    ///
    /// Called when a window closes to clean up the renderer reference.
    ///
    /// # Arguments
    /// * `window_id` - Window identifier
    pub fn remove_window_gpu_renderer(&self, window_id: u64) {
        if let Ok(mut state) = self.inner.write() {
            state.window_renderers.remove(&window_id);
        }
    }
}

impl Default for EngineState {
    fn default() -> Self {
        Self::new()
    }
}

/// Global instance storage for easy access from GPUI views
use std::sync::OnceLock;
static GLOBAL_ENGINE_STATE: OnceLock<EngineState> = OnceLock::new();

impl EngineState {
    /// Set this as the global engine state
    ///
    /// Can only be called once. Subsequent calls will be ignored.
    /// This makes the engine state accessible from anywhere via `EngineState::global()`.
    pub fn set_global(self) {
        let _ = GLOBAL_ENGINE_STATE.set(self);
    }

    /// Get the global engine state
    ///
    /// # Returns
    /// Reference to the global EngineState if it has been set, None otherwise
    pub fn global() -> Option<&'static EngineState> {
        GLOBAL_ENGINE_STATE.get()
    }
}
