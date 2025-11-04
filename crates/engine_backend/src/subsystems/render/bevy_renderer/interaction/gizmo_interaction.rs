//! Gizmo Interaction System - Mouse manipulation of gizmos
//! 
//! ⚠️ **DEPRECATED**: This module is kept for reference only.
//! All functionality has been moved to `interaction::viewport`.
//! 
//! This module handles:
//! - Mouse ray casting from viewport
//! - Gizmo axis hit testing  
//! - Drag operations (translate/rotate/scale)
//! - Transform updates back to scene database

use bevy::prelude::*;
use crate::subsystems::render::bevy_renderer::core::{GameObjectId, MainCamera};
use crate::subsystems::render::bevy_renderer::gizmos::rendering::{
    GizmoStateResource, GizmoVisual, GizmoType, GizmoAxis,
};

/// Resource for tracking active gizmo manipulation
#[derive(Resource, Default)]
pub struct GizmoInteractionState {
    pub is_dragging: bool,
    pub drag_start_pos: Vec2,
    pub drag_axis: Option<GizmoAxis>,
    pub drag_start_transform: Option<Transform>,
}

/// System to handle mouse input for gizmo manipulation
/// This would normally receive input from GPUI, but for now we'll track state
pub fn gizmo_interaction_system(
    mut interaction: ResMut<GizmoInteractionState>,
    gizmo_state: Res<GizmoStateResource>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    gizmo_query: Query<(&GizmoVisual, &GlobalTransform)>,
    mut object_query: Query<(&GameObjectId, &mut Transform)>,
) {
    // This system would be called with mouse input events
    // For now, it's a placeholder showing the architecture
    
    // TODO: Integrate with GPUI mouse events from viewport
    // TODO: Implement ray casting
    // TODO: Implement drag operations
    // TODO: Send transform updates back to editor via channel
}

/// Helper to cast a ray from screen position through the camera
pub fn screen_to_world_ray(
    screen_pos: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
    viewport_size: Vec2,
) -> Ray3d {
    // Convert screen coordinates to NDC (-1 to 1)
    let ndc = Vec2::new(
        (screen_pos.x / viewport_size.x) * 2.0 - 1.0,
        1.0 - (screen_pos.y / viewport_size.y) * 2.0,
    );
    
    // Get camera matrices
    let view_matrix = Mat4::from(camera_transform.affine());
    let projection = camera.clip_from_view();
    
    // Unproject to world space
    let inv_proj_view = (projection * view_matrix).inverse();
    
    let near_point = inv_proj_view.project_point3(Vec3::new(ndc.x, ndc.y, -1.0));
    let far_point = inv_proj_view.project_point3(Vec3::new(ndc.x, ndc.y, 1.0));
    
    let direction = (far_point - near_point).normalize();
    
    Ray3d {
        origin: near_point,
        direction: Dir3::new(direction).unwrap_or(Dir3::Y),
    }
}

/// Test if a ray intersects a gizmo axis
pub fn ray_intersects_gizmo_axis(
    ray: &Ray3d,
    gizmo_transform: &GlobalTransform,
    axis: GizmoAxis,
    gizmo_type: GizmoType,
) -> Option<f32> {
    // Simplified intersection test
    // TODO: Implement proper cylinder/torus/box intersection
    // For now, return None (no interaction)
    None
}

/// Apply drag delta to transform based on gizmo type and axis
pub fn apply_gizmo_drag(
    transform: &mut Transform,
    drag_delta: Vec2,
    gizmo_type: GizmoType,
    drag_axis: GizmoAxis,
    camera_transform: &GlobalTransform,
) {
    match gizmo_type {
        GizmoType::Translate => {
            // Convert screen space drag to world space translation
            let movement = match drag_axis {
                GizmoAxis::X => Vec3::X * drag_delta.x * 0.01,
                GizmoAxis::Y => Vec3::Y * drag_delta.y * 0.01,
                GizmoAxis::Z => Vec3::Z * drag_delta.x * 0.01,
                GizmoAxis::None => Vec3::ZERO,
            };
            transform.translation += movement;
        }
        GizmoType::Rotate => {
            // Convert drag to rotation
            let rotation_delta = drag_delta.x * 0.01;
            let rotation = match drag_axis {
                GizmoAxis::X => Quat::from_rotation_x(rotation_delta),
                GizmoAxis::Y => Quat::from_rotation_y(rotation_delta),
                GizmoAxis::Z => Quat::from_rotation_z(rotation_delta),
                GizmoAxis::None => Quat::IDENTITY,
            };
            transform.rotation = rotation * transform.rotation;
        }
        GizmoType::Scale => {
            // Convert drag to scale
            let scale_delta = 1.0 + drag_delta.y * 0.01;
            match drag_axis {
                GizmoAxis::X => transform.scale.x *= scale_delta,
                GizmoAxis::Y => transform.scale.y *= scale_delta,
                GizmoAxis::Z => transform.scale.z *= scale_delta,
                GizmoAxis::None => transform.scale *= scale_delta, // Uniform
            };
        }
        GizmoType::None => {}
    }
}
