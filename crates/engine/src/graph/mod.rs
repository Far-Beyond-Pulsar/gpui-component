use std::collections::HashMap;
use serde::{Deserialize, Serialize};

pub mod type_system;
pub use type_system::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphDescription {
    pub nodes: HashMap<String, NodeInstance>,
    pub connections: Vec<Connection>,
    pub metadata: GraphMetadata,
    #[serde(default)]
    pub comments: Vec<crate::ui::panels::blueprint_editor2::BlueprintComment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInstance {
    pub id: String,
    pub node_type: String,
    pub position: Position,
    pub properties: HashMap<String, PropertyValue>,
    pub inputs: HashMap<String, Pin>,
    pub outputs: HashMap<String, Pin>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub id: String,
    pub source_node: String,
    pub source_pin: String,
    pub target_node: String,
    pub target_pin: String,
    pub connection_type: ConnectionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pin {
    pub name: String,
    pub pin_type: PinType,
    pub data_type: DataType,
    pub connected_to: Vec<String>, // Connection IDs
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphMetadata {
    pub name: String,
    pub description: String,
    pub version: String,
    pub created_at: String,
    pub modified_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PropertyValue {
    String(String),
    Number(f64),
    Boolean(bool),
    Vector2(f32, f32),
    Vector3(f32, f32, f32),
    Color(f32, f32, f32, f32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PinType {
    Input,
    Output,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DataType {
    Execution,
    Typed(TypeInfo),
    Any, // Typeless - matches with any type (used for reroute nodes)
    // Legacy types for backward compatibility
    String,
    Number,
    Boolean,
    Vector2,
    Vector3,
    Color,
    Object,
    Array(Box<DataType>),
}

impl DataType {
    /// Create a DataType from a type string
    pub fn from_type_str(type_str: &str) -> Self {
        match type_str {
            "execution" => DataType::Execution,
            // Parse as TypeInfo for proper type system support
            _ => DataType::Typed(TypeInfo::parse(type_str)),
        }
    }

    /// Get the TypeInfo for this DataType (if applicable)
    pub fn type_info(&self) -> Option<&TypeInfo> {
        match self {
            DataType::Typed(type_info) => Some(type_info),
            _ => None,
        }
    }

    /// Get the Rust type string representation for tooltip display
    pub fn rust_type_string(&self) -> String {
        match self {
            DataType::Execution => "()".to_string(), // Execution flow has no data type
            DataType::Typed(type_info) => type_info.to_string(),
            DataType::Any => "Any".to_string(), // Typeless reroute node
            DataType::String => "String".to_string(),
            DataType::Number => "f64".to_string(),
            DataType::Boolean => "bool".to_string(),
            DataType::Vector2 => "(f32, f32)".to_string(),
            DataType::Vector3 => "(f32, f32, f32)".to_string(),
            DataType::Color => "(f32, f32, f32, f32)".to_string(),
            DataType::Object => "dyn Any".to_string(),
            DataType::Array(inner) => format!("Vec<{}>", inner.rust_type_string()),
        }
    }

    /// Check if this DataType is compatible with another
    pub fn is_compatible_with(&self, other: &DataType) -> bool {
        match (self, other) {
            // Any type is compatible with everything (for typeless reroute nodes)
            (DataType::Any, _) | (_, DataType::Any) => true,
            (DataType::Execution, DataType::Execution) => true,
            (DataType::Typed(a), DataType::Typed(b)) => a.is_compatible_with(b),
            // Legacy compatibility
            (DataType::String, DataType::String) => true,
            (DataType::Number, DataType::Number) => true,
            (DataType::Boolean, DataType::Boolean) => true,
            (DataType::Vector2, DataType::Vector2) => true,
            (DataType::Vector3, DataType::Vector3) => true,
            (DataType::Color, DataType::Color) => true,
            (DataType::Object, DataType::Object) => true,
            // Array compatibility
            (DataType::Array(a), DataType::Array(b)) => a.is_compatible_with(b),
            _ => false,
        }
    }

    /// Generate a pin style for this data type
    pub fn generate_pin_style(&self) -> PinStyle {
        match self {
            DataType::Execution => PinStyle {
                color: PinColor { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }, // Red for execution
                icon: PinIcon::Triangle,
                is_rainbow: false,
            },
            DataType::Typed(type_info) => type_info.generate_pin_style(),
            DataType::Any => PinStyle {
                color: PinColor { r: 0.5, g: 0.5, b: 0.5, a: 1.0 }, // Gray for typeless
                icon: PinIcon::Circle,
                is_rainbow: false,
            },
            // Legacy types
            DataType::String => PinStyle {
                color: PinColor { r: 0.0, g: 1.0, b: 0.0, a: 1.0 }, // Green
                icon: PinIcon::Circle,
                is_rainbow: false,
            },
            DataType::Number => PinStyle {
                color: PinColor { r: 0.0, g: 0.0, b: 1.0, a: 1.0 }, // Blue
                icon: PinIcon::Circle,
                is_rainbow: false,
            },
            DataType::Boolean => PinStyle {
                color: PinColor { r: 1.0, g: 1.0, b: 0.0, a: 1.0 }, // Yellow
                icon: PinIcon::Circle,
                is_rainbow: false,
            },
            DataType::Vector2 | DataType::Vector3 => PinStyle {
                color: PinColor { r: 1.0, g: 0.0, b: 1.0, a: 1.0 }, // Magenta
                icon: PinIcon::Circle,
                is_rainbow: false,
            },
            DataType::Color => PinStyle {
                color: PinColor { r: 0.5, g: 0.5, b: 0.5, a: 1.0 }, // Gray
                icon: PinIcon::Circle,
                is_rainbow: false,
            },
            DataType::Object => PinStyle {
                color: PinColor { r: 0.8, g: 0.4, b: 0.2, a: 1.0 }, // Brown
                icon: PinIcon::Hexagon,
                is_rainbow: false,
            },
            DataType::Array(_) => PinStyle {
                color: PinColor { r: 0.0, g: 0.8, b: 0.8, a: 1.0 }, // Cyan
                icon: PinIcon::Square,
                is_rainbow: false,
            },
        }
    }
}

impl PartialEq<&str> for DataType {
    fn eq(&self, other: &&str) -> bool {
        match (self, *other) {
            (DataType::Execution, "execution") => true,
            (DataType::String, "string") => true,
            (DataType::Number, "number") => true,
            (DataType::Boolean, "boolean") => true,
            (DataType::Vector2, "vector2") => true,
            (DataType::Vector3, "vector3") => true,
            (DataType::Color, "color") => true,
            (DataType::Object, "object") => true,
            _ => false,
        }
    }
}

impl std::fmt::Display for DataType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataType::Execution => write!(f, "execution"),
            DataType::Typed(type_info) => write!(f, "{}", type_info),
            DataType::Any => write!(f, "any"),
            // Legacy types
            DataType::String => write!(f, "string"),
            DataType::Number => write!(f, "number"),
            DataType::Boolean => write!(f, "boolean"),
            DataType::Vector2 => write!(f, "vector2"),
            DataType::Vector3 => write!(f, "vector3"),
            DataType::Color => write!(f, "color"),
            DataType::Object => write!(f, "object"),
            DataType::Array(inner) => write!(f, "array<{}>", inner),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConnectionType {
    Execution,
    Data,
}

impl GraphDescription {
    pub fn new(name: &str) -> Self {
        Self {
            nodes: HashMap::new(),
            connections: Vec::new(),
            metadata: GraphMetadata {
                name: name.to_string(),
                description: String::new(),
                version: "1.0.0".to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
                modified_at: chrono::Utc::now().to_rfc3339(),
            },
            comments: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: NodeInstance) {
        self.nodes.insert(node.id.clone(), node);
        self.metadata.modified_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn add_connection(&mut self, connection: Connection) {
        // Update pin connections
        if let Some(source_node) = self.nodes.get_mut(&connection.source_node) {
            if let Some(output_pin) = source_node.outputs.get_mut(&connection.source_pin) {
                output_pin.connected_to.push(connection.id.clone());
            }
        }

        if let Some(target_node) = self.nodes.get_mut(&connection.target_node) {
            if let Some(input_pin) = target_node.inputs.get_mut(&connection.target_pin) {
                input_pin.connected_to.push(connection.id.clone());
            }
        }

        self.connections.push(connection);
        self.metadata.modified_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn remove_node(&mut self, node_id: &str) {
        // Remove all connections involving this node
        self.connections.retain(|conn| {
            conn.source_node != node_id && conn.target_node != node_id
        });

        // Remove the node
        self.nodes.remove(node_id);
        self.metadata.modified_at = chrono::Utc::now().to_rfc3339();
    }

    pub fn remove_connection(&mut self, connection_id: &str) {
        if let Some(index) = self.connections.iter().position(|conn| conn.id == connection_id) {
            let connection = &self.connections[index];

            // Update pin connections
            if let Some(source_node) = self.nodes.get_mut(&connection.source_node) {
                if let Some(output_pin) = source_node.outputs.get_mut(&connection.source_pin) {
                    output_pin.connected_to.retain(|id| id != connection_id);
                }
            }

            if let Some(target_node) = self.nodes.get_mut(&connection.target_node) {
                if let Some(input_pin) = target_node.inputs.get_mut(&connection.target_pin) {
                    input_pin.connected_to.retain(|id| id != connection_id);
                }
            }

            self.connections.remove(index);
            self.metadata.modified_at = chrono::Utc::now().to_rfc3339();
        }
    }

    pub fn get_execution_order(&self) -> Result<Vec<String>, String> {
        // Topological sort for execution order
        let mut visited = HashMap::new();
        let mut temp_visited = HashMap::new();
        let mut result = Vec::new();

        // Find entry points (nodes with no incoming execution connections)
        for node_id in self.nodes.keys() {
            if !visited.contains_key(node_id) {
                self.visit_node(node_id, &mut visited, &mut temp_visited, &mut result)?;
            }
        }

        Ok(result)
    }

    fn visit_node(
        &self,
        node_id: &str,
        visited: &mut HashMap<String, bool>,
        temp_visited: &mut HashMap<String, bool>,
        result: &mut Vec<String>,
    ) -> Result<(), String> {
        if temp_visited.contains_key(node_id) {
            return Err(format!("Circular dependency detected involving node {}", node_id));
        }

        if visited.contains_key(node_id) {
            return Ok(());
        }

        temp_visited.insert(node_id.to_string(), true);

        // Visit all dependent nodes (those connected via execution pins)
        for connection in &self.connections {
            if connection.source_node == node_id && matches!(connection.connection_type, ConnectionType::Execution) {
                self.visit_node(&connection.target_node, visited, temp_visited, result)?;
            }
        }

        temp_visited.remove(node_id);
        visited.insert(node_id.to_string(), true);
        result.push(node_id.to_string());

        Ok(())
    }
}

impl NodeInstance {
    pub fn new(id: &str, node_type: &str, position: Position) -> Self {
        Self {
            id: id.to_string(),
            node_type: node_type.to_string(),
            position,
            properties: HashMap::new(),
            inputs: HashMap::new(),
            outputs: HashMap::new(),
        }
    }

    pub fn add_input_pin(&mut self, name: &str, data_type: DataType) {
        let pin = Pin {
            name: name.to_string(),
            pin_type: PinType::Input,
            data_type,
            connected_to: Vec::new(),
        };
        self.inputs.insert(name.to_string(), pin);
    }

    pub fn add_output_pin(&mut self, name: &str, data_type: DataType) {
        let pin = Pin {
            name: name.to_string(),
            pin_type: PinType::Output,
            data_type,
            connected_to: Vec::new(),
        };
        self.outputs.insert(name.to_string(), pin);
    }

    pub fn set_property(&mut self, name: &str, value: PropertyValue) {
        self.properties.insert(name.to_string(), value);
    }
}

impl Connection {
    pub fn new(
        id: &str,
        source_node: &str,
        source_pin: &str,
        target_node: &str,
        target_pin: &str,
        connection_type: ConnectionType,
    ) -> Self {
        Self {
            id: id.to_string(),
            source_node: source_node.to_string(),
            source_pin: source_pin.to_string(),
            target_node: target_node.to_string(),
            target_pin: target_pin.to_string(),
            connection_type,
        }
    }
}