use gpui::*;
use ui::{prelude::*, IconButton, IconName, ActiveTheme};

pub struct SearchBar {
    focus_handle: FocusHandle,
}

impl SearchBar {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn render_with_query(
        &self,
        query: &str,
        on_change: impl Fn(&str, &mut Window, &mut Context<Self>) + 'static,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let query_str = query.to_string();

        div()
            .flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_2()
            .bg(theme.surface)
            .border_1()
            .border_color(theme.border)
            .rounded_lg()
            .child(
                Icon::new(IconName::Search)
                    .size(IconSize::Small)
                    .text_color(theme.muted_foreground)
            )
            .child(
                div()
                    .flex_1()
                    .child(
                        div()
                            .id("search-input")
                            .h_8()
                            .w_full()
                            .flex()
                            .items_center()
                            .child(
                                div()
                                    .text_color(if query.is_empty() {
                                        theme.muted_foreground
                                    } else {
                                        theme.foreground
                                    })
                                    .text_sm()
                                    .child(if query.is_empty() {
                                        "Search documentation...".to_string()
                                    } else {
                                        query.to_string()
                                    })
                            )
                    )
            )
            .when(!query.is_empty(), |div| {
                div.child(
                    IconButton::new("clear-search", IconName::Close)
                        .icon_size(IconSize::Small)
                        .on_click(cx.listener(move |_this, _event, window, cx| {
                            on_change("", window, cx);
                        }))
                )
            })
    }
}

impl Focusable for SearchBar {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SearchBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_with_query("", |_, _, _| {}, cx)
    }
}
