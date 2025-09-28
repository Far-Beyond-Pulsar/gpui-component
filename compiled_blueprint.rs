use std::collections::HashMap;

fn main() {
    // Generated Blueprint Code
    let mut variables: HashMap<String, Box<dyn std::any::Any>> = HashMap::new();

    // Node: print_string
    //! A debug node that prints a string to the console.
    
    /// Print a string to the console for debugging.
    fn node_print_string() {
        println!("[DEBUG] {}", "Hello World!");
    }
    // Node: begin_play
    //! Entry point node. All nodes connected to 'exec_out' will be placed in main().
    
    /// Main entry point for execution.
    fn main() {
            //! A debug node that prints a string to the console.
        
        /// Print a string to the console for debugging.
        fn node_print_string() {
            println!("[DEBUG] {}", "Hello World!");
        }
    
    }
}
