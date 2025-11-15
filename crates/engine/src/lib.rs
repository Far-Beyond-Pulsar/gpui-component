//! Pulsar Engine Library
//!
//! This library provides access to core engine functionality for UI crates and other components.

// Re-export core modules that UI needs
pub mod assets;
pub mod compiler;
pub mod graph;
pub mod render;
pub mod settings;
pub mod themes;

// Re-export engine state
pub use engine_state;

// Re-export Assets type for convenience
pub use assets::Assets;

// Re-export action types that UI needs
pub use gpui::Action;
use serde::Deserialize;

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = pulsar_engine, no_json)]
pub struct OpenSettings;

// Engine constants
pub const ENGINE_NAME: &str = env!("CARGO_PKG_NAME");
pub const ENGINE_LICENSE: &str = env!("CARGO_PKG_LICENSE");
pub const ENGINE_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const ENGINE_HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");
pub const ENGINE_REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
pub const ENGINE_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const ENGINE_LICENSE_FILE: &str = env!("CARGO_PKG_LICENSE_FILE");
