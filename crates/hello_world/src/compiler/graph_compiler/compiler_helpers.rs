//! # Compiler Helper Methods
//!
//! Utility functions for the graph compiler including:
//! - Argument collection
//! - Input value resolution
//! - Property value conversion
//! - Template rendering
//! - Function body extraction

use std::collections::HashMap;
use tron::TronTemplate;
use crate::graph::{GraphDescription, NodeInstance, PropertyValue};
use super::TemplateType;

impl super::GraphCompiler {
    /// Get the arguments for a node function call.
    ///
    /// Collects input values from connections, properties, or defaults.
    pub(super) fn get_node_arguments(&self, node: &NodeInstance, graph: &GraphDescription) -> Result<String, String> {
        let node_def = self.node_definitions
            .get(&node.node_type)
            .ok_or_else(|| format!("Node definition not found: {}", node.node_type))?;

        let mut args = Vec::new();

        for input in &node_def.inputs {
            if input.data_type != "execution" {
                let value = self.get_input_value(node, &input.name, graph)?;
                args.push(value);
            }
        }

        Ok(args.join(", "))
    }

    /// Get the value for an input pin (from connections, properties, or defaults).
    ///
    /// Resolution order:
    /// 1. Connected value (from data flow)
    /// 2. Property value (set by user)
    /// 3. Default value (from node definition)
    pub(super) fn get_input_value(&self, node: &NodeInstance, input_name: &str, graph: &GraphDescription) -> Result<String, String> {
        // Try connected value first
        if let Some(pin) = node.inputs.get(input_name) {
            if let Some(connected_value) = self.get_connected_value(&pin.connected_to, graph) {
                return Ok(connected_value);
            }
        }

        // Try property value
        if let Some(property_value) = node.properties.get(input_name) {
            return Ok(self.property_value_to_string(property_value));
        }

        // Try default value
        let node_def = self.node_definitions.get(&node.node_type)
            .ok_or_else(|| format!("Node definition not found: {}", node.node_type))?;

        if let Some(input_def) = node_def.inputs.iter().find(|i| i.name == input_name) {
            if let Some(default) = &input_def.default_value {
                return Ok(default.clone());
            }
        }

        Err(format!("No value for input '{}' on node '{}'", input_name, node.id))
    }

    /// Get the value from a connected output pin.
    ///
    /// For pure expressions, inlines the expression directly.
    /// For functions, generates a function call.
    fn get_connected_value(&self, connection_ids: &[String], graph: &GraphDescription) -> Option<String> {
        for conn_id in connection_ids {
            if let Some(conn) = graph.connections.iter().find(|c| c.id == *conn_id) {
                if let Some(source_node) = graph.nodes.get(&conn.source_node) {
                    // For pure expressions, inline the expression
                    if matches!(self.get_template_type(&source_node.node_type).ok()?, TemplateType::PureExpression) {
                        // TODO: Render pure expression template
                        return Some(format!("/* expression: {} */", source_node.node_type));
                    } else {
                        // Function call that returns value
                        return Some(format!("{}()", source_node.node_type));
                    }
                }
            }
        }
        None
    }

    /// Convert a property value to a Rust string representation.
    pub(super) fn property_value_to_string(&self, value: &PropertyValue) -> String {
        match value {
            PropertyValue::String(s) => format!("\"{}\"", s),
            PropertyValue::Number(n) => n.to_string(),
            PropertyValue::Boolean(b) => b.to_string(),
            PropertyValue::Vector2(x, y) => format!("Vector2::new({}, {})", x, y),
            PropertyValue::Vector3(x, y, z) => format!("Vector3::new({}, {}, {})", x, y, z),
            PropertyValue::Color(r, g, b, a) => format!("Color::rgba({}, {}, {}, {})", r, g, b, a),
        }
    }

    /// Render a template with the given variables.
    ///
    /// Sets all variables on the template and renders it to a string.
    pub(super) fn render_template(
        &self,
        template: &TronTemplate,
        vars: HashMap<String, String>,
        node_type: &str,
    ) -> Result<String, String> {
        let mut template_clone = template.clone();

        for (key, value) in &vars {
            if let Err(e) = template_clone.set(key, value) {
                println!("[WARNING] Failed to set '{}' for {}: {}", key, node_type, e);
            }
        }

        template_clone
            .render()
            .map_err(|e| format!("Template render error for {}: {}", node_type, e))
    }

    /// Extract the function body from a rendered template.
    ///
    /// Templates are rendered as `fn name() { BODY }`, but when inlining we only
    /// want the BODY part. This strips the function wrapper.
    ///
    /// ## Example
    ///
    /// Input: `fn foo() {\n    println!("hello");\n}`
    /// Output: `println!("hello");`
    pub(super) fn extract_function_body(&self, template_output: &str) -> Result<String, String> {
        // Find first '{' and last '}'
        let start = template_output.find('{')
            .ok_or("Template has no opening brace")?;
        let end = template_output.rfind('}')
            .ok_or("Template has no closing brace")?;

        // Extract content between braces
        let body = &template_output[start + 1..end];

        // Clean up indentation
        Ok(body.trim().to_string())
    }
}