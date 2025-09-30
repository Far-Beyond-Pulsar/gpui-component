//! # Template Type Classification
//!
//! Classifies node templates based on their execution characteristics.
//!
//! This determines how a node should be compiled:
//! - **Pure expressions** are used in data flow only
//! - **Simple functions** can be called or inlined
//! - **Control flow** nodes MUST be inlined with their placeholders filled
//!
//! ## Examples
//!
//! ### Pure Expression
//! ```rust
//! @[left]@ + @[right]@  // No function wrapper, direct expression
//! ```
//!
//! ### Simple Function
//! ```rust
//! fn print_string() {
//!     println!("{}", @[message]@);
//! }
//! ```
//!
//! ### Control Flow
//! ```rust
//! fn thread_spawn() {
//!     let handle = std::thread::spawn(|| {
//!         @[pulsar_exec_body]@  // Execution placeholder
//!     });
//!     @[pulsar_exec_continue]@  // Another execution placeholder
//!     handle
//! }
//! ```

/// Classification of node templates based on their execution characteristics.
#[derive(Debug, Clone)]
pub(super) enum TemplateType {
    /// Pure expression like `a + b` - no function wrapper, just Rust expression.
    ///
    /// These nodes can only be used in data flow, never in execution flow.
    PureExpression,

    /// Function with no execution placeholders - can be called as a function.
    ///
    /// These nodes have a single execution input and single execution output.
    /// They can be called like normal functions and chained together.
    SimpleFunction,

    /// Has execution placeholders - MUST be inlined into execution flow.
    ///
    /// These nodes have multiple execution outputs (like `then` and `else` for a branch)
    /// and must be expanded inline with placeholders filled by connected nodes.
    ControlFlow {
        /// Names of execution placeholders (e.g., ["pulsar_exec_body", "pulsar_exec_continue"])
        exec_placeholders: Vec<String>,
    },
}