//! # Pulsar Blueprint Macros
//!
//! Procedural macros for defining blueprint nodes in Rust.
//!
//! ## Macros
//!
//! - `#[blueprint]` - Mark a function as a blueprint node
//! - `exec_output!()` - Define execution output points in control flow nodes

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn};

/// Mark a function as a blueprint node.
///
/// # Attributes
///
/// - `type`: Node type - `NodeTypes::pure`, `NodeTypes::fn`, or `NodeTypes::control_flow`
/// - `color`: Optional hex color for the node in the UI (e.g., `"#ff0000"`)
/// - `category`: Optional category for grouping nodes (e.g., `"Math"`)
/// - `docs_path`: Optional path to external documentation
///
/// # Examples
///
/// ## Pure Node
/// ```ignore
/// #[blueprint(type: NodeTypes::pure, category: "Math")]
/// fn add(a: i64, b: i64) -> i64 {
///     a + b
/// }
/// ```
///
/// ## Function Node
/// ```ignore
/// #[blueprint(type: NodeTypes::fn, category: "Debug")]
/// fn print_string(message: String) {
///     println!("[DEBUG] {}", message);
/// }
/// ```
///
/// ## Control Flow Node
/// ```ignore
/// #[blueprint(type: NodeTypes::control_flow, category: "Flow")]
/// fn branch(condition: bool) {
///     if condition {
///         exec_output!("True");
///     } else {
///         exec_output!("False");
///     }
/// }
/// ```
#[proc_macro_attribute]
pub fn blueprint(_args: TokenStream, input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as ItemFn);

    // Note: In syn 2.0, we would parse args differently, but since the compiler
    // will parse the source code directly to extract metadata, we don't need to
    // parse the attributes here. Just pass through the function.

    // The macro simply passes through the function unchanged
    // The compiler will parse the source code to extract metadata
    let expanded = quote! {
        #[allow(dead_code)]
        #input
    };

    TokenStream::from(expanded)
}

/// Mark an execution output point in a control flow node.
///
/// This macro is a marker that gets replaced by the compiler during code generation.
/// It should only be used inside functions marked with `#[blueprint(type: NodeTypes::control_flow)]`.
///
/// # Arguments
///
/// - `label`: String literal identifying this execution output (e.g., `"True"`, `"False"`, `"Body"`)
///
/// # Examples
///
/// ```ignore
/// #[blueprint(type: NodeTypes::control_flow)]
/// fn branch(condition: bool) {
///     if condition {
///         exec_output!("True");  // Nodes connected to "True" pin execute here
///     } else {
///         exec_output!("False"); // Nodes connected to "False" pin execute here
///     }
/// }
/// ```
///
/// # How it works
///
/// During compilation:
/// 1. The compiler finds all `exec_output!()` calls in the function
/// 2. For each call, it looks up which nodes are connected to that output pin
/// 3. It replaces the `exec_output!()` with the generated code for those nodes
///
/// At runtime, this macro does nothing - it's purely a compile-time marker.
#[proc_macro]
pub fn exec_output(input: TokenStream) -> TokenStream {
    let _label = parse_macro_input!(input as syn::LitStr);

    // At runtime, this expands to nothing
    // The compiler will replace it during code generation
    let expanded = quote! {
        // Execution output marker - replaced by compiler
        ()
    };

    TokenStream::from(expanded)
}

/// Documentation attribute for blueprint nodes.
///
/// This attribute is used to add rich documentation to nodes that will be
/// displayed in the visual editor.
///
/// # Example
///
/// ```ignore
/// #[bp_doc("# Add Node")]
/// #[bp_doc("Adds two numbers together.")]
/// #[blueprint(type: NodeTypes::pure)]
/// fn add(a: i64, b: i64) -> i64 {
///     a + b
/// }
/// ```
#[proc_macro_attribute]
pub fn bp_doc(_args: TokenStream, input: TokenStream) -> TokenStream {
    // Just pass through - the compiler will parse this attribute from the source
    input
}

// Note: NodeTypes enum cannot be exported from proc-macro crate.
// It should be defined in pulsar_std instead.
