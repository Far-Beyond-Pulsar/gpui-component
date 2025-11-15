//! Terminal UI
//!
//! Integrated terminal emulator

pub mod mappings;
pub mod rendering;
pub mod terminal_core;
pub mod terminal_element;
pub mod terminal_element_zed;
pub mod window;

// Re-export main types
pub use terminal_core::Terminal;
pub use terminal_element::TerminalElement;
pub use window::TerminalWindow;
