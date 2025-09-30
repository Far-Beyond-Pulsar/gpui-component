//! # Code Generation
//!
//! Handles generation of entry points and function definitions.
//!
//! ## Entry Points
//!
//! Entry point nodes (like `begin_play`) become top-level functions (`main()`).
//! The compiler finds all entry points and generates their corresponding functions.
//!
//! ## Function Generation
//!
//! Simple function nodes (those without execution placeholders) are generated as
//! callable functions. Control flow nodes are always inlined, so they never get
//! function definitions.
//!
//! ## Reachability Analysis
//!
//! Only generates functions for nodes that are actually reachable from entry points
//! via execution connections. This avoids generating dead code.

use std::collections::{HashMap, HashSet};
use crate::graph::{GraphDescription, NodeInstance};
use super::{ExecutionRouting, TemplateType};

impl super::GraphCompiler {
    /// Find all entry point nodes in the graph.
    ///
    /// Entry points are nodes that create top-level functions like `main()`.
    /// Examples: `begin_play`, `on_tick`, etc.
    pub(super) fn find_entry_points(&self, graph: &GraphDescription) -> Vec<String> {
        graph
            .nodes
            .iter()
            .filter(|(_, node)| self.is_entry_point_type(&node.node_type))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Check if a node type is an entry point.
    pub(super) fn is_entry_point_type(&self, node_type: &str) -> bool {
        matches!(node_type, "begin_play" | "on_tick" | "on_input" | "on_event")
    }

    /// Generate function definitions for simple nodes (no exec placeholders).
    ///
    /// Only generates functions for nodes that are actually used in the graph.
    /// Control flow nodes are always inlined, so they never get function definitions.
    pub(super) fn generate_simple_functions(&self, graph: &GraphDescription) -> Result<String, String> {
        let mut code = String::new();
        let mut generated = HashSet::new();

        // Find all reachable nodes
        let entry_points = self.find_entry_points(graph);
        let reachable = self.find_reachable_nodes(graph, &entry_points);

        for node_id in reachable {
            if let Some(node) = graph.nodes.get(&node_id) {
                let node_type = &node.node_type;

                // Skip if already generated or is entry point
                if generated.contains(node_type) || self.is_entry_point_type(node_type) {
                    continue;
                }

                // Only generate for simple functions
                if matches!(self.get_template_type(node_type)?, TemplateType::SimpleFunction) {
                    let function_def = self.generate_simple_function(node_type)?;
                    code.push_str(&function_def);
                    code.push_str("\n\n");
                    generated.insert(node_type.clone());
                }
            }
        }

        Ok(code)
    }

    /// Generate a function definition for a simple node.
    ///
    /// Renders the template with parameter names (not values) to create a function signature.
    fn generate_simple_function(&self, node_type: &str) -> Result<String, String> {
        let node_def = self.node_definitions
            .get(node_type)
            .ok_or_else(|| format!("Node definition not found: {}", node_type))?;

        let template = self.templates
            .get(node_type)
            .ok_or_else(|| format!("Template not found: {}", node_type))?;

        let mut vars = HashMap::new();

        // Set function ID
        vars.insert("pulsar_node_fn_id".to_string(), node_type.to_string());

        // Set input parameters (by name, not values)
        for input in &node_def.inputs {
            if input.data_type != "execution" {
                let var_name = format!("in_{}_{}", input.name, input.data_type);
                vars.insert(var_name, input.name.clone());
            }
        }

        self.render_template(template, vars, node_type)
    }

    /// Find all nodes reachable from entry points via execution connections.
    ///
    /// Uses depth-first search to traverse the execution graph.
    pub(super) fn find_reachable_nodes(&self, graph: &GraphDescription, entry_points: &[String]) -> HashSet<String> {
        let mut reachable = HashSet::new();

        for entry_id in entry_points {
            if let Some(entry_node) = graph.nodes.get(entry_id) {
                self.collect_reachable(entry_node, graph, &mut reachable);
            }
        }

        println!("[COMPILER] Found {} reachable nodes", reachable.len());
        reachable
    }

    /// Recursively collect reachable nodes via execution connections.
    ///
    /// Follows execution output pins to find connected nodes.
    fn collect_reachable(&self, node: &NodeInstance, graph: &GraphDescription, reachable: &mut HashSet<String>) {
        if reachable.contains(&node.id) {
            return;
        }
        reachable.insert(node.id.clone());

        // Follow execution outputs
        for (_pin_name, pin) in &node.outputs {
            if pin.data_type == "execution" {
                for conn_id in &pin.connected_to {
                    if let Some(conn) = graph.connections.iter().find(|c| &c.id == conn_id) {
                        if let Some(next_node) = graph.nodes.get(&conn.target_node) {
                            self.collect_reachable(next_node, graph, reachable);
                        }
                    }
                }
            }
        }
    }

    /// Compile an entry point node into a top-level function.
    ///
    /// ## Example
    ///
    /// `begin_play` becomes:
    /// ```rust
    /// fn main() {
    ///     // Inlined execution sequence
    /// }
    /// ```
    pub(super) fn compile_entry_point(
        &self,
        entry_node: &NodeInstance,
        graph: &GraphDescription,
        routing: &ExecutionRouting,
    ) -> Result<String, String> {
        let function_name = match entry_node.node_type.as_str() {
            "begin_play" => "main",
            "on_tick" => "on_tick",
            other => other,
        };

        println!("[COMPILER] Compiling entry point: {} -> fn {}", entry_node.node_type, function_name);

        let mut body = String::new();
        let mut visited = HashSet::new();

        // Find first execution output and follow it
        let node_def = self.node_definitions
            .get(&entry_node.node_type)
            .ok_or_else(|| format!("Node definition not found: {}", entry_node.node_type))?;

        if let Some(first_exec_output) = node_def.execution_outputs.first() {
            let connected = routing.get_connected_nodes(&entry_node.id, &first_exec_output.name);
            for next_node_id in connected {
                if let Some(next_node) = graph.nodes.get(next_node_id) {
                    self.compile_node_inline(
                        next_node,
                        graph,
                        routing,
                        &mut body,
                        &mut visited,
                        1, // indent level
                    )?;
                }
            }
        }

        Ok(format!("fn {}() {{\n{}}}", function_name, body))
    }
}