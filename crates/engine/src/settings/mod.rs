```
Pulsar-Native/crates/engine/src/settings/mod.rs
//! Settings module for Pulsar Engine.
//! Provides loading and saving of engine settings (theme, etc.) from TOML in the user's app data directory.

pub mod engine_settings;

pub use engine_settings::EngineSettings;
