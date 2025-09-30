// ============================================================================
// Simple Function Definitions (nodes without execution placeholders)
// ============================================================================

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

//! A node that prints a formatted message with placeholder replacements.
//!
//! This node takes a format string and up to three value strings, replacing placeholders `{0}`, `{1}`, and `{2}` in the format string with the corresponding values.
//! Useful for debugging, logging, or displaying dynamic messages with variable content.
//!
//! # Inputs
//! - `in_format_string`: The format string containing placeholders `{0}`, `{1}`, `{2}`.
//! - `in_value0_string`: The value to replace `{0}`.
//! - `in_value1_string`: The value to replace `{1}`.
//! - `in_value2_string`: The value to replace `{2}`.
//!
//! # Outputs
//! - None. The formatted message is printed to the console.
//!
//! # Example
//! If `in_format_string` is "Hello, {0}! You have {1} new messages.", `in_value0_string` is "Alice", and `in_value1_string` is "5", the output will be:
//! [DEBUG] Hello, Alice! You have 5 new messages.
//!
//! # Notes
//! Only the first three placeholders are supported. Placeholders not present in the format string are ignored. Use this node for simple string formatting in logs or UI.
//
/// Print a formatted message with placeholder replacements.
fn print_formatted() {
    let message = format
        .replace("{0}", &value0)
        .replace("{1}", &value1)
        .replace("{2}", &value2);
    println!("[DEBUG] {}", message);
}


// ============================================================================
// Entry Points (Begin Play, On Tick, etc.)
// ============================================================================

fn main() {
    print_string("Hello World!");
    let handle = std::thread::spawn(|| {
            print_formatted("", "", "", "");
        });
        for i in 0..0 {
            print_formatted("", "", "", "");
        }
        handle
}

