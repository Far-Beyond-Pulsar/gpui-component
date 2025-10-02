//! # Code Generator
//!
//! The core code generation logic for transforming node graphs into Rust code.
//!
//! This module implements different generation strategies for each node type:
//! - **Pure nodes**: Recursively inlined as expressions where used (no allocations)
//! - **Function nodes**: Generate function calls with exec chain
//! - **Control flow nodes**: Inline function body with substitutions

use std::collections::{HashMap, HashSet};
use crate::graph::{GraphDescription, NodeInstance, ConnectionType};
use super::{
    node_metadata::{NodeMetadata, NodeType},
    data_resolver::DataResolver,
    execution_routing::ExecutionRouting,
    ast_utils,
};

/// Main code generator
pub struct CodeGenerator<'a> {
    /// Node metadata from pulsar_std
    metadata: &'a HashMap<String, NodeMetadata>,

    /// Data flow resolver
    data_resolver: &'a DataResolver,

    /// Execution routing table
    exec_routing: &'a ExecutionRouting,

    /// The graph being compiled
    graph: &'a GraphDescription,

    /// Tracks visited nodes to prevent infinite loops
    visited: HashSet<String>,
}

impl<'a> CodeGenerator<'a> {
    pub fn new(
        metadata: &'a HashMap<String, NodeMetadata>,
        data_resolver: &'a DataResolver,
        exec_routing: &'a ExecutionRouting,
        graph: &'a GraphDescription,
    ) -> Self {
        Self {
            metadata,
            data_resolver,
            exec_routing,
            graph,
            visited: HashSet::new(),
        }
    }

    /// Generate code for an event entry point
    pub fn generate_event_function(&mut self, event_node: &NodeInstance) -> Result<String, String> {
        // Get event node metadata
        let node_meta = self.metadata
            .get(&event_node.node_type)
            .ok_or_else(|| format!("Unknown event node type: {}", event_node.node_type))?;

        if node_meta.node_type != NodeType::Event {
            return Err(format!("Node {} is not an event node", event_node.id));
        }

        // Use the function name from the event definition
        let fn_name = &node_meta.name;

        let mut body = String::new();

        // Follow execution chain from event's "Body" output
        let connected_nodes = self.exec_routing.get_connected_nodes(&event_node.id, "Body");
        for target_id in connected_nodes {
            if let Some(target_node) = self.graph.nodes.get(target_id) {
                self.generate_exec_chain(target_node, &mut body, 1)?;
            }
        }

        // Add imports and function definition
        Ok(format!(
            "use pulsar_std::*;\n\npub fn {}() {{\n{}}}\n",
            fn_name, body
        ))
    }

    /// Generate execution chain starting from a node
    fn generate_exec_chain(
        &mut self,
        node: &NodeInstance,
        output: &mut String,
        indent_level: usize,
    ) -> Result<(), String> {
        // Prevent infinite loops
        if self.visited.contains(&node.id) {
            return Ok(());
        }
        self.visited.insert(node.id.clone());

        let node_meta = self.metadata
            .get(&node.node_type)
            .ok_or_else(|| format!("Unknown node type: {}", node.node_type))?;

        match node_meta.node_type {
            NodeType::Pure => {
                // Pure nodes are pre-evaluated, skip in exec chain
                Ok(())
            }

            NodeType::Function => {
                self.generate_function_node(node, node_meta, output, indent_level)
            }

            NodeType::ControlFlow => {
                self.generate_control_flow_node(node, node_meta, output, indent_level)
            }

            NodeType::Event => {
                // Event nodes define the outer function, skip in exec chain
                // Their "Body" output defines where execution starts
                Ok(())
            }
        }
    }

