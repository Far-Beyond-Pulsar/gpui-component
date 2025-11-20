use gpui::*;
use ui::{ActiveTheme, h_flex, v_flex, StyledExt};
use crate::state::{DocumentationState, TreeNode};
use crate::components::TreeNodeView;

pub struct Sidebar;

impl Sidebar {
    pub fn render<V: Send + 'static>(
        state: &DocumentationState,
        on_node_click: impl Fn(usize, &mut Window, &mut Context<V>) + 'static,
        cx: &mut Context<V>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        
        v_flex()
            .w(px(280.0))
            .h_full()
            .bg(theme.sidebar)
            .border_r_1()
            .border_color(theme.border)
            .child(
                // Header
                div()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(theme.border)
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(theme.foreground)
                            .child("Documentation")
                    )
            )
            .child(
                // Tree view
                v_flex()
                    .flex_1()
                    .overflow_hidden()
                    .gap_px()
                    .py_2()
                    .children({
                        let mut items = vec![];
                        for &idx in &state.flat_visible_items {
                            let node = &state.tree_items[idx];
                            let is_selected = match node {
                                TreeNode::Item { path, .. } => {
                                    state.selected_item.as_ref() == Some(path)
                                }
                                _ => false,
                            };
                            let indent = TreeNodeView::get_indent_level(node);

                            items.push(TreeNodeView::render(
                                node,
                                idx,
                                is_selected,
                                indent,
                                move |window, cx| on_node_click(idx, window, cx),
                                cx,
                            ));
                        }
                        items
                    })
            )
    }
}
