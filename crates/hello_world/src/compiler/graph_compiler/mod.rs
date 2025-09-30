//! # Unreal Blueprint-Style Graph Compiler
//!
//! This compiler transforms visual node graphs into executable Rust code following
//! the Unreal Engine Blueprint execution model.
//!
//! ## Key Concepts
//!
//! - **Entry Points**: Nodes like `begin_play` become top-level functions (`main()`)
//! - **Execution Flow**: White execution pins control flow, exactly like Blueprint wires
//! - **Inline Expansion**: Nodes with execution placeholders are expanded inline
//! - **Execution Routing**: Each execution output pin routes to specific code
//!
//! ## Template Types
//!
//! 1. **Pure Expressions**: `a + b` - Used in data flow only
//! 2. **Simple Functions**: `print_string()` - Can be called or inlined
//! 3. **Control Flow**: `branch`, `thread_spawn` - MUST be inlined
//!
//! ## Example
//!
//! ```text
//! BeginPlay -> thread_spawn -> print("after spawn")
//!              └─body─> thread_park
//! ```
//!
//! Compiles to:
//!
//! ```rust
//! fn main() {
//!     let handle = std::thread::spawn(|| {
//!         thread_park();  // From body pin
//!     });
//!     print("after spawn");  // From continue pin
//!     handle
//! }
//! ```
//!
//! ## Module Structure
//!
//! - `execution_routing` - Tracks exec pin connections
//! - `template_type` - Classifies template types
//! - `template_analysis` - Analyzes templates to determine type
//! - `code_generation` - Generates entry points and function definitions
//! - `node_compiler` - Core recursive inline compilation (THE HEART)
//! - `compiler_helpers` - Utility functions for compilation

mod execution_routing;
mod template_type;
mod template_analysis;
mod code_generation;
mod node_compiler;
mod compiler_helpers;

use std::collections::HashMap;
use tron::TronTemplate;
use crate::graph::GraphDescription;
use crate::compiler::NodeDefinition;

use execution_routing::ExecutionRouting;
use template_type::TemplateType;

/// The main graph compiler that transforms node graphs into Rust code.
///
/// This follows the Unreal Blueprint execution model where:
/// - Entry points become top-level functions
/// - Execution flow is followed recursively
/// - Control flow nodes are expanded inline
/// - Simple nodes can be called as functions
pub struct GraphCompiler {
    /// Definitions for all available node types
    node_definitions: HashMap<String, NodeDefinition>,

    /// Templates for generating node code
    templates: HashMap<String, TronTemplate>,

    /// Cached template type analysis
    template_types: HashMap<String, TemplateType>,
}

impl GraphCompiler {
    /// Create a new graph compiler with node definitions and templates.
    ///
    /// Pre-analyzes all templates to determine their types for faster compilation.
    pub fn new(
        node_definitions: HashMap<String, NodeDefinition>,
        templates: HashMap<String, TronTemplate>,
    ) -> Self {
        let mut compiler = Self {
            node_definitions,
            templates,
            template_types: HashMap::new(),
        };

        // Pre-analyze all templates
        compiler.analyze_all_templates();

        compiler
    }

    /// Main compilation entry point.
    ///
    /// ## Algorithm
    ///
    /// 1. Build execution routing table from connections
    /// 2. Find entry points (begin_play, on_tick, etc.)
    /// 3. Generate function definitions for simple nodes
    /// 4. Generate entry point functions with inline expansion
    ///
    /// ## Output
    ///
    /// Returns generated Rust code as a string, or an error if compilation fails.
    pub fn compile_graph(&self, graph: &GraphDescription) -> Result<String, String> {
        println!("[COMPILER] === Starting Compilation ===");

        let mut generated_code = String::new();

        // Phase 1: Build execution routing table
        let routing = ExecutionRouting::build_from_graph(graph);

        // Phase 2: Find entry points
        let entry_points = self.find_entry_points(graph);
        println!("[COMPILER] Found {} entry points: {:?}",
                 entry_points.len(),
                 entry_points.iter()
                     .filter_map(|id| graph.nodes.get(id).map(|n| &n.node_type))
                     .collect::<Vec<_>>());

        // Phase 3: Generate simple function definitions
        let simple_functions = self.generate_simple_functions(graph)?;
        if !simple_functions.is_empty() {
            generated_code.push_str("// ============================================================================\n");
            generated_code.push_str("// Simple Function Definitions (nodes without execution placeholders)\n");
            generated_code.push_str("// ============================================================================\n\n");
            generated_code.push_str(&simple_functions);
            generated_code.push_str("\n");
        }

        // Phase 4: Generate entry point functions
        generated_code.push_str("// ============================================================================\n");
        generated_code.push_str("// Entry Points (Begin Play, On Tick, etc.)\n");
        generated_code.push_str("// ============================================================================\n\n");

        for entry_node_id in entry_points {
            if let Some(entry_node) = graph.nodes.get(&entry_node_id) {
                let entry_fn = self.compile_entry_point(entry_node, graph, &routing)?;
                generated_code.push_str(&entry_fn);
                generated_code.push_str("\n\n");
            }
        }

        println!("[COMPILER] === Compilation Complete ===");
        Ok(generated_code)
    }
}