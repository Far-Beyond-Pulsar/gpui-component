//! Professional terminal emulator using Alacritty's terminal engine
//! Based on Zed's terminal architecture

mod terminal_core;
mod terminal_element;
mod rendering;
mod mappings;

pub use terminal_core::{Terminal, Event as TerminalEvent, TERMINAL_CONTEXT, init};

