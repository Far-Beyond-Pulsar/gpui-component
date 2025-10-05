use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// The main settings struct for the engine, stored as TOML in the user's app data/configs directory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineSettings {
    /// The currently active theme (by name).
    pub active_theme: String,
    /// Editor settings
    pub editor: EditorSettings,
    /// Project settings
    pub project: ProjectSettings,
    /// Advanced settings
    pub advanced: AdvancedSettings,
}

/// Editor-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorSettings {
    /// Font size for the editor
    pub font_size: f32,
    /// Whether to show line numbers
    pub show_line_numbers: bool,
    /// Whether to enable word wrapping
    pub word_wrap: bool,
}

/// Project-specific settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectSettings {
    /// Default path for new projects
    pub default_project_path: Option<String>,
    /// Auto-save interval in seconds (0 = disabled)
    pub auto_save_interval: u32,
    /// Whether to create backups
    pub enable_backups: bool,
}

/// Advanced settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSettings {
    /// Performance optimization level (0-2, higher = more aggressive)
    pub performance_level: u8,
    /// Enable debug logging
    pub debug_logging: bool,
    /// Enable experimental features
    pub experimental_features: bool,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            active_theme: "Default Light".to_string(),
            editor: EditorSettings::default(),
            project: ProjectSettings::default(),
            advanced: AdvancedSettings::default(),
        }
    }
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self {
            font_size: 14.0,
            show_line_numbers: true,
            word_wrap: false,
        }
    }
}

impl Default for ProjectSettings {
    fn default() -> Self {
        Self {
            default_project_path: None,
            auto_save_interval: 300, // 5 minutes
            enable_backups: true,
        }
    }
}

impl Default for AdvancedSettings {
    fn default() -> Self {
        Self {
            performance_level: 1,
            debug_logging: false,
            experimental_features: false,
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
