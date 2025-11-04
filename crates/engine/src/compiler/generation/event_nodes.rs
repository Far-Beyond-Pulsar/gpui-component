//! # Event Node Code Generation
//!
//! Generates top-level functions from event nodes (entry points).
//!
//! Event nodes are special nodes that become entry points in the generated code:
//! - `main` - Main entry point
//! - `begin_play` - Called when actor begins play
//! - Custom events - User-defined entry points
//!
//! ## Generation Strategy
//!
//! Each event node becomes a top-level function with the execution chain
//! as its body.
//!
//! ## Example
//!
//! Graph: `main -> print("Hello, World!")`
//!
//! Generated:
//! ```rust,ignore
//! pub fn main() {
//!     print_string("Hello, World!".to_string());
//! }
//! ```

// This module's functionality is currently in code_generator.rs
// Future refactoring can extract it here

pub use super::code_generator::*;
