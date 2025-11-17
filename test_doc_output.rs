// Simple test to check documentation output
use pulsar_std::{get_node_by_name};

fn main() {
    // Test 'add' node documentation
    if let Some(add_node) = get_node_by_name("add") {
        println!("=== ADD NODE DOCUMENTATION ===");
        for (i, line) in add_node.documentation.iter().enumerate() {
            println!("{}: {}", i, line);
        }
        println!("\n");
    }

    // Test 'branch' node documentation
    if let Some(branch_node) = get_node_by_name("branch") {
        println!("=== BRANCH NODE DOCUMENTATION ===");
        for (i, line) in branch_node.documentation.iter().enumerate() {
            println!("{}: {}", i, line);
        }
        println!("\n");
    }

    // Test 'print_string' node documentation
    if let Some(print_node) = get_node_by_name("print_string") {
        println!("=== PRINT_STRING NODE DOCUMENTATION ===");
        for (i, line) in print_node.documentation.iter().enumerate() {
            println!("{}: {}", i, line);
        }
    }
}
