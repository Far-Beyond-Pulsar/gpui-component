fn main() {
    let nodes = pulsar_std::get_all_nodes();

    // Find branch nodes
    for node in nodes.iter().filter(|n| n.name == "branch") {
        println!("=== Branch Node ===");
        println!("Name: {}", node.name);
        println!("Node Type: {:?}", node.node_type);
        println!("Category: {}", node.category);
        println!("Exec Outputs: {:?}", node.exec_outputs);
        println!("\nFunction Source ({} bytes):", node.function_source.len());
        println!("{}", node.function_source);
        println!("\n");
    }
}
