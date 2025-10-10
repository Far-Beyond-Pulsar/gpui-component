use gpui::*;
use gpui_component::{
    h_flex, v_flex,
    ActiveTheme as _, StyledExt, Colorize,
    IconName,
};

use super::*;
use super::panel::BlueprintEditorPanel;

pub struct PropertiesRenderer;

impl PropertiesRenderer {
    pub fn render(panel: &BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().sidebar)
            .child(
                // STUDIO-QUALITY HEADER (Unreal Details panel style)
                v_flex()
                    .w_full()
                    .child(
                        // Main header with professional styling
                        h_flex()
                            .w_full()
                            .px_4()
                            .py_3()
                            .bg(cx.theme().secondary)
                            .border_b_2()
                            .border_color(cx.theme().border)
                            .items_center()
                            .gap_3()
                            .child(
                                // Icon container with glow
                                div()
                                    .flex_shrink_0()
                                    .w(px(32.0))
                                    .h(px(32.0))
                                    .rounded(px(6.0))
                                    .bg(cx.theme().info.opacity(0.15))
                                    .border_1()
                                    .border_color(cx.theme().info.opacity(0.3))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child(
                                        div()
                                            .text_lg()
                                            .child("🔍")
                                    )
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_bold()
                                            .text_color(cx.theme().foreground)
                                            .child("Details")
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(if panel.graph.selected_nodes.len() > 1 {
                                                format!("{} items selected", panel.graph.selected_nodes.len())
                                            } else if panel.graph.selected_nodes.len() == 1 {
                                                "1 item selected".to_string()
                                            } else {
                                                "Nothing selected".to_string()
                                            })
                                    )
                            )
                    )
                    .child(
                        // Selection type indicator bar
                        h_flex()
                            .w_full()
                            .px_4()
                            .py_2()
                            .bg(cx.theme().sidebar.darken(0.03))
                            .border_b_1()
                            .border_color(cx.theme().border.opacity(0.3))
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_semibold()
                                            .text_color(cx.theme().info)
                                            .child(if panel.graph.selected_nodes.len() > 1 {
                                                "MULTIPLE SELECTION"
                                            } else if panel.graph.selected_nodes.len() == 1 {
                                                "NODE PROPERTIES"
                                            } else {
                                                "NO SELECTION"
                                            })
                                    )
                            )
                            .child(
                                if !panel.graph.selected_nodes.is_empty() {
                                    div()
                                        .px_2()
                                        .py_1()
                                        .rounded(px(4.0))
                                        .bg(cx.theme().info.opacity(0.15))
                                        .text_xs()
                                        .font_family("JetBrainsMono-Regular")
                                        .text_color(cx.theme().info)
                                        .child(format!("{}", panel.graph.selected_nodes.len()))
                                } else {
                                    div() // Empty div when no selection
                                }
                            )
                    )
            )
            .child(
                // CONTENT AREA - clean scrollable content
                v_flex()
                    .flex_1()
                    .overflow_hidden()
                    .p_3()
                    .scrollable(Axis::Vertical)
                    .child(Self::render_properties_content(panel, cx))
            )
    }

    fn render_properties_content(panel: &BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        if let Some(selected_node_id) = panel.graph.selected_nodes.first() {
            if let Some(selected_node) = panel.graph.nodes.iter().find(|n| n.id == *selected_node_id) {
                v_flex()
                    .gap_4()
                    .child(
                        // Node header with icon and type badge
                        v_flex()
                            .gap_2()
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_3()
                                    .child(
                                        div()
                                            .text_2xl()
                                            .child(selected_node.icon.clone())
                                    )
                                    .child(
                                        div()
                                            .text_lg()
                                            .font_bold()
                                            .text_color(cx.theme().foreground)
                                            .child(selected_node.title.clone())
                                    )
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded(px(4.0))
                                    .bg(Self::get_node_type_color(&selected_node.node_type, cx).opacity(0.15))
                                    .border_1()
                                    .border_color(Self::get_node_type_color(&selected_node.node_type, cx).opacity(0.3))
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(Self::get_node_type_color(&selected_node.node_type, cx))
                                    .child(format!("{:?} Node", selected_node.node_type))
                            )
                    )
                    .child(Self::render_separator(cx))
                    .child(
                        // Properties section
                        v_flex()
                            .gap_3()
                            .child(Self::render_section_header("Properties", IconName::Settings, cx))
                            .child(Self::render_node_properties(selected_node, cx))
                    )
                    .child(Self::render_separator(cx))
                    .child(
                        // Node info section
                        v_flex()
                            .gap_3()
                            .child(Self::render_section_header("Node Info", IconName::Info, cx))
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

    fn render_section_header(title: &str, icon: IconName, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        h_flex()
            .items_center()
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .font_bold()
                    .text_color(cx.theme().accent)
                    .child(title.to_uppercase())
            )
    }

    fn render_separator(cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        div()
            .w_full()
            .h_px()
            .bg(cx.theme().border.opacity(0.3))
    }

    fn get_node_type_color(node_type: &NodeType, cx: &mut Context<BlueprintEditorPanel>) -> gpui::Hsla {
        match node_type {
            NodeType::Event => cx.theme().danger,
            NodeType::Logic => cx.theme().primary,
            NodeType::Math => cx.theme().success,
            NodeType::Object => cx.theme().warning,
            NodeType::Reroute => cx.theme().accent,
        }
    }

    fn render_empty_state(cx: &mut Context<BlueprintEditorPanel>) -> AnyElement {
        v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .gap_3()
            .child(
                div()
                    .text_xl()
                    .child("📋📋📋")
            )
            .child(
                div()
                    .text_sm()
                    .font_medium()
                    .text_color(cx.theme().muted_foreground)
                    .child("No node selected")
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground.opacity(0.7))
                    .child("Select a node to view its properties")
            )
            .into_any_element()
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
            .gap_2()
            .child(
                div()
                    .text_xs()
                    .font_semibold()
                    .text_color(cx.theme().muted_foreground)
                    .child(Self::format_property_name(key))
            )
            .child(
                div()
                    .w_full()
                    .px_3()
                    .py_2p5()
                    .bg(cx.theme().input)
                    .border_1()
                    .border_color(cx.theme().border.opacity(0.6))
                    .rounded(px(6.0))
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(value.to_string())
                    .cursor_pointer()
                    .hover(|style| {
                        style
                            .border_color(cx.theme().accent.opacity(0.8))
                            .bg(cx.theme().input.lighten(0.02))
                    })
            )
    }

    fn render_node_info(node: &BlueprintNode, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        v_flex()
            .gap_2p5()
            .child(Self::render_info_row("Node ID", &node.id, cx))
            .child(Self::render_info_row("Position", &format!("({:.0}, {:.0})", node.position.x, node.position.y), cx))
            .child(Self::render_info_row("Size", &format!("{:.0} × {:.0} px", node.size.width, node.size.height), cx))
            .child(Self::render_separator(cx))
            .child(Self::render_info_row("Input Pins", &node.inputs.len().to_string(), cx))
            .child(Self::render_info_row("Output Pins", &node.outputs.len().to_string(), cx))
    }

    fn render_info_row(label: &str, value: &str, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        h_flex()
            .w_full()
            .justify_between()
            .items_center()
            .px_3()
            .py_2()
            .rounded(px(4.0))
            .hover(|style| style.bg(cx.theme().muted.opacity(0.1)))
            .child(
                div()
                    .text_xs()
                    .font_medium()
                    .text_color(cx.theme().muted_foreground)
                    .child(label.to_string())
            )
            .child(
                div()
                    .px_2()
                    .py_1()
                    .rounded(px(4.0))
                    .bg(cx.theme().muted.opacity(0.2))
                    .text_xs()
                    .font_family("JetBrainsMono-Regular")
                    .text_color(cx.theme().foreground)
                    .child(value.to_string())
            )
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
