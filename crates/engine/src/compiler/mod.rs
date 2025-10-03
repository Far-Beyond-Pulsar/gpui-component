//! # The Pulsar Blueprint Compiler (Macro-Based)
//!
//! This compiler transforms visual node graphs into executable Rust code.
//!
//! ## New Architecture
//!
//! Unlike the old template-based system (.tron files), this compiler works with
//! Rust functions decorated with `#[blueprint]` attribute macros from `pulsar_std`.
//!
//! ## Compilation Pipeline
//!
//! 1. **Metadata Extraction**: Parse pulsar_std to extract node metadata
//! 2. **Data Flow Resolution**: Build data dependency graph and evaluation order
//! 3. **Execution Routing**: Map execution connections between nodes
//! 4. **Code Generation**: Generate Rust code with proper inlining
//!
//! ## Node Types
//!
//! - **Pure**: Inline as expressions (no exec pins)
//! - **Function**: Generate function calls (single exec chain)
//! - **Control Flow**: Inline with exec_output!() substitution
//!
//! See DESIGN.md for full architectural details.

use std::collections::HashMap;
use std::sync::OnceLock;
use crate::graph::GraphDescription;

// New modular architecture
pub mod node_metadata;
pub mod data_resolver;
pub mod execution_routing;
pub mod ast_utils;
pub mod code_generator;
pub mod type_extractor;

// Blueprint validation
pub mod validate_blueprint;
pub mod test_default_ui_graph;

// Tests
#[cfg(test)]
mod tests;

// Re-exports
pub use node_metadata::NodeMetadata;

/// Cached node metadata from pulsar_std
static NODE_METADATA: OnceLock<HashMap<String, NodeMetadata>> = OnceLock::new();

/// Get node metadata (lazily initialized)
fn get_node_metadata() -> &'static HashMap<String, NodeMetadata> {
    NODE_METADATA.get_or_init(|| {
        match node_metadata::extract_node_metadata() {
            Ok(metadata) => {
                println!("[COMPILER] Loaded {} node definitions from pulsar_std", metadata.len());
                metadata
            }
            Err(e) => {
                eprintln!("[COMPILER] Failed to extract node metadata: {}", e);
                HashMap::new()
            }
        }
    })
}

/// Main compilation entry point for the new macro-based compiler
pub fn compile_graph(graph: &GraphDescription) -> Result<String, String> {
    println!("[COMPILER] Starting macro-based compilation");

    // Phase 1: Get node metadata
    let metadata = get_node_metadata();
    if metadata.is_empty() {
        return Err("No node metadata available - check pulsar_std".to_string());
    }

    // Phase 2: Build data flow resolver (no variables for generic compile)
    let variables = HashMap::new();
    let data_resolver = data_resolver::DataResolver::build_with_variables(graph, metadata, variables.clone())?;
    println!("[COMPILER] Built data flow resolver");

    // Phase 3: Build execution routing
    let exec_routing = execution_routing::ExecutionRouting::build_from_graph(graph);
    println!("[COMPILER] Built execution routing");

    // Phase 4: Generate code
    let code = code_generator::generate_program(
        graph,
        metadata,
        &data_resolver,
        &exec_routing,
        variables,
    )?;

    println!("[COMPILER] Code generation complete");
    Ok(code)
}

/// Legacy compilation entry point (for backward compatibility)
#[deprecated(note = "Use compile_graph instead - this uses old .tron template system")]
pub fn compile_graph_legacy(graph: &GraphDescription) -> Result<String, String> {
    eprintln!("[COMPILER] Warning: Using legacy .tron template compiler");
    // This would use the old graph_compiler::GraphCompiler
    Err("Legacy compiler not fully implemented - use new macro-based compiler".to_string())
}