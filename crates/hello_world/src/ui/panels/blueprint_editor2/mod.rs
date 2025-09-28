pub mod toolbar;
pub mod node_library;
pub mod node_graph;
pub mod properties;
pub mod panel;

// Re-export the main panel
pub use panel::BlueprintEditorPanel;

use gpui::*;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

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

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum DataType {
    Execution,
    Boolean,
    Integer,
    Float,
    String,
    Vector,
    Object,
}

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
            let json_data = include_str!("../../../../assets/node_definitions.json");
            serde_json::from_str(json_data).expect("Failed to parse node definitions")
        })
    }

    pub fn get_node_definition(&self, node_id: &str) -> Option<&NodeDefinition> {
        self.categories
            .iter()
            .flat_map(|category| &category.nodes)
            .find(|node| node.id == node_id)
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
        }
    }
}
