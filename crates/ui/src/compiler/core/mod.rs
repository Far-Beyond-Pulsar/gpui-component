//! # Compiler Core
//!
//! Core compiler functionality and primary entry points for the Blueprint-to-Rust compiler.
//!
//! This module contains the main compilation pipeline that orchestrates all compilation phases:
//! 1. Metadata extraction - Loading node definitions from pulsar_std
//! 2. Graph expansion - Expanding sub-graphs and macros
//! 3. Analysis - Building data flow and execution flow graphs
//! 4. Code generation - Generating executable Rust code
//!
//! ## Architecture
//!
//! The compiler follows a multi-phase architecture inspired by traditional compilers:
//!
//! ```text
//! ┌─────────────────┐
//! │  Graph Input    │  (GraphDescription from editor)
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │ Phase 0: Expand │  (Inline sub-graphs and macros)
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │ Phase 1: Load   │  (Extract node metadata from pulsar_std)
//! │    Metadata     │
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │ Phase 2: Data   │  (Build data dependency graph)
//! │   Flow Analysis │  (Topological sort for evaluation order)
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │ Phase 3: Exec   │  (Map execution connections)
//! │ Flow Analysis   │  (Build routing table)
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │ Phase 4: Code   │  (Generate Rust source code)
//! │   Generation    │  (Apply node-specific strategies)
//! └────────┬────────┘
//!          │
//!          ▼
//! ┌─────────────────┐
//! │  Rust Source    │  (Compilable .rs file)
//! └─────────────────┘
//! ```
//!
//! ## Node Types and Generation Strategies
//!
//! The compiler handles three types of nodes, each requiring different code generation:
//!
//! ### 1. Pure Nodes (`NodeTypes::pure`)
//! - No side effects, no execution pins
//! - Generated as **inline expressions**
//! - Evaluated lazily when their outputs are needed
//! - Example: `add(a, b)` becomes `add(5, 3)` inline
//!
//! ### 2. Function Nodes (`NodeTypes::fn_`)
//! - Has side effects, linear execution flow
//! - One exec input → one exec output
//! - Generated as **function calls** in sequence
//! - Example: `print_string("Hi")` becomes `print_string("Hi");` followed by next node
//!
//! ### 3. Control Flow Nodes (`NodeTypes::control_flow`)
//! - Branch execution flow with multiple exec outputs
//! - Contains `exec_output!("Label")` macro calls
//! - Must be **inlined** with exec_output replaced by connected code
//! - Example: `branch(condition)` becomes inline `if condition { ... } else { ... }`
//!
//! ## Thread Safety and Variables
//!
//! The compiler generates thread-safe code for Blueprint class variables:
//! - Copy types (i32, f32, bool, etc.) use `Cell<T>` for interior mutability
//! - Non-Copy types (String, Vec, etc.) use `RefCell<T>`
//! - All variables wrapped in `Arc` for shared ownership across threads
//!
//! ## Error Handling
//!
//! All compilation errors include:
//! - Node ID and type for easy identification
//! - Pin names (for connection errors)
//! - Expected vs actual types (for type errors)
//! - Clear, actionable error messages
//!
//! ## Example Usage
//!
//! ```rust,no_run
//! use pulsar_engine::compiler::core::compile_graph;
//! use pulsar_engine::graph::GraphDescription;
//!
//! let graph = GraphDescription::new("my_graph");
//! // ... build graph with nodes and connections
//!
//! match compile_graph(&graph) {
//!     Ok(rust_code) => {
//!         // Write to file and compile with rustc
//!         std::fs::write("generated.rs", rust_code)?;
//!     }
//!     Err(error_msg) => {
//!         eprintln!("Compilation failed: {}", error_msg);
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

pub mod compiler;
pub mod metadata;
pub mod pipeline;

// Re-export main API
pub use compiler::*;
pub use metadata::*;
pub use pipeline::*;
