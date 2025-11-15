//! # Function Node Code Generation
//!
//! Strategy for generating code from function nodes (side effects with linear exec flow).
//!
//! Function nodes have:
//! - Side effects (I/O, state changes, etc.)
//! - One exec input, one exec output
//! - Linear execution flow
//!
//! ## Generation Strategy
//!
//! Function nodes are generated as **sequential statements** in the execution chain.
//!
//! ## Example
//!
//! Graph: `begin_play -> print("A") -> print("B") -> print("C")`
//!
//! Generated:
//! ```rust,ignore
//! pub fn main() {
//!     print_string("A".to_string());
//!     print_string("B".to_string());
//!     print_string("C".to_string());
//! }
//! ```

// This module's functionality is currently in code_generator.rs
// Future refactoring can extract it here

pub use super::code_generator::*;
