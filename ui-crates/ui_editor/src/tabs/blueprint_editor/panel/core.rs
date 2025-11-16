//! Core panel struct and initialization
//!
//! This module contains the main `BlueprintEditorPanel` struct definition,
//! constructors, and basic accessors.

use gpui::*;
use ui::{
    input::InputState,
    resizable::ResizableState,
};
use std::collections::HashMap;

use super::super::{BlueprintGraph, BlueprintNode, Connection, NodeType, Pin, PinType, DataType};
use super::super::hoverable_tooltip::HoverableTooltip;
use super::super::node_creation_menu::NodeCreationMenu;
use super::super::variables::ClassVariable;
use super::tabs::GraphTab;
use ui::graph::{DataType as GraphDataType, LibraryManager, SubGraphDefinition};

/// Main Blueprint Editor Panel struct
pub struct BlueprintEditorPanel {
    pub(super) focus_handle: FocusHandle,
    pub graph: BlueprintGraph,
    pub(super) resizable_state: Entity<ResizableState>,
    pub(super) left_sidebar_resizable_state: Entity<ResizableState>,
    
    // File I/O
    pub current_class_path: Option<std::path::PathBuf>,
    pub tab_title: Option<String>,
    
    // Node drag state
    pub dragging_node: Option<String>,
    pub drag_offset: Point<f32>,
    pub initial_drag_positions: HashMap<String, Point<f32>>,
    
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
    
    // Right-click gesture detection
    pub right_click_start: Option<Point<f32>>,
    pub right_click_threshold: f32,
    
    // Tooltip system
    pub hoverable_tooltip: Option<Entity<HoverableTooltip>>,
    pub pending_tooltip: Option<(String, Point<f32>)>,
    
    // Double-click for reroute nodes
    pub last_click_time: Option<std::time::Instant>,
    pub last_click_pos: Option<Point<f32>>,
    
    // Coordinate conversion
    pub graph_element_bounds: Option<Bounds<Pixels>>,
    
    // Variables system
    pub class_variables: Vec<ClassVariable>,
    pub is_creating_variable: bool,
    pub variable_name_input: Entity<InputState>,
    pub variable_type_dropdown: Entity<ui::dropdown::DropdownState<Vec<super::super::variables::TypeItem>>>,
    pub dragging_variable: Option<super::super::variables::VariableDrag>,
    pub variable_drop_menu_position: Option<Point<f32>>,
    
    // Comment system
    pub dragging_comment: Option<String>,
    pub resizing_comment: Option<(String, ResizeHandle)>,
    pub editing_comment: Option<String>,
    pub comment_text_input: Entity<InputState>,
    
    // Subscriptions
    pub subscriptions: Vec<Subscription>,
    
    // Compilation
    pub compilation_status: super::super::CompilationStatus,
    
    // Library/macro system
    pub library_manager: LibraryManager,
    pub local_macros: Vec<SubGraphDefinition>,
    
    // Tab system
    pub open_tabs: Vec<GraphTab>,
    pub active_tab_index: usize,
    
    // Overlay toggles
    pub show_debug_overlay: bool,
    pub show_minimap: bool,
    pub show_graph_controls: bool,
}

/// Resize handle for comment boxes
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

/// Connection drag state
#[derive(Clone, Debug)]
pub struct ConnectionDrag {
    pub from_node_id: String,
    pub from_pin_id: String,
    pub from_pin_type: GraphDataType,
    pub current_mouse_pos: Point<f32>,
    pub target_pin: Option<(String, String)>,
}

