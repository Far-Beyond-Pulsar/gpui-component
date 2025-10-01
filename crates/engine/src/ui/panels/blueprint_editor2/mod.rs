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
    pub definition_id: String, // ID from NodeDefinition to restore metadata
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
    pub color: Option<String>, // Custom color from blueprint attribute
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
    Reroute, // Visual pass-through node for organizing connections
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
    pub color: Option<String>,
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
            // Load dynamic node definitions from pulsar_std
            let metadata = crate::compiler::node_metadata::extract_node_metadata()
                .unwrap_or_else(|e| {
                    eprintln!("Failed to load node metadata: {}", e);
                    std::collections::HashMap::new()
                });

            // Convert metadata to UI format
            Self::from_node_metadata(metadata)
        })
    }

    fn from_node_metadata(metadata: std::collections::HashMap<String, crate::compiler::NodeMetadata>) -> NodeDefinitions {
        let mut categories_map: std::collections::HashMap<String, Vec<NodeDefinition>> = std::collections::HashMap::new();

        // Add special reroute node to Utility category
        categories_map
            .entry("Utility".to_string())
            .or_insert_with(Vec::new)
            .push(NodeDefinition {
                id: "reroute".to_string(),
                name: "Reroute".to_string(),
                icon: "•".to_string(),
                description: "Organize connections with a pass-through node (typeless until connected)".to_string(),
                inputs: vec![],
                outputs: vec![],
                properties: std::collections::HashMap::new(),
                color: None,
            });

        // Group nodes by category
        for (id, node_meta) in metadata {
            let mut inputs = Vec::new();
            let mut outputs = Vec::new();

            // Add execution inputs
            for exec_pin in &node_meta.exec_inputs {
                inputs.push(PinDefinition {
                    id: exec_pin.clone(),
                    name: exec_pin.clone(),
                    data_type: DataType::from_type_str("execution"),
                    pin_type: PinType::Input,
                });
            }

            // Add regular inputs
            for param in &node_meta.params {
                inputs.push(PinDefinition {
                    id: param.name.clone(),
                    name: param.name.clone(),
                    data_type: DataType::from_type_str(&param.ty),
                    pin_type: PinType::Input,
                });
            }

            // Add execution outputs
            for exec_pin in &node_meta.exec_outputs {
                outputs.push(PinDefinition {
                    id: exec_pin.clone(),
                    name: exec_pin.clone(),
                    data_type: DataType::from_type_str("execution"),
                    pin_type: PinType::Output,
                });
            }

            // Add regular outputs (return type)
            if let Some(ref return_type) = node_meta.return_type {
                outputs.push(PinDefinition {
                    id: "result".to_string(),
                    name: "result".to_string(),
                    data_type: DataType::from_type_str(return_type),
                    pin_type: PinType::Output,
                });
            }

            let category = node_meta.category.clone().unwrap_or_else(|| "General".to_string());
            let description = node_meta.documentation.join("\n");

            let static_def = NodeDefinition {
                id: id.clone(),
                name: node_meta.name.clone(),
                icon: "⚙️".to_string(), // Default icon
                description,
                inputs,
                outputs,
                properties: std::collections::HashMap::new(),
                color: node_meta.color.clone(),
            };

            categories_map
                .entry(category)
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
            definition_id: definition.id.clone(),
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
            color: definition.color.clone(),
        }
    }

    /// Create a typeless reroute node at the given position
    /// The type will be inferred from the first connection made to it
    pub fn create_reroute(position: Point<f32>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            definition_id: "reroute".to_string(),
            title: "Reroute".to_string(),
            icon: "•".to_string(),
            node_type: NodeType::Reroute,
            position,
            size: Size::new(16.0, 16.0), // Small size for reroute nodes
            inputs: vec![Pin {
                id: "input".to_string(),
                name: "".to_string(),
                pin_type: PinType::Input,
                data_type: DataType::Any, // Start as typeless
            }],
            outputs: vec![Pin {
                id: "output".to_string(),
                name: "".to_string(),
                pin_type: PinType::Output,
                data_type: DataType::Any, // Start as typeless
            }],
            properties: HashMap::new(),
            is_selected: false,
            description: "Reroute node for organizing connections".to_string(),
            color: None,
        }
    }
}
