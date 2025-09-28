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
        let execution_order = graph.get_execution_order()?;

        // Generate calls for execution order
        let mut exec_calls = String::new();
        for node_id in &execution_order {
            if let Some(node_instance) = graph.nodes.get(node_id) {
                if node_instance.node_type == "begin_play" {
                    continue; // begin_play is the main function itself
                }
                let args = self.get_instance_args(node_instance, graph)?;
                exec_calls.push_str(&format!("{}({});\n", node_instance.node_type, args));
            }
        }

        let mut function_defs = String::new();
        let mut seen_types = std::collections::HashSet::new();

        // Collect unique node types and generate function definitions
        for node_id in &execution_order {
            if let Some(node_instance) = graph.nodes.get(node_id) {
                let node_type = &node_instance.node_type;
                if !seen_types.contains(node_type) {
                    seen_types.insert(node_type.clone());
                    let node_def = self.node_definitions
                        .get(node_type)
                        .ok_or_else(|| format!("Node definition not found: {}", node_type))?;
                    let mut template_vars = std::collections::HashMap::new();
                    let params = node_def.inputs.iter().map(|i| format!("{}: {}", i.name, Self::rust_type(&i.data_type))).collect::<Vec<_>>().join(", ");
                    if node_type == "begin_play" {
                        template_vars.insert("pulsar_node_fn_id".to_string(), "main".to_string());
                        template_vars.insert("pulsar_exec_exec_out".to_string(), exec_calls.clone());
                    } else {
                        if params.is_empty() {
                            template_vars.insert("pulsar_node_fn_id".to_string(), node_type.clone());
                        } else {
                            template_vars.insert("pulsar_node_fn_id".to_string(), format!("{}({})", node_type, params));
                        }
                        for input in &node_def.inputs {
                            let var_name = format!("in_{}_{}", input.name, input.data_type);
                            template_vars.insert(var_name, input.name.clone());
                        }
                    }
                    let template = self.templates
                        .get(node_type)
                        .ok_or_else(|| format!("Node template not found: {}", node_type))?;
                    let mut template_clone = template.clone();
                    for (key, value) in &template_vars {
                        template_clone.set(key, value);
                    }
                    let rendered = template_clone.render()
                        .map_err(|e| format!("Template render error for node {}: {}", node_instance.id, e))?;
                    function_defs.push_str(&format!("{}\n", rendered));
                }
            }
        }

        // For pure nodes
        for (node_id, node_instance) in &graph.nodes {
            if !execution_order.contains(node_id) {
                let node_type = &node_instance.node_type;
                if let Some(node_def) = self.node_definitions.get(node_type) {
                    if node_def.is_pure && !seen_types.contains(node_type) {
                        seen_types.insert(node_type.clone());
                        let mut template_vars = std::collections::HashMap::new();
                        let params = node_def.inputs.iter().map(|i| format!("{}: {}", i.name, Self::rust_type(&i.data_type))).collect::<Vec<_>>().join(", ");
                        if params.is_empty() {
                            template_vars.insert("pulsar_node_fn_id".to_string(), node_type.clone());
                        } else {
                            template_vars.insert("pulsar_node_fn_id".to_string(), format!("{}({})", node_type, params));
                        }
                        for input in &node_def.inputs {
                            let var_name = format!("in_{}_{}", input.name, input.data_type);
                            template_vars.insert(var_name, input.name.clone());
                        }
                        let template = self.templates
                            .get(node_type)
                            .ok_or_else(|| format!("Node template not found: {}", node_type))?;
                        let mut template_clone = template.clone();
                        for (key, value) in &template_vars {
                            template_clone.set(key, value);
                        }
                        let rendered = template_clone.render()
                            .map_err(|e| format!("Template render error for node {}: {}", node_instance.id, e))?;
                        function_defs.push_str(&format!("{}\n", rendered));
                    }
                }
            }
        }

        // For pure nodes, generate calls
        for (node_id, node_instance) in &graph.nodes {
            if !execution_order.contains(node_id) {
                if let Some(node_def) = self.node_definitions.get(&node_instance.node_type) {
                    if node_def.is_pure {
                        let args = self.get_instance_args(node_instance, graph)?;
                        exec_calls.push_str(&format!("{}({});\n", node_instance.node_type, args));
                    }
                }
            }
        }

        // Output function definitions
        let mut generated_code = String::new();
        generated_code.push_str(&function_defs);

        Ok(generated_code)
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

    fn rust_type(data_type: &str) -> &'static str {
        match data_type {
            "string" => "String",
            "number" => "f64",
            "boolean" => "bool",
            "vector2" => "(f64, f64)",
            "vector3" => "(f64, f64, f64)",
            "color" => "(f64, f64, f64, f64)",
            "rect" => "Rect", // assume defined
            "transform" => "Transform",
            _ => "String",
        }
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