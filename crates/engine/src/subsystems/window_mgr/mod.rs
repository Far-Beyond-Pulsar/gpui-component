//! Window Management Subsystem
//!
//! Coordinates window creation, destruction, and lifecycle management.
//! This is a thin layer over the existing window/ module.

mod requests;

pub use requests::WindowRequest;

use std::sync::mpsc::Sender;

/// Window manager for coordinating window operations
pub struct WindowManager {
    request_tx: Sender<WindowRequest>,
}

impl WindowManager {
    /// Create a new window manager
    pub fn new(request_tx: Sender<WindowRequest>) -> Self {
        Self { request_tx }
    }

    /// Request a new window
    pub fn request_window(&self, request: WindowRequest) {
        let _ = self.request_tx.send(request);
    }

    /// Request settings window
    pub fn open_settings(&self) {
        self.request_window(WindowRequest::Settings);
    }

    /// Request project editor window
    pub fn open_project_editor(&self, project_path: String) {
        self.request_window(WindowRequest::ProjectEditor { project_path });
    }

    /// Request project splash screen
    pub fn open_project_splash(&self, project_path: String) {
        self.request_window(WindowRequest::ProjectSplash { project_path });
    }

    /// Request window closure
    pub fn close_window(&self, window_id: u64) {
        self.request_window(WindowRequest::CloseWindow { window_id });
    }
}
