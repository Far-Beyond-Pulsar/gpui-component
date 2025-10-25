//! Bevy components for the renderer

use bevy::prelude::*;

/// Marker component for the main camera
#[derive(Component)]
pub struct MainCamera;

/// Marker component linking a Bevy entity to a game thread object
#[derive(Component)]
pub struct GameObjectId(pub u64);

/// Marker component for objects that are currently selected in the editor
#[derive(Component)]
pub struct Selected;

/// Component for selection outline rendering
#[derive(Component)]
pub struct SelectionOutline {
    pub color: Color,
    pub width: f32,
}
