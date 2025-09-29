use std::collections::HashMap;
use tron::TronTemplate;
use crate::graph::{GraphDescription, NodeInstance, ConnectionType, PropertyValue};
use crate::compiler::NodeDefinition;

pub struct GraphCompiler {
    node_definitions: HashMap<String, NodeDefinition>,
    templates: HashMap<String, TronTemplate>,
}

impl GraphCompiler {
    pub fn new(
        node_definitions: HashMap<String, NodeDefinition>,
        templates: HashMap<String, TronTemplate>,
    ) -> Self {
        Self {
            node_definitions,
            templates,
        }
    }

    pub fn compile_graph(&self, graph: &GraphDescription) -> Result<String, String> {
        let mut generated_code = String::new();
        let mut generated_functions = std::collections::HashSet::new();

        // Find all reachable nodes from entry points
        let entry_points = self.find_entry_points(graph);
        let mut reachable_nodes = std::collections::HashSet::new();
        for entry_node_id in &entry_points {
            if let Some(entry_node) = graph.nodes.get(entry_node_id) {
                self.collect_reachable_nodes(entry_node, graph, &mut reachable_nodes);
            }
        }

        // Generate function definitions only for reachable node types
        for (_node_id, node_instance) in &graph.nodes {
            let node_type = &node_instance.node_type;

            // Skip entry points like begin_play - they become main()
            if node_type == "begin_play" {
                continue;
            }

            // Only generate functions for nodes reachable via execution
            if !reachable_nodes.contains(&node_instance.id) {
                continue;
            }

            // Only generate each function type once
            if !generated_functions.contains(node_type) {
                generated_functions.insert(node_type.clone());
                let function_def = self.compile_node_function(node_instance, graph)?;
                generated_code.push_str(&function_def);
                generated_code.push_str("\n\n");
            }
        }

        // Then generate the main function(s) from entry points
        for entry_node_id in entry_points {
            if let Some(entry_node) = graph.nodes.get(&entry_node_id) {
                let main_function = self.compile_main_function(entry_node, graph)?;
                generated_code.push_str(&main_function);
                generated_code.push_str("\n\n");
            }
        }

        Ok(generated_code)
    }

    fn find_entry_points(&self, graph: &GraphDescription) -> Vec<String> {
        // Find nodes that are actual entry points (like begin_play) or system events
        // Not just any node without incoming connections (which could be disconnected)
        let mut entry_points = Vec::new();

        for (node_id, node) in &graph.nodes {
            // Only consider nodes that are specifically entry point types
            if self.is_entry_point_type(&node.node_type) {
                entry_points.push(node_id.clone());
            }
        }

        entry_points
    }

    fn is_entry_point_type(&self, node_type: &str) -> bool {
        // Define which node types are considered entry points
        matches!(node_type, "begin_play" | "on_tick" | "on_input" | "on_event")
    }

    fn collect_reachable_nodes(
        &self,
        node: &NodeInstance,
        graph: &GraphDescription,
        reachable: &mut std::collections::HashSet<String>,
    ) {
        // Avoid infinite loops
        if reachable.contains(&node.id) {
            return;
        }
        reachable.insert(node.id.clone());

        // Follow execution output connections
        for (_pin_name, pin) in &node.outputs {
            if pin.data_type == "execution" {
                for connection_id in &pin.connected_to {
                    if let Some(connection) = graph.connections.iter().find(|c| c.id == *connection_id) {
                        if matches!(connection.connection_type, ConnectionType::Execution) {
                            if let Some(next_node) = graph.nodes.get(&connection.target_node) {
                                self.collect_reachable_nodes(next_node, graph, reachable);
                            }
                        }
                    }
                }
            }
        }
    }

