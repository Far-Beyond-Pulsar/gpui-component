//! # Unreal Blueprint-Style Graph Compiler
//!
//! This compiler transforms visual node graphs into executable Rust code following
//! the Unreal Engine Blueprint execution model.
//!
//! ## Key Concepts
//!
//! - **Entry Points**: Nodes like `begin_play` become top-level functions (`main()`)
//! - **Execution Flow**: White execution pins control flow, exactly like Blueprint wires
//! - **Inline Expansion**: Nodes with execution placeholders are expanded inline
//! - **Execution Routing**: Each execution output pin routes to specific code
//!
//! ## Template Types
//!
//! 1. **Pure Expressions**: `a + b` - Used in data flow only
//! 2. **Simple Functions**: `print_string()` - Can be called or inlined
//! 3. **Control Flow**: `branch`, `thread_spawn` - MUST be inlined
//!
//! ## Example
//!
//! ```text
//! BeginPlay -> thread_spawn -> print("after spawn")
//!              └─body─> thread_park
//! ```
//!
//! Compiles to:
//!
//! ```rust
//! fn main() {
//!     let handle = std::thread::spawn(|| {
//!         thread_park();  // From body pin
//!     });
//!     print("after spawn");  // From continue pin
//!     handle
//! }
//! ```

use std::collections::{HashMap, HashSet};
use tron::TronTemplate;
use crate::graph::{GraphDescription, NodeInstance, ConnectionType, PropertyValue};
use crate::compiler::NodeDefinition;

// ============================================================================
// SECTION 1: Core Data Structures
// ============================================================================

/// Tracks which nodes are connected to which execution output pins.
///
/// This is critical for routing execution flow correctly. In Unreal Blueprints,
/// each execution output pin can connect to different nodes, and we must
/// preserve this routing when generating code.
///
/// ## Example
///
/// For a `thread_spawn` node with two exec outputs:
/// - `body` pin connects to `thread_park` node
/// - `continue` pin connects to `print_string` node
///
/// The routing table stores:
/// ```text
/// (thread_spawn_id, "body") -> [thread_park_id]
/// (thread_spawn_id, "continue") -> [print_string_id]
/// ```
struct ExecutionRouting {
    /// Map from (source_node_id, output_pin_name) -> Vec<target_node_ids>
    routes: HashMap<(String, String), Vec<String>>,
}

impl ExecutionRouting {
    /// Build routing table from graph connections.
    ///
    /// Analyzes all execution-type connections in the graph and builds a lookup
    /// table that maps source pins to their target nodes.
    fn build_from_graph(graph: &GraphDescription) -> Self {
        let mut routes = HashMap::new();

        for connection in &graph.connections {
            if matches!(connection.connection_type, ConnectionType::Execution) {
                let key = (
                    connection.source_node.clone(),
                    connection.source_pin.clone(),
                );
                routes
                    .entry(key)
                    .or_insert_with(Vec::new)
                    .push(connection.target_node.clone());
            }
        }

        println!("[ROUTING] Built execution routing table with {} routes", routes.len());
        ExecutionRouting { routes }
    }

    /// Get all nodes connected to a specific execution output pin.
    ///
    /// Returns an empty slice if no nodes are connected to this pin.
    fn get_connected_nodes(&self, node_id: &str, output_pin: &str) -> &[String] {
        self.routes
            .get(&(node_id.to_string(), output_pin.to_string()))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}

/// Classification of node templates based on their execution characteristics.
///
/// This determines how a node should be compiled:
/// - Pure expressions are used in data flow only
/// - Simple functions can be called or inlined
/// - Control flow nodes MUST be inlined with their placeholders filled
#[derive(Debug, Clone)]
enum TemplateType {
    /// Pure expression like `a + b` - no function wrapper, just Rust expression
    PureExpression,

    /// Function with no execution placeholders - can be called as a function
    SimpleFunction,

    /// Has execution placeholders - MUST be inlined into execution flow
    ControlFlow {
        /// Names of execution placeholders (e.g., ["pulsar_exec_body", "pulsar_exec_continue"])
        exec_placeholders: Vec<String>,
    },
}

// ============================================================================
// SECTION 2: Main Compiler Structure
// ============================================================================

/// The main graph compiler that transforms node graphs into Rust code.
///
/// This follows the Unreal Blueprint execution model where:
/// - Entry points become top-level functions
/// - Execution flow is followed recursively
/// - Control flow nodes are expanded inline
/// - Simple nodes can be called as functions
pub struct GraphCompiler {
    /// Definitions for all available node types
    node_definitions: HashMap<String, NodeDefinition>,

    /// Templates for generating node code
    templates: HashMap<String, TronTemplate>,

    /// Cached template type analysis
    template_types: HashMap<String, TemplateType>,
}

impl GraphCompiler {
    pub fn new(
        node_definitions: HashMap<String, NodeDefinition>,
        templates: HashMap<String, TronTemplate>,
    ) -> Self {
        let mut compiler = Self {
            node_definitions,
            templates,
            template_types: HashMap::new(),
        };

        // Pre-analyze all templates
        compiler.analyze_all_templates();

        compiler
    }

