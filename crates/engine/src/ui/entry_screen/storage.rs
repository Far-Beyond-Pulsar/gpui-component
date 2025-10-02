use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use std::fs;

/// Configuration structure for recent projects
#[derive(Serialize, Deserialize, Debug)]
pub struct RecentProjectsConfig {
    pub recent_projects: Vec<String>,
}

/// Get the path to the recent projects configuration file
pub fn get_config_path() -> Option<PathBuf> {
    if cfg!(windows) {
        std::env::var("APPDATA").ok().map(|appdata| {
            PathBuf::from(appdata).join("Pulsar").join("recent_projects.json")
        })
    } else {
        std::env::var("HOME").ok().map(|home| {
            PathBuf::from(home)
                .join(".config")
                .join("pulsar")
                .join("recent_projects.json")
        })
    }
}

/// Load recent projects from the configuration file
pub fn load_recent_projects() -> Vec<PathBuf> {
    if let Some(config_path) = get_config_path() {
        if let Ok(contents) = fs::read_to_string(&config_path) {
            if let Ok(config) = serde_json::from_str::<RecentProjectsConfig>(&contents) {
                return config
                    .recent_projects
                    .into_iter()
                    .map(PathBuf::from)
                    .filter(|p| p.exists())
                    .collect();
            }
        }
    }
    Vec::new()
}

/// Save recent projects to the configuration file
pub fn save_recent_projects(projects: &[PathBuf]) -> Result<(), String> {
    let config_path = get_config_path()
        .ok_or_else(|| "Could not determine config path".to_string())?;

    // Ensure parent directory exists
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    let config = RecentProjectsConfig {
        recent_projects: projects.iter().map(|p| p.display().to_string()).collect(),
    };

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    fs::write(&config_path, json)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}

/// Create a blank project structure at the given path
pub fn init_blank_project(path: &PathBuf) -> Result<(), String> {
    // Create the main project directory
    fs::create_dir_all(path)
        .map_err(|e| format!("Failed to create project directory: {}", e))?;

    // Create Pulsar.toml
    let toml_content = r#"[package]
name = "new_project"
version = "0.1.0"

[pulsar]
engine_version = "0.1.0"
"#;
    fs::write(path.join("Pulsar.toml"), toml_content)
        .map_err(|e| format!("Failed to create Pulsar.toml: {}", e))?;

    // Create src directory
    fs::create_dir_all(path.join("src"))
        .map_err(|e| format!("Failed to create src directory: {}", e))?;

    // Create assets directory
    fs::create_dir_all(path.join("assets"))
        .map_err(|e| format!("Failed to create assets directory: {}", e))?;

    Ok(())
}
