//! # The Pulsar Blueprint Compiler
//!
//! A production-ready compiler transforming visual node graphs (Blueprints)
//! into executable Rust source code.
//!
//! See DESIGN.md for architectural details.

// ================================================================================================
// Public API - Core compilation functions
// ================================================================================================

pub mod core;

// Re-export main compilation functions
pub use core::{
    compile_graph, 
    compile_graph_with_library_manager,
    compile_graph_with_variables,
    get_node_metadata
};

// ================================================================================================
// Internal Modules - Implementation details
// ================================================================================================

/// Graph analysis (data flow, execution flow, types)
pub mod analysis;

/// Code generation strategies
pub mod generation;

/// Utility modules (AST, sub-graphs, types)
pub mod utils;

/// Validation and testing utilities
pub mod validation;

/// Legacy code (deprecated)
#[allow(deprecated)]
pub mod legacy;

// ================================================================================================
// Backward Compatibility Layer
// ================================================================================================
// Re-export modules at root level for backward compatibility

pub mod node_metadata {
    //! Re-export for backward compatibility
    pub use crate::compiler::core::metadata::*;
}

pub mod data_resolver {
    //! Re-export for backward compatibility  
    pub use crate::compiler::analysis::data_flow::*;
}

pub mod execution_routing {
    //! Re-export for backward compatibility
    pub use crate::compiler::analysis::exec_flow::*;
}

pub mod ast_utils {
    //! Re-export for backward compatibility
    pub use crate::compiler::utils::ast_transform::*;
}

pub mod code_generator {
    //! Re-export for backward compatibility
    pub use crate::compiler::generation::code_generator::*;
}

pub mod type_extractor {
    //! Re-export for backward compatibility
    pub use crate::compiler::utils::type_extractor::*;
}

pub mod subgraph_expander {
    //! Re-export for backward compatibility
    pub use crate::compiler::utils::subgraph_expander::*;
}

pub mod validate_blueprint {
    //! Re-export for backward compatibility
    pub use crate::compiler::validation::blueprint_validator::*;
}

pub mod test_default_ui_graph {
    //! Re-export for backward compatibility
    pub use crate::compiler::validation::test_graphs::*;
}

// Re-exports for backward compatibility
pub use node_metadata::NodeMetadata;

// Tests
#[cfg(test)]
mod tests;
