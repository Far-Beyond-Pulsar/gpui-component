use gpui::*;
use gpui_component::{
    button::Button,
    dock::{Panel, PanelEvent},
    resizable::{h_resizable, resizable_panel, ResizableState},
    h_flex, v_flex,
    ActiveTheme as _, StyledExt,
    IconName,
};
use std::collections::HashMap;

use crate::ui::shared::{Toolbar, ToolbarButton, StatusBar};
use crate::compiler::{NodeDefinition, load_all_node_definitions};

pub struct BlueprintEditorPanel {
    focus_handle: FocusHandle,
    selected_node: Option<String>,
    zoom_level: f32,
    pan_offset: (f32, f32),
    resizable_state: Entity<ResizableState>,
    node_definitions: HashMap<String, NodeDefinition>,
}

impl BlueprintEditorPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let resizable_state = ResizableState::new(cx);

        // Load node definitions
        let node_definitions = load_all_node_definitions().unwrap_or_else(|e| {
            eprintln!("Failed to load node definitions: {}", e);
            HashMap::new()
        });

        Self {
            focus_handle: cx.focus_handle(),
            selected_node: None,
            zoom_level: 1.0,
            pan_offset: (0.0, 0.0),
            resizable_state,
            node_definitions,
        }
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Toolbar::new()
            .add_button(
                ToolbarButton::new(IconName::Plus, "Add Node")
                    .tooltip("Add Node (A)")
            )
            .add_button(
                ToolbarButton::new(IconName::Copy, "Duplicate")
                    .tooltip("Duplicate Node (Ctrl+D)")
            )
            .add_button(
                ToolbarButton::new(IconName::Delete, "Delete")
                    .tooltip("Delete Node (Del)")
            )
            .add_button(
                ToolbarButton::new(IconName::Plus, "Zoom In")
                    .tooltip("Zoom In (+)")
            )
            .add_button(
                ToolbarButton::new(IconName::Minus, "Zoom Out")
                    .tooltip("Zoom Out (-)")
            )
            .add_button(
                ToolbarButton::new(IconName::CircleCheck, "Fit")
                    .tooltip("Fit to View (F)")
            )
            .render(cx)
    }

    fn render_node_library(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                    //TODO: Make this scrollable
                    .child(self.render_node_categories(cx))
            )
    }

    fn render_node_categories(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // Group nodes by category
        let mut categories: HashMap<String, Vec<&NodeDefinition>> = HashMap::new();

        for node_def in self.node_definitions.values() {
            categories
                .entry(node_def.category.clone())
                .or_insert_with(Vec::new)
                .push(node_def);
        }

        let mut v_container = v_flex().p_2().gap_3();

        // Render each category
        for (category_name, nodes) in categories.iter() {
            let node_items: Vec<(&str, &str)> = nodes
                .iter()
                .map(|node| (node.name.as_str(), node.icon.as_str()))
                .collect();

            v_container = v_container.child(
                self.render_node_category(category_name, &node_items, cx)
            );
        }

        v_container
    }

    fn render_node_category(&self, title: &str, nodes: &[(&str, &str)], cx: &mut Context<Self>) -> impl IntoElement {
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
                        nodes.iter().map(|(name, icon)| {
                            h_flex()
                                .items_center()
                                .gap_2()
                                .p_2()
                                .rounded(px(4.0))
                                .hover(|style| style.bg(cx.theme().muted.opacity(0.5)))
                                .child(icon.to_string())
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().foreground)
                                        .child((*name).to_string())
                                )
                                .into_any_element()
                        })
                    )
            )
    }

    fn render_node_graph(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .bg(cx.theme().muted.opacity(0.1))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .overflow_hidden()
            .child(self.render_grid_background(cx))
            .child(self.render_sample_nodes(cx))
            .child(self.render_graph_controls(cx))
    }

    fn render_grid_background(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // Simple grid pattern background
        div()
            .absolute()
            .inset_0()
            .child(
                div()
                    .size_full()
                    .bg(cx.theme().muted.opacity(0.05))
                    // Grid pattern would be implemented with CSS or canvas
            )
    }

    fn render_sample_nodes(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut container = div().absolute().inset_0();

        // Show some sample nodes if they exist
        if self.node_definitions.contains_key("println") {
            container = container.child(
                div()
                    .absolute()
                    .top_16()
                    .left_16()
                    .child(self.render_blueprint_node("println", cx))
            );
        }

        if self.node_definitions.contains_key("add") {
            container = container.child(
                div()
                    .absolute()
                    .top_32()
                    .left_80()
                    .child(self.render_blueprint_node("add", cx))
            );
        }

        if self.node_definitions.contains_key("branch") {
            container = container.child(
                div()
                    .absolute()
                    .top_48()
                    .left(px(144.0))
                    .child(self.render_blueprint_node("branch", cx))
            );
        }

        // Connection lines would be drawn here based on actual graph data
        container = container.child(
            div()
                .absolute()
                .top_20()
                .left_56()
                .w_6()
                .h_px()
                .bg(cx.theme().primary)
        );

        container
    }

    fn render_blueprint_node(&self, node_key: &str, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(node_def) = self.node_definitions.get(node_key) {
            let node_color = if !node_def.is_pure {
                cx.theme().danger // Impure nodes (with execution) are red
            } else {
                cx.theme().primary // Pure nodes are blue
            };

            v_flex()
                .w_48()
                .bg(cx.theme().background)
                .border_2()
                .border_color(node_color)
                .rounded(px(8.0))
                .shadow_lg()
                .child(
                    // Header
                    h_flex()
                        .w_full()
                        .p_2()
                        .bg(node_color.opacity(0.2))
                        .items_center()
                        .gap_2()
                        .child(node_def.icon.clone())
                        .child(
                            div()
                                .text_sm()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .child(node_def.name.clone())
                        )
                )
                .child(
                    // Pins
                    v_flex()
                        .p_2()
                        .gap_1()
                        .child(self.render_node_pins(node_def, cx))
                )
        } else {
            // Fallback for missing node definition
            div()
                .p_4()
                .bg(cx.theme().danger.opacity(0.2))
                .border_1()
                .border_color(cx.theme().danger)
                .rounded(px(8.0))
                .child(format!("Missing: {}", node_key))
        }
    }

    fn render_node_pins(&self, node_def: &NodeDefinition, cx: &mut Context<Self>) -> impl IntoElement {
        let max_pins = std::cmp::max(
            node_def.inputs.len() + node_def.execution_inputs.len(),
            node_def.outputs.len() + node_def.execution_outputs.len()
        );

        let mut pin_container = v_flex().gap_1();

        for i in 0..max_pins {
            let mut row = h_flex().w_full().justify_between().items_center();

            // Left side (inputs)
            if let Some(exec_input) = node_def.execution_inputs.get(i) {
                row = row.child(self.render_pin(&exec_input.name, "execution", true, cx));
            } else if let Some(input) = node_def.inputs.get(i.saturating_sub(node_def.execution_inputs.len())) {
                row = row.child(self.render_pin(&input.name, &input.data_type, true, cx));
            } else {
                row = row.child(div().size_3()); // Empty space
            }

            // Right side (outputs)
            if let Some(exec_output) = node_def.execution_outputs.get(i) {
                row = row.child(self.render_pin(&exec_output.name, "execution", false, cx));
            } else if let Some(output) = node_def.outputs.get(i.saturating_sub(node_def.execution_outputs.len())) {
                row = row.child(self.render_pin(&output.name, &output.data_type, false, cx));
            } else {
                row = row.child(div().size_3()); // Empty space
            }

            pin_container = pin_container.child(row);
        }

        pin_container
    }

    fn render_pin(&self, name: &str, data_type: &str, is_input: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let pin_color = match data_type {
            "execution" => cx.theme().danger,
            "number" => cx.theme().primary,
            "string" => cx.theme().success,
            "boolean" => cx.theme().warning,
            "vector2" | "vector3" => cx.theme().muted,
            _ => cx.theme().foreground,
        };

        div()
            .size_3()
            .bg(pin_color)
            .rounded_full()
            .border_1()
            .border_color(cx.theme().border)
    }

    fn render_graph_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .bottom_4()
            .right_4()
            .child(
                h_flex()
                    .gap_2()
                    .p_2()
                    .bg(cx.theme().background.opacity(0.9))
                    .rounded(cx.theme().radius)
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Zoom: {:.0}%", self.zoom_level * 100.0))
                    )
                    .child(
                        Button::new("zoom_fit")
                            .icon(IconName::CircleCheck)
                            .tooltip("Fit to View")
                    )
            )
    }

    fn render_properties(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                    .child(
                        if self.selected_node.is_some() {
                            v_flex()
                                .gap_3()
                                .child(
                                    div()
                                        .text_lg()
                                        .font_semibold()
                                        .text_color(cx.theme().foreground)
                                        .child("Print String")
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
                                        .child(self.render_node_properties(cx))
                                )
                                .into_any_element()
                        } else {
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(cx.theme().muted_foreground)
                                .child("No node selected")
                                .into_any_element()
                        }
                    )
            )
    }

    fn render_node_properties(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child("Message:")
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
                            .child("Hello World!")
                    )
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child("Print to Screen:")
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .size_4()
                                    .bg(cx.theme().primary)
                                    .rounded(px(2.0))
                                    .border_1()
                                    .border_color(cx.theme().border)
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().foreground)
                                    .child("Enabled")
                            )
                    )
            )
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        StatusBar::new()
            .add_left_item(format!("Nodes: {}", 2))
            .add_left_item(format!("Connections: {}", 1))
            .add_left_item("Blueprint: PlayerController")
            .add_right_item(format!("Zoom: {:.0}%", self.zoom_level * 100.0))
            .add_right_item("Visual Scripting")
            .render(cx)
    }
}

