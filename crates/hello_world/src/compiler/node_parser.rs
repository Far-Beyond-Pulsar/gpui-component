use std::collections::HashMap;
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeDefinition {
    pub name: String,
    pub description: String,
    pub category: String,
    pub icon: String,
    pub is_pure: bool, // Pure functions don't need execution pins
    pub inputs: Vec<PinDefinition>,
    pub outputs: Vec<PinDefinition>,
    pub execution_inputs: Vec<PinDefinition>,
    pub execution_outputs: Vec<PinDefinition>,
    pub template_content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PinDefinition {
    pub name: String,
    pub data_type: String,
    pub description: String,
    pub default_value: Option<String>,
}

pub struct NodeTemplateParser;

impl NodeTemplateParser {
    pub fn parse_template(template_content: &str, node_name: &str) -> Result<NodeDefinition, String> {
        let mut node_def = NodeDefinition {
            name: Self::format_node_name(node_name),
            description: Self::extract_description(template_content),
            category: Self::infer_category(node_name),
            icon: Self::infer_icon(node_name),
            is_pure: Self::is_pure_function(template_content),
            inputs: Vec::new(),
            outputs: Vec::new(),
            execution_inputs: Vec::new(),
            execution_outputs: Vec::new(),
            template_content: template_content.to_string(),
        };

        // Parse input pins
        node_def.inputs = Self::extract_input_pins(template_content)?;

        // Parse execution pins
        node_def.execution_outputs = Self::extract_execution_outputs(template_content)?;

        // Parse output pins from return type
        node_def.outputs = Self::extract_output_pins(template_content)?;

        // Add execution input if not pure
        if !node_def.is_pure {
            node_def.execution_inputs.push(PinDefinition {
                name: "exec".to_string(),
                data_type: "execution".to_string(),
                description: "Execution input".to_string(),
                default_value: None,
            });
        }

        Ok(node_def)
    }

    fn extract_description(content: &str) -> String {
        // Extract from first comment line
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("//!") {
                return line.trim_start_matches("//!").trim().to_string();
            }
        }
        "No description".to_string()
    }

    fn format_node_name(name: &str) -> String {
        name.replace('_', " ")
            .split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    fn infer_category(node_name: &str) -> String {
        match node_name {
            name if name.starts_with("vector") => "Math/Vector".to_string(),
            name if name.contains("add") || name.contains("multiply") || name.contains("divide")
                 || name.contains("subtract") || name.contains("sin") || name.contains("cos")
                 || name.contains("sqrt") || name.contains("pow") => "Math".to_string(),
            name if name.contains("string") => "String".to_string(),
            name if name.contains("array") => "Array".to_string(),
            name if name.contains("file") || name.contains("dir") => "File I/O".to_string(),
            name if name.contains("loop") || name.contains("branch") || name.contains("and")
                 || name.contains("or") || name.contains("not") => "Logic".to_string(),
            name if name.contains("color") || name.contains("transform") || name.contains("rect") => "Graphics".to_string(),
            name if name.contains("time") || name.contains("delay") || name.contains("timer") => "Time".to_string(),
            name if name.contains("random") => "Utility".to_string(),
            _ => "General".to_string(),
        }
    }

    fn infer_icon(node_name: &str) -> String {
        match node_name {
            name if name.starts_with("vector") => "📐".to_string(),
            name if name.contains("add") => "➕".to_string(),
            name if name.contains("multiply") => "✖️".to_string(),
            name if name.contains("divide") => "➗".to_string(),
            name if name.contains("subtract") => "➖".to_string(),
            name if name.contains("string") => "🔤".to_string(),
            name if name.contains("array") => "📊".to_string(),
            name if name.contains("file") => "📁".to_string(),
            name if name.contains("branch") => "🔀".to_string(),
            name if name.contains("loop") => "🔄".to_string(),
            name if name.contains("println") => "📝".to_string(),
            name if name.contains("color") => "🎨".to_string(),
            name if name.contains("time") => "⏰".to_string(),
            name if name.contains("random") => "🎲".to_string(),
            _ => "⚙️".to_string(),
        }
    }

    fn is_pure_function(content: &str) -> bool {
        // Check if function has a return type
        let return_type_regex = Regex::new(r"fn\s+@\[pulsar_node_fn_id\]@\s*\([^)]*\)\s*->\s*[^{]+").unwrap();
        return_type_regex.is_match(content)
    }

    fn extract_input_pins(content: &str) -> Result<Vec<PinDefinition>, String> {
        let mut pins = Vec::new();
        let input_regex = Regex::new(r"@\[in_([^_]+)_([^]]+)\]@").unwrap();

        for captures in input_regex.captures_iter(content) {
            let name = captures.get(1).unwrap().as_str();
            let data_type = captures.get(2).unwrap().as_str();

            pins.push(PinDefinition {
                name: name.to_string(),
                data_type: Self::normalize_data_type(data_type),
                description: format!("{} input", name),
                default_value: Self::get_default_value(data_type),
            });
        }

        Ok(pins)
    }

    fn extract_execution_outputs(content: &str) -> Result<Vec<PinDefinition>, String> {
        let mut pins = Vec::new();
        let exec_regex = Regex::new(r"@\[pulsar_exec_([^]]+)\]@").unwrap();

        for captures in exec_regex.captures_iter(content) {
            let name = captures.get(1).unwrap().as_str();

            pins.push(PinDefinition {
                name: name.to_string(),
                data_type: "execution".to_string(),
                description: format!("{} execution output", name),
                default_value: None,
            });
        }

        Ok(pins)
    }

    fn extract_output_pins(content: &str) -> Result<Vec<PinDefinition>, String> {
        let mut pins = Vec::new();

        // Parse return type to determine outputs
        let return_type_regex = Regex::new(r"fn\s+@\[pulsar_node_fn_id\]@\s*\([^)]*\)\s*->\s*([^{]+)").unwrap();

        if let Some(captures) = return_type_regex.captures(content) {
            let return_type = captures.get(1).unwrap().as_str().trim();

            // Handle tuple returns
            if return_type.starts_with('(') && return_type.ends_with(')') {
                let inner = &return_type[1..return_type.len()-1];
                let types: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();

                for (i, type_str) in types.iter().enumerate() {
                    pins.push(PinDefinition {
                        name: format!("output_{}", i),
                        data_type: Self::normalize_data_type(type_str),
                        description: format!("Output {}", i),
                        default_value: None,
                    });
                }
            } else {
                // Single return type
                pins.push(PinDefinition {
                    name: "result".to_string(),
                    data_type: Self::normalize_data_type(return_type),
                    description: "Result".to_string(),
                    default_value: None,
                });
            }
        }

        Ok(pins)
    }

    fn normalize_data_type(rust_type: &str) -> String {
        match rust_type.trim() {
            "f32" | "f64" | "i32" | "i64" | "u32" | "u64" | "isize" | "usize" => "number".to_string(),
            "bool" => "boolean".to_string(),
            "String" | "&str" => "string".to_string(),
            "(f32, f32)" => "vector2".to_string(),
            "(f32, f32, f32)" => "vector3".to_string(),
            "(f32, f32, f32, f32)" => "color".to_string(),
            other => other.to_lowercase(),
        }
    }

    fn get_default_value(data_type: &str) -> Option<String> {
        match data_type {
            "number" => Some("0".to_string()),
            "bool" | "boolean" => Some("false".to_string()),
            "string" => Some("\"\"".to_string()),
            _ => None,
        }
    }
}

pub fn load_all_node_definitions() -> Result<HashMap<String, NodeDefinition>, String> {
    let mut definitions = HashMap::new();

    // Load from ./nodes directory
    if let Ok(entries) = std::fs::read_dir("./nodes") {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("tron") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        match std::fs::read_to_string(&path) {
                            Ok(content) => {
                                match NodeTemplateParser::parse_template(&content, name) {
                                    Ok(def) => {
                                        definitions.insert(name.to_string(), def);
                                    }
                                    Err(e) => {
                                        eprintln!("Error parsing template {}: {}", path.display(), e);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Error reading file {}: {}", path.display(), e);
                            }
                        }
                    }
                }
            }
        }
    }

    // Load from ./user_nodes directory
    if let Ok(entries) = std::fs::read_dir("./user_nodes") {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("tron") {
                    if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                        match std::fs::read_to_string(&path) {
                            Ok(content) => {
                                match NodeTemplateParser::parse_template(&content, name) {
                                    Ok(def) => {
                                        definitions.insert(name.to_string(), def);
                                    }
                                    Err(e) => {
                                        eprintln!("Error parsing template {}: {}", path.display(), e);
                                    }
                                }
                            }
                            Err(e) => {
                                eprintln!("Error reading file {}: {}", path.display(), e);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(definitions)
}