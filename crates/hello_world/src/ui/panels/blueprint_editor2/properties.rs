use gpui::*;
use gpui_component::{
    h_flex, v_flex,
    ActiveTheme as _, StyledExt,
};

use super::*;
use super::panel::BlueprintEditorPanel;

pub struct PropertiesRenderer;

impl PropertiesRenderer {
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
                            .child("Details")
                    )
            )
            .child(
                div()
                    .flex_1()
                    .p_3()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(Self::render_properties_content(panel, cx))
            )
    }

    fn render_properties_content(panel: &BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        if let Some(selected_node_id) = panel.graph.selected_nodes.first() {
            if let Some(selected_node) = panel.graph.nodes.iter().find(|n| n.id == *selected_node_id) {
                v_flex()
                    .gap_3()
                    .child(
                        div()
                            .text_lg()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child(selected_node.title.clone())
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child("Properties")
                            )
                            .child(Self::render_node_properties(selected_node, cx))
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child("Node Info")
                            )
                            .child(Self::render_node_info(selected_node, cx))
                    )
                    .into_any_element()
            } else {
                Self::render_empty_state(cx)
            }
        } else {
            Self::render_empty_state(cx)
        }
    }

    fn render_node_properties(node: &BlueprintNode, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .children(
                node.properties.iter().map(|(key, value)| {
                    Self::render_property_field(key, value, cx)
                })
            )
    }

    fn render_property_field(key: &str, value: &str, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        v_flex()
            .gap_1()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(Self::format_property_name(key))
            )
            .child(
                div()
                    .w_full()
                    .px_3()
                    .py_2()
                    .bg(cx.theme().input)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(value.to_string())
                    .cursor_pointer()
                    .hover(|style| style.border_color(cx.theme().accent))
            )
    }

    fn render_node_info(node: &BlueprintNode, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(Self::render_info_row("Node Type", &format!("{:?}", node.node_type), cx))
            .child(Self::render_info_row("Position", &format!("{:.0}, {:.0}", node.position.x, node.position.y), cx))
            .child(Self::render_info_row("Size", &format!("{:.0} × {:.0}", node.size.width, node.size.height), cx))
            .child(Self::render_info_row("Inputs", &node.inputs.len().to_string(), cx))
            .child(Self::render_info_row("Outputs", &node.outputs.len().to_string(), cx))
    }

    fn render_info_row(label: &str, value: &str, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        h_flex()
            .justify_between()
            .items_center()
            .py_1()
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(label.to_string())
            )
            .child(
                div()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(value.to_string())
            )
    }

    fn render_empty_state(cx: &mut Context<BlueprintEditorPanel>) -> AnyElement {
        div()
            .flex()
            .items_center()
            .justify_center()
            .text_color(cx.theme().muted_foreground)
            .child("No node selected")
            .into_any_element()
    }

    fn format_property_name(key: &str) -> String {
        // Convert snake_case to Title Case
        key.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<String>>()
            .join(" ")
    }
}
