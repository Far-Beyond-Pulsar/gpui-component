fn main() {
    let nodes = pulsar_std::get_all_nodes();

    // Find the "add" node as an example
    if let Some(add_node) = nodes.iter().find(|n| n.name == "add") {
        println!("=== Node: {} ===", add_node.name);
        println!("Category: {}", add_node.category);
        println!("Color: {:?}", add_node.color);
        println!("Documentation ({} lines):", add_node.documentation.len());
        for (i, doc) in add_node.documentation.iter().enumerate() {
            println!("  [{}] {}", i, doc);
        }
    } else {
        println!("Could not find 'add' node");
    }

    println!("\n=== Total nodes registered: {} ===", nodes.len());

    // Show first 5 nodes with their doc counts
    for node in nodes.iter().take(5) {
        println!("{}: {} docs, color: {:?}",
            node.name,
            node.documentation.len(),
            node.color
        );
    }
}
