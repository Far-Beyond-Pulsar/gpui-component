//! # Compiler Utilities
//!
//! Helper modules providing common functionality for the compiler.
//!
//! ## AST Transformation (`ast_transform`)
//! Tools for parsing and transforming Rust abstract syntax trees:
//! - Parse function bodies
//! - Replace `exec_output!()` macro calls
//! - Substitute parameter values
//! - Extract function bodies
//!
//! ## Sub-Graph Expansion (`subgraph_expander`)
//! Handles inlining of Blueprint macros (sub-graphs):
//! - Recursive expansion with circular reference detection
//! - Connection rewiring through sub-graph boundaries
//! - Position offsetting for visual layout
//!
//! ## Type Extraction (`type_extractor`)
//! Enumerates all types used in the Blueprint system:
//! - Extract types from node metadata
//! - Generate type lists for variable declarations
//! - Provide display names for UI
//!
//! ## Variable Generation (`variable_gen`)
//! Generates thread-safe variable wrappers:
//! - Cell/RefCell selection based on Copy trait
//! - Arc wrapping for shared ownership
//! - Getter/setter node code generation

pub mod ast_transform;
pub mod subgraph_expander;
pub mod type_extractor;
pub mod variable_gen;

// Re-export main types
pub use ast_transform::{ExecOutputReplacer, ParameterSubstitutor, inline_control_flow_function};
pub use subgraph_expander::SubGraphExpander;
pub use type_extractor::extract_all_blueprint_types;
pub use variable_gen::VariableInfo;
