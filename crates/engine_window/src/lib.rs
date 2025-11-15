//! Engine Window Management
//!
//! This crate handles OS window management via Winit.
//! It provides:
//! - Multi-window support
//! - Event handling and conversion
//! - Zero-copy GPU composition (Windows)
//! - Integration with GPUI

pub mod app;
pub mod d3d11;
pub mod events;
pub mod state;

pub use app::WinitGpuiApp;
pub use events::{convert_modifiers, convert_mouse_button, MotionSmoother, SimpleClickState};
pub use state::WindowState;

