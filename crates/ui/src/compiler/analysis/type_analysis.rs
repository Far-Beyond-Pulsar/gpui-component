//! # Type Analysis and Validation
//!
//! Validates type compatibility in Blueprint graphs.
//!
//! This module ensures that all data connections are type-safe:
//! - Output types must match or be compatible with input types
//! - Type conversions must be explicit
//! - Generic types are properly instantiated
//!
//! ## Type Compatibility Rules
//!
//! ### Exact Match
//! Types must match exactly (e.g., `i32` to `i32`, `String` to `String`).
//!
//! ### Numeric Widening
//! Smaller numeric types can be automatically widened:
//! - `i32` → `i64`
//! - `f32` → `f64`
//! - Integer → Float (with potential precision loss warning)
//!
//! ### Reference/Ownership
//! - `&str` can connect to `String` (will clone)
//! - `&T` can connect to `T` (will clone if T: Clone)
//!
//! ### Tuples
//! Tuple types must match element-wise.

use crate::graph::{Connection, GraphDescription, DataType, TypeInfo};
use crate::compiler::core::NodeMetadata;
use std::collections::HashMap;

/// Type checker for validating graph connections
pub struct TypeChecker<'a> {
    graph: &'a GraphDescription,
    metadata: &'a HashMap<String, NodeMetadata>,
}

/// Type checking error
#[derive(Debug, Clone)]
pub struct TypeError {
    pub connection_id: String,
    pub source_node: String,
    pub source_pin: String,
    pub target_node: String,
    pub target_pin: String,
    pub expected_type: String,
    pub actual_type: String,
    pub message: String,
}

impl TypeError {
    /// Format error message for display
    pub fn format(&self) -> String {
        format!(
            "Type mismatch in connection '{}'\n\
             Source: {} (pin '{}') outputs type '{}'\n\
             Target: {} (pin '{}') expects type '{}'\n\
             {}",
            self.connection_id,
            self.source_node,
            self.source_pin,
            self.actual_type,
            self.target_node,
            self.target_pin,
            self.expected_type,
            self.message
        )
    }
}

impl<'a> TypeChecker<'a> {
    /// Create a new type checker
    pub fn new(graph: &'a GraphDescription, metadata: &'a HashMap<String, NodeMetadata>) -> Self {
        Self { graph, metadata }
    }

    /// Validate all connections in the graph
    ///
    /// Returns a list of type errors, or an empty vector if all connections are valid.
    pub fn validate_all_connections(&self) -> Vec<TypeError> {
        let mut errors = Vec::new();

        for connection in &self.graph.connections {
            // Skip execution connections (not typed)
            if matches!(connection.connection_type, crate::graph::ConnectionType::Execution) {
                continue;
            }

            if let Err(error) = self.validate_connection(connection) {
                errors.push(error);
            }
        }

        errors
    }

    /// Validate a single connection
    fn validate_connection(&self, connection: &Connection) -> Result<(), TypeError> {
        // Get source node and its output type
        let source_node = self.graph.nodes.get(&connection.source_node)
            .ok_or_else(|| TypeError {
                connection_id: connection.id.clone(),
                source_node: connection.source_node.clone(),
                source_pin: connection.source_pin.clone(),
                target_node: connection.target_node.clone(),
                target_pin: connection.target_pin.clone(),
                expected_type: "unknown".to_string(),
                actual_type: "unknown".to_string(),
                message: format!("Source node '{}' not found", connection.source_node),
            })?;

        let target_node = self.graph.nodes.get(&connection.target_node)
            .ok_or_else(|| TypeError {
                connection_id: connection.id.clone(),
                source_node: connection.source_node.clone(),
                source_pin: connection.source_pin.clone(),
                target_node: connection.target_node.clone(),
                target_pin: connection.target_pin.clone(),
                expected_type: "unknown".to_string(),
                actual_type: "unknown".to_string(),
                message: format!("Target node '{}' not found", connection.target_node),
            })?;

        // Get output pin type from source node
        let source_pin_data = source_node.outputs.iter()
            .find(|p| p.id == connection.source_pin)
            .ok_or_else(|| TypeError {
                connection_id: connection.id.clone(),
                source_node: connection.source_node.clone(),
                source_pin: connection.source_pin.clone(),
                target_node: connection.target_node.clone(),
                target_pin: connection.target_pin.clone(),
                expected_type: "unknown".to_string(),
                actual_type: "unknown".to_string(),
                message: format!("Source pin '{}' not found on node '{}'", 
                                connection.source_pin, connection.source_node),
            })?;

        // Get input pin type from target node
        let target_pin_data = target_node.inputs.iter()
            .find(|p| p.id == connection.target_pin)
            .ok_or_else(|| TypeError {
                connection_id: connection.id.clone(),
                source_node: connection.source_node.clone(),
                source_pin: connection.source_pin.clone(),
                target_node: connection.target_node.clone(),
                target_pin: connection.target_pin.clone(),
                expected_type: "unknown".to_string(),
                actual_type: "unknown".to_string(),
                message: format!("Target pin '{}' not found on node '{}'",
                                connection.target_pin, connection.target_node),
            })?;

        // Extract types
        let source_type = &source_pin_data.data_type;
        let target_type = &target_pin_data.data_type;

        // Check compatibility
        if !self.are_types_compatible(source_type, target_type) {
            return Err(TypeError {
                connection_id: connection.id.clone(),
                source_node: connection.source_node.clone(),
                source_pin: connection.source_pin.clone(),
                target_node: connection.target_node.clone(),
                target_pin: connection.target_pin.clone(),
                expected_type: format!("{:?}", target_type),
                actual_type: format!("{:?}", source_type),
                message: "Types are not compatible".to_string(),
            });
        }

        Ok(())
    }

