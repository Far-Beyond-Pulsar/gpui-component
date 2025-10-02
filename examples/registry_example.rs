/// Example demonstrating the new blueprint node registry system
///
/// Run with: cargo run --example registry_example

use pulsar_std::{get_all_nodes, get_nodes_by_category, get_all_categories};

fn main() {
    println!("=== Blueprint Node Registry Example ===\n");

    // Get all registered nodes
    let all_nodes = get_all_nodes();
    println!("Total registered nodes: {}", all_nodes.len());

    // List all categories
    let categories = get_all_categories();
    println!("\nAvailable categories:");
    for category in &categories {
        println!("  - {}", category);
    }

    // Show nodes by category
    println!("\n=== Nodes by Category ===");
    for category in &categories {
        let nodes = get_nodes_by_category(category);
        println!("\n{}:", category);
        for node in nodes {
            let params: Vec<String> = node.params.iter()
                .map(|p| format!("{}: {}", p.name, p.ty))
                .collect();

            let return_type = node.return_type.unwrap_or("()");

            println!("  - {}({}) -> {}",
                node.name,
                params.join(", "),
                return_type
            );
        }
    }

    // Show some specific examples
    println!("\n=== Example Nodes ===");

    if let Some(node) = pulsar_std::get_node_by_name("add") {
        println!("\n'add' node:");
        println!("  Category: {}", node.category);
        println!("  Type: {:?}", node.node_type);
        println!("  Color: {:?}", node.color);
        println!("  Params: {:?}", node.params);
        println!("  Returns: {:?}", node.return_type);
    }

    if let Some(node) = pulsar_std::get_node_by_name("branch") {
        println!("\n'branch' node:");
        println!("  Category: {}", node.category);
        println!("  Type: {:?}", node.node_type);
        println!("  Exec inputs: {:?}", node.exec_inputs);
        println!("  Exec outputs: {:?}", node.exec_outputs);
        println!("  Documentation: {}", node.documentation.join(" "));
    }
}
