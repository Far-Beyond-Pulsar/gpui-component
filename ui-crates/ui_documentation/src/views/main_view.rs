use gpui::*;
use ui::prelude::*;
use ui::{ActiveTheme, StyledExt}
use crate::state::DocumentationState;
use crate::components::{Sidebar, ContentView, SearchBar};

pub struct MainView {
    state: DocumentationState,
    search_bar: View<SearchBar>,
}

impl MainView {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = DocumentationState::new(cx);
        let search_bar = cx.new(|cx| SearchBar::new(cx));

        Self {
            state,
            search_bar,
        }
    }

    fn handle_node_click(&mut self, idx: usize, _window: &mut Window, cx: &mut Context<Self>) {
        self.state.toggle_node(idx, cx);
        cx.notify();
    }

    fn handle_search_change(&mut self, query: &str, _window: &mut Window, cx: &mut Context<Self>) {
        self.state.update_search(query.to_string(), cx);
        cx.notify();
    }
}

impl Render for MainView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let current_query = self.state.search_query.to_string();

        v_flex()
            .size_full()
            .bg(theme.background)
            .child(
                // Top bar with search
                div()
                    .w_full()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(theme.border)
                    .bg(theme.surface)
                    .child(
                        self.search_bar.update(cx, |search_bar, cx| {
                            search_bar.render_with_query(
                                &current_query,
                                cx.listener(|this, query: &str, window, cx| {
                                    this.handle_search_change(query, window, cx);
                                }),
                                cx,
                            )
                        })
                    )
            )
            .child(
                // Main content area
                h_flex()
                    .flex_1()
                    .w_full()
                    .child(
                        Sidebar::render(
                            &self.state,
                            cx.listener(|this, idx, window, cx| {
                                this.handle_node_click(idx, window, cx);
                            }),
                            cx,
                        )
                    )
                    .child(
                        ContentView::render(
                            &self.state.current_doc_content,
                            cx,
                        )
                    )
            )
    }
}