impl Panel for BlueprintEditorPanel {
    fn panel_name(&self) -> &'static str {
        "Blueprint Editor"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        div().child("Blueprint Editor").into_any_element()
    }

    fn dump(&self, _cx: &App) -> gpui_component::dock::PanelState {
        gpui_component::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}

impl Focusable for BlueprintEditorPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for BlueprintEditorPanel {}

impl Render for BlueprintEditorPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_toolbar(cx))
            .child(
                div()
                    .flex_1()
                    .child(
                        h_resizable("blueprint-editor-panels", self.resizable_state.clone())
                            .child(
                                resizable_panel()
                                    .size(px(260.))
                                    .size_range(px(200.)..px(400.))
                                    .child(
                                        div()
                                            .size_full()
                                            .bg(cx.theme().sidebar)
                                            .border_1()
                                            .border_color(cx.theme().border)
                                            .rounded(cx.theme().radius)
                                            .p_2()
                                            .child(self.render_node_library(cx))
                                    )
                            )
                            .child(
                                resizable_panel()
                                    .child(
                                        div()
                                            .size_full()
                                            .p_2()
                                            .child(self.render_node_graph(cx))
                                    )
                            )
                            .child(
                                resizable_panel()
                                    .size(px(320.))
                                    .size_range(px(250.)..px(500.))
                                    .child(
                                        div()
                                            .size_full()
                                            .bg(cx.theme().sidebar)
                                            .border_1()
                                            .border_color(cx.theme().border)
                                            .rounded(cx.theme().radius)
                                            .p_2()
                                            .child(self.render_properties(cx))
                                    )
                            )
                    )
            )
            .child(self.render_status_bar(cx))
    }
}