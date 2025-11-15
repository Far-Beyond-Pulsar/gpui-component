//! # Import Collection
//!
//! Collects all required `use` statements from nodes in the graph.
//!
//! Each node type may require specific imports. This module analyzes
//! the graph and generates the minimal set of imports needed.
//!
//! ## Example
//!
//! If the graph uses:
//! - `thread_spawn` (requires `std::thread`)
//! - `fs_read` (requires `std::fs`)
//! - Math nodes (requires `pulsar_std::*`)
//!
//! Generates:
//! ```rust,ignore
//! use pulsar_std::*;
//! use std::thread;
//! use std::fs;
//! ```

// This module's functionality is currently in code_generator.rs (collect_node_imports)
// Future refactoring can extract it here

pub use super::code_generator::*;