    /// Main compilation entry point.
    ///
    /// ## Algorithm
    ///
    /// 1. Build execution routing table from connections
    /// 2. Find entry points (begin_play, on_tick, etc.)
    /// 3. Generate function definitions for simple nodes
    /// 4. Generate entry point functions with inline expansion
    ///
    /// ## Output
    ///
    /// Returns generated Rust code as a string, or an error if compilation fails.
    pub fn compile_graph(&self, graph: &GraphDescription) -> Result<String, String> {
        println!("[COMPILER] === Starting Compilation ===");

        let mut generated_code = String::new();

        // Phase 1: Build execution routing table
        let routing = ExecutionRouting::build_from_graph(graph);

        // Phase 2: Find entry points
        let entry_points = self.find_entry_points(graph);
        println!("[COMPILER] Found {} entry points: {:?}",
                 entry_points.len(),
                 entry_points.iter()
                     .filter_map(|id| graph.nodes.get(id).map(|n| &n.node_type))
                     .collect::<Vec<_>>());

        // Phase 3: Generate simple function definitions
        let simple_functions = self.generate_simple_functions(graph)?;
        if !simple_functions.is_empty() {
            generated_code.push_str("// ============================================================================\n");
            generated_code.push_str("// Simple Function Definitions (nodes without execution placeholders)\n");
            generated_code.push_str("// ============================================================================\n\n");
            generated_code.push_str(&simple_functions);
            generated_code.push_str("\n");
        }

        // Phase 4: Generate entry point functions
        generated_code.push_str("// ============================================================================\n");
        generated_code.push_str("// Entry Points (Begin Play, On Tick, etc.)\n");
        generated_code.push_str("// ============================================================================\n\n");

        for entry_node_id in entry_points {
            if let Some(entry_node) = graph.nodes.get(&entry_node_id) {
                let entry_fn = self.compile_entry_point(entry_node, graph, &routing)?;
                generated_code.push_str(&entry_fn);
                generated_code.push_str("\n\n");
            }
        }

        println!("[COMPILER] === Compilation Complete ===");
        Ok(generated_code)
    }

    // ========================================================================
    // SECTION 3: Template Analysis
    // ========================================================================

    /// Pre-analyze all templates to determine their type.
    ///
    /// This caches the template type for faster compilation. A template is:
    /// - **PureExpression** if it has no function wrapper
    /// - **SimpleFunction** if it has a function but no exec placeholders
    /// - **ControlFlow** if it has exec placeholders
    fn analyze_all_templates(&mut self) {
        println!("[COMPILER] Analyzing {} templates...", self.templates.len());

        for (node_type, template) in &self.templates {
            let template_type = self.analyze_template_type(template);
            println!("[COMPILER]   {}: {:?}", node_type, template_type);
            self.template_types.insert(node_type.clone(), template_type);
        }
    }

    /// Analyze a single template to determine its type.
    fn analyze_template_type(&self, template: &TronTemplate) -> TemplateType {
        // Get template content - we need to inspect it
        let template_str = format!("{:?}", template);

        // Check if it has a function wrapper
        if !template_str.contains("fn ") {
            return TemplateType::PureExpression;
        }

        // Find all execution placeholders
        let mut exec_placeholders = Vec::new();
        for part in template_str.split("@[") {
            if let Some(end) = part.find("]@") {
                let placeholder = &part[..end];
                if placeholder.starts_with("pulsar_exec_") {
                    exec_placeholders.push(placeholder.to_string());
                }
            }
        }

        if exec_placeholders.is_empty() {
            TemplateType::SimpleFunction
        } else {
            TemplateType::ControlFlow { exec_placeholders }
        }
    }

    /// Get the cached template type for a node type.
    fn get_template_type(&self, node_type: &str) -> Result<&TemplateType, String> {
        self.template_types
            .get(node_type)
            .ok_or_else(|| format!("Template type not found for: {}", node_type))
    }

    // ========================================================================
    // SECTION 4: Entry Point & Function Generation
    // ========================================================================

    /// Find all entry point nodes in the graph.
    ///
    /// Entry points are nodes that create top-level functions like `main()`.
    /// Examples: `begin_play`, `on_tick`, etc.
    fn find_entry_points(&self, graph: &GraphDescription) -> Vec<String> {
        graph
            .nodes
            .iter()
            .filter(|(_, node)| self.is_entry_point_type(&node.node_type))
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Check if a node type is an entry point.
    fn is_entry_point_type(&self, node_type: &str) -> bool {
        matches!(node_type, "begin_play" | "on_tick" | "on_input" | "on_event")
    }

    /// Generate function definitions for simple nodes (no exec placeholders).
    ///
    /// Only generates functions for nodes that are actually used in the graph.
    /// Control flow nodes are always inlined, so they never get function definitions.
    fn generate_simple_functions(&self, graph: &GraphDescription) -> Result<String, String> {
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
    fn find_reachable_nodes(&self, graph: &GraphDescription, entry_points: &[String]) -> HashSet<String> {
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
    fn compile_entry_point(
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

    // ========================================================================
    // SECTION 5: Core Recursive Inline Compilation (THE HEART OF THE COMPILER)
    // ========================================================================

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
    fn compile_node_inline(
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
                // Generate function call
                let args = self.get_node_arguments(node, graph)?;
                output.push_str(&format!("{}{}({});\n", indent, node.node_type, args));

                // Follow single execution output
                let node_def = self.node_definitions.get(&node.node_type).unwrap();
                if let Some(exec_output) = node_def.execution_outputs.first() {
                    let connected = routing.get_connected_nodes(&node.id, &exec_output.name);
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
                }

                Ok(())
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
    fn compile_control_flow_inline(
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

    // ========================================================================
    // SECTION 6: Helper Methods
    // ========================================================================

    /// Get the arguments for a node function call.
    ///
    /// Collects input values from connections, properties, or defaults.
    fn get_node_arguments(&self, node: &NodeInstance, graph: &GraphDescription) -> Result<String, String> {
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
    fn get_input_value(&self, node: &NodeInstance, input_name: &str, graph: &GraphDescription) -> Result<String, String> {
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
    fn property_value_to_string(&self, value: &PropertyValue) -> String {
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
    fn render_template(
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
    fn extract_function_body(&self, template_output: &str) -> Result<String, String> {
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