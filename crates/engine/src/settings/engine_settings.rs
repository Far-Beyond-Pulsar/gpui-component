use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// The main settings struct for the engine, stored as TOML in the user's app data/configs directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineSettings {
    /// The currently active theme (by name).
    pub active_theme: String,
    // TODO: Other settings can be added here.
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            active_theme: "default".to_string(),
        }
    }
}

impl EngineSettings {
    /// Load settings from the given path, or return default if not found or invalid.
    pub fn load(path: &Path) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    /// Save settings to the given path, creating parent directories if needed.
    pub fn save(&self, path: &Path) {
        if let Some(parent) = path.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("Failed to create config directory: {e}");
                return;
            }
        }
        let toml = toml::to_string_pretty(self).expect("Failed to serialize settings");
        if let Err(e) = fs::write(path, toml) {
            eprintln!("Failed to write settings: {e}");
        }
    }

    /// Get the default config file path in the user's app data directory.
    pub fn default_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("com", "Pulsar", "Pulsar_Engine")
            .map(|proj| proj.data_dir().join("configs/engine.toml"))
    }
}
