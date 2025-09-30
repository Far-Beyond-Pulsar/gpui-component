// Auto-generated code from Pulsar Blueprint
// DO NOT EDIT - Changes will be overwritten

use pulsar_std::*;

pub fn main() {
    // Pure node evaluations
    let node_add_node_result = add(2, 3);
    let node_greater_node_result = greater_than(node_add_node_result, 3);

    // Execution chain
    if node_greater_node_result { print_string ("Result is greater than 3! \u{2713}") ; } else { print_string ("Result is 3 or less. \u{2717}") ; }
}

