//! # Node Compiler - The Heart of the Compiler
//!
//! This module contains the core recursive inline compilation algorithm that
//! transforms nodes into Rust code following the Unreal Blueprint execution model.
//!
//! ## Key Algorithm: Recursive Inline Compilation
//!
//! The `compile_node_inline` method is called recursively to build up the execution
//! sequence by following execution pins exactly like Blueprint wires.
//!
//! ### For Simple Functions:
//! 1. Generate function call
//! 2. Follow the single execution output to next node
//! 3. Recursively compile the next node
//!
//! ### For Control Flow Nodes:
//! 1. Find nodes connected to each execution output pin
//! 2. Recursively compile those nodes to fill placeholders
//! 3. Inline the template with filled placeholders
//!
//! ## Example: thread_spawn
//!
//! ```text
//! thread_spawn:
//!   - body pin -> [thread_park]
//!   - continue pin -> [print_string]
//! ```
//!
//! Compiles to:
//! ```rust
//! let handle = std::thread::spawn(|| {
//!     thread_park();  // Recursively compiled from body pin
//! });
//! print_string();  // Recursively compiled from continue pin
//! handle
//! ```

use std::collections::{HashMap, HashSet};
use crate::graph::{GraphDescription, NodeInstance};
use super::{ExecutionRouting, TemplateType};

impl super::GraphCompiler {
    /// Compile a node inline into the execution sequence.
    ///
    /// This is the core recursive algorithm that follows the Unreal Blueprint model:
    ///
    /// 1. **Simple Functions**: Call the function, then follow single exec output
    /// 2. **Control Flow**: Inline the template, filling exec placeholders recursively
    ///
    /// ## Recursion
    ///
    /// When filling exec placeholders, this method calls itself recursively to
    /// generate code for connected nodes. This naturally handles nested control
    /// flow (e.g., a branch inside a thread spawn body).
    ///
    /// ## Visited Tracking
    ///
    /// Tracks visited nodes to avoid infinite loops in cyclic graphs.
    pub(super) fn compile_node_inline(
        &self,
        node: &NodeInstance,
        graph: &GraphDescription,
        routing: &ExecutionRouting,
        output: &mut String,
        visited: &mut HashSet<String>,
        indent_level: usize,
    ) -> Result<(), String> {
        // Avoid infinite loops
        if visited.contains(&node.id) {
            return Ok(());
        }
        visited.insert(node.id.clone());

        let indent = "    ".repeat(indent_level);
        let template_type = self.get_template_type(&node.node_type)?;

        println!("[INLINE] Compiling node '{}' (type: {}, template: {:?})",
                 node.id, node.node_type, template_type);

        match template_type {
            TemplateType::PureExpression => {
                Err(format!("Pure expression '{}' cannot be in execution flow", node.node_type))
            }

            TemplateType::SimpleFunction => {
                self.compile_simple_function_inline(node, graph, routing, output, visited, indent_level, &indent)
            }

            TemplateType::ControlFlow { ref exec_placeholders } => {
                // Inline the control flow template
                self.compile_control_flow_inline(
                    node,
                    exec_placeholders,
                    graph,
                    routing,
                    output,
                    visited,
                    indent_level,
                )
            }
        }
    }

    /// Compile a simple function node inline.
    ///
    /// Generates a function call and follows the single execution output.
    fn compile_simple_function_inline(
        &self,
        node: &NodeInstance,
        graph: &GraphDescription,
        routing: &ExecutionRouting,
        output: &mut String,
        visited: &mut HashSet<String>,
        indent_level: usize,
        indent: &str,
    ) -> Result<(), String> {
        // Generate function call
        let args = self.get_node_arguments(node, graph)?;
        output.push_str(&format!("{}{}({});\n", indent, node.node_type, args));

        // Follow single execution output
        let node_def = self.node_definitions.get(&node.node_type).unwrap();
        println!("[INLINE] Node '{}' has {} execution outputs: {:?}",
            node.node_type,
            node_def.execution_outputs.len(),
            node_def.execution_outputs.iter().map(|p| &p.name).collect::<Vec<_>>());

        if let Some(exec_output) = node_def.execution_outputs.first() {
            let connected = routing.get_connected_nodes(&node.id, &exec_output.name);
            println!("[INLINE] Following exec output '{}' from node '{}', found {} connected nodes: {:?}",
                exec_output.name, node.id, connected.len(), connected);

            for next_node_id in connected {
                if let Some(next_node) = graph.nodes.get(next_node_id) {
                    self.compile_node_inline(
                        next_node,
                        graph,
                        routing,
                        output,
                        visited,
                        indent_level,
                    )?;
                }
            }
        } else {
            println!("[INLINE] Node '{}' has NO execution outputs, stopping chain", node.node_type);
        }

        Ok(())
    }

