//! Terminal Window - Displays a full terminal emulator in a separate window

use gpui::*;
use gpui_component::{
    v_flex, ActiveTheme as _, TitleBar,
};

use super::terminal_drawer::TerminalDrawer;

pub struct TerminalWindow {
    terminal_drawer: Entity<TerminalDrawer>,
}

impl TerminalWindow {
    pub fn new(
        terminal_drawer: Entity<TerminalDrawer>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self { terminal_drawer }
    }

    pub fn terminal_drawer(&self) -> &Entity<TerminalDrawer> {
        &self.terminal_drawer
    }
}

impl Render for TerminalWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .size_full()
            .bg(theme.background)
            .child(TitleBar::new().child("Terminal"))
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.terminal_drawer.clone())
            )
    }
}
