use gpui::*;
use ui::{prelude::*, ActiveTheme};

pub struct ContentView;

impl ContentView {
    pub fn render(
        content: String,
        cx: &mut Context<impl Send>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        
        div()
            .flex_1()
            .h_full()
            .overflow_hidden()
            .bg(theme.background)
            .child(
                div()
                    .p_8()
                    .max_w(px(900.0))
                    .mx_auto()
                    .text_color(theme.foreground)
                    .child(content)
            )
    }
}