impl BlueprintEditorPanel {
    /// Create a new blueprint editor panel
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_internal(None, window, cx)
    }

    /// Create a new blueprint editor panel with a file to load
    pub fn new_with_file(file_path: std::path::PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut panel = Self::new_internal(Some(file_path.clone()), window, cx);
        
        // Try to load the blueprint file
        if let Err(e) = panel.load_blueprint(file_path.to_str().unwrap(), window, cx) {
            eprintln!("❌ Failed to load blueprint: {}", e);
        } else {
            println!("✅ Loaded blueprint from {:?}", file_path);
        }
        
        panel
    }

    /// Create a new blueprint editor for an engine library (virtual blueprint)
    pub fn new_for_library(
        library_id: String, 
        library_name: String, 
        window: &mut Window, 
        cx: &mut Context<Self>
    ) -> Self {
        let mut panel = Self::new_internal(None, window, cx);
        panel.tab_title = Some(format!("📚 {} Library", library_name));
        
        if let Some(main_tab) = panel.open_tabs.get_mut(0) {
            main_tab.name = format!("{} Overview", library_name);
        }
        
        println!("📚 Created blueprint editor for library: {}", library_name);
        panel
    }

    /// Internal constructor with sample graph
    fn new_internal(
        project_path: Option<std::path::PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>
    ) -> Self {
        let resizable_state = ResizableState::new(cx);
        let left_sidebar_resizable_state = ResizableState::new(cx);

        // Create demo graph with sample nodes (only if no file is being loaded)
        let main_graph = if project_path.is_some() {
            // Empty graph - will be loaded from file
            BlueprintGraph {
                nodes: Vec::new(),
                connections: Vec::new(),
                comments: Vec::new(),
                selected_nodes: Vec::new(),
                selected_comments: Vec::new(),
                zoom_level: 1.0,
                pan_offset: Point::new(0.0, 0.0),
                virtualization_stats: VirtualizationStats::default(),
            }
        } else {
            // No file to load - create sample graph
            Self::create_sample_graph()
        };

        Self {
            focus_handle: cx.focus_handle(),
            graph: main_graph.clone(),
            resizable_state,
            left_sidebar_resizable_state,
            current_class_path: None,
            tab_title: None,
            dragging_node: None,
            drag_offset: Point::new(0.0, 0.0),
            initial_drag_positions: HashMap::new(),
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
            right_click_threshold: 5.0,
            hoverable_tooltip: None,
            pending_tooltip: None,
            last_click_time: None,
            last_click_pos: None,
            graph_element_bounds: None,
            class_variables: Vec::new(),
            is_creating_variable: false,
            variable_name_input: cx.new(|cx| {
                InputState::new(window, cx).placeholder("Variable name...")
            }),
            variable_type_dropdown: cx.new(|cx| {
                ui::dropdown::DropdownState::new(Vec::new(), None, window, cx)
            }),
            dragging_variable: None,
            variable_drop_menu_position: None,
            dragging_comment: None,
            resizing_comment: None,
            editing_comment: None,
            comment_text_input: cx.new(|cx| {
                InputState::new(window, cx).placeholder("Comment text...")
            }),
            subscriptions: Vec::new(),
            compilation_status: super::super::CompilationStatus::default(),
            library_manager: {
                let mut lib_manager = LibraryManager::default();
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
            show_debug_overlay: true,
            show_minimap: true,
            show_graph_controls: true,
        }
    }

    /// Create a sample graph for demonstration - demonstrates all compiler features
    fn create_sample_graph() -> BlueprintGraph {
        use super::super::{BlueprintGraph, BlueprintNode, Connection, NodeType, Pin, PinType, Size, VirtualizationStats};
        use ui::graph::DataType as GraphDataType;
        
        let mut nodes = Vec::new();

        // Main event node (defines pub fn main())
        nodes.push(BlueprintNode {
            id: "main_event".to_string(),
            definition_id: "main".to_string(),
            title: "Main".to_string(),
            icon: "⚡".to_string(),
            node_type: NodeType::Event,
            position: Point::new(100.0, 200.0),
            size: Size::new(180.0, 80.0),
            inputs: vec![],
            outputs: vec![Pin {
                id: "Body".to_string(),
                name: "Body".to_string(),
                pin_type: PinType::Output,
                data_type: GraphDataType::from_type_str("execution"),
            }],
            properties: HashMap::new(),
            is_selected: false,
            description: "Entry point for the main function".to_string(),
            color: None,
        });

        // Pure node: add(2, 3)
        let mut add_props = HashMap::new();
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
            properties: HashMap::new(),
            is_selected: false,
            description: "Branches execution based on a condition.".to_string(),
            color: None,
        });

        // Function node: print (true path)
        let mut print_true_props = HashMap::new();
        print_true_props.insert(
            "message".to_string(),
            "Result is greater than 3! ✅".to_string(),
        );

        nodes.push(BlueprintNode {
            id: "print_true".to_string(),
            definition_id: "print_string".to_string(),
            title: "Print String".to_string(),
            icon: "📢".to_string(),
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
        let mut print_false_props = HashMap::new();
        print_false_props.insert("message".to_string(), "Result is 3 or less. ❌".to_string());

        nodes.push(BlueprintNode {
            id: "print_false".to_string(),
            definition_id: "print_string".to_string(),
            title: "Print String".to_string(),
            icon: "📢".to_string(),
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
        let mut gt_props = HashMap::new();
        gt_props.insert("b".to_string(), "3".to_string());

        nodes.push(BlueprintNode {
            id: "greater_node".to_string(),
            definition_id: "greater_than".to_string(),
            title: "Greater Than".to_string(),
            icon: "⚖".to_string(),
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

        BlueprintGraph {
            nodes,
            connections,
            comments: vec![],
            selected_nodes: vec![],
            selected_comments: vec![],
            zoom_level: 1.0,
            pan_offset: Point::new(0.0, 0.0),
            virtualization_stats: VirtualizationStats::default(),
        }
    }

    /// Get immutable reference to graph
    pub fn get_graph(&self) -> &BlueprintGraph {
        &self.graph
    }

    /// Get mutable reference to graph
    pub fn get_graph_mut(&mut self) -> &mut BlueprintGraph {
        &mut self.graph
    }

    /// Get focus handle
    pub fn focus_handle(&self) -> &FocusHandle {
        &self.focus_handle
    }
}
