//! # Pure Node Code Generation
//!
//! Strategy for generating code from pure nodes (no side effects, no exec pins).
//!
//! Pure nodes are mathematical or data transformation functions that:
//! - Take inputs and produce outputs
//! - Have no side effects
//! - Can be safely inlined as expressions
//!
//! ## Generation Strategy
//!
//! Pure nodes are generated as **inline expressions** where they're used.
//! This eliminates function call overhead and enables compiler optimizations.
//!
//! ## Example
//!
//! Graph: `multiply(add(2, 3), 4)`
//!
//! Generated: `let result = multiply(add(2, 3), 4);`
//!
//! Both `add` and `multiply` are inlined as expressions.

// This module's functionality is currently in code_generator.rs
// Future refactoring can extract it here

pub use super::code_generator::*;
