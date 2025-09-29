use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    dock::{Panel, PanelEvent},
    resizable::{h_resizable, resizable_panel, ResizableState},
    v_flex,
    ActiveTheme as _, StyledExt,
    context_menu::ContextMenuExt,
    input::{InputState, InputEvent},
};

use super::*;
use super::toolbar::ToolbarRenderer;
use super::node_graph::NodeGraphRenderer;
use super::properties::PropertiesRenderer;
use super::node_creation_menu::{NodeCreationMenu, NodeCreationEvent};
use crate::graph::{GraphDescription, DataType as GraphDataType};

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
    // Selection state
    pub selection_start: Option<Point<f32>>,
    pub selection_end: Option<Point<f32>>,
    pub last_mouse_pos: Option<Point<f32>>,
    // Node creation menu
    pub node_creation_menu: Option<Entity<NodeCreationMenu>>,
}

#[derive(Clone, Debug)]
pub struct ConnectionDrag {
    pub from_node_id: String,
    pub from_pin_id: String,
    pub from_pin_type: GraphDataType,
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
                data_type: GraphDataType::from_type_str("execution"),
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
                    data_type: GraphDataType::from_type_str("execution"),
                },
                Pin {
                    id: "message".to_string(),
                    name: "Message".to_string(),
                    pin_type: PinType::Input,
                    data_type: GraphDataType::from_type_str("string"),
                },
            ],
            outputs: vec![Pin {
                id: "exec_out".to_string(),
                name: "".to_string(),
                pin_type: PinType::Output,
                data_type: GraphDataType::from_type_str("execution"),
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
            virtualization_stats: super::VirtualizationStats::default(),
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
            selection_start: None,
            selection_end: None,
            last_mouse_pos: None,
            node_creation_menu: None,
        }
    }

    pub fn get_graph(&self) -> &BlueprintGraph {
        &self.graph
    }

    pub fn get_graph_mut(&mut self) -> &mut BlueprintGraph {
        &mut self.graph
    }

    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }



    pub fn add_node(&mut self, node: BlueprintNode, cx: &mut Context<Self>) {
        self.graph.nodes.push(node);
        cx.notify();
    }

    /// Compile the current graph to Rust source code
    pub fn compile_to_rust(&self) -> Result<String, String> {
        // Convert blueprint graph to our graph description format
        let graph_description = self.convert_to_graph_description()?;

        // Create compiler and compile
        let compiler = crate::compiler::create_graph_compiler()?;
        compiler.compile_graph(&graph_description)
    }

    /// Convert blueprint graph to graph description format
    fn convert_to_graph_description(&self) -> Result<crate::graph::GraphDescription, String> {
        use crate::graph::*;
        use std::collections::HashMap;

        let mut graph_desc = GraphDescription::new("Blueprint Graph");

        // Convert nodes
        for bp_node in &self.graph.nodes {
            let mut node_instance = NodeInstance::new(
                &bp_node.id,
                &self.get_node_type_from_blueprint(&bp_node)?,
                Position { x: bp_node.position.x, y: bp_node.position.y }
            );

            // Convert pins
            for pin in &bp_node.inputs {
                // Pin data types are already in the unified format
                node_instance.add_input_pin(&pin.id, pin.data_type.clone());
            }

            for pin in &bp_node.outputs {
                // Pin data types are already in the unified format
                node_instance.add_output_pin(&pin.id, pin.data_type.clone());
            }

            // Convert properties
            for (key, value) in &bp_node.properties {
                let prop_value = if value.parse::<f64>().is_ok() {
                    PropertyValue::Number(value.parse().unwrap())
                } else if value.parse::<bool>().is_ok() {
                    PropertyValue::Boolean(value.parse().unwrap())
                } else {
                    PropertyValue::String(value.clone())
                };
                node_instance.set_property(key, prop_value);
            }

            graph_desc.add_node(node_instance);
        }

        // Convert connections
        for connection in &self.graph.connections {
            let graph_connection = Connection::new(
                &connection.id,
                &connection.from_node_id,
                &connection.from_pin_id,
                &connection.to_node_id,
                &connection.to_pin_id,
                ConnectionType::Execution, // Determine type based on pins
            );
            graph_desc.add_connection(graph_connection);
        }

        Ok(graph_desc)
    }

    fn get_node_type_from_blueprint(&self, bp_node: &BlueprintNode) -> Result<String, String> {
        // Try to find the original node type from definitions
        let node_definitions = NodeDefinitions::load();

        // Search through all categories for a matching node
        for category in &node_definitions.categories {
            for node_def in &category.nodes {
                if node_def.name == bp_node.title {
                    return Ok(node_def.id.clone());
                }
            }
        }

        // Fallback: use title as type
        Ok(bp_node.title.replace(" ", "_").to_lowercase())
    }

    // Conversion function no longer needed since we use the unified DataType system

    /// Save the current graph to a JSON file
    pub fn save_blueprint(&self, file_path: &str) -> Result<(), String> {
        let graph_description = self.convert_to_graph_description()?;
        let json = serde_json::to_string_pretty(&graph_description)
            .map_err(|e| format!("Failed to serialize graph: {}", e))?;

        std::fs::write(file_path, json)
            .map_err(|e| format!("Failed to write file: {}", e))?;

        Ok(())
    }

    /// Load a graph from a JSON file
    pub fn load_blueprint(&mut self, file_path: &str, cx: &mut Context<Self>) -> Result<(), String> {
        let json = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        let graph_description: crate::graph::GraphDescription = serde_json::from_str(&json)
            .map_err(|e| format!("Failed to parse JSON: {}", e))?;

        // Convert back to blueprint format
        self.graph = self.convert_from_graph_description(&graph_description)?;
        cx.notify();

        Ok(())
    }

    fn convert_from_graph_description(&self, graph_desc: &crate::graph::GraphDescription) -> Result<BlueprintGraph, String> {
        let mut nodes = Vec::new();
        let mut connections = Vec::new();

        // Convert nodes
        for (node_id, node_instance) in &graph_desc.nodes {
            let bp_node = BlueprintNode {
                id: node_id.clone(),
                title: node_instance.node_type.replace('_', " "),
                icon: "⚙️".to_string(), // Default icon
                node_type: NodeType::Logic, // Default type
                position: Point::new(node_instance.position.x, node_instance.position.y),
                size: Size::new(150.0, 100.0),
                inputs: node_instance.inputs.iter().map(|(pin_id, pin)| Pin {
                    id: pin_id.clone(),
                    name: pin.name.clone(),
                    pin_type: PinType::Input,
                    data_type: pin.data_type.clone(),
                }).collect(),
                outputs: node_instance.outputs.iter().map(|(pin_id, pin)| Pin {
                    id: pin_id.clone(),
                    name: pin.name.clone(),
                    pin_type: PinType::Output,
                    data_type: pin.data_type.clone(),
                }).collect(),
                properties: node_instance.properties.iter().map(|(k, v)| {
                    let value_str = match v {
                        crate::graph::PropertyValue::String(s) => s.clone(),
                        crate::graph::PropertyValue::Number(n) => n.to_string(),
                        crate::graph::PropertyValue::Boolean(b) => b.to_string(),
                        _ => "".to_string(),
                    };
                    (k.clone(), value_str)
                }).collect(),
                is_selected: false,
            };
            nodes.push(bp_node);
        }

        // Convert connections
        for connection in &graph_desc.connections {
            let bp_connection = Connection {
                id: connection.id.clone(),
                from_node_id: connection.source_node.clone(),
                from_pin_id: connection.source_pin.clone(),
                to_node_id: connection.target_node.clone(),
                to_pin_id: connection.target_pin.clone(),
            };
            connections.push(bp_connection);
        }

        Ok(BlueprintGraph {
            nodes,
            connections,
            selected_nodes: vec![],
            zoom_level: 1.0,
            pan_offset: Point::new(0.0, 0.0),
            virtualization_stats: VirtualizationStats::default(),
        })
    }

    // Conversion function no longer needed since we use the unified DataType system


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
                        let is_execution_pin = pin.data_type == GraphDataType::from_type_str("execution");

                        // Handle execution pin connection logic
                        if is_execution_pin {
                            // For execution pins, implement special connection moving logic

                            // 1. Remove any existing connection from the same source exec output pin
                            //    (only one exec out connection at a time)
                            self.graph.connections.retain(|conn| {
                                !(conn.from_node_id == drag.from_node_id && conn.from_pin_id == drag.from_pin_id)
                            });

                            // 2. For exec input pins: When reconnecting, move the connection
                            //    (remove existing connections to this input pin)
                            self.graph.connections.retain(|conn| {
                                !(conn.to_node_id == node_id && conn.to_pin_id == pin_id)
                            });

                            println!("Creating exec connection from {}:{} to {}:{}",
                                     drag.from_node_id, drag.from_pin_id, node_id, pin_id);
                        } else {
                            // For non-execution pins, only allow one input connection
                            let already_connected = self.graph.connections.iter().any(|conn|
                                conn.to_node_id == node_id && conn.to_pin_id == pin_id
                            );

                            if already_connected {
                                println!("Input pin already has a connection");
                                cx.notify();
                                return;
                            }

                            println!("Creating data connection from {}:{} to {}:{}",
                                     drag.from_node_id, drag.from_pin_id, node_id, pin_id);
                        }

                        // Create the new connection
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
                        println!("Incompatible pin types or same node");
                    }
                }
            }
            cx.notify();
        }
    }

    // Node creation menu methods
    pub fn show_node_creation_menu(&mut self, position: Point<f32>, cx: &mut Context<Self>) {
        if self.node_creation_menu.is_some() {
            self.dismiss_node_creation_menu(cx);
        }

        let menu = cx.new(|cx| NodeCreationMenu::new(position, cx));
        self.node_creation_menu = Some(menu);
        cx.notify();
    }

    pub fn dismiss_node_creation_menu(&mut self, cx: &mut Context<Self>) {
        self.node_creation_menu = None;
        cx.notify();
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

    // Swap scroll direction: invert the zoom factor mapping so wheel delta
    // signs produce the opposite zoom direction than before.
    let zoom_factor = if delta_y > 0.0 { 1.1 } else { 0.9 };
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

    // Selection methods
    pub fn select_node(&mut self, node_id: Option<String>, cx: &mut Context<Self>) {
        self.graph.selected_nodes.clear();
        if let Some(id) = node_id {
            self.graph.selected_nodes.push(id);
        }
        cx.notify();
    }

    pub fn start_selection_drag(&mut self, start_pos: Point<f32>, add_to_selection: bool, cx: &mut Context<Self>) {
        self.selection_start = Some(start_pos);
        self.selection_end = Some(start_pos);

        if !add_to_selection {
            self.graph.selected_nodes.clear();
        }
        cx.notify();
    }

    pub fn is_selecting(&self) -> bool {
        self.selection_start.is_some() && self.selection_end.is_some()
    }

    pub fn update_selection_drag(&mut self, current_pos: Point<f32>, cx: &mut Context<Self>) {
        if self.selection_start.is_some() {
            self.selection_end = Some(current_pos);

            // Update selection based on current drag area
            self.update_node_selection_from_drag(cx);
        }
    }

    pub fn end_selection_drag(&mut self, cx: &mut Context<Self>) {
        self.selection_start = None;
        self.selection_end = None;
        cx.notify();
    }

    fn update_node_selection_from_drag(&mut self, cx: &mut Context<Self>) {
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            let min_x = start.x.min(end.x);
            let max_x = start.x.max(end.x);
            let min_y = start.y.min(end.y);
            let max_y = start.y.max(end.y);

            // Check ALL nodes (not just rendered ones) for intersection with selection box
            for node in &self.graph.nodes {
                let node_left = node.position.x;
                let node_top = node.position.y;
                let node_right = node.position.x + node.size.width;
                let node_bottom = node.position.y + node.size.height;

                // Check if node intersects with selection box
                let intersects = !(node_right < min_x || node_left > max_x ||
                                  node_bottom < min_y || node_top > max_y);

                if intersects {
                    if !self.graph.selected_nodes.contains(&node.id) {
                        self.graph.selected_nodes.push(node.id.clone());
                    }
                } else {
                    // Remove from selection if not intersecting (for live drag selection)
                    self.graph.selected_nodes.retain(|id| id != &node.id);
                }
            }
            cx.notify();
        }
    }

    pub fn delete_selected_nodes(&mut self, cx: &mut Context<Self>) {
        if !self.graph.selected_nodes.is_empty() {
            // Remove selected nodes
            self.graph.nodes.retain(|node| !self.graph.selected_nodes.contains(&node.id));

            // Remove connections involving deleted nodes
            self.graph.connections.retain(|connection| {
                !self.graph.selected_nodes.contains(&connection.from_node_id) &&
                !self.graph.selected_nodes.contains(&connection.to_node_id)
            });

            // Clear selection
            self.graph.selected_nodes.clear();
            cx.notify();
        }
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
            .when_some(self.node_creation_menu.clone(), |this, menu| {
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .child(menu)
                )
            })
    }
}
