use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::Button,
    h_flex, v_flex,
    ActiveTheme as _, StyledExt,
    IconName,
    context_menu::ContextMenuExt,
};

use super::*;
use super::panel::BlueprintEditorPanel;

pub struct NodeGraphRenderer;

impl NodeGraphRenderer {
    pub fn render(panel: &mut BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .bg(cx.theme().muted.opacity(0.1))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .overflow_hidden()
            .child(Self::render_grid_background(cx))
            .child(Self::render_nodes(panel, cx))
            .child(Self::render_connections(panel, cx))
            .child(Self::render_graph_controls(panel, cx))
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(|panel, event: &MouseDownEvent, _window, cx| {
                let mouse_pos = Point::new(event.position.x.0, event.position.y.0);

                // Only start panning if not connecting and not already dragging a node
                if panel.dragging_connection.is_none() && panel.dragging_node.is_none() {
                    // Start panning if not connecting
                    panel.start_panning(mouse_pos, cx);
                }
                // Click on background deselects all nodes
                panel.select_node(None, cx);
            }))
            .on_mouse_move(cx.listener(|panel, event: &MouseMoveEvent, _window, cx| {
                let mouse_pos = Point::new(event.position.x.0, event.position.y.0);

                if panel.dragging_node.is_some() {
                    let graph_pos = Self::screen_to_graph_pos(event.position, &panel.graph);
                    panel.update_drag(graph_pos, cx);
                } else if panel.dragging_connection.is_some() {
                    // Update mouse position for drag line rendering
                    panel.update_connection_drag(mouse_pos, cx);
                } else if panel.is_panning() && panel.dragging_node.is_none() {
                    // Only update panning if we're not dragging a node
                    panel.update_pan(mouse_pos, cx);
                }
            }))
            .on_mouse_up(gpui::MouseButton::Left, cx.listener(|panel, _event: &MouseUpEvent, _window, cx| {
                if panel.dragging_node.is_some() {
                    panel.end_drag(cx);
                } else if panel.dragging_connection.is_some() {
                    // Cancel connection if not dropped on a pin
                    panel.cancel_connection_drag(cx);
                } else if panel.is_panning() {
                    panel.end_panning(cx);
                }
            }))
            .on_scroll_wheel(cx.listener(|panel, event: &ScrollWheelEvent, _window, cx| {
                // Zoom with scroll wheel
                let delta_y = match event.delta {
                    ScrollDelta::Pixels(p) => p.y.0,
                    ScrollDelta::Lines(l) => l.y * 20.0, // Convert lines to pixels
                };
                panel.handle_zoom(delta_y, cx);
            }))
            .on_key_down(cx.listener(|panel, event: &KeyDownEvent, _window, cx| {
                if event.keystroke.key == "Escape" && panel.dragging_connection.is_some() {
                    panel.cancel_connection_drag(cx);
                }
            }))
    }

    fn render_grid_background(cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        // Simple grid pattern background
        div()
            .absolute()
            .inset_0()
            .child(
                div()
                    .size_full()
                    .bg(cx.theme().muted.opacity(0.05))
                    // Grid pattern would be implemented with CSS patterns or canvas
                    .child(
                        // Simple dot grid pattern
                        div()
                            .absolute()
                            .inset_0()
                            .opacity(0.3)
                            .child("") // In a real implementation, this would use CSS background patterns
                    )
            )
    }

    fn render_nodes(panel: &mut BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        // Clone the nodes so we don't hold a borrow on panel
        let nodes: Vec<BlueprintNode> = panel.graph.nodes.iter().cloned().collect();
        div()
            .absolute()
            .inset_0()
            .children(
                nodes.into_iter().map(|node| {
                    Self::render_blueprint_node(&node, panel, cx)
                })
            )
    }

    fn render_blueprint_node(node: &BlueprintNode, panel: &mut BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        let node_color = match node.node_type {
            NodeType::Event => cx.theme().danger,
            NodeType::Logic => cx.theme().primary,
            NodeType::Math => cx.theme().success,
            NodeType::Object => cx.theme().warning,
        };

        let graph_pos = Self::graph_to_screen_pos(node.position, &panel.graph);
        let node_id = node.id.clone();
        let is_dragging = panel.dragging_node.as_ref() == Some(&node.id);

        // Scale node size with zoom level
        let scaled_width = node.size.width * panel.graph.zoom_level;
        let scaled_height = node.size.height * panel.graph.zoom_level;

        div()
            .absolute()
            .left(px(graph_pos.x))
            .top(px(graph_pos.y))
            .w(px(scaled_width))
            .h(px(scaled_height))
            .child(
                v_flex()
                    .bg(cx.theme().background)
                    .border_2()
                    .border_color(if node.is_selected {
                        cx.theme().accent
                    } else {
                        node_color
                    })
                    .rounded(px(8.0 * panel.graph.zoom_level))
                    .shadow_lg()
                    .when(is_dragging, |style| {
                        style.opacity(0.8).shadow_2xl()
                    })
                    .cursor_pointer()
                    .child(
                        // Header - this is the draggable area
                        h_flex()
                            .w_full()
                            .p(px(8.0 * panel.graph.zoom_level))
                            .bg(node_color.opacity(0.2))
                            .items_center()
                            .gap(px(8.0 * panel.graph.zoom_level))
                            .child(
                                div()
                                    .text_size(px(16.0 * panel.graph.zoom_level))
                                    .child(node.icon.clone())
                            )
                            .child(
                                div()
                                    .text_size(px(14.0 * panel.graph.zoom_level))
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child(node.title.clone())
                            )
                            .on_mouse_down(gpui::MouseButton::Left, {
                                let node_id = node_id.clone();
                                cx.listener(move |panel, event: &MouseDownEvent, _window, cx| {
                                    // Select this node
                                    panel.select_node(Some(node_id.clone()), cx);

                                    // Start dragging
                                    let graph_pos = Self::screen_to_graph_pos(event.position, &panel.graph);
                                    panel.start_drag(node_id.clone(), graph_pos, cx);
                                })
                            })
                    )
                    .child(
                        // Pins
                        v_flex()
                            .p(px(8.0 * panel.graph.zoom_level))
                            .gap(px(4.0 * panel.graph.zoom_level))
                            .child(Self::render_node_pins(node, panel, cx))
                    )
                    .on_mouse_down(gpui::MouseButton::Left, {
                        let node_id = node_id.clone();
                        cx.listener(move |panel, _event: &MouseDownEvent, _window, cx| {
                            panel.select_node(Some(node_id.clone()), cx);
                        })
                    })
            )
            .into_any_element()
    }

    fn render_node_pins(node: &BlueprintNode, panel: &BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        let max_pins = std::cmp::max(node.inputs.len(), node.outputs.len());

        v_flex()
            .gap(px(4.0 * panel.graph.zoom_level))
            .children(
                (0..max_pins).map(|i| {
                    h_flex()
                        .justify_between()
                        .items_center()
                        .child(
                            // Input pin
                            if let Some(input_pin) = node.inputs.get(i) {
                                Self::render_pin(input_pin, true, &node.id, panel, cx).into_any_element()
                            } else {
                                div().w(px(12.0 * panel.graph.zoom_level)).into_any_element()
                            }
                        )
                        .child(
                            // Pin label (only show if there's a named pin)
                            if let Some(input_pin) = node.inputs.get(i) {
                                if !input_pin.name.is_empty() {
                                    div()
                                        .text_size(px(12.0 * panel.graph.zoom_level))
                                        .text_color(cx.theme().muted_foreground)
                                        .child(input_pin.name.clone())
                                        .into_any_element()
                                } else {
                                    div().into_any_element()
                                }
                            } else if let Some(output_pin) = node.outputs.get(i) {
                                if !output_pin.name.is_empty() {
                                    div()
                                        .text_size(px(12.0 * panel.graph.zoom_level))
                                        .text_color(cx.theme().muted_foreground)
                                        .child(output_pin.name.clone())
                                        .into_any_element()
                                } else {
                                    div().into_any_element()
                                }
                            } else {
                                div().into_any_element()
                            }
                        )
                        .child(
                            // Output pin
                            if let Some(output_pin) = node.outputs.get(i) {
                                Self::render_pin(output_pin, false, &node.id, panel, cx).into_any_element()
                            } else {
                                div().w(px(12.0 * panel.graph.zoom_level)).into_any_element()
                            }
                        )
                })
            )
    }

    fn render_pin(pin: &Pin, is_input: bool, node_id: &str, panel: &BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        let pin_color = match pin.data_type {
            DataType::Execution => cx.theme().muted,
            DataType::Boolean   => cx.theme().danger,
            DataType::Integer   => cx.theme().info,
            DataType::Float     => cx.theme().success,
            DataType::String    => cx.theme().warning,
            DataType::Vector    => cx.theme().primary,
            DataType::Object    => cx.theme().accent,
        };

        // Check if this pin is compatible with the current drag
        let is_compatible = if let Some(ref drag) = panel.dragging_connection {
            is_input && node_id != drag.from_node_id && pin.data_type == drag.from_pin_type
        } else {
            false
        };

        let pin_size = 12.0 * panel.graph.zoom_level;

        div()
            .size(px(pin_size))
            .bg(pin_color)
            .rounded_full()
            .border_1()
            .border_color(if is_compatible {
                cx.theme().accent
            } else {
                cx.theme().border
            })
            .when(is_compatible, |style| style.border_2().shadow_md())
            .cursor_pointer()
            .hover(|style| style.opacity(0.8))
            .when(!is_input, |div| {
                // Only output pins can start connections
                let pin_id = pin.id.clone();
                let node_id = node_id.to_string();
                div.on_mouse_down(gpui::MouseButton::Left, {
                    cx.listener(move |panel, _event: &MouseDownEvent, _window, cx| {
                        // Start connection drag from this output pin - no coordinate calculation needed
                        panel.start_connection_drag_from_pin(node_id.clone(), pin_id.clone(), cx);
                    })
                })
            })
            .when(is_input && panel.dragging_connection.is_some(), |div| {
                // Input pins become drop targets when dragging
                let pin_id = pin.id.clone();
                let node_id = node_id.to_string();
                let _pin_type = pin.data_type.clone();
                div.on_mouse_up(gpui::MouseButton::Left, {
                    cx.listener(move |panel, _event: &MouseUpEvent, _window, cx| {
                        panel.complete_connection_on_pin(node_id.clone(), pin_id.clone(), cx);
                    })
                })
            })
            .into_any_element()
    }

    fn render_connections(panel: &BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        let mut elements = Vec::new();

        // Render existing connections
        for connection in &panel.graph.connections {
            elements.push(Self::render_connection(connection, panel, cx));
        }

        // Render dragging connection if present
        if let Some(ref drag) = panel.dragging_connection {
            elements.push(Self::render_dragging_connection(drag, panel, cx));
        }

        div()
            .absolute()
            .inset_0()
            .children(elements)
    }

    fn render_connection(connection: &Connection, panel: &BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> AnyElement {
        // Find the from and to nodes
        let from_node = panel.graph.nodes.iter().find(|n| n.id == connection.from_node_id);
        let to_node = panel.graph.nodes.iter().find(|n| n.id == connection.to_node_id);

        if let (Some(from_node), Some(to_node)) = (from_node, to_node) {
            // Calculate exact pin positions
            if let (Some(from_pin_pos), Some(to_pin_pos)) = (
                Self::calculate_pin_position(from_node, &connection.from_pin_id, false, &panel.graph),
                Self::calculate_pin_position(to_node, &connection.to_pin_id, true, &panel.graph)
            ) {
                // Get pin data type for color
                let pin_color = if let Some(pin) = from_node.outputs.iter().find(|p| p.id == connection.from_pin_id) {
                    Self::get_pin_color(&pin.data_type, cx)
                } else {
                    cx.theme().primary
                };

                // Create bezier curve connection
                Self::render_bezier_connection(from_pin_pos, to_pin_pos, pin_color, cx)
            } else {
                div().into_any_element()
            }
        } else {
            div().into_any_element()
        }
    }

    fn render_dragging_connection(drag: &super::panel::ConnectionDrag, panel: &BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> AnyElement {
        // Find the from node and pin position
        if let Some(from_node) = panel.graph.nodes.iter().find(|n| n.id == drag.from_node_id) {
            if let Some(from_pin_pos) = Self::calculate_pin_position(from_node, &drag.from_pin_id, false, &panel.graph) {
                let pin_color = Self::get_pin_color(&drag.from_pin_type, cx);

                // Determine the end position - either target pin or mouse position
                let end_pos = if let Some((target_node_id, target_pin_id)) = &drag.target_pin {
                    // If hovering over a compatible pin, connect to that pin
                    if let Some(target_node) = panel.graph.nodes.iter().find(|n| n.id == *target_node_id) {
                        Self::calculate_pin_position(target_node, target_pin_id, true, &panel.graph)
                            .unwrap_or(drag.current_mouse_pos)
                    } else {
                        drag.current_mouse_pos
                    }
                } else {
                    // Default to mouse position
                    drag.current_mouse_pos
                };

                // Create bezier curve from pin to end position
                Self::render_bezier_connection(from_pin_pos, end_pos, pin_color, cx)
            } else {
                div().into_any_element()
            }
        } else {
            div().into_any_element()
        }
    }

    fn get_pin_color(data_type: &DataType, cx: &mut Context<BlueprintEditorPanel>) -> gpui::Hsla {
        match data_type {
            DataType::Execution => cx.theme().muted,
            DataType::Boolean   => cx.theme().danger,
            DataType::Integer   => cx.theme().info,
            DataType::Float     => cx.theme().success,
            DataType::String    => cx.theme().warning,
            DataType::Vector    => cx.theme().primary,
            DataType::Object    => cx.theme().accent,
        }
    }

    fn calculate_pin_position(node: &BlueprintNode, pin_id: &str, is_input: bool, graph: &BlueprintGraph) -> Option<Point<f32>> {
        // Calculate pin position in container coordinates (same as mouse events)
        let node_screen_pos = Self::graph_to_screen_pos(node.position, graph);
        let header_height = 40.0 * graph.zoom_level; // Scaled height of node header
        let pin_size = 12.0 * graph.zoom_level; // Scaled size of pin
        let pin_spacing = 20.0 * graph.zoom_level; // Scaled vertical spacing between pins
        let pin_margin = 8.0 * graph.zoom_level; // Scaled margin from node edge

        if is_input {
            // Find input pin index
            if let Some((index, _)) = node.inputs.iter().enumerate().find(|(_, pin)| pin.id == pin_id) {
                let pin_y = node_screen_pos.y + header_height + pin_margin + (index as f32 * pin_spacing) + (pin_size / 2.0);
                Some(Point::new(node_screen_pos.x, pin_y))
            } else {
                None
            }
        } else {
            // Find output pin index
            if let Some((index, _)) = node.outputs.iter().enumerate().find(|(_, pin)| pin.id == pin_id) {
                let pin_y = node_screen_pos.y + header_height + pin_margin + (index as f32 * pin_spacing) + (pin_size / 2.0);
                Some(Point::new(node_screen_pos.x + node.size.width * graph.zoom_level, pin_y))
            } else {
                None
            }
        }
    }

    fn render_bezier_connection(from_pos: Point<f32>, to_pos: Point<f32>, color: gpui::Hsla, _cx: &mut Context<BlueprintEditorPanel>) -> AnyElement {
        let distance = (to_pos.x - from_pos.x).abs();
        let control_offset = (distance * 0.4).max(50.0).min(150.0);
        let control1 = Point::new(from_pos.x + control_offset, from_pos.y);
        let control2 = Point::new(to_pos.x - control_offset, to_pos.y);

        // Render as a thicker curve using overlapping circles for better visibility
        let segments = 40;
        let mut line_segments = Vec::new();

        for i in 0..=segments {
            let t = i as f32 / segments as f32;
            let point = Self::bezier_point(from_pos, control1, control2, to_pos, t);

            // Create a thicker line by using overlapping circles
            line_segments.push(
                div()
                    .absolute()
                    .left(px(point.x - 2.0))
                    .top(px(point.y - 2.0))
                    .w(px(4.0))
                    .h(px(4.0))
                    .bg(color)
                    .rounded_full()
            );
        }

        div()
            .absolute()
            .inset_0()
            .children(line_segments)
            .into_any_element()
    }

    fn bezier_point(p0: Point<f32>, p1: Point<f32>, p2: Point<f32>, p3: Point<f32>, t: f32) -> Point<f32> {
        let u = 1.0 - t;
        let tt = t * t;
        let uu = u * u;
        let uuu = uu * u;
        let ttt = tt * t;

        Point::new(
            uuu * p0.x + 3.0 * uu * t * p1.x + 3.0 * u * tt * p2.x + ttt * p3.x,
            uuu * p0.y + 3.0 * uu * t * p1.y + 3.0 * u * tt * p2.y + ttt * p3.y,
        )
    }

    fn render_graph_controls(panel: &BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
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
                            .child(format!("Zoom: {:.0}%", panel.graph.zoom_level * 100.0))
                    )
                    .child(
                        Button::new("zoom_fit")
                            .icon(IconName::CircleCheck)
                            .tooltip("Fit to View")
                            .on_click(cx.listener(|panel, _, _window, cx| {
                                let graph = panel.get_graph_mut();
                                graph.zoom_level = 1.0;
                                graph.pan_offset = Point::new(0.0, 0.0);
                                cx.notify();
                            }))
                    )
            )
    }

    // Helper functions for coordinate conversion
    pub fn graph_to_screen_pos(graph_pos: Point<f32>, graph: &BlueprintGraph) -> Point<f32> {
        Point::new(
            (graph_pos.x + graph.pan_offset.x) * graph.zoom_level,
            (graph_pos.y + graph.pan_offset.y) * graph.zoom_level,
        )
    }

    pub fn screen_to_graph_pos(screen_pos: Point<Pixels>, graph: &BlueprintGraph) -> Point<f32> {
        Point::new(
            (screen_pos.x.0 / graph.zoom_level) - graph.pan_offset.x,
            (screen_pos.y.0 / graph.zoom_level) - graph.pan_offset.y,
        )
    }
}