    /// Check if two types are compatible
    fn are_types_compatible(&self, source_type: &DataType, target_type: &DataType) -> bool {
        match (source_type, target_type) {
            // Exact match
            (DataType::Typed(s), DataType::Typed(t)) if s == t => true,
            
            // Legacy to Typed conversion
            (DataType::Number, DataType::Typed(t)) if t.name() == "f64" || t.name() == "i64" => true,
            (DataType::Typed(s), DataType::Number) if s.name() == "f64" || s.name() == "i64" => true,
            
            (DataType::Boolean, DataType::Typed(t)) if t.name() == "bool" => true,
            (DataType::Typed(s), DataType::Boolean) if s.name() == "bool" => true,
            
            (DataType::Text, DataType::Typed(t)) if t.name() == "String" || t.name() == "&str" => true,
            (DataType::Typed(s), DataType::Text) if s.name() == "String" || s.name() == "&str" => true,
            
            // &str to String conversion (will clone)
            (DataType::Typed(s), DataType::Typed(t)) 
                if s.name() == "&str" && t.name() == "String" => true,
            
            // Numeric widening
            (DataType::Typed(s), DataType::Typed(t)) => {
                self.is_numeric_widening(s.name(), t.name())
            }
            
            // Any to Any
            (DataType::Any, _) | (_, DataType::Any) => true,
            
            _ => false,
        }
    }

    /// Check if numeric widening is allowed (e.g., i32 → i64)
    fn is_numeric_widening(&self, from: &str, to: &str) -> bool {
        matches!(
            (from, to),
            // Integer widening
            ("i8", "i16") | ("i8", "i32") | ("i8", "i64") |
            ("i16", "i32") | ("i16", "i64") |
            ("i32", "i64") |
            
            ("u8", "u16") | ("u8", "u32") | ("u8", "u64") |
            ("u16", "u32") | ("u16", "u64") |
            ("u32", "u64") |
            
            // Float widening
            ("f32", "f64") |
            
            // Integer to float (with warning)
            ("i8", "f32") | ("i8", "f64") |
            ("i16", "f32") | ("i16", "f64") |
            ("i32", "f64") |
            ("u8", "f32") | ("u8", "f64") |
            ("u16", "f32") | ("u16", "f64") |
            ("u32", "f64")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::*;

    fn create_test_graph() -> GraphDescription {
        GraphDescription::new("test")
    }

    #[test]
    fn test_exact_type_match() {
        let graph = create_test_graph();
        let metadata = HashMap::new();
        let checker = TypeChecker::new(&graph, &metadata);

        let t1 = DataType::Typed(TypeInfo::parse("i32"));
        let t2 = DataType::Typed(TypeInfo::parse("i32"));
        
        assert!(checker.are_types_compatible(&t1, &t2));
    }

    #[test]
    fn test_numeric_widening() {
        let graph = create_test_graph();
        let metadata = HashMap::new();
        let checker = TypeChecker::new(&graph, &metadata);

        let i32_type = DataType::Typed(TypeInfo::parse("i32"));
        let i64_type = DataType::Typed(TypeInfo::parse("i64"));
        
        assert!(checker.are_types_compatible(&i32_type, &i64_type));
        assert!(!checker.are_types_compatible(&i64_type, &i32_type)); // No narrowing
    }

    #[test]
    fn test_str_to_string() {
        let graph = create_test_graph();
        let metadata = HashMap::new();
        let checker = TypeChecker::new(&graph, &metadata);

        let str_type = DataType::Typed(TypeInfo::parse("&str"));
        let string_type = DataType::Typed(TypeInfo::parse("String"));
        
        assert!(checker.are_types_compatible(&str_type, &string_type));
    }
}
