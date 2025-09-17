use gpui::*;
use gpui_component::{
    dock::{Panel, PanelEvent},
    resizable::{h_resizable, resizable_panel, ResizableState},
    v_flex,
    ActiveTheme as _, StyledExt,
    context_menu::ContextMenuExt,
};

use super::*;
use super::toolbar::ToolbarRenderer;
use super::node_library::NodeLibraryRenderer;
use super::node_graph::NodeGraphRenderer;
use super::properties::PropertiesRenderer;

pub struct BlueprintEditorPanel {
    focus_handle: FocusHandle,
    pub graph: BlueprintGraph,
    resizable_state: Entity<ResizableState>,
    // Drag state
    pub dragging_node: Option<String>,
    pub drag_offset: Point<f32>,
    // Connection drag state
    pub dragging_connection: Option<ConnectionDrag>,
    // Panning state
    pub is_panning: bool,
    pub pan_start: Point<f32>,
    pub pan_start_offset: Point<f32>,
}

#[derive(Clone, Debug)]
pub struct ConnectionDrag {
    pub from_node_id: String,
    pub from_pin_id: String,
    pub from_pin_type: super::DataType,
    pub current_mouse_pos: Point<f32>,
    pub target_pin: Option<(String, String)>, // (node_id, pin_id)
}

