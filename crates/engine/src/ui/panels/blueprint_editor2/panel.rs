use gpui::prelude::FluentBuilder;
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dock::{Panel, PanelEvent},
    input::InputState,
    resizable::{h_resizable, v_resizable, resizable_panel, ResizableState},
    tab::{Tab, TabBar},
    v_flex, h_flex, ActiveTheme as _, PixelsExt, IconName,
};
use smol::Timer;
use std::time::Duration;

use super::hoverable_tooltip::HoverableTooltip;
use super::node_creation_menu::{NodeCreationEvent, NodeCreationMenu};
use super::node_graph::NodeGraphRenderer;
use super::toolbar::ToolbarRenderer;
use super::*;
use crate::graph::{DataType as GraphDataType, GraphDescription};

// Tab entry for the flat tab system (like Unreal)
#[derive(Clone, Debug)]
pub struct GraphTab {
    pub id: String,
    pub name: String,
    pub graph: BlueprintGraph,
    pub is_main: bool, // True for the main event graph
    pub is_dirty: bool, // True if there are unsaved changes
    pub is_library_macro: bool, // True if from global library (not local)
    pub library_id: Option<String>, // Parent library ID for library macros
}

// Serializable version of GraphTab for persistence
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SerializedGraphTab {
    pub id: String,
    pub name: String,
    pub is_main: bool,
    pub is_library_macro: bool,
    pub library_id: Option<String>,
}

// Constants for node creation menu dimensions
// These must match the values in node_creation_menu.rs
const NODE_MENU_WIDTH: f32 = 280.0;
const NODE_MENU_MAX_HEIGHT: f32 = 350.0;

pub struct BlueprintEditorPanel {
    focus_handle: FocusHandle,
    pub graph: BlueprintGraph,
    resizable_state: Entity<ResizableState>,
    left_sidebar_resizable_state: Entity<ResizableState>,
    // Current class path (for saving/compiling)
    pub current_class_path: Option<std::path::PathBuf>,
    // Tab title for display in the UI
    pub tab_title: Option<String>,
    // Drag state
    pub dragging_node: Option<String>,
    pub drag_offset: Point<f32>,
    pub initial_drag_positions: std::collections::HashMap<String, Point<f32>>, // Store initial positions of all dragged nodes
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
    pub node_creation_menu_position: Option<Point<f32>>,
    // Right-click state for gesture detection
    pub right_click_start: Option<Point<f32>>,
    pub right_click_threshold: f32,
    // Hoverable tooltip
    pub hoverable_tooltip: Option<Entity<HoverableTooltip>>,
    pub pending_tooltip: Option<(String, Point<f32>)>, // (content, position) waiting to show
    // Double-click tracking for creating reroute nodes on connections
    pub last_click_time: Option<std::time::Instant>,
    pub last_click_pos: Option<Point<f32>>,
    // Graph element bounds for coordinate conversion (GPUI mouse events are window-relative)
    pub graph_element_bounds: Option<gpui::Bounds<gpui::Pixels>>,
    // Class variables
    pub class_variables: Vec<super::variables::ClassVariable>,
    // Variable creation state
    pub is_creating_variable: bool,
    pub variable_name_input: Entity<gpui_component::input::InputState>,
    pub variable_type_dropdown:
        Entity<gpui_component::dropdown::DropdownState<Vec<super::variables::TypeItem>>>,
    // Variable drag state
    pub dragging_variable: Option<super::variables::VariableDrag>,
    pub variable_drop_menu_position: Option<Point<f32>>,
    // Comment state
    pub dragging_comment: Option<String>, // Comment ID being dragged
    pub resizing_comment: Option<(String, ResizeHandle)>, // (comment ID, handle being dragged)
    pub editing_comment: Option<String>,  // Comment ID being edited
    pub comment_text_input: Entity<gpui_component::input::InputState>,
    // Store subscriptions to keep them alive
    pub subscriptions: Vec<gpui::Subscription>,
    // Compilation status for UI feedback
    pub compilation_status: super::CompilationStatus,
    // Library manager for loading global/engine sub-graphs
    pub library_manager: crate::graph::LibraryManager,
    // Local macros defined within this blueprint class
    pub local_macros: Vec<crate::graph::SubGraphDefinition>,
    // Tab system - flat navigation like Unreal
    pub open_tabs: Vec<GraphTab>,
    pub active_tab_index: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResizeHandle {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Top,
    Bottom,
    Left,
    Right,
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
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_internal(None, window, cx)
    }

    /// Create a new blueprint editor for an engine library (virtual blueprint)
    pub fn new_for_library(library_id: String, library_name: String, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut panel = Self::new_internal(None, window, cx);
        
        // Set a special flag or title to indicate this is a library view
        panel.tab_title = Some(format!("📚 {} Library", library_name));
        
        // The EventGraph tab should show an overview or README for the library
        if let Some(main_tab) = panel.open_tabs.get_mut(0) {
            main_tab.name = format!("{} Overview", library_name);
        }
        
        println!("📚 Created blueprint editor for library: {}", library_name);
        panel
    }

