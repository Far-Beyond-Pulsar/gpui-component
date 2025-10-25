//! Bevy components for the renderer

use bevy::prelude::*;

/// Marker component for the main camera
#[derive(Component)]
pub struct MainCamera;

/// Marker component linking a Bevy entity to a game thread object
#[derive(Component)]
pub struct GameObjectId(pub u64);
