pub mod toolbar;
pub mod node_graph;
pub mod properties;
pub mod panel;
pub mod node_creation_menu;
pub mod hoverable_tooltip;

// Re-export the main panel
pub use panel::BlueprintEditorPanel;

use gpui::*;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use crate::graph::{DataType, PinStyle};

// Context menu actions for blueprint editor
#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = blueprint_editor)]
pub struct DuplicateNode {
    pub node_id: String,
}

#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = blueprint_editor)]
pub struct DeleteNode {
    pub node_id: String,
}

#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = blueprint_editor)]
pub struct CopyNode {
    pub node_id: String,
}

#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = blueprint_editor)]
pub struct PasteNode;

#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = blueprint_editor)]
pub struct DisconnectPin {
    pub node_id: String,
    pub pin_id: String,
}

// Shared types and state
#[derive(Clone, Debug)]
pub struct BlueprintNode {
    pub id: String,
    pub title: String,
    pub icon: String,
    pub node_type: NodeType,
    pub position: Point<f32>,
    pub size: Size<f32>,
    pub inputs: Vec<Pin>,
    pub outputs: Vec<Pin>,
    pub properties: HashMap<String, String>,
    pub is_selected: bool,
    pub description: String, // Markdown documentation for the node
}

#[derive(Clone, Debug)]
pub struct Pin {
    pub id: String,
    pub name: String,
    pub pin_type: PinType,
    pub data_type: DataType,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum NodeType {
    Event,
    Logic,
    Math,
    Object,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PinType {
    Input,
    Output,
}

// DataType is now imported from crate::graph module
// This provides the new TypeInfo-based type system with deterministic colors

#[derive(Clone, Debug)]
pub struct Connection {
    pub id: String,
    pub from_node_id: String,
    pub from_pin_id: String,
    pub to_node_id: String,
    pub to_pin_id: String,
}

#[derive(Clone)]
pub struct BlueprintGraph {
    pub nodes: Vec<BlueprintNode>,
    pub connections: Vec<Connection>,
    pub selected_nodes: Vec<String>,
    pub zoom_level: f32,
    pub pan_offset: Point<f32>,
    pub virtualization_stats: VirtualizationStats,
}

#[derive(Clone, Debug, Default)]
pub struct VirtualizationStats {
    pub total_nodes: usize,
    pub rendered_nodes: usize,
    pub total_connections: usize,
    pub rendered_connections: usize,
    pub last_update_ms: f32,
}

// JSON schema structures for loading node definitions
#[derive(Debug, Deserialize)]
pub struct NodeDefinitions {
    pub categories: Vec<NodeCategory>,
}

#[derive(Debug, Deserialize)]
pub struct NodeCategory {
    pub name: String,
    pub color: String,
    pub nodes: Vec<NodeDefinition>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeDefinition {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub description: String,
    pub inputs: Vec<PinDefinition>,
    pub outputs: Vec<PinDefinition>,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PinDefinition {
    pub id: String,
    pub name: String,
    pub data_type: DataType,
    pub pin_type: PinType,
}

// Global node definitions (loaded once at startup)
use std::sync::OnceLock;
static NODE_DEFINITIONS: OnceLock<NodeDefinitions> = OnceLock::new();

impl NodeDefinitions {
    pub fn load() -> &'static NodeDefinitions {
        NODE_DEFINITIONS.get_or_init(|| {
            // Load dynamic node definitions from .tron templates
            let dynamic_nodes = crate::compiler::load_all_node_definitions()
                .unwrap_or_else(|e| {
                    eprintln!("Failed to load dynamic node definitions: {}", e);
                    std::collections::HashMap::new()
                });

            // Convert dynamic nodes to static format
            Self::from_dynamic_nodes(dynamic_nodes)
        })
    }

    fn from_dynamic_nodes(dynamic_nodes: std::collections::HashMap<String, crate::compiler::NodeDefinition>) -> NodeDefinitions {
        let mut categories_map: std::collections::HashMap<String, Vec<NodeDefinition>> = std::collections::HashMap::new();

        // Group nodes by category
        for (id, dynamic_def) in dynamic_nodes {
            let mut inputs = Vec::new();
            let mut outputs = Vec::new();

            // Add execution inputs
            for exec_pin in &dynamic_def.execution_inputs {
                inputs.push(PinDefinition {
                    id: exec_pin.name.clone(),
                    name: exec_pin.name.clone(),
                    data_type: DataType::from_type_str("execution"),
                    pin_type: PinType::Input,
                });
            }

            // Add regular inputs
            for pin in &dynamic_def.inputs {
                inputs.push(PinDefinition {
                    id: pin.name.clone(),
                    name: pin.name.clone(),
                    data_type: Self::convert_data_type(&pin.data_type),
                    pin_type: PinType::Input,
                });
            }

            // Add execution outputs
            for exec_pin in &dynamic_def.execution_outputs {
                outputs.push(PinDefinition {
                    id: exec_pin.name.clone(),
                    name: exec_pin.name.clone(),
                    data_type: DataType::from_type_str("execution"),
                    pin_type: PinType::Output,
                });
            }

            // Add regular outputs
            for pin in &dynamic_def.outputs {
                outputs.push(PinDefinition {
                    id: pin.name.clone(),
                    name: pin.name.clone(),
                    data_type: Self::convert_data_type(&pin.data_type),
                    pin_type: PinType::Output,
                });
            }

            let static_def = NodeDefinition {
                id: id.clone(),
                name: dynamic_def.name,
                icon: dynamic_def.icon,
                description: dynamic_def.description,
                inputs,
                outputs,
                properties: std::collections::HashMap::new(),
            };

            categories_map
                .entry(dynamic_def.category)
                .or_insert_with(Vec::new)
                .push(static_def);
        }

        // Convert to categories
        let categories = categories_map
            .into_iter()
            .map(|(name, nodes)| NodeCategory {
                name: name.clone(),
                color: Self::get_category_color(&name),
                nodes,
            })
            .collect();

        NodeDefinitions { categories }
    }

    fn convert_data_type(data_type: &str) -> DataType {
        // Use the new DataType system that supports TypeInfo
        DataType::from_type_str(data_type)
    }

    fn get_category_color(category: &str) -> String {
        match category {
            "Math" | "Math/Vector" => "#4A90E2".to_string(),
            "Logic" => "#E2A04A".to_string(),
            "String" => "#7ED321".to_string(),
            "Array" => "#BD10E0".to_string(),
            "File I/O" => "#50E3C2".to_string(),
            "Graphics" => "#F5A623".to_string(),
            "Time" => "#9013FE".to_string(),
            "Utility" => "#B8E986".to_string(),
            _ => "#9B9B9B".to_string(),
        }
    }

    pub fn get_node_definition(&self, node_id: &str) -> Option<&NodeDefinition> {
        self.categories
            .iter()
            .flat_map(|category| &category.nodes)
            .find(|node| node.id == node_id)
    }

    pub fn get_node_definition_by_name(&self, node_name: &str) -> Option<&NodeDefinition> {
        self.categories
            .iter()
            .flat_map(|category| &category.nodes)
            .find(|node| node.name == node_name)
    }

    pub fn get_category_for_node(&self, node_id: &str) -> Option<&NodeCategory> {
        self.categories
            .iter()
            .find(|category| category.nodes.iter().any(|node| node.id == node_id))
    }
}

impl BlueprintNode {
    pub fn from_definition(definition: &NodeDefinition, position: Point<f32>) -> Self {
        let inputs: Vec<Pin> = definition.inputs.iter().map(|pin_def| Pin {
            id: pin_def.id.clone(),
            name: pin_def.name.clone(),
            pin_type: pin_def.pin_type.clone(),
            data_type: pin_def.data_type.clone(),
        }).collect();

        let outputs: Vec<Pin> = definition.outputs.iter().map(|pin_def| Pin {
            id: pin_def.id.clone(),
            name: pin_def.name.clone(),
            pin_type: pin_def.pin_type.clone(),
            data_type: pin_def.data_type.clone(),
        }).collect();

        // Determine node type based on category
        let node_definitions = NodeDefinitions::load();
        let category = node_definitions.get_category_for_node(&definition.id);
        let node_type = match category.map(|c| c.name.as_str()) {
            Some("Events") => NodeType::Event,
            Some("Logic") => NodeType::Logic,
            Some("Math") => NodeType::Math,
            Some("Object") => NodeType::Object,
            _ => NodeType::Logic,
        };

        Self {
            id: uuid::Uuid::new_v4().to_string(),
            title: definition.name.clone(),
            icon: definition.icon.clone(),
            node_type,
            position,
            size: Size::new(150.0, 100.0), // Default size
            inputs,
            outputs,
            properties: definition.properties.clone(),
            is_selected: false,
            description: definition.description.clone(),
        }
    }
}
