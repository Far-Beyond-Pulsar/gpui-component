use gpui::*;
use ui::prelude::*;
use ui::markdown::Markdown;

pub struct ContentView;

impl ContentView {
    pub fn render(
        content: &str,
        cx: &mut Context<impl Send>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        
        div()
            .flex_1()
            .h_full()
            .overflow_y_scroll()
            .bg(theme.background)
            .child(
                div()
                    .p_8()
                    .max_w(px(900.0))
                    .mx_auto()
                    .child(
                        Markdown::new(content.to_string(), cx)
                            .base_text_size(TextSize::Default)
                    )
            )
    }
}