    fn new_internal(
        project_path: Option<std::path::PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>
    ) -> Self {
        let resizable_state = ResizableState::new(cx);
        let left_sidebar_resizable_state = ResizableState::new(cx);

        // Initialize subscriptions vector

        // Create sample nodes - demonstrates all compiler features
        let mut nodes = Vec::new();

        // Main event node (defines pub fn main())
        nodes.push(BlueprintNode {
            id: "main_event".to_string(),
            definition_id: "main".to_string(),
            title: "Main".to_string(),
            icon: "▶️".to_string(),
            node_type: NodeType::Event,
            position: Point::new(100.0, 200.0),
            size: Size::new(180.0, 80.0),
            inputs: vec![],
            outputs: vec![Pin {
                id: "Body".to_string(),
                name: "Body".to_string(),
                pin_type: PinType::Output,
                data_type: DataType::Execution,
            }],
            properties: HashMap::new(),
            is_selected: false,
            description: "Entry point for the main function".to_string(),
            color: None,
        });

        // Pure node: add(2, 3)
        let mut add_props = std::collections::HashMap::new();
        add_props.insert("a".to_string(), "2".to_string());
        add_props.insert("b".to_string(), "3".to_string());

        nodes.push(BlueprintNode {
            id: "add_node".to_string(),
            definition_id: "add".to_string(),
            title: "Add".to_string(),
            icon: "➕".to_string(),
            node_type: NodeType::Math,
            position: Point::new(400.0, 80.0),
            size: Size::new(150.0, 100.0),
            inputs: vec![
                Pin {
                    id: "a".to_string(),
                    name: "A".to_string(),
                    pin_type: PinType::Input,
                    data_type: GraphDataType::from_type_str("i64"),
                },
                Pin {
                    id: "b".to_string(),
                    name: "B".to_string(),
                    pin_type: PinType::Input,
                    data_type: GraphDataType::from_type_str("i64"),
                },
            ],
            outputs: vec![Pin {
                id: "result".to_string(),
                name: "Result".to_string(),
                pin_type: PinType::Output,
                data_type: GraphDataType::from_type_str("i64"),
            }],
            properties: add_props,
            is_selected: false,
            description: "Adds two numbers: (2 + 3) = 5".to_string(),
            color: None,
        });

        // Control flow: branch
        nodes.push(BlueprintNode {
            id: "branch_node".to_string(),
            definition_id: "branch".to_string(),
            title: "Branch".to_string(),
            icon: "🔀".to_string(),
            node_type: NodeType::Logic,
            position: Point::new(400.0, 280.0),
            size: Size::new(180.0, 120.0),
            inputs: vec![
                Pin {
                    id: "exec".to_string(),
                    name: "".to_string(),
                    pin_type: PinType::Input,
                    data_type: GraphDataType::from_type_str("execution"),
                },
                Pin {
                    id: "condition".to_string(),
                    name: "Condition".to_string(),
                    pin_type: PinType::Input,
                    data_type: GraphDataType::from_type_str("bool"),
                },
            ],
            outputs: vec![
                Pin {
                    id: "True".to_string(),
                    name: "True".to_string(),
                    pin_type: PinType::Output,
                    data_type: GraphDataType::from_type_str("execution"),
                },
                Pin {
                    id: "False".to_string(),
                    name: "False".to_string(),
                    pin_type: PinType::Output,
                    data_type: GraphDataType::from_type_str("execution"),
                },
            ],
            properties: std::collections::HashMap::new(),
            is_selected: false,
            description: "Branches execution based on a condition.".to_string(),
            color: None,
        });

        // Function node: print (true path)
        let mut print_true_props = std::collections::HashMap::new();
        print_true_props.insert(
            "message".to_string(),
            "Result is greater than 3! ✓".to_string(),
        );

        nodes.push(BlueprintNode {
            id: "print_true".to_string(),
            definition_id: "print_string".to_string(),
            title: "Print String".to_string(),
            icon: "📝".to_string(),
            node_type: NodeType::Logic,
            position: Point::new(680.0, 220.0),
            size: Size::new(200.0, 100.0),
            inputs: vec![
                Pin {
                    id: "exec".to_string(),
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
            properties: print_true_props,
            is_selected: false,
            description: "Prints success message.".to_string(),
            color: None,
        });

        // Function node: print (false path)
        let mut print_false_props = std::collections::HashMap::new();
        print_false_props.insert("message".to_string(), "Result is 3 or less. ✗".to_string());

        nodes.push(BlueprintNode {
            id: "print_false".to_string(),
            definition_id: "print_string".to_string(),
            title: "Print String".to_string(),
            icon: "📝".to_string(),
            node_type: NodeType::Logic,
            position: Point::new(680.0, 360.0),
            size: Size::new(200.0, 100.0),
            inputs: vec![
                Pin {
                    id: "exec".to_string(),
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
            properties: print_false_props,
            is_selected: false,
            description: "Prints alternative message.".to_string(),
            color: None,
        });

        // Pure node: greater than
        let mut gt_props = std::collections::HashMap::new();
        gt_props.insert("b".to_string(), "3".to_string());

        nodes.push(BlueprintNode {
            id: "greater_node".to_string(),
            definition_id: "greater_than".to_string(),
            title: "Greater Than".to_string(),
            icon: "▶".to_string(),
            node_type: NodeType::Logic,
            position: Point::new(620.0, 80.0),
            size: Size::new(160.0, 100.0),
            inputs: vec![
                Pin {
                    id: "a".to_string(),
                    name: "A".to_string(),
                    pin_type: PinType::Input,
                    data_type: GraphDataType::from_type_str("i64"),
                },
                Pin {
                    id: "b".to_string(),
                    name: "B".to_string(),
                    pin_type: PinType::Input,
                    data_type: GraphDataType::from_type_str("i64"),
                },
            ],
            outputs: vec![Pin {
                id: "result".to_string(),
                name: "Result".to_string(),
                pin_type: PinType::Output,
                data_type: GraphDataType::from_type_str("bool"),
            }],
            properties: gt_props,
            is_selected: false,
            description: "Checks if A > B: result > 3?".to_string(),
            color: None,
        });

        let connections = vec![
            // Execution: main -> branch
            Connection {
                id: "conn_main_branch".to_string(),
                from_node_id: "main_event".to_string(),
                from_pin_id: "Body".to_string(),
                to_node_id: "branch_node".to_string(),
                to_pin_id: "exec".to_string(),
            },
            // Data: add -> greater_than
            Connection {
                id: "conn_add_gt".to_string(),
                from_node_id: "add_node".to_string(),
                from_pin_id: "result".to_string(),
                to_node_id: "greater_node".to_string(),
                to_pin_id: "a".to_string(),
            },
            // Data: greater_than -> branch
            Connection {
                id: "conn_gt_branch".to_string(),
                from_node_id: "greater_node".to_string(),
                from_pin_id: "result".to_string(),
                to_node_id: "branch_node".to_string(),
                to_pin_id: "condition".to_string(),
            },
            // Execution: branch(True) -> print_true
            Connection {
                id: "conn_branch_true".to_string(),
                from_node_id: "branch_node".to_string(),
                from_pin_id: "True".to_string(),
                to_node_id: "print_true".to_string(),
                to_pin_id: "exec".to_string(),
            },
            // Execution: branch(False) -> print_false
            Connection {
                id: "conn_branch_false".to_string(),
                from_node_id: "branch_node".to_string(),
                from_pin_id: "False".to_string(),
                to_node_id: "print_false".to_string(),
                to_pin_id: "exec".to_string(),
            },
        ];

        // Create the initial main event graph
        let main_graph = BlueprintGraph {
            nodes,
            connections,
            comments: vec![],
            selected_nodes: vec![],
            selected_comments: vec![],
            zoom_level: 1.0,
            pan_offset: Point::new(0.0, 0.0),
            virtualization_stats: super::VirtualizationStats::default(),
        };

        let mut result = Self {
            focus_handle: cx.focus_handle(),
            graph: main_graph.clone(), // Current active tab's graph
            resizable_state,
            left_sidebar_resizable_state,
            current_class_path: None,
            tab_title: None,
            dragging_node: None,
            drag_offset: Point::new(0.0, 0.0),
            initial_drag_positions: std::collections::HashMap::new(),
            dragging_connection: None,
            is_panning: false,
            pan_start: Point::new(0.0, 0.0),
            pan_start_offset: Point::new(0.0, 0.0),
            selection_start: None,
            selection_end: None,
            last_mouse_pos: None,
            node_creation_menu: None,
            node_creation_menu_position: None,
            right_click_start: None,
            right_click_threshold: 5.0, // pixels
            hoverable_tooltip: None,
            pending_tooltip: None,
            last_click_time: None,
            last_click_pos: None,
            graph_element_bounds: None, // Will be set during rendering
            class_variables: Vec::new(),
            is_creating_variable: false,
            variable_name_input: cx.new(|cx| {
                gpui_component::input::InputState::new(window, cx).placeholder("Variable name...")
            }),
            variable_type_dropdown: cx.new(|cx| {
                gpui_component::dropdown::DropdownState::new(Vec::new(), None, window, cx)
            }),
            dragging_variable: None,
            variable_drop_menu_position: None,
            dragging_comment: None,
            resizing_comment: None,
            editing_comment: None,
            comment_text_input: cx.new(|cx| {
                gpui_component::input::InputState::new(window, cx).placeholder("Comment text...")
            }),
            subscriptions: Vec::<gpui::Subscription>::new(),
            compilation_status: super::CompilationStatus::default(),
            library_manager: {
                let mut lib_manager = crate::graph::LibraryManager::default();
                if let Err(e) = lib_manager.load_all_libraries() {
                    eprintln!("Failed to load sub-graph libraries: {}", e);
                }
                lib_manager
            },
            local_macros: Vec::new(),
            open_tabs: vec![GraphTab {
                id: "main".to_string(),
                name: "EventGraph".to_string(),
                graph: main_graph,
                is_main: true,
                is_dirty: false,
                is_library_macro: false,
                library_id: None,
            }],
            active_tab_index: 0,
        };

        result
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
        println!(
            "Adding node: {} at position {:?}",
            node.title, node.position
        );
        self.graph.nodes.push(node);
        // Mark tab as dirty
        if let Some(tab) = self.open_tabs.get_mut(self.active_tab_index) {
            tab.is_dirty = true;
        }
        cx.notify();
    }

    /// Sync the current graph state to the active tab
    fn sync_graph_to_active_tab(&mut self) {
        let tab_id = if let Some(tab) = self.open_tabs.get(self.active_tab_index) {
            tab.id.clone()
        } else {
            return;
        };
        
        let is_main = if let Some(tab) = self.open_tabs.get(self.active_tab_index) {
            tab.is_main
        } else {
            return;
        };
        
        // Update the tab
        if let Some(tab) = self.open_tabs.get_mut(self.active_tab_index) {
            tab.graph = self.graph.clone();
            tab.is_dirty = true;
        }
        
        // If this is a local macro tab, sync back to local_macros list
        if !is_main && !tab_id.starts_with("🌐") {
            if let Ok(graph_desc) = self.convert_to_graph_description() {
                if let Some(macro_def) = self.local_macros.iter_mut().find(|m| m.id == tab_id) {
                    macro_def.graph = graph_desc;
                    macro_def.metadata.modified_at = chrono::Utc::now().to_rfc3339();
                }
            }
        }
    }

    /// Load the active tab's graph into self.graph
    fn load_active_tab_graph(&mut self) {
        if let Some(tab) = self.open_tabs.get(self.active_tab_index) {
            self.graph = tab.graph.clone();
        }
    }

    /// Switch to a different tab
    pub fn switch_to_tab(&mut self, tab_index: usize, cx: &mut Context<Self>) {
        if tab_index < self.open_tabs.len() && tab_index != self.active_tab_index {
            // Save current graph to current tab
            self.sync_graph_to_active_tab();
            
            // Switch to new tab
            self.active_tab_index = tab_index;
            self.load_active_tab_graph();
            
            cx.notify();
        }
    }

    /// Close a tab by index
    pub fn close_tab(&mut self, tab_index: usize, cx: &mut Context<Self>) {
        // NEVER allow closing the main event graph tab
        if tab_index >= self.open_tabs.len() || self.open_tabs[tab_index].is_main {
            println!("⚠️ Cannot close the main EventGraph tab");
            return;
        }

        self.open_tabs.remove(tab_index);
        
        // Adjust active tab index if needed
        if self.active_tab_index >= self.open_tabs.len() {
            self.active_tab_index = self.open_tabs.len().saturating_sub(1);
        }
        if self.active_tab_index >= tab_index && self.active_tab_index > 0 {
            self.active_tab_index -= 1;
        }
        
        self.load_active_tab_graph();
        cx.notify();
    }

    /// Open a local macro in a new tab (or switch to existing tab)
    pub fn open_local_macro(&mut self, macro_id: String, macro_name: String, cx: &mut Context<Self>) {
        // Check if tab already exists
        if let Some(index) = self.open_tabs.iter().position(|tab| tab.id == macro_id) {
            self.switch_to_tab(index, cx);
            return;
        }

        // Find the macro definition in local macros
        if let Some(macro_def) = self.local_macros.iter().find(|m| m.id == macro_id) {
            // Convert to BlueprintGraph
            match self.convert_graph_description_to_blueprint(&macro_def.graph) {
                Ok(blueprint_graph) => {
                    // Save current graph to current tab before switching
                    self.sync_graph_to_active_tab();

                    // Create new tab
                    let new_tab = GraphTab {
                        id: macro_id.clone(),
                        name: macro_name.clone(),
                        graph: blueprint_graph,
                        is_main: false,
                        is_dirty: false,
                        is_library_macro: false,
                        library_id: None,
                    };

                    self.open_tabs.push(new_tab);
                    self.active_tab_index = self.open_tabs.len() - 1;
                    self.load_active_tab_graph();

                    println!("📂 Opened local macro in tab: {}", macro_name);
                    cx.notify();
                }
                Err(e) => {
                    eprintln!("Failed to convert local macro to blueprint: {}", e);
                }
            }
        }
    }

    /// Open a global/engine macro in a new tab (or switch to existing tab)
    /// Also opens the parent library tab if needed (for engine macros navigation)
    pub fn open_global_macro(&mut self, macro_id: String, macro_name: String, cx: &mut Context<Self>) {
        // Check if tab already exists in THIS blueprint editor
        if let Some(index) = self.open_tabs.iter().position(|tab| tab.id == macro_id) {
            self.switch_to_tab(index, cx);
            return;
        }

        // Find the macro definition in library manager
        if let Some(macro_def) = self.library_manager.get_subgraph(&macro_id) {
            // Find which library this macro belongs to
            let library_id = self.library_manager.get_libraries()
                .iter()
                .find(|(_, lib)| lib.subgraphs.iter().any(|sg| sg.id == macro_id))
                .map(|(id, _)| id.clone());

            // TODO: If this is being opened from a user blueprint and the macro belongs
            // to an engine library, we should:
            // 1. Signal the app to open/focus the engine library blueprint tab
            // 2. Then open the macro within that context
            // For now, we open it in the current blueprint editor context

            // Convert to BlueprintGraph
            match self.convert_graph_description_to_blueprint(&macro_def.graph) {
                Ok(blueprint_graph) => {
                    // Save current graph to current tab before switching
                    self.sync_graph_to_active_tab();

                    // Create new tab
                    let new_tab = GraphTab {
                        id: macro_id.clone(),
                        name: format!("🌐 {}", macro_name), // Prefix with globe to indicate it's global
                        graph: blueprint_graph,
                        is_main: false,
                        is_dirty: false,
                        is_library_macro: true,
                        library_id: library_id.clone(),
                    };

                    self.open_tabs.push(new_tab);
                    self.active_tab_index = self.open_tabs.len() - 1;
                    self.load_active_tab_graph();

                    if let Some(lib_id) = library_id {
                        println!("📂 Opened global macro '{}' from library '{}' in tab", macro_name, lib_id);
                        println!("ℹ️  Note: For proper context, engine macros should be opened from their library tab");
                    } else {
                        println!("📂 Opened global macro '{}' in tab", macro_name);
                    }
                    cx.notify();
                }
                Err(e) => {
                    eprintln!("Failed to convert global macro to blueprint: {}", e);
                }
            }
        }
    }

    /// Get the library ID that a macro belongs to (for smart navigation)
    pub fn get_macro_library_id(&self, macro_id: &str) -> Option<String> {
        // Check local macros first
        if self.local_macros.iter().any(|m| m.id == macro_id) {
            return None; // Local macro, no library
        }

        // Check global libraries
        self.library_manager.get_libraries()
            .iter()
            .find(|(_, lib)| lib.subgraphs.iter().any(|sg| sg.id == macro_id))
            .map(|(id, _)| id.clone())
    }

    /// Request to open an engine library in main tabs (emits event)
    /// This is used by file drawer and node graph to request app-level navigation
    pub fn request_open_engine_library(
        &self,
        library_id: String,
        library_name: String,
        macro_id: Option<String>,
        macro_name: Option<String>,
        cx: &mut Context<Self>,
    ) {
        // Emit the request event - app.rs will handle it
        cx.emit(super::OpenEngineLibraryRequest {
            library_id,
            library_name,
            macro_id,
            macro_name,
        });
    }

    /// Compile the current graph to Rust source code
    pub fn compile_to_rust(&self) -> Result<String, String> {
        // Convert blueprint graph to our graph description format
        let graph_description = self.convert_to_graph_description()?;

        // Use new macro-based compiler
        crate::compiler::compile_graph(&graph_description)
    }

    /// Compile and save events to class directory structure
    pub fn compile_to_class_directory(&self) -> Result<(), String> {
        let class_path = self
            .current_class_path
            .as_ref()
            .ok_or("No class loaded - cannot compile")?;

        // Save variables and generate vars module first
        self.save_variables_to_class()?;
        self.generate_vars_module()?;

        // Create events directory
        let events_dir = class_path.join("events");
        std::fs::create_dir_all(&events_dir)
            .map_err(|e| format!("Failed to create events directory: {}", e))?;

        // Find all event nodes in the graph
        let event_nodes: Vec<_> = self
            .graph
            .nodes
            .iter()
            .filter(|node| node.node_type == super::NodeType::Event)
            .collect();

        if event_nodes.is_empty() {
            return Err("No event nodes found in graph".to_string());
        }

        // Compile each event individually
        let graph_description = self.convert_to_graph_description()?;
        let metadata = crate::compiler::node_metadata::extract_node_metadata()
            .map_err(|e| format!("Failed to get node metadata: {}", e))?;

        // Build variables HashMap from class_variables
        let variables: std::collections::HashMap<String, String> = self
            .class_variables
            .iter()
            .map(|v| (v.name.clone(), v.var_type.clone()))
            .collect();

        let data_resolver = crate::compiler::data_resolver::DataResolver::build_with_variables(
            &graph_description,
            &metadata,
            variables.clone(),
        )?;
        let exec_routing = crate::compiler::execution_routing::ExecutionRouting::build_from_graph(
            &graph_description,
        );

        let mut mod_exports = Vec::new();

        for event_node in &event_nodes {
            // Find the graph node for this event
            let graph_event = graph_description
                .nodes
                .values()
                .find(|n| n.id == event_node.id)
                .ok_or(format!("Event node {} not found in graph", event_node.id))?;

            // Generate code for this specific event
            let mut generator = crate::compiler::code_generator::CodeGenerator::new(
                &metadata,
                &data_resolver,
                &exec_routing,
                &graph_description,
                variables.clone(),
            );

            let event_code = generator.generate_event_function(graph_event)?;

            // Write to individual file
            let event_name = event_node.definition_id.to_lowercase();
            let event_file = events_dir.join(format!("{}.rs", event_name));

            std::fs::write(&event_file, &event_code)
                .map_err(|e| format!("Failed to write {}: {}", event_file.display(), e))?;

            mod_exports.push(event_name.clone());
            println!(
                "Compiled event '{}' to {}",
                event_node.title,
                event_file.display()
            );
        }

        // Create mod.rs that re-exports all events with header
        let now = chrono::Local::now();
        let version = crate::ENGINE_VERSION;
        let mod_header = format!(
            "//! Auto Generated by the Pulsar Blueprint Editor\n\
             //! DO NOT EDIT MANUALLY - YOUR CHANGES WILL BE OVERWRITTEN\n\
             //! Generated on {} - Engine version {}\n\
             //!\n\
             //! This file re-exports all event modules for this class.\n\
             //! Individual event implementations are in separate files.\n\
             //! To modify events, open the class in the Pulsar Blueprint Editor.\n\
             //!\n\
             //! EDITING ANYTHING IN THIS FILE COULD BREAK THE EDITOR\n\
             //! AND PREVENT THE GUI FROM OPENING THIS CLASS - BE CAREFUL\n\n",
            now.format("%Y-%m-%d %H:%M:%S"),
            version
        );

        let mod_exports_str = mod_exports
            .iter()
            .map(|name| format!("pub mod {};\npub use {}::*;", name, name))
            .collect::<Vec<_>>()
            .join("\n");

        let mod_content = format!("{}{}", mod_header, mod_exports_str);

        let mod_path = events_dir.join("mod.rs");
        std::fs::write(&mod_path, mod_content)
            .map_err(|e| format!("Failed to write mod.rs: {}", e))?;

        Ok(())
    }

    /// Convert blueprint graph to graph description format
    fn convert_to_graph_description(&self) -> Result<crate::graph::GraphDescription, String> {
        self.convert_graph_to_description(&self.graph)
    }
    
    /// Convert a specific blueprint graph to graph description format
    fn convert_graph_to_description(&self, graph: &BlueprintGraph) -> Result<crate::graph::GraphDescription, String> {
        use crate::graph::*;
        let mut graph_desc = GraphDescription::new("Blueprint Graph");

        // Convert nodes
        for bp_node in &graph.nodes {
            let mut node_instance = NodeInstance::new(
                &bp_node.id,
                &self.get_node_type_from_blueprint(&bp_node)?,
                Position {
                    x: bp_node.position.x,
                    y: bp_node.position.y,
                },
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
        for connection in &graph.connections {
            // Determine connection type based on source pin's data type
            let conn_type = graph
                .nodes
                .iter()
                .find(|n| n.id == connection.from_node_id)
                .and_then(|node| node.outputs.iter().find(|p| p.id == connection.from_pin_id))
                .map(|pin| match &pin.data_type {
                    GraphDataType::Execution => ConnectionType::Execution,
                    _ => ConnectionType::Data,
                })
                .unwrap_or(ConnectionType::Data);

            let graph_connection = Connection::new(
                &connection.id,
                &connection.from_node_id,
                &connection.from_pin_id,
                &connection.to_node_id,
                &connection.to_pin_id,
                conn_type,
            );
            graph_desc.add_connection(graph_connection);
        }

        // Add comments to graph description
        graph_desc.comments = graph.comments.clone();

        Ok(graph_desc)
    }

    fn get_node_type_from_blueprint(&self, bp_node: &BlueprintNode) -> Result<String, String> {
        // Use the stored definition_id directly
        Ok(bp_node.definition_id.clone())
    }

    // Conversion function no longer needed since we use the unified DataType system

    /// Save the complete blueprint to a unified JSON file (like Unreal Engine)
    /// This saves EVERYTHING: main event graph, ALL local macros, variables, and editor state
    pub fn save_blueprint(&mut self, file_path: &str) -> Result<(), String> {
        // STEP 1: Sync the currently active tab's self.graph content to that tab's storage
        // This ensures any unsaved changes in the editor are captured
        self.sync_graph_to_active_tab();
        
        // STEP 2: Sync ALL open tabs back to their permanent storage
        // - Main tab graph -> stored in main_tab.graph
        // - Macro tab graphs -> stored in self.local_macros[i].graph
        // This ensures all open tabs are saved, not just the active one
        self.sync_all_tabs_to_storage();
        
        // STEP 3: Find the main event graph tab (must always exist)
        let main_tab = self.open_tabs.iter()
            .find(|tab| tab.is_main)
            .ok_or_else(|| "No main event graph found".to_string())?;
        
        // STEP 4: Convert the main tab's graph to GraphDescription format for serialization
        // This is the MAIN event graph - never a macro graph
        let main_graph_description = self.convert_graph_to_description(&main_tab.graph)?;
        
        // STEP 5: Convert class variables to the serializable format
        let variables: Vec<crate::graph::ClassVariable> = self.class_variables.iter().map(|v| {
            crate::graph::ClassVariable {
                id: uuid::Uuid::new_v4().to_string(), // Generate ID for compatibility
                name: v.name.clone(),
                data_type: crate::graph::DataType::String, // TODO: parse var_type properly
                default_value: v.default_value.clone(),
                description: String::new(), // UI ClassVariable doesn't have description
            }
        }).collect();
        
        // STEP 6: Build editor state (open tabs, active tab, view states)
        let open_tab_ids: Vec<String> = self.open_tabs.iter()
            .map(|tab| tab.id.clone())
            .collect();
        
        // Collect view states for ALL open tabs (main + macros)
        let mut graph_view_states = std::collections::HashMap::new();
        for tab in &self.open_tabs {
            graph_view_states.insert(
                tab.id.clone(),
                crate::graph::GraphViewState {
                    pan_offset_x: tab.graph.pan_offset.x,
                    pan_offset_y: tab.graph.pan_offset.y,
                    zoom: tab.graph.zoom_level,
                }
            );
        }
        
        let editor_state = crate::graph::BlueprintEditorState {
            open_tab_ids,
            active_tab_index: self.active_tab_index,
            graph_view_states,
        };
        
        // STEP 7: Create the unified blueprint asset with EVERYTHING
        // - main_graph: The main event graph ONLY
        // - local_macros: ALL local macro graphs with their content and pins
        // - variables: All class variables
        // - editor_state: UI state for restoration
        // - blueprint_metadata: Blueprint type and metadata
        let blueprint_asset = crate::graph::BlueprintAsset {
            format_version: 1,
            main_graph: main_graph_description,
            local_macros: self.local_macros.clone(),
            variables,
            editor_state: Some(editor_state),
            blueprint_metadata: crate::graph::BlueprintMetadata::default(),
        };
        
        // Serialize to JSON
        let json = serde_json::to_string_pretty(&blueprint_asset)
            .map_err(|e| format!("Failed to serialize blueprint: {}", e))?;

        // Add header comment to JSON file
        let now = chrono::Local::now();
        let version = crate::ENGINE_VERSION;
        let header = format!(
            "// Auto Generated by the Pulsar Blueprint Editor\n\
             // DO NOT EDIT MANUALLY - YOUR CHANGES WILL BE OVERWRITTEN\n\
             // Generated on {} - Engine version {}\n\
             //\n\
             // This file contains the COMPLETE blueprint for this class including:\n\
             //   - Main event graph\n\
             //   - All local macro graphs\n\
             //   - Class variables\n\
             //   - Editor state (open tabs, camera positions, etc.)\n\
             //\n\
             // You can modify the graph by opening this class in the Pulsar Blueprint Editor.\n\
             // The graph is saved in JSON format for human readability and version control.\n\
             //\n\
             // EDITING THE JSON STRUCTURE COULD BREAK THE EDITOR\n\
             // AND PREVENT THE GUI FROM OPENING THIS CLASS - BE CAREFUL\n\n",
            now.format("%Y-%m-%d %H:%M:%S"),
            version
        );

        let content = format!("{}{}", header, json);

        std::fs::write(file_path, content).map_err(|e| format!("Failed to write file: {}", e))?;

        println!("💾 ═══════════════════════════════════════════════════════════════");
        println!("💾 BLUEPRINT SAVED SUCCESSFULLY");
        println!("💾 ═══════════════════════════════════════════════════════════════");
        println!("💾 File: {}", file_path);
        println!("💾");
        println!("💾 📊 Content Summary:");
        println!("💾   ✓ Main Event Graph: {} nodes, {} connections",
            main_tab.graph.nodes.len(),
            main_tab.graph.connections.len());
        println!("💾   ✓ Local Macros: {}", self.local_macros.len());
        for macro_def in &self.local_macros {
            println!("💾     - {} ({})", macro_def.name, macro_def.id);
        }
        println!("💾   ✓ Class Variables: {}", self.class_variables.len());
        println!("💾   ✓ Open Tabs: {}", self.open_tabs.len());
        println!("💾   ✓ Active Tab: {} ({})", 
            self.open_tabs.get(self.active_tab_index).map(|t| t.name.as_str()).unwrap_or("Unknown"),
            self.active_tab_index);
        println!("💾 ═══════════════════════════════════════════════════════════════");

        Ok(())
    }
    
    /// Sync all open tabs back to their storage locations
    /// - Main tab: graph stays in tab.graph (used during save)
    /// - Local macro tabs: graph goes to self.local_macros[i].graph (converted to GraphDescription)
    /// - Library macro tabs: read-only, not synced
    fn sync_all_tabs_to_storage(&mut self) {
        // Collect graphs to sync (to avoid borrow checker issues)
        // First convert all graphs to GraphDescription format
        let mut converted_graphs: Vec<(String, crate::graph::GraphDescription)> = Vec::new();
        
        for tab in self.open_tabs.iter() {
            if !tab.is_main && !tab.is_library_macro {
                // This is a local macro tab that needs syncing
                if let Ok(graph_desc) = self.convert_graph_to_description(&tab.graph) {
                    converted_graphs.push((tab.id.clone(), graph_desc));
                } else {
                    eprintln!("⚠️  Failed to convert macro tab '{}' to GraphDescription", tab.id);
                }
            }
        }
        
        // Now sync them to local_macros storage (mutably)
        for (tab_id, graph_desc) in converted_graphs {
            if let Some(macro_def) = self.local_macros.iter_mut().find(|m| m.id == tab_id) {
                macro_def.graph = graph_desc;
                macro_def.metadata.modified_at = chrono::Local::now().to_rfc3339();
                println!("📝 Synced macro '{}' to storage", macro_def.name);
            } else {
                eprintln!("⚠️  Macro tab '{}' not found in local_macros storage", tab_id);
            }
        }
    }

    /// Save local macros to macros.json in the class directory
    fn save_local_macros(&self) -> Result<(), String> {
        if let Some(class_path) = &self.current_class_path {
            let macros_file = class_path.join("macros.json");
            
            // Sync all open macro tabs back to local_macros before saving
            // This is handled elsewhere, but we ensure it here too
            
            let json = serde_json::to_string_pretty(&self.local_macros)
                .map_err(|e| format!("Failed to serialize local macros: {}", e))?;
            
            std::fs::write(&macros_file, json)
                .map_err(|e| format!("Failed to write macros.json: {}", e))?;
            
            println!("💾 Saved {} local macros to macros.json", self.local_macros.len());
        }
        Ok(())
    }

    /// Save open tabs state to tabs.json for restoration on load
    fn save_tabs_state(&self) -> Result<(), String> {
        if let Some(class_path) = &self.current_class_path {
            let tabs_file = class_path.join("tabs.json");
            
            // Serialize just the tab metadata (not the full graphs)
            let serialized_tabs: Vec<SerializedGraphTab> = self.open_tabs.iter().map(|tab| {
                SerializedGraphTab {
                    id: tab.id.clone(),
                    name: tab.name.clone(),
                    is_main: tab.is_main,
                    is_library_macro: tab.is_library_macro,
                    library_id: tab.library_id.clone(),
                }
            }).collect();
            
            let json = serde_json::to_string_pretty(&serialized_tabs)
                .map_err(|e| format!("Failed to serialize tabs: {}", e))?;
            
            std::fs::write(&tabs_file, json)
                .map_err(|e| format!("Failed to write tabs.json: {}", e))?;
            
            println!("💾 Saved {} tab states to tabs.json", serialized_tabs.len());
        }
        Ok(())
    }

    /// Load local macros from macros.json in the class directory
    fn load_local_macros(&mut self, class_path: &std::path::Path) -> Result<(), String> {
        let macros_file = class_path.join("macros.json");
        
        if !macros_file.exists() {
            // No macros file yet, that's ok - start with empty
            self.local_macros.clear();
            return Ok(());
        }
        
        let content = std::fs::read_to_string(&macros_file)
            .map_err(|e| format!("Failed to read macros.json: {}", e))?;
        
        let macros: Vec<crate::graph::SubGraphDefinition> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse macros.json: {}", e))?;
        
        self.local_macros = macros;
        println!("📂 Loaded {} local macros from macros.json", self.local_macros.len());
        Ok(())
    }

    /// Restore open tabs from tabs.json after loading blueprint
    fn restore_tabs_state(&mut self, class_path: &std::path::Path, window: &mut gpui::Window, cx: &mut Context<Self>) -> Result<(), String> {
        let tabs_file = class_path.join("tabs.json");
        
        if !tabs_file.exists() {
            // No tabs file yet, just keep the main tab
            return Ok(());
        }
        
        let content = std::fs::read_to_string(&tabs_file)
            .map_err(|e| format!("Failed to read tabs.json: {}", e))?;
        
        let serialized_tabs: Vec<SerializedGraphTab> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse tabs.json: {}", e))?;
        
        // Clear all tabs except main
        self.open_tabs.retain(|tab| tab.is_main);
        self.active_tab_index = 0;
        
        // Restore each tab
        for ser_tab in serialized_tabs {
            if ser_tab.is_main {
                continue; // Skip main, it's already there
            }
            
            // Find the macro definition and restore the tab
            if ser_tab.is_library_macro {
                // Global library macro
                if let Some(macro_def) = self.library_manager.get_subgraph(&ser_tab.id) {
                    if let Ok(blueprint_graph) = self.convert_graph_description_to_blueprint(&macro_def.graph) {
                        self.open_tabs.push(GraphTab {
                            id: ser_tab.id.clone(),
                            name: ser_tab.name.clone(),
                            graph: blueprint_graph,
                            is_main: false,
                            is_dirty: false,
                            is_library_macro: true,
                            library_id: ser_tab.library_id.clone(),
                        });
                    }
                }
            } else {
                // Local macro
                if let Some(macro_def) = self.local_macros.iter().find(|m| m.id == ser_tab.id) {
                    if let Ok(blueprint_graph) = self.convert_graph_description_to_blueprint(&macro_def.graph) {
                        self.open_tabs.push(GraphTab {
                            id: ser_tab.id.clone(),
                            name: ser_tab.name.clone(),
                            graph: blueprint_graph,
                            is_main: false,
                            is_dirty: false,
                            is_library_macro: false,
                            library_id: None,
                        });
                    }
                }
            }
        }
        
        println!("📂 Restored {} tabs from tabs.json", self.open_tabs.len());
        Ok(())
    }

    /// Load a complete blueprint from a unified JSON file
    /// This loads EVERYTHING: main event graph, ALL local macros, variables, and editor state
    pub fn load_blueprint(
        &mut self,
        file_path: &str,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Strip header comments if present
        let json = content
            .lines()
            .skip_while(|line| line.trim().starts_with("//"))
            .collect::<Vec<_>>()
            .join("\n");

        // Try to load as the new unified format first
        if let Ok(blueprint_asset) = serde_json::from_str::<crate::graph::BlueprintAsset>(&json) {
            // New unified format
            self.load_from_blueprint_asset(blueprint_asset, file_path, window, cx)?;
        } else {
            // Legacy format - try to load as GraphDescription only
            let graph_description: crate::graph::GraphDescription =
                serde_json::from_str(&json).map_err(|e| format!("Failed to parse JSON: {}", e))?;
            
            // Convert back to blueprint format
            self.graph = self.convert_from_graph_description(&graph_description, window, cx)?;

            // Reset to main tab only
            self.open_tabs = vec![GraphTab {
                id: "main".to_string(),
                name: "EventGraph".to_string(),
                graph: self.graph.clone(),
                is_main: true,
                is_dirty: false,
                is_library_macro: false,
                library_id: None,
            }];
            self.active_tab_index = 0;

            // Try to load separate files for legacy support (macros.json, tabs.json, vars_save.json)
            let file_path_buf = std::path::Path::new(file_path);
            if let Some(parent) = file_path_buf.parent() {
                self.current_class_path = Some(parent.to_path_buf());
                let _ = self.load_local_macros(parent); // Ignore errors
                let _ = self.restore_tabs_state(parent, window, cx); // Ignore errors
                let _ = self.load_variables_from_class(parent); // Ignore errors
            }
            
            println!("📂 Loaded blueprint in legacy format (separate files)");
        }

        // Reload library manager to ensure engine library macros list is populated
        self.library_manager = crate::graph::LibraryManager::default();
        if let Err(e) = self.library_manager.load_all_libraries() {
            eprintln!("Failed to reload sub-graph libraries: {}", e);
        }

        cx.notify();

        Ok(())
    }
    
    /// Load blueprint from the new unified format
    fn load_from_blueprint_asset(
        &mut self,
        asset: crate::graph::BlueprintAsset,
        file_path: &str,
        window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> Result<(), String> {
        // Set current_class_path
        let file_path_buf = std::path::Path::new(file_path);
        if let Some(parent) = file_path_buf.parent() {
            self.current_class_path = Some(parent.to_path_buf());
        }
        
        // Load main graph
        self.graph = self.convert_from_graph_description(&asset.main_graph, window, cx)?;
        
        // Load local macros
        self.local_macros = asset.local_macros;
        
        // Load variables
        self.class_variables = asset.variables.iter().map(|v| {
            super::variables::ClassVariable {
                name: v.name.clone(),
                var_type: format!("{:?}", v.data_type), // Convert DataType to string
                default_value: v.default_value.clone(),
            }
        }).collect();
        
        // Initialize main tab
        self.open_tabs = vec![GraphTab {
            id: "main".to_string(),
            name: "EventGraph".to_string(),
            graph: self.graph.clone(),
            is_main: true,
            is_dirty: false,
            is_library_macro: false,
            library_id: None,
        }];
        self.active_tab_index = 0;
        
        // Restore editor state (open tabs, active tab, view states)
        if let Some(editor_state) = asset.editor_state {
            // Restore open tabs
            for tab_id in &editor_state.open_tab_ids {
                if tab_id == "main" {
                    continue; // Already added
                }
                
                // Check if this is a local macro
                if let Some(macro_def) = self.local_macros.iter().find(|m| &m.id == tab_id) {
                    if let Ok(mut blueprint_graph) = self.convert_graph_description_to_blueprint(&macro_def.graph) {
                        // Restore view state for this tab if available
                        if let Some(view_state) = editor_state.graph_view_states.get(tab_id) {
                            blueprint_graph.pan_offset = gpui::Point {
                                x: view_state.pan_offset_x,
                                y: view_state.pan_offset_y,
                            };
                            blueprint_graph.zoom_level = view_state.zoom;
                        }
                        
                        self.open_tabs.push(GraphTab {
                            id: tab_id.clone(),
                            name: macro_def.name.clone(),
                            graph: blueprint_graph,
                            is_main: false,
                            is_dirty: false,
                            is_library_macro: false,
                            library_id: None,
                        });
                    }
                }
            }
            
            // Restore view state for main tab
            if let Some(view_state) = editor_state.graph_view_states.get("main") {
                if let Some(main_tab) = self.open_tabs.iter_mut().find(|t| t.is_main) {
                    main_tab.graph.pan_offset = gpui::Point {
                        x: view_state.pan_offset_x,
                        y: view_state.pan_offset_y,
                    };
                    main_tab.graph.zoom_level = view_state.zoom;
                }
                
                // Also update self.graph with main tab's view state
                self.graph.pan_offset = gpui::Point {
                    x: view_state.pan_offset_x,
                    y: view_state.pan_offset_y,
                };
                self.graph.zoom_level = view_state.zoom;
            }
            
            // Restore active tab index (with bounds check)
            self.active_tab_index = editor_state.active_tab_index.min(self.open_tabs.len().saturating_sub(1));
            
            // Load the active tab's graph into self.graph
            if let Some(active_tab) = self.open_tabs.get(self.active_tab_index) {
                self.graph = active_tab.graph.clone();
            }
        }
        
        println!("📂 Loaded complete blueprint from {}", file_path);
        println!("   ├─ Main event graph");
        println!("   ├─ {} local macros", self.local_macros.len());
        println!("   ├─ {} variables", self.class_variables.len());
        println!("   └─ {} open tabs restored", self.open_tabs.len());
        
        Ok(())
    }

    fn convert_from_graph_description(
        &mut self,
        graph_desc: &crate::graph::GraphDescription,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> Result<BlueprintGraph, String> {
        let mut nodes = Vec::new();
        let mut connections = Vec::new();

        // Load node definitions to populate descriptions and other metadata
        let node_definitions = NodeDefinitions::load();

        // Convert nodes
        for (node_id, node_instance) in &graph_desc.nodes {
            // Use node_type field as the definition_id to look up full node metadata
            let definition_id = node_instance.node_type.clone();

            // Look up node definition by ID to restore all metadata
            let node_def = node_definitions.get_node_definition(&definition_id);

            let (title, icon, description, node_type, color) = if definition_id == "reroute" {
                // Special handling for reroute nodes
                (
                    "Reroute".to_string(),
                    "•".to_string(),
                    "Reroute node for organizing connections".to_string(),
                    NodeType::Reroute,
                    None,
                )
            } else if let Some(def) = node_def {
                let category = node_definitions.get_category_for_node(&def.id);
                let node_type = match category.map(|c| c.name.as_str()) {
                    Some("Events") => NodeType::Event,
                    Some("Logic") => NodeType::Logic,
                    Some("Math") => NodeType::Math,
                    Some("Object") => NodeType::Object,
                    _ => NodeType::Logic,
                };
                (
                    def.name.clone(),
                    def.icon.clone(),
                    def.description.clone(),
                    node_type,
                    def.color.clone(),
                )
            } else {
                // Fallback if definition not found
                (
                    definition_id.replace('_', " "),
                    "⚙️".to_string(),
                    String::new(),
                    NodeType::Logic,
                    None,
                )
            };

            let bp_node = BlueprintNode {
                id: node_id.clone(),
                definition_id,
                title,
                icon,
                node_type,
                position: Point::new(node_instance.position.x, node_instance.position.y),
                size: Size::new(150.0, 100.0),
                inputs: node_instance
                    .inputs
                    .iter()
                    .map(|pin_inst| {
                        let pin = &pin_inst.pin;
                        Pin {
                            id: pin_inst.id.clone(),
                            name: pin.name.clone(),
                            pin_type: match pin.pin_type {
                                crate::graph::PinType::Input => PinType::Input,
                                crate::graph::PinType::Output => PinType::Output,
                            },
                            data_type: pin.data_type.clone(),
                        }
                    })
                    .collect(),
                outputs: node_instance
                    .outputs
                    .iter()
                    .map(|pin_inst| {
                        let pin = &pin_inst.pin;
                        Pin {
                            id: pin_inst.id.clone(),
                            name: pin.name.clone(),
                            pin_type: match pin.pin_type {
                                crate::graph::PinType::Input => PinType::Input,
                                crate::graph::PinType::Output => PinType::Output,
                            },
                            data_type: pin.data_type.clone(),
                        }
                    })
                    .collect(),
                properties: node_instance
                    .properties
                    .iter()
                    .map(|(k, v)| {
                        let value_str = match v {
                            crate::graph::PropertyValue::String(s) => s.clone(),
                            crate::graph::PropertyValue::Number(n) => n.to_string(),
                            crate::graph::PropertyValue::Boolean(b) => b.to_string(),
                            _ => "".to_string(),
                        };
                        (k.clone(), value_str)
                    })
                    .collect(),
                is_selected: false,
                description,
                color,
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

        let mut comments = graph_desc.comments.clone();
        // Ensure all comments have a color_picker_state
        for comment in &mut comments {
            if comment.color_picker_state.is_none() {
                comment.color_picker_state = Some(
                    cx.new(|cx| gpui_component::color_picker::ColorPickerState::new(window, cx)),
                );
            }
        }

        // Subscribe to color picker changes for each comment
        for comment in &mut comments {
            if let Some(picker_state) = comment.color_picker_state.as_ref() {
                let comment_id = comment.id.clone();
                let subscription = cx.subscribe_in(
                    picker_state,
                    window,
                    move |this: &mut BlueprintEditorPanel,
                          _picker,
                          event: &gpui_component::color_picker::ColorPickerEvent,
                          _window,
                          cx| {
                        if let gpui_component::color_picker::ColorPickerEvent::Change(Some(color)) =
                            event
                        {
                            if let Some(comment) =
                                this.graph.comments.iter_mut().find(|c| c.id == comment_id)
                            {
                                comment.color = *color;
                                cx.notify();
                            }
                        }
                    },
                );
                // Store the subscription to keep it alive
            }
        }

        Ok(BlueprintGraph {
            nodes,
            connections,
            comments,
            selected_nodes: vec![],
            selected_comments: vec![],
            zoom_level: 1.0,
            pan_offset: Point::new(0.0, 0.0),
            virtualization_stats: VirtualizationStats::default(),
        })
    }

    // Conversion function no longer needed since we use the unified DataType system

    pub fn start_drag(&mut self, node_id: String, mouse_pos: Point<f32>, cx: &mut Context<Self>) {
        print!(
            "Starting drag for node {} at mouse position {:?}",
            node_id, mouse_pos
        );
        if let Some(node) = self.graph.nodes.iter().find(|n| n.id == node_id) {
            self.dragging_node = Some(node_id.clone());
            self.drag_offset =
                Point::new(mouse_pos.x - node.position.x, mouse_pos.y - node.position.y);

            // Store initial positions of all selected nodes for multi-node dragging
            self.initial_drag_positions.clear();

            // If the dragged node is selected, drag all selected nodes
            if self.graph.selected_nodes.contains(&node_id) {
                for selected_id in &self.graph.selected_nodes {
                    if let Some(selected_node) =
                        self.graph.nodes.iter().find(|n| n.id == *selected_id)
                    {
                        self.initial_drag_positions
                            .insert(selected_id.clone(), selected_node.position);
                    }
                }
            } else {
                // If dragging a non-selected node, just drag that one
                self.initial_drag_positions
                    .insert(node_id.clone(), node.position);
            }

            // Close any open tooltips when starting drag
            self.hide_hoverable_tooltip(cx);
            cx.notify();
        }
    }

    pub fn update_drag(&mut self, mouse_pos: Point<f32>, cx: &mut Context<Self>) {
        if let Some(dragging_id) = &self.dragging_node.clone() {
            // Calculate the new position of the main dragged node
            let new_position = Point::new(
                mouse_pos.x - self.drag_offset.x,
                mouse_pos.y - self.drag_offset.y,
            );

            // Get the initial position of the dragged node
            if let Some(initial_pos) = self.initial_drag_positions.get(dragging_id) {
                // Calculate the delta from the initial position
                let delta = Point::new(
                    new_position.x - initial_pos.x,
                    new_position.y - initial_pos.y,
                );

                // Move all nodes that were selected when dragging started
                for (node_id, initial_position) in &self.initial_drag_positions {
                    if let Some(node) = self.graph.nodes.iter_mut().find(|n| n.id == *node_id) {
                        node.position =
                            Point::new(initial_position.x + delta.x, initial_position.y + delta.y);
                    }
                }

                cx.notify();
            }
        }
    }

    pub fn end_drag(&mut self, cx: &mut Context<Self>) {
        // Update comment containment after drag
        for comment in self.graph.comments.iter_mut() {
            comment.update_contained_nodes(&self.graph.nodes);
        }

        self.dragging_node = None;
        self.initial_drag_positions.clear();
        cx.notify();
    }

    pub fn update_comment_drag(&mut self, mouse_pos: Point<f32>, cx: &mut Context<Self>) {
        if let Some(comment_id) = &self.dragging_comment.clone() {
            let new_position = Point::new(
                mouse_pos.x - self.drag_offset.x,
                mouse_pos.y - self.drag_offset.y,
            );

            if let Some(comment) = self.graph.comments.iter_mut().find(|c| c.id == *comment_id) {
                let delta = Point::new(
                    new_position.x - comment.position.x,
                    new_position.y - comment.position.y,
                );

                comment.position = new_position;

                // Move all contained nodes with the comment
                for node_id in &comment.contained_node_ids {
                    if let Some(node) = self.graph.nodes.iter_mut().find(|n| n.id == *node_id) {
                        node.position.x += delta.x;
                        node.position.y += delta.y;
                    }
                }

                cx.notify();
            }
        }
    }

    pub fn end_comment_drag(&mut self, cx: &mut Context<Self>) {
        // Update contained nodes before ending drag
        if let Some(comment_id) = &self.dragging_comment.clone() {
            if let Some(comment) = self.graph.comments.iter_mut().find(|c| c.id == *comment_id) {
                comment.update_contained_nodes(&self.graph.nodes);
            }
        }

        self.dragging_comment = None;
        cx.notify();
    }

    pub fn update_comment_resize(&mut self, mouse_pos: Point<f32>, cx: &mut Context<Self>) {
        if let Some((comment_id, handle)) = &self.resizing_comment.clone() {
            if let Some(comment) = self.graph.comments.iter_mut().find(|c| c.id == *comment_id) {
                let delta_x = mouse_pos.x - self.drag_offset.x;
                let delta_y = mouse_pos.y - self.drag_offset.y;

                match handle {
                    ResizeHandle::TopLeft => {
                        comment.position.x += delta_x;
                        comment.position.y += delta_y;
                        comment.size.width -= delta_x;
                        comment.size.height -= delta_y;
                    }
                    ResizeHandle::TopRight => {
                        comment.position.y += delta_y;
                        comment.size.width += delta_x;
                        comment.size.height -= delta_y;
                    }
                    ResizeHandle::BottomLeft => {
                        comment.position.x += delta_x;
                        comment.size.width -= delta_x;
                        comment.size.height += delta_y;
                    }
                    ResizeHandle::BottomRight => {
                        comment.size.width += delta_x;
                        comment.size.height += delta_y;
                    }
                    ResizeHandle::Top => {
                        comment.position.y += delta_y;
                        comment.size.height -= delta_y;
                    }
                    ResizeHandle::Bottom => {
                        comment.size.height += delta_y;
                    }
                    ResizeHandle::Left => {
                        comment.position.x += delta_x;
                        comment.size.width -= delta_x;
                    }
                    ResizeHandle::Right => {
                        comment.size.width += delta_x;
                    }
                }

                // Enforce minimum size
                comment.size.width = comment.size.width.max(100.0);
                comment.size.height = comment.size.height.max(50.0);

                self.drag_offset = mouse_pos;
                cx.notify();
            }
        }
    }

    pub fn end_comment_resize(&mut self, cx: &mut Context<Self>) {
        // Update contained nodes before ending resize
        if let Some((comment_id, _)) = &self.resizing_comment.clone() {
            if let Some(comment) = self.graph.comments.iter_mut().find(|c| c.id == *comment_id) {
                comment.update_contained_nodes(&self.graph.nodes);
            }
        }

        self.resizing_comment = None;
        cx.notify();
    }

    pub fn finish_comment_editing(&mut self, cx: &mut Context<Self>) {
        if let Some(comment_id) = &self.editing_comment.clone() {
            // Get the edited text from the input
            let new_text = self.comment_text_input.read(cx).text().to_string();

            // Update the comment
            if let Some(comment) = self.graph.comments.iter_mut().find(|c| c.id == *comment_id) {
                comment.text = new_text;
            }

            self.editing_comment = None;
            cx.notify();
        }
    }

    pub fn create_comment_at_center(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        // Create a new comment at the center of the current view
        // TODO: This should NOT be hardcoded
        let center_screen = Point::new(1920.0 / 2.0, 1080.0 / 2.0); // Center of typical view
        let center_graph = super::node_graph::NodeGraphRenderer::screen_to_graph_pos(
            gpui::Point::new(px(center_screen.x), px(center_screen.y)),
            &self.graph,
        );

        let mut new_comment = super::BlueprintComment::new(center_graph, window, cx);

        // Subscribe to color picker changes for this comment
        if let Some(picker_state) = new_comment.color_picker_state.as_ref() {
            let comment_id = new_comment.id.clone();
            cx.subscribe_in(
                picker_state,
                window,
                move |this: &mut BlueprintEditorPanel,
                      _picker,
                      event: &gpui_component::color_picker::ColorPickerEvent,
                      _window,
                      cx| {
                    if let gpui_component::color_picker::ColorPickerEvent::Change(Some(color)) =
                        event
                    {
                        if let Some(comment) =
                            this.graph.comments.iter_mut().find(|c| c.id == comment_id)
                        {
                            comment.color = *color;
                            cx.notify();
                        }
                    }
                },
            );
        }

        self.graph.comments.push(new_comment);

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
        self.graph
            .connections
            .retain(|conn| conn.from_node_id != node_id && conn.to_node_id != node_id);

        // Remove from selected nodes
        self.graph.selected_nodes.retain(|id| *id != node_id);

        cx.notify();
    }

    pub fn copy_node(&mut self, node_id: String, _cx: &mut Context<Self>) {
        // For now, just store in a simple static location
        // TODO: We should use the system clipboard
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
            !(conn.from_node_id == node_id && conn.from_pin_id == pin_id)
                && !(conn.to_node_id == node_id && conn.to_pin_id == pin_id)
        });
        cx.notify();
    }

    pub fn start_connection_drag_from_pin(
        &mut self,
        node_id: String,
        pin_id: String,
        cx: &mut Context<Self>,
    ) {
        // Find the pin to get its data type
        if let Some(node) = self.graph.nodes.iter().find(|n| n.id == node_id) {
            if let Some(pin) = node.outputs.iter().find(|p| p.id == pin_id) {
                println!(
                    "Starting connection drag from pin {} on node {}",
                    pin_id, node_id
                );
                self.dragging_connection = Some(ConnectionDrag {
                    from_node_id: node_id,
                    from_pin_id: pin_id,
                    from_pin_type: pin.data_type.clone(),
                    current_mouse_pos: Point::new(0.0, 0.0), // Will be updated by mouse move
                    target_pin: None,
                });
                // Close any open tooltips when starting connection drag
                self.hide_hoverable_tooltip(cx);
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

    pub fn set_connection_target(
        &mut self,
        target: Option<(String, String)>,
        cx: &mut Context<Self>,
    ) {
        if let Some(ref mut drag) = self.dragging_connection {
            drag.target_pin = target;
            cx.notify();
        }
    }

    pub fn complete_connection_on_pin(
        &mut self,
        node_id: String,
        pin_id: String,
        cx: &mut Context<Self>,
    ) {
        if let Some(drag) = self.dragging_connection.take() {
            // Find the target pin to check compatibility
            if let Some(node) = self.graph.nodes.iter().find(|n| n.id == node_id) {
                if let Some(pin) = node.inputs.iter().find(|p| p.id == pin_id) {
                    // Check if compatible and not same node
                    // Use is_compatible_with to allow Any type to match with everything
                    if drag.from_pin_type.is_compatible_with(&pin.data_type)
                        && drag.from_node_id != node_id
                    {
                        let is_execution_pin =
                            pin.data_type == GraphDataType::from_type_str("execution");

                        // Check if source or target is a reroute node
                        let source_is_reroute =
                            self.graph.nodes.iter().any(|n| {
                                n.id == drag.from_node_id && n.node_type == NodeType::Reroute
                            });
                        let target_is_reroute = self
                            .graph
                            .nodes
                            .iter()
                            .any(|n| n.id == node_id && n.node_type == NodeType::Reroute);

                        // Remove old connections based on pin and node types
                        if is_execution_pin || source_is_reroute || target_is_reroute {
                            // For execution pins, reroute outputs, remove existing connections from source
                            if is_execution_pin || source_is_reroute {
                                println!(
                                    "Removing old connection from source {}:{}",
                                    drag.from_node_id, drag.from_pin_id
                                );
                                self.graph.connections.retain(|conn| {
                                    !(conn.from_node_id == drag.from_node_id
                                        && conn.from_pin_id == drag.from_pin_id)
                                });
                            }

                            // For execution pins, reroute inputs, or regular inputs, remove existing connections to target
                            if is_execution_pin || target_is_reroute {
                                println!(
                                    "Removing old connection to target {}:{}",
                                    node_id, pin_id
                                );
                                self.graph.connections.retain(|conn| {
                                    !(conn.to_node_id == node_id && conn.to_pin_id == pin_id)
                                });
                            }
                        }

                        // For non-reroute, non-execution data pins, apply standard single-input rule
                        if !is_execution_pin && !target_is_reroute {
                            // Remove any existing connection to this input pin (move connection behavior)
                            println!(
                                "Removing old data connection to target {}:{}",
                                node_id, pin_id
                            );
                            self.graph.connections.retain(|conn| {
                                !(conn.to_node_id == node_id && conn.to_pin_id == pin_id)
                            });
                        }

                        println!(
                            "Creating connection from {}:{} to {}:{}",
                            drag.from_node_id, drag.from_pin_id, node_id, pin_id
                        );

                        // Create the new connection
                        let connection = super::Connection {
                            id: uuid::Uuid::new_v4().to_string(),
                            from_node_id: drag.from_node_id.clone(),
                            from_pin_id: drag.from_pin_id.clone(),
                            to_node_id: node_id.clone(),
                            to_pin_id: pin_id.clone(),
                        };
                        self.graph.connections.push(connection);
                        println!("Connection created successfully!");

                        // Propagate types through reroute nodes (reuse the checks from above)
                        if source_is_reroute || target_is_reroute {
                            // Propagate the non-Any type through the reroute chain
                            if target_is_reroute {
                                self.propagate_reroute_types(
                                    node_id.clone(),
                                    drag.from_pin_type.clone(),
                                    cx,
                                );
                            } else if source_is_reroute {
                                self.propagate_reroute_types(
                                    drag.from_node_id.clone(),
                                    pin.data_type.clone(),
                                    cx,
                                );
                            }
                        }
                    } else {
                        println!("Incompatible pin types or same node");
                    }
                }
            }
            cx.notify();
        }
    }

    // Node creation menu methods
    pub fn show_node_creation_menu(
        &mut self,
        position: Point<f32>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.node_creation_menu.is_some() {
            self.dismiss_node_creation_menu(cx);
        }

        // Calculate smart positioning to avoid going off-screen
        let adjusted_position = self.calculate_menu_position(position, window);

        // Create the search input state that the menu will use
        let search_input_state =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search nodes..."));

        // Get weak reference to self for the menu
        let panel_weak = cx.entity().downgrade();

        let menu = cx.new(|cx| {
            NodeCreationMenu::new(
                adjusted_position,
                search_input_state.clone(),
                panel_weak,
                cx,
            )
        });
        // Subscribe to the menu events
        cx.subscribe(&menu, Self::on_node_creation_event).detach();
        self.node_creation_menu = Some(menu);
        self.node_creation_menu_position = Some(adjusted_position);
        cx.notify();
    }

    /// Calculate smart menu positioning to prevent off-screen placement
    fn calculate_menu_position(
        &self,
        requested_position: Point<f32>,
        window: &Window,
    ) -> Point<f32> {
        // Get actual window viewport size
        let window_bounds = window.bounds();
        let viewport_width = window_bounds.size.width.as_f32();
        let viewport_height = window_bounds.size.height.as_f32();

        // Simple edge padding to keep menu away from window edges
        let edge_padding = 10.0;

        // Start with requested position (where mouse clicked)
        let mut adjusted_x = requested_position.x;
        let mut adjusted_y = requested_position.y;

        // Clamp to window bounds with padding
        // Ensure menu doesn't go off right edge
        if adjusted_x + NODE_MENU_WIDTH + edge_padding > viewport_width {
            adjusted_x = viewport_width - NODE_MENU_WIDTH - edge_padding;
        }
        // Ensure menu doesn't go off left edge
        if adjusted_x < edge_padding {
            adjusted_x = edge_padding;
        }

        // Ensure menu doesn't go off bottom edge
        if adjusted_y + NODE_MENU_MAX_HEIGHT + edge_padding > viewport_height {
            adjusted_y = viewport_height - NODE_MENU_MAX_HEIGHT - edge_padding;
        }
        // Ensure menu doesn't go off top edge
        if adjusted_y < edge_padding {
            adjusted_y = edge_padding;
        }

        Point::new(adjusted_x, adjusted_y)
    }

    fn on_node_creation_event(
        &mut self,
        _menu: Entity<NodeCreationMenu>,
        event: &NodeCreationEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            NodeCreationEvent::CreateNode(node) => {
                self.add_node(node.clone(), cx);
                self.dismiss_node_creation_menu(cx);
            }
            NodeCreationEvent::Dismiss => {
                self.dismiss_node_creation_menu(cx);
            }
        }
    }

    pub fn dismiss_node_creation_menu(&mut self, cx: &mut Context<Self>) {
        self.node_creation_menu = None;
        self.node_creation_menu_position = None;
        cx.notify();
    }

    // Hoverable tooltip methods
    pub fn show_hoverable_tooltip(
        &mut self,
        content: String,
        position: Point<f32>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Store pending tooltip and start timer
        self.pending_tooltip = Some((content.clone(), position));

        cx.spawn(async move |view, mut cx| {
            // Wait 2 seconds
            Timer::after(Duration::from_secs(2)).await;

            // Show tooltip if still pending
            cx.update(|cx| {
                view.update(cx, |panel, cx| {
                    // Only show if we still have a pending tooltip (user hasn't moved away)
                    if let Some((pending_content, pending_pos)) = panel.pending_tooltip.take() {
                        let pixel_pos = Point::new(px(pending_pos.x), px(pending_pos.y));
                        panel.hoverable_tooltip =
                            Some(HoverableTooltip::new(pending_content, pixel_pos, cx));
                        cx.notify();
                    }
                });
            })
            .ok();
        })
        .detach();
    }

    pub fn hide_hoverable_tooltip(&mut self, cx: &mut Context<Self>) {
        self.hoverable_tooltip = None;
        self.pending_tooltip = None; // Cancel pending tooltip
        cx.notify();
    }

    pub fn update_tooltip_position(&mut self, position: Point<f32>, cx: &mut Context<Self>) {
        if let Some(tooltip) = &self.hoverable_tooltip {
            let pixel_pos = Point::new(px(position.x), px(position.y));
            tooltip.update(cx, |tooltip, cx| {
                tooltip.set_position(pixel_pos, cx);
            });
        }
    }

    pub fn check_tooltip_hover(&mut self, mouse_pos: Point<f32>, cx: &mut Context<Self>) {
        if let Some(tooltip) = &self.hoverable_tooltip {
            let pixel_pos = Point::new(px(mouse_pos.x), px(mouse_pos.y));
            tooltip.update(cx, |tooltip, cx| {
                tooltip.check_to_hide(pixel_pos, cx);
            });

            // Remove tooltip if it's been hidden
            let is_open = tooltip.read(cx).open;
            if !is_open {
                self.hoverable_tooltip = None;
                cx.notify();
            }
        }
    }

    /// Check if a screen position is inside the node creation menu bounds
    pub fn is_position_inside_menu(&self, screen_pos: Point<f32>) -> bool {
        if let (Some(_), Some(position)) =
            (&self.node_creation_menu, &self.node_creation_menu_position)
        {
            let menu_left = position.x;
            let menu_top = position.y;
            let menu_right = menu_left + NODE_MENU_WIDTH;
            let menu_bottom = menu_top + NODE_MENU_MAX_HEIGHT;

            screen_pos.x >= menu_left
                && screen_pos.x <= menu_right
                && screen_pos.y >= menu_top
                && screen_pos.y <= menu_bottom
        } else {
            false
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
        let screen = Point::new(screen_pos.x.as_f32(), screen_pos.y.as_f32());

        // Compute graph/world position under cursor before zoom using the shared helper
        // (keeps conversion identical to other codepaths that use this helper)
        let focus_graph_pos = super::node_graph::NodeGraphRenderer::screen_to_graph_pos(
            Point::new(px(screen.x), px(screen.y)),
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

        let screen_after =
            super::node_graph::NodeGraphRenderer::graph_to_screen_pos(focus_graph_pos, &self.graph);
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

    pub fn start_selection_drag(
        &mut self,
        start_pos: Point<f32>,
        add_to_selection: bool,
        cx: &mut Context<Self>,
    ) {
        self.selection_start = Some(start_pos);
        self.selection_end = Some(start_pos);

        // DON'T clear selection here - wait until mouse actually moves
        // This prevents clearing selection on simple clicks
        // The selection will be cleared in update_selection_drag if not adding to selection

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

    // TODO: Hot path, avoid alloc here
    pub fn end_selection_drag(&mut self, cx: &mut Context<Self>) {
        // If selection start and end are the same (or very close), it was a click, not a drag
        // Clear the selection in this case
        if let (Some(start), Some(end)) = (self.selection_start, self.selection_end) {
            let distance = ((end.x - start.x).powi(2) + (end.y - start.y).powi(2)).sqrt();
            if distance < 5.0 {
                // It was just a click on empty space, clear selection
                self.graph.selected_nodes.clear();
                println!("[SELECTION] Cleared selection (click on empty space)");
            }
        }

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
                let intersects = !(node_right < min_x
                    || node_left > max_x
                    || node_bottom < min_y
                    || node_top > max_y);

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
        println!(
            "[DELETE] Selected nodes count: {}",
            self.graph.selected_nodes.len()
        );
        println!(
            "[DELETE] Selected node IDs: {:?}",
            self.graph.selected_nodes
        );

        if !self.graph.selected_nodes.is_empty() {
            let node_count_before = self.graph.nodes.len();

            // Remove selected nodes
            self.graph
                .nodes
                .retain(|node| !self.graph.selected_nodes.contains(&node.id));

            let node_count_after = self.graph.nodes.len();
            println!(
                "[DELETE] Deleted {} nodes ({} -> {})",
                node_count_before - node_count_after,
                node_count_before,
                node_count_after
            );

            // Remove connections involving deleted nodes
            self.graph.connections.retain(|connection| {
                !self.graph.selected_nodes.contains(&connection.from_node_id)
                    && !self.graph.selected_nodes.contains(&connection.to_node_id)
            });

            // Clear selection
            self.graph.selected_nodes.clear();
            cx.notify();
        } else {
            println!("[DELETE] No nodes selected, nothing to delete");
        }
    }

    /// Handle click on empty space - detects double-clicks on connections to create reroute nodes
    /// Returns true if a double-click was handled
    pub fn handle_empty_space_click(
        &mut self,
        graph_pos: Point<f32>,
        cx: &mut Context<Self>,
    ) -> bool {
        let now = std::time::Instant::now();
        let is_double_click = if let (Some(last_time), Some(last_pos)) =
            (self.last_click_time, self.last_click_pos)
        {
            let time_diff = now.duration_since(last_time).as_millis();
            let pos_diff =
                ((graph_pos.x - last_pos.x).powi(2) + (graph_pos.y - last_pos.y).powi(2)).sqrt();
            println!(
                "[REROUTE] Double-click check: time_diff={}ms, pos_diff={:.2}px",
                time_diff, pos_diff
            );
            time_diff < 500 && pos_diff < 50.0 // 500ms and 50 pixels threshold (relaxed)
        } else {
            false
        };

        if is_double_click {
            println!("[REROUTE] Double-click detected! Checking for nearby connections...");
            // Check if we're near any connection
            if let Some(connection) = self.find_connection_near_point(graph_pos) {
                println!("[REROUTE] Found connection near click point!");
                // Get the data type of the connection
                if let Some(data_type) = self.get_connection_data_type(&connection) {
                    // Create a typeless reroute node at the click position
                    let reroute_node = BlueprintNode::create_reroute(graph_pos);
                    let reroute_id = reroute_node.id.clone();

                    // Add the reroute node
                    self.graph.nodes.push(reroute_node);

                    // Split the connection: remove original and create two new ones
                    let from_node = connection.from_node_id.clone();
                    let from_pin = connection.from_pin_id.clone();
                    let to_node = connection.to_node_id.clone();
                    let to_pin = connection.to_pin_id.clone();

                    // Remove original connection
                    self.graph.connections.retain(|c| c.id != connection.id);

                    // Create first connection: original source -> reroute
                    self.graph.connections.push(super::Connection {
                        id: uuid::Uuid::new_v4().to_string(),
                        from_node_id: from_node,
                        from_pin_id: from_pin,
                        to_node_id: reroute_id.clone(),
                        to_pin_id: "input".to_string(),
                    });

                    // Create second connection: reroute -> original target
                    self.graph.connections.push(super::Connection {
                        id: uuid::Uuid::new_v4().to_string(),
                        from_node_id: reroute_id.clone(),
                        from_pin_id: "output".to_string(),
                        to_node_id: to_node,
                        to_pin_id: to_pin,
                    });

                    // Propagate types through the reroute chain
                    self.propagate_reroute_types(reroute_id, data_type, cx);

                    cx.notify();
                    // Reset double-click tracking
                    self.last_click_time = None;
                    self.last_click_pos = None;
                    return true; // Double-click was handled
                }
            }

            // Reset double-click tracking if we didn't handle it
            self.last_click_time = None;
            self.last_click_pos = None;
        } else {
            // Record this click for double-click detection
            self.last_click_time = Some(now);
            self.last_click_pos = Some(graph_pos);
        }

        false // No double-click handled
    }

    /// Find a connection near the given point (within a threshold distance)
    fn find_connection_near_point(&self, point: Point<f32>) -> Option<super::Connection> {
        const CLICK_THRESHOLD: f32 = 30.0; // pixels (increased for easier clicking)

        println!(
            "[REROUTE] Checking {} connections",
            self.graph.connections.len()
        );

        for connection in &self.graph.connections {
            // Get the from and to positions
            let from_node = self
                .graph
                .nodes
                .iter()
                .find(|n| n.id == connection.from_node_id)?;
            let to_node = self
                .graph
                .nodes
                .iter()
                .find(|n| n.id == connection.to_node_id)?;

            // Calculate pin positions (simplified - using node centers for now)
            let from_pos = Point::new(
                from_node.position.x + from_node.size.width,
                from_node.position.y + from_node.size.height / 2.0,
            );
            let to_pos = Point::new(
                to_node.position.x,
                to_node.position.y + to_node.size.height / 2.0,
            );

            // Check if point is near the connection line using bezier approximation
            if self.is_point_near_bezier(point, from_pos, to_pos, CLICK_THRESHOLD) {
                println!("[REROUTE] Found connection within threshold!");
                return Some(connection.clone());
            }
        }

        println!("[REROUTE] No connection found near point");
        None
    }

    /// Check if a point is near a bezier curve
    fn is_point_near_bezier(
        &self,
        point: Point<f32>,
        from: Point<f32>,
        to: Point<f32>,
        threshold: f32,
    ) -> bool {
        // Sample the bezier curve and check distance to each sample
        let distance = (to.x - from.x).abs();
        let control_offset = (distance * 0.4).max(50.0).min(150.0);
        let control1 = Point::new(from.x + control_offset, from.y);
        let control2 = Point::new(to.x - control_offset, to.y);

        // Sample 20 points along the curve
        for i in 0..=20 {
            let t = i as f32 / 20.0;
            let curve_point = self.bezier_point(from, control1, control2, to, t);
            let dist =
                ((point.x - curve_point.x).powi(2) + (point.y - curve_point.y).powi(2)).sqrt();
            if dist < threshold {
                return true;
            }
        }

        false
    }

    /// Calculate a point on a cubic bezier curve
    fn bezier_point(
        &self,
        p0: Point<f32>,
        p1: Point<f32>,
        p2: Point<f32>,
        p3: Point<f32>,
        t: f32,
    ) -> Point<f32> {
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

    /// Get the data type of a connection
    fn get_connection_data_type(
        &self,
        connection: &super::Connection,
    ) -> Option<crate::graph::DataType> {
        let from_node = self
            .graph
            .nodes
            .iter()
            .find(|n| n.id == connection.from_node_id)?;
        let output_pin = from_node
            .outputs
            .iter()
            .find(|p| p.id == connection.from_pin_id)?;
        Some(output_pin.data_type.clone())
    }

    /// Propagate data types through connected reroute nodes
    /// When a typed connection is made to/from a reroute node, all connected reroute nodes should adopt that type
    fn propagate_reroute_types(
        &mut self,
        start_node_id: String,
        data_type: crate::graph::DataType,
        _cx: &mut Context<Self>,
    ) {
        use std::collections::{HashSet, VecDeque};

        // Skip propagation for Any type (already typeless)
        if data_type == crate::graph::DataType::Any {
            return;
        }

        // BFS to find all connected reroute nodes
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(start_node_id);

        while let Some(node_id) = queue.pop_front() {
            if visited.contains(&node_id) {
                continue;
            }
            visited.insert(node_id.clone());

            // Check if this is a reroute node
            if let Some(node) = self.graph.nodes.iter_mut().find(|n| n.id == node_id) {
                if node.node_type == NodeType::Reroute {
                    // Set the type of all pins to the propagated type
                    for pin in &mut node.inputs {
                        pin.data_type = data_type.clone();
                    }
                    for pin in &mut node.outputs {
                        pin.data_type = data_type.clone();
                    }

                    // Find all connected reroute nodes
                    for connection in &self.graph.connections {
                        if connection.from_node_id == node_id {
                            // Check if target is a reroute node
                            if let Some(target_node) = self
                                .graph
                                .nodes
                                .iter()
                                .find(|n| n.id == connection.to_node_id)
                            {
                                if target_node.node_type == NodeType::Reroute {
                                    queue.push_back(connection.to_node_id.clone());
                                }
                            }
                        } else if connection.to_node_id == node_id {
                            // Check if source is a reroute node
                            if let Some(source_node) = self
                                .graph
                                .nodes
                                .iter()
                                .find(|n| n.id == connection.from_node_id)
                            {
                                if source_node.node_type == NodeType::Reroute {
                                    queue.push_back(connection.from_node_id.clone());
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Variable management methods

    /// Start creating a new variable
    pub fn start_creating_variable(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.is_creating_variable = true;

        // Create a new empty input state
        self.variable_name_input = cx.new(|cx| {
            gpui_component::input::InputState::new(window, cx).placeholder("Variable name...")
        });

        // Populate dropdown with available types
        let available_types = self.get_available_types();
        let type_items: Vec<super::variables::TypeItem> = available_types
            .into_iter()
            .map(|type_str| super::variables::TypeItem::new(type_str))
            .collect();

        self.variable_type_dropdown.update(cx, |dropdown, cx| {
            dropdown.set_items(type_items, window, cx);
            dropdown.set_selected_index(Some(gpui_component::IndexPath::default()), window, cx);
        });

        cx.notify();
    }

    /// Cancel variable creation
    pub fn cancel_creating_variable(&mut self, cx: &mut Context<Self>) {
        self.is_creating_variable = false;
        cx.notify();
    }

    /// Complete variable creation - add the variable to the class
    pub fn complete_creating_variable(&mut self, cx: &mut Context<Self>) {
        let name = self
            .variable_name_input
            .read(cx)
            .text()
            .to_string()
            .trim()
            .to_string();
        let selected_type = self
            .variable_type_dropdown
            .read(cx)
            .selected_value()
            .map(|v| v.to_string())
            .unwrap_or_else(|| "i32".to_string());

        if !name.is_empty() {
            let variable = super::variables::ClassVariable {
                name,
                var_type: selected_type,
                default_value: None,
            };
            self.class_variables.push(variable);

            // Auto-save variables when modified
            if let Err(e) = self.save_variables_to_class() {
                eprintln!("Failed to save variables: {}", e);
            }
        }
        self.is_creating_variable = false;
        cx.notify();
    }

    /// Remove a variable from the class by name
    pub fn remove_variable(&mut self, name: &str, cx: &mut Context<Self>) {
        self.class_variables.retain(|v| v.name != name);

        // Auto-save variables when modified
        if let Err(e) = self.save_variables_to_class() {
            eprintln!("Failed to save variables: {}", e);
        }

        cx.notify();
    }

    /// Get all available types from blueprint nodes
    pub fn get_available_types(&self) -> Vec<String> {
        crate::compiler::type_extractor::extract_all_blueprint_types()
    }

    /// Add a new pin to the subgraph input node
    pub fn add_input_pin(&mut self, cx: &mut Context<Self>) {
        if let Some(input_node) = self.graph.nodes.iter_mut().find(|n| n.definition_id == "subgraph_input") {
            let pin_count = input_node.outputs.len();
            let new_pin = Pin {
                id: format!("input_{}", pin_count),
                name: format!("Input {}", pin_count + 1),
                pin_type: PinType::Output, // Input node has outputs
                data_type: DataType::Execution,
            };
            input_node.outputs.push(new_pin);
            cx.notify();
        }
    }

    /// Add a new pin to the subgraph output node
    pub fn add_output_pin(&mut self, cx: &mut Context<Self>) {
        if let Some(output_node) = self.graph.nodes.iter_mut().find(|n| n.definition_id == "subgraph_output") {
            let pin_count = output_node.inputs.len();
            let new_pin = Pin {
                id: format!("output_{}", pin_count),
                name: format!("Output {}", pin_count + 1),
                pin_type: PinType::Input, // Output node has inputs
                data_type: DataType::Execution,
            };
            output_node.inputs.push(new_pin);
            cx.notify();
        }
    }

    /// Remove a pin from the subgraph input node
    pub fn remove_input_pin(&mut self, pin_id: &str, cx: &mut Context<Self>) {
        if let Some(input_node) = self.graph.nodes.iter_mut().find(|n| n.definition_id == "subgraph_input") {
            input_node.outputs.retain(|p| p.id != pin_id);
            cx.notify();
        }
    }

    /// Remove a pin from the subgraph output node
    pub fn remove_output_pin(&mut self, pin_id: &str, cx: &mut Context<Self>) {
        if let Some(output_node) = self.graph.nodes.iter_mut().find(|n| n.definition_id == "subgraph_output") {
            output_node.inputs.retain(|p| p.id != pin_id);
            cx.notify();
        }
    }

    /// Load variables from vars_save.json
    fn load_variables_from_class(&mut self, class_path: &std::path::Path) -> Result<(), String> {
        let vars_file = class_path.join("vars_save.json");

        if !vars_file.exists() {
            // No vars file yet, that's ok - start with empty variables
            self.class_variables.clear();
            return Ok(());
        }

        let content = std::fs::read_to_string(&vars_file)
            .map_err(|e| format!("Failed to read vars_save.json: {}", e))?;

        let variables: Vec<super::variables::ClassVariable> = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse vars_save.json: {}", e))?;

        self.class_variables = variables;
        Ok(())
    }

    /// Save variables to vars_save.json
    pub fn save_variables_to_class(&self) -> Result<(), String> {
        let class_path = self
            .current_class_path
            .as_ref()
            .ok_or_else(|| "No class currently loaded".to_string())?;

        let vars_file = class_path.join("vars_save.json");

        let json = serde_json::to_string_pretty(&self.class_variables)
            .map_err(|e| format!("Failed to serialize variables: {}", e))?;

        std::fs::write(&vars_file, json)
            .map_err(|e| format!("Failed to write vars_save.json: {}", e))?;

        Ok(())
    }

    /// Generate vars/mod.rs from current variables
    pub fn generate_vars_module(&self) -> Result<(), String> {
        let class_path = self
            .current_class_path
            .as_ref()
            .ok_or_else(|| "No class currently loaded".to_string())?;

        let vars_dir = class_path.join("vars");
        std::fs::create_dir_all(&vars_dir)
            .map_err(|e| format!("Failed to create vars directory: {}", e))?;

        let mut code = String::new();
        code.push_str("//! Auto-generated variables module\n");
        code.push_str("//! DO NOT EDIT MANUALLY - YOUR CHANGES WILL BE OVERWRITTEN\n\n");

        // Check if we need RefCell
        let needs_refcell = self.class_variables.iter().any(|v| {
            !matches!(
                v.var_type.as_str(),
                "i32"
                    | "i64"
                    | "u32"
                    | "u64"
                    | "f32"
                    | "f64"
                    | "bool"
                    | "char"
                    | "usize"
                    | "isize"
                    | "i8"
                    | "i16"
                    | "u8"
                    | "u16"
            )
        });

        code.push_str("use std::cell::Cell;\n");
        if needs_refcell {
            code.push_str("use std::cell::RefCell;\n");
        }
        code.push_str("\n");

        // Generate variable declarations using thread_local for type safety
        for var in &self.class_variables {
            let default_value = if let Some(default) = &var.default_value {
                default.clone()
            } else {
                // Use type defaults
                match var.var_type.as_str() {
                    "i32" | "i64" | "u32" | "u64" | "f32" | "f64" => "0".to_string(),
                    "bool" => "false".to_string(),
                    "&str" => "\"\"".to_string(),
                    "String" => "String::new()".to_string(),
                    _ => "Default::default()".to_string(),
                }
            };

            // Determine if we should use Cell or RefCell based on type
            let use_cell = matches!(
                var.var_type.as_str(),
                "i32"
                    | "i64"
                    | "u32"
                    | "u64"
                    | "f32"
                    | "f64"
                    | "bool"
                    | "char"
                    | "usize"
                    | "isize"
                    | "i8"
                    | "i16"
                    | "u8"
                    | "u16"
            );

            if use_cell {
                code.push_str(&format!(
                    "thread_local! {{\n    pub static {}: Cell<{}> = Cell::new({});\n}}\n\n",
                    var.name.to_uppercase(),
                    var.var_type,
                    default_value
                ));
            } else {
                code.push_str(&format!(
                    "thread_local! {{\n    pub static {}: RefCell<{}> = RefCell::new({});\n}}\n\n",
                    var.name.to_uppercase(),
                    var.var_type,
                    default_value
                ));
            }
        }

        let mod_file = vars_dir.join("mod.rs");
        std::fs::write(&mod_file, code)
            .map_err(|e| format!("Failed to write vars/mod.rs: {}", e))?;

        Ok(())
    }

    /// Start dragging a variable from the variables panel
    pub fn start_dragging_variable(
        &mut self,
        var_name: String,
        var_type: String,
        cx: &mut Context<Self>,
    ) {
        self.dragging_variable = Some(super::variables::VariableDrag { var_name, var_type });
        cx.notify();
    }

    /// Finish dragging a variable and show context menu at drop position
    pub fn finish_dragging_variable(&mut self, drop_position: Point<f32>, cx: &mut Context<Self>) {
        if self.dragging_variable.is_some() {
            self.variable_drop_menu_position = Some(drop_position);
            cx.notify();
        }
    }

    /// Cancel variable drag
    pub fn cancel_dragging_variable(&mut self, cx: &mut Context<Self>) {
        self.dragging_variable = None;
        self.variable_drop_menu_position = None;
        cx.notify();
    }

    // Stub methods for node library (not currently used)
    pub fn get_search_input_state(&self) -> &Entity<gpui_component::input::InputState> {
        &self.variable_name_input // Reuse existing input state
    }

    pub fn get_search_query(&self) -> &str {
        "" // Return empty string for now
    }

    /// Start the compilation process with progress tracking
    pub fn start_compilation(&mut self, cx: &mut Context<Self>) {
        self.compilation_status = super::CompilationStatus {
            state: super::CompilationState::Compiling,
            message: "Starting compilation...".to_string(),
            progress: 0.0,
            is_compiling: true,
        };
        cx.notify();

        // Spawn async compilation task
        cx.spawn(async move |view, mut cx| {
            // Phase 1: Validate blueprint
            cx.update(|cx| {
                view.update(cx, |panel, cx| {
                    panel.compilation_status.message = "Validating blueprint graph...".to_string();
                    panel.compilation_status.progress = 0.2;
                    cx.notify();
                }).ok();
            }).ok();

            smol::Timer::after(std::time::Duration::from_millis(100)).await;

            // Phase 2: Generate code
            let compile_result = cx.update(|cx| {
                view.update(cx, |panel, cx| {
                    panel.compilation_status.message = "Generating Rust code...".to_string();
                    panel.compilation_status.progress = 0.5;
                    cx.notify();
                    panel.compile_to_class_directory()
                }).ok()
            }).ok().flatten();

            smol::Timer::after(std::time::Duration::from_millis(100)).await;

            // Phase 3: Write files
            cx.update(|cx| {
                view.update(cx, |panel, cx| {
                    panel.compilation_status.message = "Writing files...".to_string();
                    panel.compilation_status.progress = 0.8;
                    cx.notify();
                }).ok();
            }).ok();

            smol::Timer::after(std::time::Duration::from_millis(100)).await;

            // Phase 4: Complete
            cx.update(|cx| {
                view.update(cx, |panel, cx| {
                    match compile_result {
                        Some(Ok(())) => {
                            panel.compilation_status = super::CompilationStatus {
                                state: super::CompilationState::Success,
                                message: "Compilation successful!".to_string(),
                                progress: 1.0,
                                is_compiling: false,
                            };
                            println!("✅ Blueprint compiled successfully!");
                        }
                        Some(Err(e)) => {
                            panel.compilation_status = super::CompilationStatus {
                                state: super::CompilationState::Error,
                                message: format!("Compilation failed: {}", e),
                                progress: 0.0,
                                is_compiling: false,
                            };
                            eprintln!("❌ Compilation error: {}", e);
                        }
                        None => {
                            panel.compilation_status = super::CompilationStatus {
                                state: super::CompilationState::Error,
                                message: "Compilation cancelled".to_string(),
                                progress: 0.0,
                                is_compiling: false,
                            };
                        }
                    }
                    cx.notify();
                }).ok();
            }).ok();

            // Reset to idle after 3 seconds
            smol::Timer::after(std::time::Duration::from_secs(3)).await;
            cx.update(|cx| {
                view.update(cx, |panel, cx| {
                    if panel.compilation_status.state != super::CompilationState::Compiling {
                        panel.compilation_status = super::CompilationStatus::default();
                        cx.notify();
                    }
                }).ok();
            }).ok();
        }).detach();
    }

    /// Create a getter node for a variable at the specified position
    pub fn create_getter_node(
        &mut self,
        var_name: String,
        var_type: String,
        position: Point<f32>,
        cx: &mut Context<Self>,
    ) {
        let node_id = format!("get_{}_node_{}", var_name, uuid::Uuid::new_v4());

        let node = BlueprintNode {
            id: node_id,
            definition_id: format!("get_{}", var_name),
            title: format!("Get {}", var_name),
            icon: "📥".to_string(),
            node_type: NodeType::Logic,
            position,
            size: Size::new(180.0, 80.0),
            inputs: vec![],
            outputs: vec![Pin {
                id: "value".to_string(),
                name: var_name.clone(),
                pin_type: PinType::Output,
                data_type: GraphDataType::from_type_str(&var_type),
            }],
            properties: std::collections::HashMap::new(),
            is_selected: false,
            description: format!("Gets the value of {}", var_name),
            color: None,
        };

        self.add_node(node, cx);
        self.cancel_dragging_variable(cx);
    }

    /// Create a setter node for a variable at the specified position
    pub fn create_setter_node(
        &mut self,
        var_name: String,
        var_type: String,
        position: Point<f32>,
        cx: &mut Context<Self>,
    ) {
        let node_id = format!("set_{}_node_{}", var_name, uuid::Uuid::new_v4());

        let node = BlueprintNode {
            id: node_id,
            definition_id: format!("set_{}", var_name),
            title: format!("Set {}", var_name),
            icon: "📤".to_string(),
            node_type: NodeType::Logic,
            position,
            size: Size::new(180.0, 100.0),
            inputs: vec![
                Pin {
                    id: "exec".to_string(),
                    name: "".to_string(),
                    pin_type: PinType::Input,
                    data_type: GraphDataType::from_type_str("execution"),
                },
                Pin {
                    id: "value".to_string(),
                    name: var_name.clone(),
                    pin_type: PinType::Input,
                    data_type: GraphDataType::from_type_str(&var_type),
                },
            ],
            outputs: vec![Pin {
                id: "exec_out".to_string(),
                name: "".to_string(),
                pin_type: PinType::Output,
                data_type: GraphDataType::from_type_str("execution"),
            }],
            properties: std::collections::HashMap::new(),
            is_selected: false,
            description: format!("Sets the value of {}", var_name),
            color: None,
        };

        self.add_node(node, cx);
        self.cancel_dragging_variable(cx);
    }

    /// Render the variable drop context menu with Get/Set options
    fn render_variable_drop_menu(
        &self,
        var_drag: Option<super::variables::VariableDrag>,
        position: Point<f32>,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        use gpui_component::{
            button::{Button, ButtonVariants as _},
            v_flex,
        };

        let var_name = var_drag
            .as_ref()
            .map(|v| v.var_name.clone())
            .unwrap_or_default();
        let var_type = var_drag
            .as_ref()
            .map(|v| v.var_type.clone())
            .unwrap_or_default();

        let get_var_name = var_name.clone();
        let get_var_type = var_type.clone();
        let set_var_name = var_name.clone();
        let set_var_type = var_type.clone();

        v_flex()
            .w(px(180.))
            .gap_1()
            .p_2()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .shadow_lg()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .p_1()
                    .child(format!("Variable: {}", var_name)),
            )
            .child(
                Button::new("get-variable")
                    .ghost()
                    .label(format!("Get {}", get_var_name))
                    .on_click(cx.listener(move |panel, _, _, cx| {
                        panel.create_getter_node(
                            get_var_name.clone(),
                            get_var_type.clone(),
                            position,
                            cx,
                        );
                    })),
            )
            .child(
                Button::new("set-variable")
                    .ghost()
                    .label(format!("Set {}", set_var_name))
                    .on_click(cx.listener(move |panel, _, _, cx| {
                        panel.create_setter_node(
                            set_var_name.clone(),
                            set_var_type.clone(),
                            position,
                            cx,
                        );
                    })),
            )
    }

    // ===== Sub-graph Navigation Methods =====

    /// Convert a GraphDescription to BlueprintGraph (reverse of convert_to_graph_description)
    fn convert_graph_description_to_blueprint(&self, graph_desc: &GraphDescription) -> Result<BlueprintGraph, String> {
        let mut blueprint_nodes = Vec::new();
        let node_definitions = super::NodeDefinitions::load();

        // Convert nodes
        for node_instance in graph_desc.nodes.values() {
            // Look up node definition to get visual metadata
            let node_def = node_definitions.get_node_definition(&node_instance.node_type);

            // Determine node type
            let node_type = if let Some(def) = node_def {
                let category = node_definitions.get_category_for_node(&def.id);
                match category.map(|c| c.name.as_str()) {
                    Some("Events") => super::NodeType::Event,
                    Some("Logic") => super::NodeType::Logic,
                    Some("Math") => super::NodeType::Math,
                    Some("Object") => super::NodeType::Object,
                    _ => super::NodeType::Logic,
                }
            } else {
                super::NodeType::Logic
            };

            // Convert pins
            let inputs: Vec<super::Pin> = node_instance.inputs.iter().map(|pin_inst| {
                super::Pin {
                    id: pin_inst.id.clone(),
                    name: pin_inst.pin.name.clone(),
                    pin_type: super::PinType::Input,
                    data_type: pin_inst.pin.data_type.clone(),
                }
            }).collect();

            let outputs: Vec<super::Pin> = node_instance.outputs.iter().map(|pin_inst| {
                super::Pin {
                    id: pin_inst.id.clone(),
                    name: pin_inst.pin.name.clone(),
                    pin_type: super::PinType::Output,
                    data_type: pin_inst.pin.data_type.clone(),
                }
            }).collect();

            // Convert properties
            let properties: std::collections::HashMap<String, String> = node_instance.properties.iter()
                .map(|(k, v)| {
                    let value_str = match v {
                        crate::graph::PropertyValue::String(s) => s.clone(),
                        crate::graph::PropertyValue::Number(n) => n.to_string(),
                        crate::graph::PropertyValue::Boolean(b) => b.to_string(),
                        crate::graph::PropertyValue::Vector2(x, y) => format!("({}, {})", x, y),
                        crate::graph::PropertyValue::Vector3(x, y, z) => format!("({}, {}, {})", x, y, z),
                        crate::graph::PropertyValue::Color(r, g, b, a) => format!("({}, {}, {}, {})", r, g, b, a),
                    };
                    (k.clone(), value_str)
                })
                .collect();

            let blueprint_node = BlueprintNode {
                id: node_instance.id.clone(),
                definition_id: node_instance.node_type.clone(),
                title: node_def.map(|d| d.name.clone()).unwrap_or_else(|| node_instance.node_type.clone()),
                icon: node_def.map(|d| d.icon.clone()).unwrap_or_else(|| "⚙️".to_string()),
                node_type,
                position: Point::new(node_instance.position.x, node_instance.position.y),
                size: Size::new(150.0, 100.0), // Default size
                inputs,
                outputs,
                properties,
                is_selected: false,
                description: node_def.map(|d| d.description.clone()).unwrap_or_default(),
                color: node_def.and_then(|d| d.color.clone()),
            };

            blueprint_nodes.push(blueprint_node);
        }

        // Convert connections
        let blueprint_connections: Vec<super::Connection> = graph_desc.connections.iter().map(|conn| {
            super::Connection {
                id: conn.id.clone(),
                from_node_id: conn.source_node.clone(),
                from_pin_id: conn.source_pin.clone(),
                to_node_id: conn.target_node.clone(),
                to_pin_id: conn.target_pin.clone(),
            }
        }).collect();

        Ok(BlueprintGraph {
            nodes: blueprint_nodes,
            connections: blueprint_connections,
            comments: graph_desc.comments.clone(),
            selected_nodes: vec![],
            selected_comments: vec![],
            zoom_level: 1.0,
            pan_offset: Point::new(0.0, 0.0),
            virtualization_stats: VirtualizationStats::default(),
        })
    }

    // open_subgraph method removed - now using flat tab-based navigation with open_local_macro

    /// Create a new local macro (sub-graph) within the current blueprint class
    pub fn create_new_local_macro(&mut self, cx: &mut Context<Self>) {
        // Generate a unique ID for the new macro
        let macro_id = format!("local_macro_{}", std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis());

        let macro_name = "New Macro".to_string();

        // Create a new empty graph with macro_entry and macro_exit nodes
        let entry_node = BlueprintNode {
            id: "macro_entry".to_string(),
            definition_id: "macro_entry".to_string(),
            title: "Macro Entry".to_string(),
            icon: "→".to_string(),
            node_type: NodeType::MacroEntry,
            position: Point::new(100.0, 200.0),
            size: Size::new(180.0, 100.0),
            inputs: vec![],
            outputs: vec![
                // Start with a default execution output
                Pin {
                    id: "exec_in".to_string(),
                    name: "Body".to_string(),
                    pin_type: PinType::Output,
                    data_type: DataType::Execution,
                },
            ],
            properties: std::collections::HashMap::new(),
            is_selected: false,
            description: "Entry point for this macro. Outputs correspond to macro inputs.".to_string(),
            color: Some("#8B5CF6".to_string()), // Purple for macros
        };

        let exit_node = BlueprintNode {
            id: "macro_exit".to_string(),
            definition_id: "macro_exit".to_string(),
            title: "Macro Exit".to_string(),
            icon: "←".to_string(),
            node_type: NodeType::MacroExit,
            position: Point::new(600.0, 200.0),
            size: Size::new(180.0, 100.0),
            inputs: vec![
                // Start with a default execution input
                Pin {
                    id: "exec_out".to_string(),
                    name: "Then".to_string(),
                    pin_type: PinType::Input,
                    data_type: DataType::Execution,
                },
            ],
            outputs: vec![],
            properties: std::collections::HashMap::new(),
            is_selected: false,
            description: "Exit point for this macro. Inputs correspond to macro outputs.".to_string(),
            color: Some("#8B5CF6".to_string()), // Purple for macros
        };

        // Create the new graph
        let new_graph = BlueprintGraph {
            nodes: vec![entry_node, exit_node],
            connections: vec![],
            comments: vec![],
            selected_nodes: vec![],
            selected_comments: vec![],
            zoom_level: 1.0,
            pan_offset: Point::new(0.0, 0.0),
            virtualization_stats: VirtualizationStats::default(),
        };

        // Create the macro definition with proper structure
        let graph_desc = crate::graph::GraphDescription::new(&macro_name);

        let macro_def = crate::graph::SubGraphDefinition {
            id: macro_id.clone(),
            name: macro_name.clone(),
            description: "A custom macro for reusable graph logic".to_string(),
            graph: graph_desc,
            interface: crate::graph::SubGraphInterface {
                inputs: vec![crate::graph::SubGraphPin {
                    id: "exec_in".to_string(),
                    name: "Body".to_string(),
                    data_type: crate::graph::DataType::Execution,
                    description: Some("Execution input".to_string()),
                    default_value: None,
                    is_instance_editable: false,
                    category: None,
                }],
                outputs: vec![crate::graph::SubGraphPin {
                    id: "exec_out".to_string(),
                    name: "Then".to_string(),
                    data_type: crate::graph::DataType::Execution,
                    description: Some("Execution output".to_string()),
                    default_value: None,
                    is_instance_editable: false,
                    category: None,
                }],
            },
            metadata: crate::graph::SubGraphMetadata {
                created_at: chrono::Utc::now().to_rfc3339(),
                modified_at: chrono::Utc::now().to_rfc3339(),
                author: None,
                tags: vec![],
            },
            macro_config: crate::graph::MacroConfiguration {
                is_pure: false,
                compact_node_title: None,
                category: "Macros".to_string(),
                tooltip: None,
                keywords: vec!["macro".to_string(), "custom".to_string()],
                instance_editable_pins: vec![],
                color: Some((0.545, 0.361, 0.965)), // Purple RGB
                icon: Some("📦".to_string()),
                parent_class_filter: vec![],
                hide_in_palette: false,
            },
        };

        // Add to local macros list
        self.local_macros.push(macro_def);

        // Save current graph to current tab before switching
        self.sync_graph_to_active_tab();

        // Create and open a new tab for this macro
        let new_tab = GraphTab {
            id: macro_id.clone(),
            name: macro_name.clone(),
            graph: new_graph,
            is_main: false,
            is_dirty: true, // New macro is unsaved
            is_library_macro: false,
            library_id: None,
        };

        self.open_tabs.push(new_tab);
        self.active_tab_index = self.open_tabs.len() - 1;
        self.load_active_tab_graph();

        println!("✨ Created new local macro: {} (ID: {})", macro_name, macro_id);
        cx.notify();
    }

    /// Render the proper tab bar (Unreal-style) for graph navigation
    fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        TabBar::new("graph-tabs")
            .w_full()
            .bg(cx.theme().secondary)
            .border_b_1()
            .border_color(cx.theme().border)
            .selected_index(self.active_tab_index)
            .on_click(cx.listener(|this, index: &usize, _window, cx| {
                this.switch_to_tab(*index, cx);
            }))
            .children(
                self.open_tabs.iter().enumerate().map(|(index, tab)| {
                    Tab::new(tab.name.clone())
                        .when(!tab.is_main, |t| {
                            let tab_index = index;
                            t.child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        Button::new(("close-tab", index))
                                            .icon(IconName::Close)
                                            .ghost()
                                            .on_click(cx.listener(move |this, _, _window, cx| {
                                                this.close_tab(tab_index, cx);
                                            }))
                                    )
                            )
                        })
                        .when(tab.is_dirty, |t| {
                            t.child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().accent)
                                    .child("*")
                            )
                        })
                })
            )
    }
}

impl Panel for BlueprintEditorPanel {
    fn panel_name(&self) -> &'static str {
        "Blueprint Editor"
    }

    fn title(&self, _window: &Window, cx: &App) -> AnyElement {
        // STUDIO-QUALITY TAB TITLE with icon
        h_flex()
            .gap_2()
            .items_center()
            .child(
                // Blueprint icon
                div()
                    .text_sm()
                    .child("⚡")
            )
            .child(
                div()
                    .text_sm()
                    .child(if let Some(title) = &self.tab_title {
                        title.clone()
                    } else {
                        "Blueprint Editor".to_string()
                    })
            )
            .into_any_element()
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

impl EventEmitter<OpenEngineLibraryRequest> for BlueprintEditorPanel {}

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
            .child(self.render_tab_bar(cx))
            .child(
                div().flex_1().child(
                    h_resizable("blueprint-editor-panels", self.resizable_state.clone())
                        .child(
                            // Left sidebar with vertical split: macros (top) and variables (bottom)
                            resizable_panel()
                                .size(px(280.))
                                .size_range(px(200.)..px(400.))
                                .child(
                                    v_resizable("left-sidebar-split", self.left_sidebar_resizable_state.clone())
                                        .child(
                                            resizable_panel()
                                                .size(px(200.))
                                                .size_range(px(150.)..px(500.))
                                                .child(super::macros::MacrosRenderer::render(self, cx))
                                        )
                                        .child(
                                            resizable_panel()
                                                .child(super::variables::VariablesRenderer::render(self, cx))
                                        )
                                )
                        )
                        .child(
                            resizable_panel().child(NodeGraphRenderer::render(self, cx))
                        )
                        .child(
                            resizable_panel()
                                .size(px(320.))
                                .size_range(px(250.)..px(500.))
                                .child(super::properties::PropertiesRenderer::render(self, cx))
                        ),
                ),
            )
            .when_some(self.node_creation_menu.clone(), |this, menu| {
                // Position the menu at the cursor location
                let menu_entity = menu.clone();
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .child(div().absolute().child(menu_entity)),
                )
            })
            .when_some(self.hoverable_tooltip.clone(), |this, tooltip| {
                // Render hoverable tooltip
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .on_mouse_move(cx.listener(|panel, event: &MouseMoveEvent, _window, cx| {
                            // Check if mouse is outside tooltip and hide if so
                            let mouse_pos =
                                Point::new(event.position.x.as_f32(), event.position.y.as_f32());
                            panel.check_tooltip_hover(mouse_pos, cx);
                        }))
                        .child(tooltip),
                )
            })
            .when_some(self.variable_drop_menu_position, |this, position| {
                // Render variable drop context menu (Get/Set selection)
                let var_drag = self.dragging_variable.clone();
                this.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .on_mouse_down(
                            gpui::MouseButton::Left,
                            cx.listener(|panel, _event, _window, cx| {
                                // Click outside menu cancels it
                                panel.cancel_dragging_variable(cx);
                            }),
                        )
                        .child(
                            div()
                                .absolute()
                                .left(px(position.x))
                                .top(px(position.y))
                                .child(self.render_variable_drop_menu(var_drag, position, cx)),
                        ),
                )
            })
    }
}