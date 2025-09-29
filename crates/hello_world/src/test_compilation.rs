use crate::graph::{GraphDescription, NodeInstance, Connection, ConnectionType, PropertyValue, Position};
use crate::compiler::create_graph_compiler;

/// Test function to verify the fixed compilation
pub fn test_blueprint_compilation() -> Result<(), String> {
    println!("Testing blueprint compilation...");

    // Load the blueprint.json file
    let blueprint_content = std::fs::read_to_string("../../blueprint.json")
        .map_err(|e| format!("Failed to read blueprint.json: {}", e))?;

    // Parse the blueprint JSON
    let blueprint: serde_json::Value = serde_json::from_str(&blueprint_content)
        .map_err(|e| format!("Failed to parse blueprint JSON: {}", e))?;

    // Convert to graph description (simplified version)
    let mut graph = GraphDescription::new("Test Blueprint");

    // Add a simple begin_play node
    let mut begin_play = NodeInstance::new("begin_play", "begin_play", Position { x: 100.0, y: 100.0 });
    graph.add_node(begin_play);

    // Add one connected print_string node
    let mut print_node = NodeInstance::new("print_1", "print_string", Position { x: 400.0, y: 100.0 });
    print_node.set_property("message", PropertyValue::String("Hello World!".to_string()));
    graph.add_node(print_node);

    // Add two disconnected print_string nodes (like the ones causing issues)
    let mut disconnected_1 = NodeInstance::new("disconnected_1", "print_string", Position { x: 400.0, y: 300.0 });
    graph.add_node(disconnected_1);

    let mut disconnected_2 = NodeInstance::new("disconnected_2", "print_string", Position { x: 400.0, y: 500.0 });
    graph.add_node(disconnected_2);

    // Create connections
    let connection = Connection {
        id: "conn_1".to_string(),
        source_node: "begin_play".to_string(),
        source_pin: "exec_out".to_string(),
        target_node: "print_1".to_string(),
        target_pin: "exec_in".to_string(),
        connection_type: ConnectionType::Execution,
    };
    graph.add_connection(connection);

    // Create compiler and compile
    let compiler = create_graph_compiler()?;
    let result = compiler.compile_graph(&graph)?;

    println!("Compilation successful!");
    println!("Generated code:\n{}", result);

    // Check that we don't have duplicate function definitions
    let function_count = result.matches("fn print_string(").count();
    println!("Found {} print_string function definitions", function_count);

    if function_count > 1 {
        return Err("Still generating duplicate function definitions!".to_string());
    }

    // Write the result to a test file
    std::fs::write("test_compiled_blueprint.rs", &result)
        .map_err(|e| format!("Failed to write test output: {}", e))?;

    println!("Test compilation completed successfully - no duplicate functions!");
    Ok(())
}