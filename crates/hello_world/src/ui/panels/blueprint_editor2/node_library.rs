use gpui::*;
use gpui_component::{
    button::Button,
    h_flex, v_flex,
    ActiveTheme as _, StyledExt,
    IconName,
};

use super::{panel::BlueprintEditorPanel, NodeDefinitions};

pub struct NodeLibraryRenderer;

impl NodeLibraryRenderer {
    pub fn render(panel: &BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_2()
            .child(
                h_flex()
                    .w_full()
                    .p_2()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Node Library")
                    )
                    .child(
                        Button::new("search")
                            .icon(IconName::Search)
                            .tooltip("Search Nodes")
                    )
            )
            .child(
                div()
                    .flex_1()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(Self::render_node_categories(cx))
            )
    }

    fn render_node_categories(cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        let node_definitions = NodeDefinitions::load();

        v_flex()
            .p_2()
            .gap_3()
            .children(
                node_definitions.categories.iter().map(|category| {
                    Self::render_node_category(&category.name, category, cx)
                })
            )
    }

    fn render_node_category(title: &str, category: &crate::ui::panels::blueprint_editor2::NodeCategory, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        v_flex()
            .gap_1()
            .child(
                div()
                    .p_2()
                    .bg(cx.theme().muted.opacity(0.3))
                    .rounded(px(4.0))
                    .text_sm()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .child(title.to_string())
            )
            .child(
                v_flex()
                    .gap_1()
                    .children(
                        category.nodes.iter().map(|node_def| {
                            let node_name = node_def.name.clone();
                            let node_icon = node_def.icon.clone();
                            let node_id = node_def.id.clone();

                            h_flex()
                                .items_center()
                                .gap_2()
                                .p_2()
                                .rounded(px(4.0))
                                .hover(|style| style.bg(cx.theme().muted.opacity(0.5)))
                                .cursor_pointer()
                                .child(node_icon)
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().foreground)
                                        .child(node_name)
                                )
                                .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |panel, event: &MouseDownEvent, _window, cx| {
                                    // Create new node from definition at mouse position
                                    let node_definitions = NodeDefinitions::load();
                                    if let Some(definition) = node_definitions.get_node_definition(&node_id) {
                                        let graph_pos = crate::ui::panels::blueprint_editor2::node_graph::NodeGraphRenderer::screen_to_graph_pos(
                                            event.position,
                                            &panel.graph
                                        );
                                        let new_node = crate::ui::panels::blueprint_editor2::BlueprintNode::from_definition(definition, graph_pos);
                                        panel.add_node(new_node, cx);
                                    }
                                }))
                                .into_any_element()
                        })
                    )
            )
    }
}
