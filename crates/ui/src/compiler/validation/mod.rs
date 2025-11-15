//! # Validation and Testing
//!
//! Utilities for validating Blueprint graphs and testing the compiler.
//!
//! ## Blueprint Validator (`blueprint_validator`)
//! Validates that a graph is well-formed before compilation:
//! - All referenced nodes exist
//! - All connections are valid
//! - Type compatibility
//! - No circular dependencies
//! - At least one event node
//!
//! ## Test Graph Generators (`test_graphs`)
//! Helpers for creating test graphs programmatically:
//! - Common test patterns
//! - Validation against known-good outputs
//! - Integration test helpers

pub mod blueprint_validator;
pub mod test_graphs;

// Re-export main types
pub use blueprint_validator::{validate_blueprint, ValidationError};
pub use test_graphs::test_default_ui_graph_compilation;
