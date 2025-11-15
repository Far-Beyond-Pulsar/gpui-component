//! # Legacy Compiler Code
//!
//! Deprecated code from the old .tron template-based compiler system.
//!
//! This module is kept for reference and backward compatibility but should
//! not be used for new code. The new macro-based compiler in the `core` module
//! is the preferred approach.
//!
//! ## Old System (.tron templates)
//!
//! The original compiler used template files with placeholders:
//! - Nodes defined in .tron files with `@[placeholder]@` syntax
//! - Compiler performed string substitution
//! - Error-prone and hard to maintain
//! - No compile-time validation
//!
//! ## New System (#[blueprint] macros)
//!
//! The current compiler uses Rust macros:
//! - Nodes defined as Rust functions with `#[blueprint]` attribute
//! - Compile-time validation and type checking
//! - Better IDE support
//! - Easier to maintain and extend
//!
//! ## Migration
//!
//! If you have old .tron files, they should be converted to #[blueprint] functions:
//!
//! ```rust,ignore
//! // Old .tron file
//! // add.tron: @[in_a_number]@ + @[in_b_number]@
//!
//! // New #[blueprint] function
//! #[blueprint(type: NodeTypes::pure, category: "Math")]
//! fn add(a: i64, b: i64) -> i64 {
//!     a + b
//! }
//! ```

#[deprecated(note = "Use the new macro-based compiler instead")]
pub mod node_parser;
