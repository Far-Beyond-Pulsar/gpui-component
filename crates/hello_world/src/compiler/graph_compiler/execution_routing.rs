//! # Execution Routing Table
//!
//! Tracks which nodes are connected to which execution output pins.
//!
//! This is critical for routing execution flow correctly. In Unreal Blueprints,
//! each execution output pin can connect to different nodes, and we must
//! preserve this routing when generating code.
//!
//! ## Example
//!
//! For a `thread_spawn` node with two exec outputs:
//! - `body` pin connects to `thread_park` node
//! - `continue` pin connects to `print_string` node
//!
//! The routing table stores:
//! ```text
//! (thread_spawn_id, "body") -> [thread_park_id]
//! (thread_spawn_id, "continue") -> [print_string_id]
//! ```

use std::collections::HashMap;
use crate::graph::{GraphDescription, ConnectionType};

/// Tracks which nodes are connected to which execution output pins.
///
/// Maps (source_node_id, output_pin_name) -> Vec<target_node_ids>
pub(super) struct ExecutionRouting {
    routes: HashMap<(String, String), Vec<String>>,
}

impl ExecutionRouting {
    /// Build routing table from graph connections.
    ///
    /// Analyzes all execution-type connections in the graph and builds a lookup
    /// table that maps source pins to their target nodes.
    pub(super) fn build_from_graph(graph: &GraphDescription) -> Self {
        let mut routes = HashMap::new();

        for connection in &graph.connections {
            if matches!(connection.connection_type, ConnectionType::Execution) {
                let key = (
                    connection.source_node.clone(),
                    connection.source_pin.clone(),
                );
                routes
                    .entry(key)
                    .or_insert_with(Vec::new)
                    .push(connection.target_node.clone());
            }
        }

        println!("[ROUTING] Built execution routing table with {} routes", routes.len());
        for ((node_id, pin_name), targets) in &routes {
            println!("[ROUTING]   ({}, {}) -> {:?}", node_id, pin_name, targets);
        }
        ExecutionRouting { routes }
    }

    /// Get all nodes connected to a specific execution output pin.
    ///
    /// Returns an empty slice if no nodes are connected to this pin.
    pub(super) fn get_connected_nodes(&self, node_id: &str, output_pin: &str) -> &[String] {
        self.routes
            .get(&(node_id.to_string(), output_pin.to_string()))
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }
}