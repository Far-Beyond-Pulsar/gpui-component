//! # Pulsar Standard Library
//!
//! Built-in blueprint nodes for the Pulsar visual programming system.
//!
//! All nodes are defined as Rust functions with the `#[blueprint]` attribute macro.
//!
//! ## Node Categories
//!
//! - **Math**: Arithmetic operations (add, subtract, multiply, divide)
//! - **Logic**: Boolean operations and control flow
//! - **Debug**: Debugging utilities (print, inspect)
//! - **Flow**: Execution flow control (branch, sequence)
//! - **Events**: Entry points (begin_play, on_tick)

// Re-export macros
pub use pulsar_macros::*;

// =============================================================================
// Node Type Enum (for blueprint attribute)
// =============================================================================

/// Node type for the `#[blueprint(type: ...)]` attribute
#[allow(non_camel_case_types)]
pub enum NodeTypes {
    /// Pure function: no side effects, no exec pins, only data flow
    pure,

    /// Function with side effects: one exec in, one exec out
    fn_,

    /// Control flow: one exec in, multiple exec outs via exec_output!()
    control_flow,

    /// Event: defines an entry point function (e.g., main, begin_play)
    /// Events define the outer function signature and have exec_output!("Body")
    event,
}

// =============================================================================
// Entry Points / Event Definitions
// =============================================================================

/// Main entry point - defines the standard Rust main() function.
///
/// This event node defines the outer main() function. The execution chain
/// connected to the "Body" output pin will become the function body.
#[bp_doc("# Main")]
#[bp_doc("Defines the default Rust entry point `fn main()`.")]
#[blueprint(type: NodeTypes::event, category: "Events")]
pub fn main() {
    exec_output!("Body");
}

/// Begin Play event - runs when the program/engine starts.
///
/// This is typically used in game/engine contexts as an initialization point.
#[bp_doc("# Begin Play")]
#[bp_doc("Entry point that executes when the engine starts.")]
#[blueprint(type: NodeTypes::event, category: "Events")]
pub fn begin_play() {
    exec_output!("Body");
}

// =============================================================================
// Math - Pure Nodes
// =============================================================================

/// Add two numbers together.
///
/// # Inputs
/// - `a`: First number (i64)
/// - `b`: Second number (i64)
///
/// # Returns
/// The sum of a and b
#[bp_doc("# Add")]
#[bp_doc("Adds two numbers together and returns the result.")]
#[blueprint(type: NodeTypes::pure, category: "Math", color: "#4A90E2")]
pub fn add(a: i64, b: i64) -> i64 {
    a + b
}

/// Subtract one number from another.
#[bp_doc("# Subtract")]
#[bp_doc("Subtracts b from a and returns the result.")]
#[blueprint(type: NodeTypes::pure, category: "Math", color: "#4A90E2")]
pub fn subtract(a: i64, b: i64) -> i64 {
    a - b
}

/// Multiply two numbers together.
#[bp_doc("# Multiply")]
#[bp_doc("Multiplies two numbers together and returns the result.")]
#[blueprint(type: NodeTypes::pure, category: "Math", color: "#4A90E2")]
pub fn multiply(a: i64, b: i64) -> i64 {
    a * b
}

/// Divide one number by another.
#[bp_doc("# Divide")]
#[bp_doc("Divides a by b and returns the result. Returns 0 if b is 0.")]
#[blueprint(type: NodeTypes::pure, category: "Math", color: "#4A90E2")]
pub fn divide(a: i64, b: i64) -> i64 {
    if b == 0 {
        0
    } else {
        a / b
    }
}

/// Add two floating point numbers.
#[bp_doc("# Add Float")]
#[bp_doc("Adds two floating point numbers and returns the result.")]
#[blueprint(type: NodeTypes::pure, category: "Math", color: "#4A90E2")]
pub fn add_float(a: f64, b: f64) -> f64 {
    a + b
}

// =============================================================================
// Logic - Pure Nodes
// =============================================================================

/// Logical AND operation.
#[bp_doc("# And")]
#[bp_doc("Returns true if both inputs are true.")]
#[blueprint(type: NodeTypes::pure, category: "Logic", color: "#E2A04A")]
pub fn and(a: bool, b: bool) -> bool {
    a && b
}

/// Logical OR operation.
#[bp_doc("# Or")]
#[bp_doc("Returns true if either input is true.")]
#[blueprint(type: NodeTypes::pure, category: "Logic", color: "#E2A04A")]
pub fn or(a: bool, b: bool) -> bool {
    a || b
}

