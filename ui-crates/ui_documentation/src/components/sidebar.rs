use gpui::*;
use ui::prelude::*;
use crate::state::{DocumentationState, TreeNode};
use crate::components::TreeNodeView;

pub struct Sidebar;

impl Sidebar {
    pub fn render(
        state: &DocumentationState,
        on_node_click: impl Fn(usize, &mut Window, &mut Context<impl Send>) + 'static + Clone,
        cx: &mut Context<impl Send>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        
        v_flex()
            .w(px(280.0))
            .h_full()
            .bg(theme.surface)
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
                    .overflow_y_scroll()
                    .gap_px()
                    .py_2()
                    .children(
                        state.flat_visible_items.iter().map(|&idx| {
                            let node = &state.tree_items[idx];
                            let is_selected = match node {
                                TreeNode::Item { path, .. } => {
                                    state.selected_item.as_ref() == Some(path)
                                }
                                _ => false,
                            };
                            let indent = TreeNodeView::get_indent_level(node);
                            let on_click = on_node_click.clone();
                            
                            TreeNodeView::render(
                                node,
                                is_selected,
                                indent,
                                move |window, cx| on_click(idx, window, cx),
                                cx,
                            )
                        })
                    )
            )
    }
}
