//! Terminal UI
//!
//! Integrated terminal emulator

pub mod mappings;
pub mod rendering;
pub mod terminal_core;
pub mod terminal_drawer;
pub mod terminal_element;
// pub mod terminal_element_zed; // TODO: Needs significant refactoring - commented out for now
pub mod window;

// Re-export main types
pub use terminal_core::{Terminal, TERMINAL_CONTEXT, Event as TerminalEvent, init};
pub use terminal_drawer::TerminalDrawer;
pub use terminal_element::TerminalElement;
pub use window::TerminalWindow;
