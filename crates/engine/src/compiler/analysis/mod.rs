//! # Analysis Phase
//!
//! Graph analysis modules for the Blueprint compiler.
//!
//! This phase analyzes the node graph to understand both data flow and execution flow:
//!
//! ## Data Flow Analysis (`data_flow`)
//!
//! Analyzes how data moves between nodes:
//! - Traces data dependencies between pure nodes
//! - Performs topological sorting to determine evaluation order
//! - Resolves input sources (connections, constants, or defaults)
//! - Detects circular dependencies
//!
//! ## Execution Flow Analysis (`exec_flow`)
//!
//! Analyzes how control flows through the graph:
//! - Maps execution connections between nodes
//! - Builds routing tables for exec output pins
//! - Identifies execution entry points (event nodes)
//! - Validates execution connectivity
//!
//! ## Type Analysis (`type_analysis`)
//!
//! Validates type compatibility:
//! - Checks connection type safety
//! - Infers types for ambiguous connections
//! - Reports type mismatches with detailed errors
//!
//! ## Usage
//!
//! These analyzers are used during the compilation pipeline:
//!
//! ```rust,ignore
//! use pulsar_engine::compiler::analysis::*;
//!
//! // Data flow
//! let data_resolver = DataResolver::build(&graph, &metadata)?;
//! let eval_order = data_resolver.get_pure_evaluation_order();
//!
//! // Execution flow
//! let exec_routing = ExecutionRouting::build_from_graph(&graph);
//! let targets = exec_routing.get_connected_nodes("node1", "exec_out");
//! ```

pub mod data_flow;
pub mod exec_flow;
// pub mod type_analysis; // TODO: Fix TypeInfo method calls

// Re-export main types
pub use data_flow::{DataResolver, DataSource};
pub use exec_flow::ExecutionRouting;
// pub use type_analysis::TypeChecker;