    /// Generate code for a function node
    fn generate_function_node(
        &mut self,
        node: &NodeInstance,
        node_meta: &NodeMetadata,
        output: &mut String,
        indent_level: usize,
    ) -> Result<(), String> {
        let indent = "    ".repeat(indent_level);

        // Collect arguments
        let args = self.collect_arguments(node, node_meta)?;

        // Check if this function returns a value
        let has_return = node_meta.return_type.is_some();

        if has_return {
            // Store result in variable
            let result_var = self.data_resolver
                .get_result_variable(&node.id)
                .ok_or_else(|| format!("No result variable for node: {}", node.id))?;

            output.push_str(&format!(
                "{}let {} = {}({});\n",
                indent,
                result_var,
                node_meta.name,
                args.join(", ")
            ));
        } else {
            // Just call the function
            output.push_str(&format!(
                "{}{}({});\n",
                indent,
                node_meta.name,
                args.join(", ")
            ));
        }

        // Follow execution chain
        if let Some(exec_out) = node_meta.exec_outputs.first() {
            let connected = self.exec_routing.get_connected_nodes(&node.id, exec_out);
            for next_node_id in connected {
                if let Some(next_node) = self.graph.nodes.get(next_node_id) {
                    self.generate_exec_chain(next_node, output, indent_level)?;
                }
            }
        }

        Ok(())
    }

    /// Generate code for a control flow node (inline)
    fn generate_control_flow_node(
        &mut self,
        node: &NodeInstance,
        node_meta: &NodeMetadata,
        output: &mut String,
        indent_level: usize,
    ) -> Result<(), String> {
        let indent = "    ".repeat(indent_level);

        // Build exec_output replacements
        let mut exec_replacements = HashMap::new();
        for exec_pin in &node_meta.exec_outputs {
            let connected = self.exec_routing.get_connected_nodes(&node.id, exec_pin);

            let mut exec_code = String::new();
            let mut local_visited = self.visited.clone();

            for next_node_id in connected {
                if let Some(next_node) = self.graph.nodes.get(next_node_id) {
                    // Create a sub-generator with local visited set
                    let mut sub_gen = CodeGenerator {
                        metadata: self.metadata,
                        data_resolver: self.data_resolver,
                        exec_routing: self.exec_routing,
                        graph: self.graph,
                        visited: local_visited.clone(),
                    };

                    sub_gen.generate_exec_chain(next_node, &mut exec_code, 0)?;
                    local_visited = sub_gen.visited;
                }
            }

            exec_replacements.insert(exec_pin.clone(), exec_code.trim().to_string());
        }

        // Build parameter substitutions
        let mut param_substitutions = HashMap::new();
        for param in &node_meta.params {
            let value = self.data_resolver
                .generate_input_expression(&node.id, &param.name, self.graph)?;
            param_substitutions.insert(param.name.clone(), value);
        }

        // Inline the function with substitutions
        let inlined_body = ast_utils::inline_control_flow_function(
            &node_meta.function_source,
            exec_replacements,
            param_substitutions,
        )?;

        // Add inlined code with proper indentation
        for line in inlined_body.lines() {
            if !line.trim().is_empty() {
                output.push_str(&format!("{}{}\n", indent, line));
            }
        }

        Ok(())
    }

    /// Collect arguments for a function call
    fn collect_arguments(
        &self,
        node: &NodeInstance,
        node_meta: &NodeMetadata,
    ) -> Result<Vec<String>, String> {
        let mut args = Vec::new();

        for param in &node_meta.params {
            let value = self.data_resolver
                .generate_input_expression(&node.id, &param.name, self.graph)?;
            args.push(value);
        }

        Ok(args)
    }
}

/// Generate complete Rust program from graph
pub fn generate_program(
    graph: &GraphDescription,
    metadata: &HashMap<String, NodeMetadata>,
    data_resolver: &DataResolver,
    exec_routing: &ExecutionRouting,
) -> Result<String, String> {
    let mut code = String::new();

    // Add imports
    code.push_str("// Auto-generated code from Pulsar Blueprint\n");
    code.push_str("// DO NOT EDIT - Changes will be overwritten\n\n");
    code.push_str("use pulsar_std::*;\n\n");

    // Find event nodes using metadata
    let event_nodes: Vec<_> = graph
        .nodes
        .values()
        .filter(|node| {
            // Check if this node's type is an event in metadata
            metadata
                .get(&node.node_type)
                .map(|meta| meta.node_type == NodeType::Event)
                .unwrap_or(false)
        })
        .collect();

    if event_nodes.is_empty() {
        return Err("No event nodes found in graph - add a 'main' or 'begin_play' event".to_string());
    }

    // Generate each event function
    for event_node in event_nodes {
        let mut generator = CodeGenerator::new(metadata, data_resolver, exec_routing, graph);
        let event_code = generator.generate_event_function(event_node)?;
        code.push_str(&event_code);
        code.push_str("\n");
    }

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Add tests once we have the full compiler pipeline
}
