#[bp_doc("# This is a title")]
#[bp_doc("This is some docs")]

#[blueprint(type: NodeTypes::control_flow, docs_path: "./docs/thing.md")]
fn branch(thing: bool) {
    if thing {
       exec_output!("True"); // These dynamically create output exec pins on the node matching names map to the same exec pin on the same node 
    } else {
       exec_output!("False");
   }
}

#[blueprint(type: NodeTypes::fn, color: "#ff0000", category: "utils")]
fn my_node(thing: String) -> String {
    thing
}

// This is a pure fn the fn wrapped is purely for type safety and all contents get inlined into the outer fn
#[blueprint(type: NodeTypes::pure, color: "#ff0000", category: "utils")]
fn add(a: i64, b: i64) -> i64 {
    a + b
}