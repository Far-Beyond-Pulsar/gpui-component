//! # World Subsystem
//!
//! This module manages the game world, including loading levels, managing actors, and handling world state.
//! It utilizes the PebbleVault crate for efficient spatial storage and retrieval of game objects.
//! The World struct provides methods to add, remove, and query actors within the game world.
//! It is designed to be extensible for future features such as dynamic world generation and environmental effects.
//!
//! # Design decisions
//! - **PebbleVault Integration**: Chosen for its efficient spatial storage capabilities, allowing for quick access
//!   to game objects based on their location and for levels to be partitioned into regions by default.
//! - **Modular Structure**: The World subsystem is designed to be modular, making it easy to extend with additional
//!
//! # Features
//! - Load and manage game levels
//! - Add, remove, and query actors
//! - Spatial partitioning of the world for performance optimization
//! - Future support for dynamic world generation and environmental effects
//! - Integration with other subsystems like rendering and physics
//! - Designed for scalability to handle large game worlds
//! - Error handling and logging for robust operation
//! - Unit tests for core functionalities

use std::sync::Arc;
use PebbleVault as pebble;
use PebbleVault::spacial_store::sqlite_backend::SqliteDatabase;
use super::classes::actor::Actor;
use uuid::Uuid;

pub struct World {
    // World data and methods
    vault: pebble::VaultManager<Actor>,
}
impl World {
    pub fn new(level_name: &str) -> Self {
        let db = SqliteDatabase::new(level_name).expect(
            "Failed to create SqliteDatabase, probably FS permission issues"
        );
        let vault = pebble::VaultManager::new(Box::new(db)).expect("Failed to create VaultManager");

        Self {
            vault,
        }
    }

    fn add_actor_private(
        &self,
        region_id: Uuid,
        uuid: Uuid,
        object_type: &str,
        x: f64,
        y: f64,
        z: f64,
        size_x: f64,
        size_y: f64,
        size_z: f64,
        custom_data: Arc<Actor>
    ) -> Result<(), String> {
        self.vault.add_object(region_id, uuid, object_type, x, y, z, size_x, size_y, size_z, custom_data)
    }

    pub fn get_actor(&self, object_id: Uuid) -> Option<&Actor> {
        match self.vault.get_object(object_id) {
            Ok(Some(spatial_obj)) => Some(&spatial_obj.data),
            Ok(None) => None,
            Err(_) => None,
        }
    }

    pub fn remove_actor(&mut self, id: Uuid) -> Option<Actor> {
        let actor = self.get_actor(id).cloned();
        match self.vault.remove_object(id) {
            Ok(_) => actor,
            Err(_) => None,
        }
    }
}
