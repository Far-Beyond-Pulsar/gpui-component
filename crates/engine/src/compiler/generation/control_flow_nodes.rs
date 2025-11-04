//! # Control Flow Node Code Generation
//!
//! Strategy for generating code from control flow nodes (branching execution).
//!
//! Control flow nodes have:
//! - Multiple exec outputs (e.g., True/False for branch)
//! - `exec_output!("Label")` calls in their body
//! - Must be inlined (cannot be called as functions)
//!
//! ## Generation Strategy
//!
//! Control flow nodes must be **inlined** with transformation:
//! 1. Parse the node's function body as AST
//! 2. Replace each `exec_output!("Label")` with code from connected nodes
//! 3. Substitute parameter values with actual expressions
//! 4. Inline the transformed body
//!
//! ## Example
//!
//! Graph: `branch(x > 5) -> [True: print("big"), False: print("small")]`
//!
//! Generated:
//! ```rust,ignore
//! if x > 5 {
//!     print_string("big".to_string());
//! } else {
//!     print_string("small".to_string());
//! }
//! ```

// This module's functionality is currently in code_generator.rs
// Future refactoring can extract it here

pub use super::code_generator::*;
