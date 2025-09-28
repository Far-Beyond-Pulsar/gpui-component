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

        let mut function_defs = String::new();
        let mut main_body = String::new();
        let mut seen_functions = std::collections::HashSet::new();

        // Collect function definitions and calls for main()
        for node_id in &execution_order {
            if let Some(node_instance) = graph.nodes.get(node_id) {
                let node_def = self.node_definitions
                    .get(&node_instance.node_type)
                    .ok_or_else(|| format!("Node definition not found: {}", node_instance.node_type))?;

                let fn_name = format!("{}_{}", node_instance.node_type, node_instance.id.replace("-", "_"));
                let mut template_vars = std::collections::HashMap::new();
                template_vars.insert("pulsar_node_fn_id".to_string(), fn_name.clone());
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

                // Use compile_node with correct template_vars
                let node_code = self.compile_node_with_vars(node_instance, graph, &template_vars)?;
                if let Some(extracted_fn_name) = extract_fn_name(&node_code) {
                    if let Some(fn_block) = extract_fn_block(&node_code) {
                        if !seen_functions.contains(&fn_name) {
                            function_defs.push_str(&format!("{}\n", fn_block));
                            seen_functions.insert(fn_name.clone());
                        }
                        // For execution order nodes, calls are handled by exec chains, don't add here
                    }
                } else {
                    // For statement-based nodes, add to main_body if needed, but since no wrapper, perhaps not
                }
            }
        }

        // Collect pure node function definitions and calls
        for (node_id, node_instance) in &graph.nodes {
            if !execution_order.contains(node_id) {
                if let Some(node_def) = self.node_definitions.get(&node_instance.node_type) {
                    if node_def.is_pure {
                        let fn_name = format!("{}_{}", node_instance.node_type, node_instance.id.replace("-", "_"));
                        let mut template_vars = std::collections::HashMap::new();
                        template_vars.insert("pulsar_node_fn_id".to_string(), fn_name.clone());
                        let node_code = self.compile_node_with_vars(node_instance, graph, &template_vars)?;
                        if let Some(extracted_fn_name) = extract_fn_name(&node_code) {
                            if let Some(fn_block) = extract_fn_block(&node_code) {
                                if !seen_functions.contains(&fn_name) {
                                    function_defs.push_str(&format!("{}\n", fn_block));
                                    seen_functions.insert(fn_name.clone());
                                }
                                main_body.push_str(&format!("    {}();\n", fn_name));
                            }
                        } else {
                            main_body.push_str(&format!("    {}\n", node_code));
                        }
                    }
                }
            }
        }

        // Output all code blocks as-is
        let mut generated_code = String::new();
        generated_code.push_str(&function_defs);
        if !main_body.is_empty() {
            generated_code.push_str(&format!("\nfn main() {{\n{}}}\n", main_body));
        } else {
            generated_code.push_str(&main_body);
        }

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
                        // Generate a call to the target node's function
                        let fn_name = format!("{}_{}", target_node.node_type, target_node.id.replace("-", "_"));
                        exec_code.push_str(&format!("{}();\n", fn_name));
                    }
                }
            }
        }

        Ok(exec_code)
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