    fn compile_node_function(&self, node_instance: &NodeInstance, graph: &GraphDescription) -> Result<String, String> {
        let node_def = self.node_definitions
            .get(&node_instance.node_type)
            .ok_or_else(|| format!("Node definition not found: {}", node_instance.node_type))?;

        let template = self.templates
            .get(&node_instance.node_type)
            .ok_or_else(|| format!("Node template not found: {}", node_instance.node_type))?;

        // Generate function parameters from input definitions
        let params = node_def.inputs
            .iter()
            .filter(|input| input.data_type != "execution") // Skip execution pins
            .map(|input| format!("{}: {}", input.name, self.rust_type(&input.data_type)))
            .collect::<Vec<_>>()
            .join(", ");

        // Set template variables
        let mut template_vars = std::collections::HashMap::new();

        // Set function name only - template will add parameters
        template_vars.insert("pulsar_node_fn_id".to_string(), node_instance.node_type.clone());

        // Set input placeholders to parameter names
        for input in &node_def.inputs {
            if input.data_type != "execution" {
                let var_name = format!("in_{}_{}", input.name, input.data_type);
                template_vars.insert(var_name, input.name.clone());
            }
        }

        // Set execution placeholders to empty (no chaining in function definitions)
        for output in &node_def.outputs {
            if output.data_type == "execution" {
                let exec_var = format!("pulsar_exec_{}", output.name);
                template_vars.insert(exec_var, "".to_string());
            }
        }

        // Render template
        let mut template_clone = template.clone();
        for (key, value) in template_vars {
            let _ = template_clone.set(&key, &value);
        }

        template_clone.render()
            .map_err(|e| format!("Template render error for node function {}: {}", node_instance.node_type, e))
    }

    fn compile_main_function(&self, start_node: &NodeInstance, graph: &GraphDescription) -> Result<String, String> {
        // Generate the function body by following execution pin connections
        let mut execution_body = String::new();
        let mut visited = std::collections::HashSet::new();

        self.generate_function_calls(start_node, graph, &mut execution_body, &mut visited)?;

        // Determine function name based on node type
        let function_name = if start_node.node_type == "begin_play" {
            "main".to_string()
        } else {
            start_node.node_type.clone()
        };

        // Wrap in function definition
        Ok(format!(
            "fn {}() {{\n{}\n}}",
            function_name,
            execution_body
        ))
    }

