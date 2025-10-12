//! Professional terminal emulator using Alacritty's terminal engine
//! Based on Zed's terminal architecture

mod terminal_core;
mod terminal_element;
mod mappings;

pub use terminal_core::{Terminal, TerminalSession, Event as TerminalEvent, TerminalBounds};
pub use terminal_element::TerminalElement;

use gpui::*;

pub fn init(cx: &mut App) {
    // Initialize terminal subsystem
}
