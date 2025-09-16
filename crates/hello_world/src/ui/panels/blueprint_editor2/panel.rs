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
