//! Terminal Window - Displays a full terminal emulator in a separate window

use gpui::*;
use ui::{
    v_flex, ActiveTheme as _, TitleBar,
};

use crate::Terminal;

pub struct TerminalWindow {
    terminal: Entity<Terminal>,
}

impl TerminalWindow {
    pub fn new(
        terminal: Entity<Terminal>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self { terminal }
    }

    pub fn terminal(&self) -> &Entity<Terminal> {
        &self.terminal
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
                    .child(self.terminal.clone())
            )
    }
}