    fn generate_function_calls(
        &self,
        node: &NodeInstance,
        graph: &GraphDescription,
        execution_body: &mut String,
        visited: &mut std::collections::HashSet<String>,
    ) -> Result<(), String> {
        // Avoid infinite loops
        if visited.contains(&node.id) {
            return Ok(());
        }
        visited.insert(node.id.clone());

        // Skip generating call for begin_play - it's just the entry point
        if node.node_type != "begin_play" {
            let function_call = self.generate_node_call(node, graph)?;
            execution_body.push_str(&format!("    {};\n", function_call));
        }

        // Follow execution output connections
        for (_pin_name, pin) in &node.outputs {
            if pin.data_type == "execution" {
                for connection_id in &pin.connected_to {
                    if let Some(connection) = graph.connections.iter().find(|c| c.id == *connection_id) {
                        if matches!(connection.connection_type, ConnectionType::Execution) {
                            if let Some(next_node) = graph.nodes.get(&connection.target_node) {
                                self.generate_function_calls(next_node, graph, execution_body, visited)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn generate_node_call(&self, node_instance: &NodeInstance, graph: &GraphDescription) -> Result<String, String> {
        let node_def = self.node_definitions
            .get(&node_instance.node_type)
            .ok_or_else(|| format!("Node definition not found: {}", node_instance.node_type))?;

        let mut args = Vec::new();

        // Generate arguments from input pins (excluding execution pins)
        for input_def in &node_def.inputs {
            if input_def.data_type == "execution" {
                continue; // Skip execution pins
            }

            let pin_name = &input_def.name;

            if let Some(pin) = node_instance.inputs.get(pin_name) {
                if let Some(connected_value) = self.get_connected_value(&pin.connected_to, graph) {
                    args.push(connected_value);
                } else if let Some(property_value) = node_instance.properties.get(pin_name) {
                    args.push(self.property_value_to_string(property_value));
                } else if let Some(default) = &input_def.default_value {
                    args.push(default.clone());
                } else {
                    return Err(format!("No value provided for input pin: {}", pin_name));
                }
            } else {
                return Err(format!("Input pin not found: {}", pin_name));
            }
        }

        Ok(format!("{}({})", node_instance.node_type, args.join(", ")))
    }

    fn rust_type(&self, data_type: &str) -> &'static str {
        match data_type {
            "string" => "String",
            "number" => "f64",
            "boolean" => "bool",
            "vector2" => "(f64, f64)",
            "vector3" => "(f64, f64, f64)",
            "color" => "(f64, f64, f64, f64)",
            _ => "String", // Default fallback
        }
    }

    fn compile_execution_chain(&self, start_node: &NodeInstance, graph: &GraphDescription) -> Result<String, String> {
        // Generate the function body by following execution pin connections
        let mut execution_body = String::new();
        let mut visited = std::collections::HashSet::new();

        self.compile_node_sequence(start_node, graph, &mut execution_body, &mut visited)?;

        // Determine function name based on node type
        let function_name = if start_node.node_type == "begin_play" {
            "main".to_string()
        } else {
            start_node.node_type.clone()
        };

        // Wrap in function definition
        Ok(format!(
            "fn {}() {{\n{}\n}}",
            function_name,
            execution_body
        ))
    }

    fn compile_node_sequence(
        &self,
        node: &NodeInstance,
        graph: &GraphDescription,
        execution_body: &mut String,
        visited: &mut std::collections::HashSet<String>,
    ) -> Result<(), String> {
        // Avoid infinite loops
        if visited.contains(&node.id) {
            return Ok(());
        }
        visited.insert(node.id.clone());

        // Skip generating code for begin_play - it's just the entry point
        if node.node_type != "begin_play" {
            let node_code = self.compile_node_inline(node, graph)?;
            if !node_code.is_empty() {
                // Add proper indentation for each line
                let indented_code = node_code
                    .lines()
                    .map(|line| format!("    {}", line))
                    .collect::<Vec<_>>()
                    .join("\n");
                execution_body.push_str(&format!("{}\n", indented_code));
            }
        }

        // Follow execution output connections
        for (_pin_name, pin) in &node.outputs {
            if pin.data_type == "execution" {
                for connection_id in &pin.connected_to {
                    if let Some(connection) = graph.connections.iter().find(|c| c.id == *connection_id) {
                        if matches!(connection.connection_type, ConnectionType::Execution) {
                            if let Some(next_node) = graph.nodes.get(&connection.target_node) {
                                self.compile_node_sequence(next_node, graph, execution_body, visited)?;
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn compile_node_inline(&self, node_instance: &NodeInstance, graph: &GraphDescription) -> Result<String, String> {
        let node_def = self.node_definitions
            .get(&node_instance.node_type)
            .ok_or_else(|| format!("Node definition not found: {}", node_instance.node_type))?;

        let template = self.templates
            .get(&node_instance.node_type)
            .ok_or_else(|| format!("Node template not found: {}", node_instance.node_type))?;

        // Collect template variables
        let mut template_vars = HashMap::new();

        // Set a dummy function ID for template compatibility (will be stripped out)
        template_vars.insert("pulsar_node_fn_id".to_string(), "dummy".to_string());

        // Process input connections and properties
        for (pin_name, pin) in &node_instance.inputs {
            let variable_name = format!("in_{}_{}", pin_name, pin.data_type);

            if let Some(connected_value) = self.get_connected_value(&pin.connected_to, graph) {
                template_vars.insert(variable_name, connected_value);
            } else if let Some(property_value) = node_instance.properties.get(pin_name) {
                template_vars.insert(variable_name, self.property_value_to_string(property_value));
            } else {
                // Use default value
                if let Some(input_def) = node_def.inputs.iter().find(|i| i.name == *pin_name) {
                    if let Some(default) = &input_def.default_value {
                        template_vars.insert(variable_name, default.clone());
                    } else {
                        return Err(format!("No value provided for input pin: {}", pin_name));
                    }
                }
            }
        }

        // For inline nodes, execution pins are empty (no chaining in template)
        for (pin_name, pin) in &node_instance.outputs {
            if pin.data_type == "execution" {
                let exec_var = format!("pulsar_exec_{}", pin_name);
                template_vars.insert(exec_var, "".to_string()); // Empty - execution continues to next node
            }
        }

        // Render the template
        let mut template_clone = template.clone();
        for (key, value) in template_vars {
            template_clone.set(&key, &value);
        }
        let rendered = template_clone.render()
            .map_err(|e| format!("Template render error for node {}: {}", node_instance.id, e))?;

        // Extract just the body content, not the function wrapper
        self.extract_template_body(&rendered)
    }

    fn extract_template_body(&self, template_output: &str) -> Result<String, String> {
        // Look for content between function braces
        if let Some(start) = template_output.find('{') {
            if let Some(end) = template_output.rfind('}') {
                let body = &template_output[start + 1..end];
                // Clean up indentation and empty lines, preserve meaningful content
                let cleaned_lines: Vec<_> = body
                    .lines()
                    .map(|line| line.trim())
                    .filter(|line| !line.is_empty())
                    .collect();

                if cleaned_lines.is_empty() {
                    return Ok(String::new());
                }

                return Ok(cleaned_lines.join("\n"));
            }
        }

        // Fallback: return the template as-is, cleaned up
        Ok(template_output.trim().to_string())
    }

    fn compile_node(&self, node_instance: &NodeInstance, graph: &GraphDescription) -> Result<String, String> {
        let node_def = self.node_definitions
            .get(&node_instance.node_type)
            .ok_or_else(|| format!("Node definition not found: {}", node_instance.node_type))?;

        let template = self.templates
            .get(&node_instance.node_type)
            .ok_or_else(|| format!("Node template not found: {}", node_instance.node_type))?;

        // Collect template variables
        let mut template_vars = HashMap::new();

        // Set the function ID
        template_vars.insert("pulsar_node_fn_id".to_string(), format!("node_{}", node_instance.id));

        // Process input connections and properties
        for (pin_name, pin) in &node_instance.inputs {
            let variable_name = format!("in_{}_{}", pin_name, pin.data_type);

            if let Some(connected_value) = self.get_connected_value(&pin.connected_to, graph) {
                template_vars.insert(variable_name, connected_value);
            } else if let Some(property_value) = node_instance.properties.get(pin_name) {
                template_vars.insert(variable_name, self.property_value_to_string(property_value));
            } else {
                // Use default value
                if let Some(input_def) = node_def.inputs.iter().find(|i| i.name == *pin_name) {
                    if let Some(default) = &input_def.default_value {
                        template_vars.insert(variable_name, default.clone());
                    } else {
                        return Err(format!("No value provided for input pin: {}", pin_name));
                    }
                }
            }
        }

        // Process execution connections
        for (pin_name, pin) in &node_instance.outputs {
            if pin.data_type == "execution" {
                let exec_var = format!("pulsar_exec_{}", pin_name);
                let exec_code = self.get_execution_chain(&pin.connected_to, graph)?;
                template_vars.insert(exec_var, exec_code);
            }
        }

        // Render the template
        let mut template_clone = template.clone();
        // Log the template variables and template content before rendering
        println!(
            "Rendering node template for '{}':\n  Template variables: {:?}\n  Template content: {}",
            node_instance.node_type,
            template_vars.keys().collect::<Vec<_>>(),
            node_def.template_content
        );
        for (key, value) in template_vars {
            template_clone.set(&key, &value);
        }
        let rendered = template_clone.render()
            .map_err(|e| format!("Template render error for node {}: {}", node_instance.id, e))?;

        Ok(format!("    {}", rendered.replace('\n', "\n    ")))
    }

    // Add a new helper to allow passing template_vars to compile_node
    fn compile_node_with_vars(&self, node_instance: &NodeInstance, graph: &GraphDescription, template_vars: &std::collections::HashMap<String, String>) -> Result<String, String> {
        let node_def = self.node_definitions
            .get(&node_instance.node_type)
            .ok_or_else(|| format!("Node definition not found: {}", node_instance.node_type))?;
        let template = self.templates
            .get(&node_instance.node_type)
            .ok_or_else(|| format!("Node template not found: {}", node_instance.node_type))?;
        let mut vars = template_vars.clone();
        // Set input pins
        for (pin_name, pin) in &node_instance.inputs {
            let variable_name = format!("in_{}_{}", pin_name, pin.data_type);
            if let Some(connected_value) = self.get_connected_value(&pin.connected_to, graph) {
                vars.insert(variable_name, connected_value);
            } else if let Some(property_value) = node_instance.properties.get(pin_name) {
                vars.insert(variable_name, self.property_value_to_string(property_value));
            } else {
                if let Some(input_def) = node_def.inputs.iter().find(|i| i.name == *pin_name) {
                    if let Some(default) = &input_def.default_value {
                        vars.insert(variable_name, default.clone());
                    } else {
                        return Err(format!("No value provided for input pin: {}", pin_name));
                    }
                }
            }
        }
        // Set execution pins
        for (pin_name, pin) in &node_instance.outputs {
            if pin.data_type == "execution" {
                let exec_var = format!("pulsar_exec_{}", pin_name);
                let exec_code = self.get_execution_chain(&pin.connected_to, graph)?;
                vars.insert(exec_var, exec_code);
            }
        }
        let mut template_clone = template.clone();
        for (key, value) in vars {
            template_clone.set(&key, &value);
        }
        let rendered = template_clone.render()
            .map_err(|e| format!("Template render error for node {}: {}", node_instance.id, e))?;
        Ok(rendered)
    }

    fn get_connected_value(&self, connection_ids: &[String], graph: &GraphDescription) -> Option<String> {
        // For now, return a placeholder that references the output of the connected node
        if let Some(connection_id) = connection_ids.first() {
            if let Some(connection) = graph.connections.iter().find(|c| c.id == *connection_id) {
                if matches!(connection.connection_type, ConnectionType::Data) {
                    // Generate a variable reference to the output of the source node
                    return Some(format!(
                        "node_{}_{}",
                        connection.source_node,
                        connection.source_pin
                    ));
                }
            }
        }
        None
    }

    fn get_execution_chain(&self, connection_ids: &[String], graph: &GraphDescription) -> Result<String, String> {
        let mut exec_code = String::new();

        for connection_id in connection_ids {
            if let Some(connection) = graph.connections.iter().find(|c| c.id == *connection_id) {
                if matches!(connection.connection_type, ConnectionType::Execution) {
                    if let Some(target_node) = graph.nodes.get(&connection.target_node) {
                        // Generate a call to the target node's function with args
                        let args = self.get_instance_args(target_node, graph)?;
                        exec_code.push_str(&format!("{}({});\n", target_node.node_type, args));
                    }
                }
            }
        }

        Ok(exec_code)
    }

    fn get_instance_args(&self, node_instance: &NodeInstance, graph: &GraphDescription) -> Result<String, String> {
        let node_def = self.node_definitions
            .get(&node_instance.node_type)
            .ok_or_else(|| format!("Node definition not found: {}", node_instance.node_type))?;

        let mut args = Vec::new();
        for input_def in &node_def.inputs {
            let pin_name = &input_def.name;
            let variable_name = format!("in_{}_{}", pin_name, input_def.data_type);

            if let Some(pin) = node_instance.inputs.get(pin_name) {
                if let Some(connected_value) = self.get_connected_value(&pin.connected_to, graph) {
                    args.push(connected_value);
                } else if let Some(property_value) = node_instance.properties.get(pin_name) {
                    args.push(self.property_value_to_string(property_value));
                } else {
                    // Use default value
                    if let Some(default) = &input_def.default_value {
                        args.push(default.clone());
                    } else {
                        return Err(format!("No value provided for input pin: {}", pin_name));
                    }
                }
            } else {
                return Err(format!("Input pin not found: {}", pin_name));
            }
        }

        Ok(args.join(", "))
    }


    fn property_value_to_string(&self, value: &PropertyValue) -> String {
        match value {
            PropertyValue::String(s) => format!("\"{}\"", s.replace('"', "\\\"")),
            PropertyValue::Number(n) => n.to_string(),
            PropertyValue::Boolean(b) => b.to_string(),
            PropertyValue::Vector2(x, y) => format!("({}, {})", x, y),
            PropertyValue::Vector3(x, y, z) => format!("({}, {}, {})", x, y, z),
            PropertyValue::Color(r, g, b, a) => format!("({}, {}, {}, {})", r, g, b, a),
        }
    }
}

// Helper function to create a compiler with loaded data
pub fn create_graph_compiler() -> Result<GraphCompiler, String> {
    let node_definitions = crate::compiler::load_all_node_definitions()?;

    // Convert node definitions to templates
    let mut templates = HashMap::new();
    for (key, node_def) in &node_definitions {
        match TronTemplate::new(&node_def.template_content) {
            Ok(template) => {
                templates.insert(key.clone(), template);
            }
            Err(e) => {
                eprintln!("Failed to create template for {}: {}", key, e);
            }
        }
    }

    Ok(GraphCompiler::new(node_definitions, templates))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::{Position, GraphMetadata};

    #[test]
    fn test_simple_graph_compilation() {
        // Create a simple graph with a println node
        let mut graph = GraphDescription {
            nodes: HashMap::new(),
            connections: Vec::new(),
            metadata: GraphMetadata {
                name: "Test Graph".to_string(),
                description: "Test".to_string(),
                version: "1.0.0".to_string(),
                created_at: "2024-01-01T00:00:00Z".to_string(),
                modified_at: "2024-01-01T00:00:00Z".to_string(),
            },
        };

        let mut println_node = NodeInstance::new("node1", "println", Position { x: 0.0, y: 0.0 });
        println_node.set_property("message", PropertyValue::String("Hello, World!".to_string()));

        graph.add_node(println_node);

        // Test compilation (would need actual templates loaded)
        // This is just a structure test
        assert_eq!(graph.nodes.len(), 1);
    }
}

// Helper to robustly extract function name and definition from a Rust code block
fn extract_fn_name(code: &str) -> Option<String> {
    // Find the first line starting with 'fn '
    for line in code.lines() {
        let line = line.trim_start();
        if line.starts_with("fn ") {
            let after_fn = &line[3..];
            if let Some(paren_idx) = after_fn.find('(') {
                let name = &after_fn[..paren_idx].trim();
                let name = name.split_whitespace().last().unwrap_or("");
                if !name.is_empty() {
                    return Some(name.to_string());
                }
            }
        }
    }
    None
}

// Helper to robustly extract the full function definition from a Rust code block
fn extract_fn_block(code: &str) -> Option<String> {
    let mut in_fn = false;
    let mut brace_count = 0;
    let mut fn_lines = Vec::new();
    for line in code.lines() {
        let line_trimmed = line.trim_start();
        if !in_fn && line_trimmed.starts_with("fn ") {
            in_fn = true;
        }
        if in_fn {
            fn_lines.push(line);
            brace_count += line.chars().filter(|&c| c == '{').count();
            brace_count -= line.chars().filter(|&c| c == '}').count();
            if brace_count == 0 && in_fn {
                break;
            }
        }
    }
    if !fn_lines.is_empty() {
        Some(fn_lines.join("\n"))
    } else {
        None
    }
}