impl BlueprintEditorPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let resizable_state = ResizableState::new(cx);

        // Create sample nodes
        let mut nodes = Vec::new();

        // Begin Play event node
        nodes.push(BlueprintNode {
            id: "begin_play".to_string(),
            title: "Begin Play".to_string(),
            icon: "▶️".to_string(),
            node_type: NodeType::Event,
            position: Point::new(100.0, 100.0),
            size: Size::new(192.0, 80.0),
            inputs: vec![],
            outputs: vec![Pin {
                id: "exec_out".to_string(),
                name: "".to_string(),
                pin_type: PinType::Output,
                data_type: DataType::Execution,
            }],
            properties: std::collections::HashMap::new(),
            is_selected: false,
        });

        // Print String node
        let mut print_props = std::collections::HashMap::new();
        print_props.insert("message".to_string(), "Hello World!".to_string());
        print_props.insert("print_to_screen".to_string(), "true".to_string());

        nodes.push(BlueprintNode {
            id: "print_string".to_string(),
            title: "Print String".to_string(),
            icon: "📝".to_string(),
            node_type: NodeType::Logic,
            position: Point::new(400.0, 100.0),
            size: Size::new(192.0, 120.0),
            inputs: vec![
                Pin {
                    id: "exec_in".to_string(),
                    name: "".to_string(),
                    pin_type: PinType::Input,
                    data_type: DataType::Execution,
                },
                Pin {
                    id: "text_in".to_string(),
                    name: "In String".to_string(),
                    pin_type: PinType::Input,
                    data_type: DataType::String,
                },
            ],
            outputs: vec![Pin {
                id: "exec_out".to_string(),
                name: "".to_string(),
                pin_type: PinType::Output,
                data_type: DataType::Execution,
            }],
            properties: print_props,
            is_selected: false,
        });

        let connections = vec![Connection {
            id: "connection_1".to_string(),
            from_node_id: "begin_play".to_string(),
            from_pin_id: "exec_out".to_string(),
            to_node_id: "print_string".to_string(),
            to_pin_id: "exec_in".to_string(),
        }];

        let graph = BlueprintGraph {
            nodes,
            connections,
            selected_nodes: vec![],
            zoom_level: 1.0,
            pan_offset: Point::new(0.0, 0.0),
        };

        Self {
            focus_handle: cx.focus_handle(),
            graph,
            resizable_state,
            dragging_node: None,
            drag_offset: Point::new(0.0, 0.0),
            dragging_connection: None,
            is_panning: false,
            pan_start: Point::new(0.0, 0.0),
            pan_start_offset: Point::new(0.0, 0.0),
        }
    }

    pub fn get_graph(&self) -> &BlueprintGraph {
        &self.graph
    }

    pub fn get_graph_mut(&mut self) -> &mut BlueprintGraph {
        &mut self.graph
    }

    pub fn add_node(&mut self, node: BlueprintNode, cx: &mut Context<Self>) {
        self.graph.nodes.push(node);
        cx.notify();
    }

    pub fn select_node(&mut self, node_id: Option<String>, cx: &mut Context<Self>) {
        match node_id {
            Some(id) => {
                self.graph.selected_nodes.clear();
                self.graph.selected_nodes.push(id.clone());
                for node in &mut self.graph.nodes {
                    node.is_selected = node.id == id;
                }
            }
            None => {
                self.graph.selected_nodes.clear();
                for node in &mut self.graph.nodes {
                    node.is_selected = false;
                }
            }
        }
        cx.notify();
    }

    pub fn start_drag(&mut self, node_id: String, mouse_pos: Point<f32>, cx: &mut Context<Self>) {
        if let Some(node) = self.graph.nodes.iter().find(|n| n.id == node_id) {
            self.dragging_node = Some(node_id);
            self.drag_offset = Point::new(
                mouse_pos.x - node.position.x,
                mouse_pos.y - node.position.y,
            );
            cx.notify();
        }
    }

    pub fn update_drag(&mut self, mouse_pos: Point<f32>, cx: &mut Context<Self>) {
        if let Some(dragging_id) = &self.dragging_node {
            if let Some(node) = self.graph.nodes.iter_mut().find(|n| n.id == *dragging_id) {
                node.position = Point::new(
                    mouse_pos.x - self.drag_offset.x,
                    mouse_pos.y - self.drag_offset.y,
                );
                cx.notify();
            }
        }
    }

    pub fn end_drag(&mut self, cx: &mut Context<Self>) {
        self.dragging_node = None;
        cx.notify();
    }

    pub fn duplicate_node(&mut self, node_id: String, cx: &mut Context<Self>) {
        if let Some(node) = self.graph.nodes.iter().find(|n| n.id == node_id).cloned() {
            let mut new_node = node;
            new_node.id = uuid::Uuid::new_v4().to_string();
            new_node.position.x += 20.0; // Offset the duplicate slightly
            new_node.position.y += 20.0;
            new_node.is_selected = false;
            self.graph.nodes.push(new_node);
            cx.notify();
        }
    }

    pub fn delete_node(&mut self, node_id: String, cx: &mut Context<Self>) {
        // Remove the node
        self.graph.nodes.retain(|n| n.id != node_id);

        // Remove any connections involving this node
        self.graph.connections.retain(|conn| {
            conn.from_node_id != node_id && conn.to_node_id != node_id
        });

        // Remove from selected nodes
        self.graph.selected_nodes.retain(|id| *id != node_id);

        cx.notify();
    }

    pub fn copy_node(&mut self, node_id: String, _cx: &mut Context<Self>) {
        // For now, just store in a simple static location
        // In a real implementation, this would use the system clipboard
        if let Some(node) = self.graph.nodes.iter().find(|n| n.id == node_id) {
            // TODO: Store node in clipboard
            println!("Copied node: {}", node.title);
        }
    }

    pub fn paste_node(&mut self, cx: &mut Context<Self>) {
        // TODO: Paste from clipboard
        println!("Paste node not yet implemented");
        cx.notify();
    }

    pub fn disconnect_pin(&mut self, node_id: String, pin_id: String, cx: &mut Context<Self>) {
        self.graph.connections.retain(|conn| {
            !(conn.from_node_id == node_id && conn.from_pin_id == pin_id) &&
            !(conn.to_node_id == node_id && conn.to_pin_id == pin_id)
        });
        cx.notify();
    }

    pub fn start_connection_drag_from_pin(&mut self, node_id: String, pin_id: String, cx: &mut Context<Self>) {
        // Find the pin to get its data type
        if let Some(node) = self.graph.nodes.iter().find(|n| n.id == node_id) {
            if let Some(pin) = node.outputs.iter().find(|p| p.id == pin_id) {
                println!("Starting connection drag from pin {} on node {}", pin_id, node_id);
                self.dragging_connection = Some(ConnectionDrag {
                    from_node_id: node_id,
                    from_pin_id: pin_id,
                    from_pin_type: pin.data_type.clone(),
                    current_mouse_pos: Point::new(0.0, 0.0), // Will be updated by mouse move
                    target_pin: None,
                });
                cx.notify();
            }
        }
    }

    pub fn update_connection_drag(&mut self, mouse_pos: Point<f32>, cx: &mut Context<Self>) {
        if let Some(ref mut drag) = self.dragging_connection {
            drag.current_mouse_pos = mouse_pos;
            cx.notify();
        }
    }


    pub fn cancel_connection_drag(&mut self, cx: &mut Context<Self>) {
        self.dragging_connection = None;
        cx.notify();
    }

    pub fn set_connection_target(&mut self, target: Option<(String, String)>, cx: &mut Context<Self>) {
        if let Some(ref mut drag) = self.dragging_connection {
            drag.target_pin = target;
            cx.notify();
        }
    }

    pub fn complete_connection_on_pin(&mut self, node_id: String, pin_id: String, cx: &mut Context<Self>) {
        if let Some(drag) = self.dragging_connection.take() {
            // Find the target pin to check compatibility
            if let Some(node) = self.graph.nodes.iter().find(|n| n.id == node_id) {
                if let Some(pin) = node.inputs.iter().find(|p| p.id == pin_id) {
                    // Check if compatible and not same node
                    if drag.from_pin_type == pin.data_type && drag.from_node_id != node_id {
                        // Additional validation: only one connection per input pin (except execution pins)
                        let can_connect = if pin.data_type == super::DataType::Execution {
                            true // Execution pins can have multiple connections
                        } else {
                            // Non-execution pins can only have one input connection
                            !self.graph.connections.iter().any(|conn|
                                conn.to_node_id == node_id && conn.to_pin_id == pin_id
                            )
                        };

                        if can_connect {
                            println!("Creating connection from {} to {}", drag.from_pin_id, pin_id);
                            let connection = super::Connection {
                                id: uuid::Uuid::new_v4().to_string(),
                                from_node_id: drag.from_node_id,
                                from_pin_id: drag.from_pin_id,
                                to_node_id: node_id,
                                to_pin_id: pin_id,
                            };
                            self.graph.connections.push(connection);
                            println!("Connection created successfully!");
                        } else {
                            println!("Input pin already has a connection");
                        }
                    } else {
                        println!("Incompatible pin types or same node");
                    }
                }
            }
            cx.notify();
        }
    }

    // Panning methods
    pub fn start_panning(&mut self, start_pos: Point<f32>, cx: &mut Context<Self>) {
        self.is_panning = true;
        self.pan_start = start_pos;
        self.pan_start_offset = self.graph.pan_offset;
        cx.notify();
    }

    pub fn is_panning(&self) -> bool {
        self.is_panning
    }

    pub fn update_pan(&mut self, current_pos: Point<f32>, cx: &mut Context<Self>) {
        if self.is_panning {
            let delta = Point::new(
                current_pos.x - self.pan_start.x,
                current_pos.y - self.pan_start.y,
            );
            self.graph.pan_offset = Point::new(
                self.pan_start_offset.x + delta.x / self.graph.zoom_level,
                self.pan_start_offset.y + delta.y / self.graph.zoom_level,
            );
            cx.notify();
        }
    }

    pub fn end_panning(&mut self, cx: &mut Context<Self>) {
        self.is_panning = false;
        cx.notify();
    }

    // Zooming methods
    // Screen position is the cursor position in pixels; the function computes the graph/world
    // coordinates under the cursor using the current zoom and pan, then adjusts pan_offset
    // so that after zooming the same graph point remains under the cursor (zoom around mouse).
    pub fn handle_zoom(&mut self, delta_y: f32, screen_pos: Point<Pixels>, cx: &mut Context<Self>) {
        // Convert screen pixels to f32 point
        let screen = Point::new(screen_pos.x.0, screen_pos.y.0);

        // Compute graph/world position under cursor before zoom using the shared helper
        // (keeps conversion identical to other codepaths that use this helper)
        let focus_graph_pos = super::node_graph::NodeGraphRenderer::screen_to_graph_pos(
            Point::new(gpui::Pixels(screen.x), gpui::Pixels(screen.y)),
            &self.graph,
        );

        let zoom_factor = if delta_y > 0.0 { 0.9 } else { 1.1 };
        let new_zoom = (self.graph.zoom_level * zoom_factor).clamp(0.1, 3.0);

        // Use an equivalent delta-based formula that is numerically stable and avoids
        // inconsistencies with other conversion helpers:
        // new_pan = old_pan + screen * (1/new_zoom - 1/old_zoom)
        // Derivation: focus = (screen/old_zoom) - old_pan; plug into new_pan formula.
        let old_zoom = self.graph.zoom_level;
        let old_pan = self.graph.pan_offset;

        // DEBUG: print diagnostic info to help trace why zoom isn't centering
        println!("[ZOOM DEBUG] screen=({},{}), focus_graph=({},{}), old_zoom={}, old_pan=({},{}), delta_y={}",
            screen.x, screen.y,
            focus_graph_pos.x, focus_graph_pos.y,
            old_zoom,
            old_pan.x, old_pan.y,
            delta_y
        );

        // Compute new pan so the focused graph point stays under the cursor:
        // screen = (focus + pan_new) * new_zoom => pan_new = (screen / new_zoom) - focus
        // Initial pan calculation that should keep focus under cursor
        let mut new_pan_offset = Point::new(
            (screen.x / new_zoom) - focus_graph_pos.x,
            (screen.y / new_zoom) - focus_graph_pos.y,
        );

        // Apply temporarily to measure any residual offset that may come from
        // coordinate-space differences (padding, layout origin, DPI, etc.). We'll
        // then correct the pan by subtracting the measured screen diff divided by
        // the new zoom (since pan is in graph-space units).
        let old_zoom = self.graph.zoom_level;
        let old_pan = self.graph.pan_offset;

        self.graph.zoom_level = new_zoom;
        self.graph.pan_offset = new_pan_offset;

        let screen_after = super::node_graph::NodeGraphRenderer::graph_to_screen_pos(focus_graph_pos, &self.graph);
        let diff_x = screen_after.x - screen.x;
        let diff_y = screen_after.y - screen.y;

        // Correct pan by removing the measured diffusion in graph-space
        new_pan_offset.x -= diff_x / new_zoom;
        new_pan_offset.y -= diff_y / new_zoom;

        // Commit corrected values
        self.graph.zoom_level = new_zoom;
        self.graph.pan_offset = new_pan_offset;

        // Debug log to help verify correctness
        println!(
            "[ZOOM DEBUG] screen_before=({:.2},{:.2}), screen_after=({:.2},{:.2}), diff=({:.2},{:.2}), new_zoom={:.3}, new_pan=({:.3},{:.3}), old_zoom={:.3}, old_pan=({:.3},{:.3})",
            screen.x,
            screen.y,
            screen_after.x - diff_x,
            screen_after.y - diff_y,
            diff_x,
            diff_y,
            new_zoom,
            new_pan_offset.x,
            new_pan_offset.y,
            old_zoom,
            old_pan.x,
            old_pan.y
        );

        cx.notify();
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
            .on_action(cx.listener(|panel, action: &DuplicateNode, _window, cx| {
                panel.duplicate_node(action.node_id.clone(), cx);
            }))
            .on_action(cx.listener(|panel, action: &DeleteNode, _window, cx| {
                panel.delete_node(action.node_id.clone(), cx);
            }))
            .on_action(cx.listener(|panel, action: &CopyNode, _window, cx| {
                panel.copy_node(action.node_id.clone(), cx);
            }))
            .on_action(cx.listener(|panel, _action: &PasteNode, _window, cx| {
                panel.paste_node(cx);
            }))
            .on_action(cx.listener(|panel, action: &DisconnectPin, _window, cx| {
                panel.disconnect_pin(action.node_id.clone(), action.pin_id.clone(), cx);
            }))
            .child(ToolbarRenderer::render(self, cx))
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
                                            .child(NodeLibraryRenderer::render(self, cx))
                                    )
                            )
                            .child(
                                resizable_panel()
                                    .child(
                                        div()
                                            .size_full()
                                            .p_2()
                                            .child(NodeGraphRenderer::render(self, cx))
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
                                            .child(PropertiesRenderer::render(self, cx))
                                    )
                            )
                    )
            )
    }
}
