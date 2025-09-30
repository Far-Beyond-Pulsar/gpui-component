//! # Code Generator
//!
//! The core code generation logic for transforming node graphs into Rust code.
//!
//! This module implements different generation strategies for each node type:
//! - **Pure nodes**: Inline as expressions
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

    /// Generate code for an entry point node
    pub fn generate_entry_point(&mut self, entry_node: &NodeInstance) -> Result<String, String> {
        let fn_name = match entry_node.node_type.as_str() {
            "begin_play" => "main",
            "on_tick" => "on_tick",
            _ => &entry_node.node_type,
        };

        let mut body = String::new();

        // First, evaluate all pure nodes in dependency order
        body.push_str("    // Pure node evaluations\n");
        for pure_node_id in self.data_resolver.get_pure_evaluation_order() {
            if let Some(node) = self.graph.nodes.get(pure_node_id) {
                let code = self.generate_pure_node(node)?;
                body.push_str("    ");
                body.push_str(&code);
                body.push_str("\n");
            }
        }

        body.push_str("\n    // Execution chain\n");

        // Follow execution chain from entry point
        self.generate_exec_chain(entry_node, &mut body, 1)?;

        Ok(format!("fn {}() {{\n{}}}\n", fn_name, body))
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
        }
    }

    /// Generate code for a pure node (pre-evaluation)
    fn generate_pure_node(&self, node: &NodeInstance) -> Result<String, String> {
        let node_meta = self.metadata
            .get(&node.node_type)
            .ok_or_else(|| format!("Unknown node type: {}", node.node_type))?;

        // Collect arguments
        let args = self.collect_arguments(node, node_meta)?;

        // Get result variable name
        let result_var = self.data_resolver
            .get_result_variable(&node.id)
            .ok_or_else(|| format!("No result variable for node: {}", node.id))?;

        // Generate: let result_var = function_name(args);
        Ok(format!(
            "let {} = {}({});",
            result_var,
            node_meta.name,
            args.join(", ")
        ))
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

    // Find entry points
    let entry_points: Vec<_> = graph
        .nodes
        .values()
        .filter(|node| {
            matches!(
                node.node_type.as_str(),
                "begin_play" | "on_tick" | "on_event"
            )
        })
        .collect();

    if entry_points.is_empty() {
        return Err("No entry points found in graph".to_string());
    }

    // Generate each entry point
    for entry_node in entry_points {
        let mut generator = CodeGenerator::new(metadata, data_resolver, exec_routing, graph);
        let entry_code = generator.generate_entry_point(entry_node)?;
        code.push_str(&entry_code);
        code.push_str("\n");
    }

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: Add tests once we have the full compiler pipeline
}