    /// Compile a control flow node by inlining its template with filled placeholders.
    ///
    /// This is where the magic happens for nodes like `branch`, `thread_spawn`, etc.
    ///
    /// ## Algorithm
    ///
    /// 1. Collect input values for the node
    /// 2. For each exec placeholder:
    ///    - Find nodes connected to that specific output pin
    ///    - Recursively compile them to fill the placeholder
    /// 3. Render the template with all placeholders filled
    /// 4. Extract just the function body (strip wrapper)
    /// 5. Add to output with proper indentation
    pub(super) fn compile_control_flow_inline(
        &self,
        node: &NodeInstance,
        exec_placeholders: &[String],
        graph: &GraphDescription,
        routing: &ExecutionRouting,
        output: &mut String,
        visited: &mut HashSet<String>,
        indent_level: usize,
    ) -> Result<(), String> {
        let template = self.templates
            .get(&node.node_type)
            .ok_or_else(|| format!("Template not found: {}", node.node_type))?;

        let node_def = self.node_definitions
            .get(&node.node_type)
            .ok_or_else(|| format!("Node definition not found: {}", node.node_type))?;

        let mut vars = HashMap::new();

        // Set node-specific ID
        vars.insert("pulsar_node_fn_id".to_string(), format!("node_{}", node.id));

        // Set input values
        for input in &node_def.inputs {
            if input.data_type != "execution" {
                let var_name = format!("in_{}_{}", input.name, input.data_type);
                let value = self.get_input_value(node, &input.name, graph)?;
                vars.insert(var_name, value);
            }
        }

        // CRITICAL: Fill execution placeholders
        // This is where we route nodes to their correct execution outputs
        for placeholder in exec_placeholders {
            // Extract pin name: "pulsar_exec_body" -> "body"
            let pin_name = placeholder
                .strip_prefix("pulsar_exec_")
                .ok_or_else(|| format!("Invalid exec placeholder: {}", placeholder))?;

            // Get nodes connected to THIS specific exec output pin
            let connected = routing.get_connected_nodes(&node.id, pin_name);

            println!("[INLINE]   Filling placeholder '{}' for pin '{}' with {} connections",
                     placeholder, pin_name, connected.len());

            if connected.is_empty() {
                // No connections - empty block
                vars.insert(placeholder.clone(), "{}".to_string());
            } else {
                // Recursively compile connected nodes
                let mut exec_body = String::new();
                let mut local_visited = visited.clone();

                for next_node_id in connected {
                    if let Some(next_node) = graph.nodes.get(next_node_id) {
                        self.compile_node_inline(
                            next_node,
                            graph,
                            routing,
                            &mut exec_body,
                            &mut local_visited,
                            0, // Reset indent - will be re-indented when inserted
                        )?;
                    }
                }

                vars.insert(placeholder.clone(), exec_body.trim().to_string());
            }
        }

        // Render template
        let rendered = self.render_template(template, vars, &node.node_type)?;

        // Extract function body (strip "fn name() { BODY }")
        let body = self.extract_function_body(&rendered)?;

        // Add to output with proper indentation
        let indent = "    ".repeat(indent_level);
        for line in body.lines() {
            if !line.trim().is_empty() {
                output.push_str(&format!("{}{}\n", indent, line));
            }
        }

        Ok(())
    }
}