//! A node that prints a string to the console for debugging.
//!
//! This node outputs the provided string to the console, prefixed with "[DEBUG]".
//! Useful for inspecting values, debugging logic, or tracing execution in your graph.
//!
//! # Inputs
//! - `in_message_string`: The string message to print.
//!
//! # Outputs
//! - None. The message is printed to the console.
//!
//! # Example
//! If `in_message_string` is "Hello, world!", the output will be:
//! [DEBUG] Hello, world!
//!
//! # Notes
//! Use this node for development and troubleshooting. Remove or disable in production code.

/// Print a string to the console for debugging.
fn print_string() {
    println!("[DEBUG] {}", message);
}

fn main() {
    print_string("Hello World!");

}