/// Logical NOT operation.
#[bp_doc("# Not")]
#[bp_doc("Returns the opposite boolean value.")]
#[blueprint(type: NodeTypes::pure, category: "Logic", color: "#E2A04A")]
pub fn not(value: bool) -> bool {
    !value
}

/// Check if two numbers are equal.
#[bp_doc("# Equals")]
#[bp_doc("Returns true if the two numbers are equal.")]
#[blueprint(type: NodeTypes::pure, category: "Logic", color: "#E2A04A")]
pub fn equals(a: i64, b: i64) -> bool {
    a == b
}

/// Check if a is greater than b.
#[bp_doc("# Greater Than")]
#[bp_doc("Returns true if a is greater than b.")]
#[blueprint(type: NodeTypes::pure, category: "Logic", color: "#E2A04A")]
pub fn greater_than(a: i64, b: i64) -> bool {
    a > b
}

/// Check if a is less than b.
#[bp_doc("# Less Than")]
#[bp_doc("Returns true if a is less than b.")]
#[blueprint(type: NodeTypes::pure, category: "Logic", color: "#E2A04A")]
pub fn less_than(a: i64, b: i64) -> bool {
    a < b
}

// =============================================================================
// Debug - Function Nodes
// =============================================================================

/// Print a string to the console for debugging.
///
/// # Inputs
/// - `message`: The string message to print
#[bp_doc("# Print String")]
#[bp_doc("Prints a string to the console with [DEBUG] prefix.")]
#[blueprint(type: NodeTypes::fn_, category: "Debug", color: "#7ED321")]
pub fn print_string(message: String) {
    println!("[DEBUG] {}", message);
}

/// Print a number to the console.
#[bp_doc("# Print Number")]
#[bp_doc("Prints a number to the console with [DEBUG] prefix.")]
#[blueprint(type: NodeTypes::fn_, category: "Debug", color: "#7ED321")]
pub fn print_number(value: i64) {
    println!("[DEBUG] {}", value);
}

/// Print a boolean to the console.
#[bp_doc("# Print Boolean")]
#[bp_doc("Prints a boolean value to the console.")]
#[blueprint(type: NodeTypes::fn_, category: "Debug", color: "#7ED321")]
pub fn print_bool(value: bool) {
    println!("[DEBUG] {}", value);
}

/// Print a formatted string to the console.
#[bp_doc("# Print Formatted")]
#[bp_doc("Prints a formatted string with a number value.")]
#[blueprint(type: NodeTypes::fn_, category: "Debug", color: "#7ED321")]
pub fn print_formatted(label: String, value: i64) {
    println!("[DEBUG] {}: {}", label, value);
}

// =============================================================================
// Flow Control - Control Flow Nodes
// =============================================================================

/// Branch execution based on a boolean condition.
///
/// # Inputs
/// - `condition`: Boolean value to test
///
/// # Execution Outputs
/// - `True`: Executes if condition is true
/// - `False`: Executes if condition is false
#[bp_doc("# Branch")]
#[bp_doc("Routes execution based on a boolean condition.")]
#[bp_doc("If the condition is true, the True pin executes. Otherwise, the False pin executes.")]
#[blueprint(type: NodeTypes::control_flow, category: "Flow", color: "#BD10E0")]
pub fn branch(condition: bool) {
    if condition {
        exec_output!("True");
    } else {
        exec_output!("False");
    }
}

/// Execute a sequence of nodes in order.
///
/// # Execution Outputs
/// - `Then_0`: First node to execute
/// - `Then_1`: Second node to execute
/// - `Then_2`: Third node to execute
#[bp_doc("# Sequence")]
#[bp_doc("Executes multiple execution pins in order.")]
#[blueprint(type: NodeTypes::control_flow, category: "Flow", color: "#BD10E0")]
pub fn sequence() {
    exec_output!("Then_0");
    exec_output!("Then_1");
    exec_output!("Then_2");
}

/// Multi-way branch based on an integer value.
///
/// # Inputs
/// - `value`: Integer value to switch on
///
/// # Execution Outputs
/// - `Case_0`: Executes if value is 0
/// - `Case_1`: Executes if value is 1
/// - `Case_2`: Executes if value is 2
/// - `Default`: Executes for any other value
#[bp_doc("# Switch on Int")]
#[bp_doc("Routes execution based on an integer value.")]
#[blueprint(type: NodeTypes::control_flow, category: "Flow", color: "#BD10E0")]
pub fn switch_on_int(value: i64) {
    match value {
        0 => exec_output!("Case_0"),
        1 => exec_output!("Case_1"),
        2 => exec_output!("Case_2"),
        _ => exec_output!("Default"),
    }
}

