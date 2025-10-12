//! Terminal rendering element with batched text runs
//! Based on Zed's terminal_element.rs

use super::terminal_core::{Terminal, TerminalBounds, TerminalSession};
use gpui::*;

/// Terminal element that renders the terminal content
pub struct TerminalElement {
    terminal: Entity<Terminal>,
    focus_handle: FocusHandle,
}

impl TerminalElement {
    pub fn new(terminal: Entity<Terminal>, focus_handle: FocusHandle) -> Self {
        Self {
            terminal,
            focus_handle,
        }
    }
}

impl IntoElement for TerminalElement {
    type Element = Div;

    fn into_element(self) -> Self::Element {
        use gpui_component::{v_flex, StyledExt};
        
        v_flex()
            .size_full()
            .bg(hsla(0.0, 0.0, 0.05, 1.0))
            .font_family("monospace")
            .text_sm()
            .child(
                div()
                    .text_color(hsla(0.0, 0.0, 0.9, 1.0))
                    .child("Terminal rendering (Alacritty integration)")
            )
    }
}
