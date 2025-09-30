//! # The Pulsar Blueprint Compiler
//! 
//! This module contains the implementation of the Pulsar Blueprint Compiler.
//! The compiler is responsible for transforming high-level blueprint definitions
//! into executable rust code that can be run through either rust-script or compiled
//! into a binary via internal `cargo build`.
//! 
//! This process involves several steps, including parsing the blueprint syntax,
//! type checking, and generating the final rust code. The compiler is designed to
//! be extensible, allowing for the addition of new features and optimizations in the future.
//! 
//! ## Defining Nodes
//! 
//! Nodes are defined using the Tron template language, which provides a simple and expressive way
//! to describe the structure and behavior of rust code and insert dynamic values. Each node is
//! it's very own Tron template file in the ./nodes directory.
//! 
//! ## User-defined Nodes
//! 
//! Users can also define their own custom nodes by creating new .tron files in the ./nodes directory
//! of their project. These files should follow the same structure as the built-in nodes, and can include
//! any necessary metadata such as input and output types, descriptions, and categories.
//! The compiler will automatically detect and include these user-defined nodes when generating
//! the library of available nodes for blueprints via the project build.rs script.
//! 
//! ### Reading Node Definitions
//! 
//! The compiler will read these .tron files and use them to generate the library of nodes that can
//! be used in blueprints via the UI. From there, users can drag and drop nodes into their blueprints
//! and connect them together to create complex workflows. Nodes that connect cause the Tron template
//! engine to render the templates together, inserting dynamic values and interconnecting the code as
//! needed.
//! 
//! ## Node execution
//! 
//! Once the blueprint has been fully defined and the nodes have been connected, the compiler will
//! generate the final rust code by rendering the Tron templates together. This code can then be
//! executed using rust-script for quick iteration and testing in the editor, or compiled into a binary for
//! production use.
//! 
//! The compiler also includes error handling and reporting features, allowing users to easily identify
//! and fix issues in their blueprints. Overall, the Pulsar Blueprint Compiler is designed to be a powerful
//! and flexible tool for creating and executing complex workflows using a visual programming approach.

use tron::TronTemplate;
use dashmap::DashMap;
use std::path::PathBuf;

pub mod node_parser;
pub mod graph_compiler;

pub use node_parser::{NodeDefinition, load_all_node_definitions};
pub use graph_compiler::GraphCompiler;

pub fn init() -> DashMap<String, TronTemplate> {
    let mut templates: DashMap<String, TronTemplate> = DashMap::new();
    // Initialize templates here
    
    let engine_nodes = load_all_node_templates(&PathBuf::from("./nodes"));
    let user_nodes = load_all_node_templates(&PathBuf::from("./user_nodes"));

    // Merge the two DashMaps into a combined dashmap of all available nodes
    templates.extend(engine_nodes);
    templates.extend(user_nodes);

    templates
}

fn load_node_template(path: &PathBuf) -> Option<TronTemplate> {
    match std::fs::read_to_string(path) {
        Ok(content) => {
            match TronTemplate::new(&content) {
                Ok(template) => Some(template),
                Err(e) => {
                    eprintln!("Error parsing template {}: {}", path.display(), e);
                    None
                }
            }
        },
        Err(e) => {
            eprintln!("Error reading file {}: {}", path.display(), e);
            None
        }
    }
}

fn load_all_node_templates(dir: &PathBuf) -> DashMap<String, TronTemplate> {
    let templates: DashMap<String, TronTemplate> = DashMap::new();

    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("tron") {
                    if let Some(template) = load_node_template(&path) {
                        if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                            templates.insert(name.to_string(), template);
                        }
                    }
                }
            }
        }
    } else {
        eprintln!("Error reading directory: {}", dir.display());
    }

    templates
}