/// Execute a node only once, then ignore subsequent calls.
///
/// # Execution Outputs
/// - `Once`: Executes only on the first call
/// - `After`: Always executes after checking
#[bp_doc("# Do Once")]
#[bp_doc("Executes the connected nodes only once, then blocks further execution.")]
#[blueprint(type: NodeTypes::control_flow, category: "Flow", color: "#BD10E0")]
pub fn do_once() {
    // Note: This is a simplified version
    // A real implementation would need static state to track execution
    static mut HAS_EXECUTED: bool = false;

    unsafe {
        if !HAS_EXECUTED {
            HAS_EXECUTED = true;
            exec_output!("Once");
        }
        exec_output!("After");
    }
}

/// Execute nodes N times.
///
/// # Inputs
/// - `count`: Number of times to execute the loop body
///
/// # Execution Outputs
/// - `Body`: Executes for each iteration
/// - `Completed`: Executes after all iterations complete
#[bp_doc("# For Loop")]
#[bp_doc("Executes the body a specified number of times.")]
#[blueprint(type: NodeTypes::control_flow, category: "Flow", color: "#BD10E0")]
pub fn for_loop(count: i64) {
    for _i in 0..count {
        exec_output!("Body");
    }
    exec_output!("Completed");
}

/// Gate that can block or allow execution.
///
/// # Inputs
/// - `open`: Whether the gate is open
///
/// # Execution Outputs
/// - `Open`: Executes if gate is open
/// - `Closed`: Executes if gate is closed
#[bp_doc("# Gate")]
#[bp_doc("A gate that can block or allow execution flow.")]
#[blueprint(type: NodeTypes::control_flow, category: "Flow", color: "#BD10E0")]
pub fn gate(open: bool) {
    if open {
        exec_output!("Open");
    } else {
        exec_output!("Closed");
    }
}

// =============================================================================
// String Operations - Pure Nodes
// =============================================================================

/// Concatenate two strings.
#[bp_doc("# String Concat")]
#[bp_doc("Joins two strings together.")]
#[blueprint(type: NodeTypes::pure, category: "String", color: "#7ED321")]
pub fn string_concat(a: String, b: String) -> String {
    format!("{}{}", a, b)
}

/// Get the length of a string.
#[bp_doc("# String Length")]
#[bp_doc("Returns the number of characters in a string.")]
#[blueprint(type: NodeTypes::pure, category: "String", color: "#7ED321")]
pub fn string_length(s: String) -> i64 {
    s.len() as i64
}

/// Convert a number to a string.
#[bp_doc("# Number to String")]
#[bp_doc("Converts a number to its string representation.")]
#[blueprint(type: NodeTypes::pure, category: "String", color: "#7ED321")]
pub fn number_to_string(value: i64) -> String {
    value.to_string()
}

/// Check if a string contains a substring.
#[bp_doc("# String Contains")]
#[bp_doc("Returns true if the string contains the specified substring.")]
#[blueprint(type: NodeTypes::pure, category: "String", color: "#7ED321")]
pub fn string_contains(haystack: String, needle: String) -> bool {
    haystack.contains(&needle)
}

// =============================================================================
// Conversion Nodes
// =============================================================================

/// Convert an integer to a float.
#[bp_doc("# Int to Float")]
#[bp_doc("Converts an integer to a floating point number.")]
#[blueprint(type: NodeTypes::pure, category: "Conversion")]
pub fn int_to_float(value: i64) -> f64 {
    value as f64
}

/// Convert a float to an integer (truncating).
#[bp_doc("# Float to Int")]
#[bp_doc("Converts a float to an integer by truncating the decimal part.")]
#[blueprint(type: NodeTypes::pure, category: "Conversion")]
pub fn float_to_int(value: f64) -> i64 {
    value as i64
}

/// Convert a boolean to a string.
#[bp_doc("# Bool to String")]
#[bp_doc("Converts a boolean to its string representation.")]
#[blueprint(type: NodeTypes::pure, category: "Conversion")]
pub fn bool_to_string(value: bool) -> String {
    value.to_string()
}
