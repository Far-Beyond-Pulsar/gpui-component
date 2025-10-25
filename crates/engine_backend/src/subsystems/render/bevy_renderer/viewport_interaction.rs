//! Viewport Interaction System
//! 
//! Handles mouse interaction with the viewport for:
//! - Object selection via raycasting (async, spread over multiple frames)
//! - Gizmo manipulation (drag to transform objects)
//! - Camera-independent picking

use bevy::prelude::*;
use std::sync::{Arc, Mutex};
use super::components::{GameObjectId, MainCamera};
use super::gizmos_bevy::{GizmoStateResource, GizmoVisual, GizmoAxis, GizmoType};
use super::resources::SharedGizmoStateResource;

/// Mouse input for viewport interaction (set from GPUI)
#[derive(Resource, Default, Clone)]
pub struct ViewportMouseInput {
    /// Mouse position in viewport space (0,0 = top-left, normalized 0-1)
    pub mouse_pos: Vec2,
    /// Left button just pressed
    pub left_clicked: bool,
    /// Left button is held down
    pub left_down: bool,
    /// Mouse delta since last frame (for dragging)
    pub mouse_delta: Vec2,
}

/// State for active gizmo manipulation
#[derive(Resource, Default)]
pub struct GizmoInteractionState {
    pub is_dragging: bool,
    pub drag_axis: Option<GizmoAxis>,
    pub drag_start_pos: Vec2,
    pub drag_start_transform: Option<Transform>,
    pub drag_start_object_id: Option<u64>,
}

/// Pending raycast request (processed asynchronously over multiple frames)
#[derive(Resource)]
pub struct PendingRaycast {
    pub active: bool,
    pub ray_origin: Vec3,
    pub ray_direction: Dir3,
    pub objects_to_test: Vec<(u64, Vec3)>, // (id, position)
    pub current_index: usize,
    pub closest_hit: Option<(u64, f32)>, // (id, distance)
    pub frame_budget_us: u64, // Microseconds per frame for raycasting
}

impl Default for PendingRaycast {
    fn default() -> Self {
        Self::new()
    }
}

impl PendingRaycast {
    pub fn new() -> Self {
        Self {
            active: false,
            ray_origin: Vec3::ZERO,
            ray_direction: Dir3::new_unchecked(Vec3::Z),
            objects_to_test: Vec::new(),
            current_index: 0,
            closest_hit: None,
            frame_budget_us: 100, // 100us = 0.1ms per frame budget
        }
    }
}

/// System to initiate async raycast on click
pub fn viewport_click_initiate_raycast_system(
    mouse_input: Res<ViewportMouseInput>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    object_query: Query<(Entity, &GameObjectId, &GlobalTransform)>,
    mut pending_raycast: ResMut<PendingRaycast>,
) {
    // Only process on NEW click (not drag, not if already processing)
    if !mouse_input.left_clicked || pending_raycast.active {
        return;
    }
    
    println!("[RAYCAST] 🎯 Initiating async raycast for object selection");
    println!("[RAYCAST] 📍 Mouse position: ({:.3}, {:.3})", mouse_input.mouse_pos.x, mouse_input.mouse_pos.y);
    
    // Get camera
    let Ok((camera, camera_transform)) = camera_query.single() else {
        println!("[RAYCAST] ⚠️ No camera found!");
        return;
    };
    
    // Create ray from mouse position
    let ray = screen_to_world_ray(
        mouse_input.mouse_pos,
        camera,
        camera_transform,
    );
    
    println!("[RAYCAST] ➡️ Ray origin: {:?}, direction: {:?}", ray.origin, ray.direction);
    
    // Collect all objects to test (just positions, lightweight)
    let objects: Vec<(u64, Vec3)> = object_query.iter()
        .map(|(_entity, game_obj_id, transform)| (game_obj_id.0, transform.translation()))
        .collect();
    
    println!("[RAYCAST] 📦 Queued {} objects for async testing", objects.len());
    
    // Initialize async raycast state
    pending_raycast.active = true;
    pending_raycast.ray_origin = ray.origin;
    pending_raycast.ray_direction = ray.direction;
    pending_raycast.objects_to_test = objects;
    pending_raycast.current_index = 0;
    pending_raycast.closest_hit = None;
}

/// System to process raycast incrementally (runs every frame with time budget)
pub fn viewport_raycast_process_system(
    mut pending_raycast: ResMut<PendingRaycast>,
    mut gizmo_state: ResMut<GizmoStateResource>,
    shared_gizmo_state: Res<SharedGizmoStateResource>,
    object_query: Query<(Entity, &GameObjectId, &GlobalTransform)>,
) {
    if !pending_raycast.active {
        return; // No active raycast
    }
    
    let frame_start = std::time::Instant::now();
    let budget = std::time::Duration::from_micros(pending_raycast.frame_budget_us);
    
    // Process objects within time budget
    let mut tests_this_frame = 0;
    while pending_raycast.current_index < pending_raycast.objects_to_test.len() {
        // Check if we've exceeded time budget
        if frame_start.elapsed() > budget && tests_this_frame > 0 {
            // Exceeded budget - continue next frame
            println!("[RAYCAST] ⏱️ Frame budget exceeded ({} tests), continuing next frame...", tests_this_frame);
            return;
        }
        
        let (obj_id, obj_pos) = pending_raycast.objects_to_test[pending_raycast.current_index];
        
        // Perform ray-sphere intersection test
        let ray_to_object = obj_pos - pending_raycast.ray_origin;
        let projection = ray_to_object.dot(*pending_raycast.ray_direction);
        
        if projection > 0.0 {
            let closest_point = pending_raycast.ray_origin + *pending_raycast.ray_direction * projection;
            let dist_to_ray = closest_point.distance(obj_pos);
            
            let radius = 1.5; // Fixed radius for now
            
            if dist_to_ray < radius {
                // Hit! Check if closer than previous hits
                if pending_raycast.closest_hit.is_none() || projection < pending_raycast.closest_hit.unwrap().1 {
                    pending_raycast.closest_hit = Some((obj_id, projection));
                }
            }
        }
        
        pending_raycast.current_index += 1;
        tests_this_frame += 1;
    }
    
    // Finished all objects - apply selection
    println!("[RAYCAST] ✅ Async raycast complete! Tested {} objects in {} frames", 
        pending_raycast.objects_to_test.len(),
        (pending_raycast.objects_to_test.len() + tests_this_frame - 1) / tests_this_frame.max(1)
    );
    
    if let Some((selected_id, distance)) = pending_raycast.closest_hit {
        // Convert numeric ID to string ID
        let string_id = match selected_id {
            1 => "red_cube".to_string(),
            2 => "blue_sphere".to_string(),
            3 => "gold_sphere".to_string(),
            _ => format!("object_{}", selected_id),
        };
        
        println!("[RAYCAST] 🎯 Selected object ID {} ('{}') at distance {:.3}", selected_id, string_id, distance);
        
        // Update local Bevy ECS resource
        gizmo_state.selected_object_id = Some(string_id.clone());
        
        // Update gizmo position to selected object
        if let Some((_entity, _id, transform)) = object_query.iter()
            .find(|(_, id, _)| id.0 == selected_id) 
        {
            gizmo_state.target_position = transform.translation();
        }
        
        // Update shared resource for GPUI
        if let Ok(mut shared) = shared_gizmo_state.0.try_lock() {
            shared.selected_object_id = Some(string_id.clone());
            shared.target_position = gizmo_state.target_position;
            println!("[RAYCAST] 🔄 Synced selection to GPUI: '{}'", string_id);
        }
    } else {
        // No hits - deselect
        println!("[RAYCAST] ⭕ No hits - deselected");
        gizmo_state.selected_object_id = None;
        
        if let Ok(mut shared) = shared_gizmo_state.0.try_lock() {
            shared.selected_object_id = None;
        }
    }
    
    // Mark raycast as complete
    pending_raycast.active = false;
}

/// System to handle gizmo dragging for object manipulation
pub fn gizmo_drag_system(
    mouse_input: Res<ViewportMouseInput>,
    gizmo_state: Res<GizmoStateResource>,
    mut interaction_state: ResMut<GizmoInteractionState>,
    camera_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    gizmo_query: Query<(&GizmoVisual, &GlobalTransform)>,
    mut object_query: Query<(&GameObjectId, &mut Transform)>,
) {
    // Only in edit mode with gizmo enabled
    if !gizmo_state.enabled || gizmo_state.selected_object_id.is_none() {
        interaction_state.is_dragging = false;
        return;
    }
    
    let Ok((camera, camera_transform)) = camera_query.single() else {
        return;
    };
    
    // Start drag
    if mouse_input.left_clicked && !interaction_state.is_dragging {
        // Check if clicked on a gizmo axis
        let ray = screen_to_world_ray(mouse_input.mouse_pos, camera, camera_transform);
        
        // Test gizmo visuals for hit
        let mut closest_gizmo: Option<(GizmoAxis, f32)> = None;
        
        for (gizmo_visual, gizmo_transform) in gizmo_query.iter() {
            if let Some(distance) = test_ray_gizmo_hit(&ray, gizmo_visual, gizmo_transform) {
                if closest_gizmo.is_none() || distance < closest_gizmo.unwrap().1 {
                    closest_gizmo = Some((gizmo_visual.axis, distance));
                }
            }
        }
        
        if let Some((axis, _)) = closest_gizmo {
            // Start dragging this axis
            interaction_state.is_dragging = true;
            interaction_state.drag_axis = Some(axis);
            interaction_state.drag_start_pos = mouse_input.mouse_pos;
            
            // Store starting transform of selected object
            let selected_id_numeric = gizmo_state.selected_object_id.as_ref()
                .and_then(|id| match id.as_str() {
                    "red_cube" => Some(1),
                    "blue_sphere" => Some(2),
                    "gold_sphere" => Some(3),
                    _ => id.parse::<u64>().ok(),
                });
            
            if let Some(id) = selected_id_numeric {
                for (game_obj_id, transform) in object_query.iter() {
                    if game_obj_id.0 == id {
                        interaction_state.drag_start_transform = Some(*transform);
                        interaction_state.drag_start_object_id = Some(id);
                        break;
                    }
                }
            }
            
            println!("[VIEWPORT] 🔧 Started dragging gizmo axis: {:?}", axis);
        }
    }
    
    // Continue drag
    if interaction_state.is_dragging && mouse_input.left_down {
        if let (Some(axis), Some(start_transform), Some(obj_id)) = (
            interaction_state.drag_axis,
            interaction_state.drag_start_transform,
            interaction_state.drag_start_object_id,
        ) {
            // Find and update the object
            for (game_obj_id, mut transform) in object_query.iter_mut() {
                if game_obj_id.0 == obj_id {
                    // Apply transformation based on gizmo type and axis
                    apply_gizmo_transformation(
                        &mut transform,
                        &start_transform,
                        gizmo_state.gizmo_type,
                        axis,
                        mouse_input.mouse_delta,
                        camera_transform,
                    );
                    break;
                }
            }
        }
    }
    
    // End drag
    if !mouse_input.left_down && interaction_state.is_dragging {
        println!("[VIEWPORT] ✅ Finished dragging gizmo");
        interaction_state.is_dragging = false;
        interaction_state.drag_axis = None;
        interaction_state.drag_start_transform = None;
        interaction_state.drag_start_object_id = None;
    }
}

/// Convert screen position to world ray
fn screen_to_world_ray(
    screen_pos: Vec2,
    camera: &Camera,
    camera_transform: &GlobalTransform,
) -> Ray3d {
    // screen_pos is normalized (0-1), convert to NDC (-1 to 1)
    let ndc = Vec2::new(
        screen_pos.x * 2.0 - 1.0,
        1.0 - screen_pos.y * 2.0, // Flip Y
    );
    
    println!("[RAYCAST] 🎲 NDC: ({:.3}, {:.3})", ndc.x, ndc.y);
    
    // Get camera position (ray origin in world space)
    let camera_pos = camera_transform.translation();
    println!("[RAYCAST] 📹 Camera position (ray origin): {:?}", camera_pos);
    
    // Get projection matrix and inverse
    let projection = camera.clip_from_view();
    let inv_projection = projection.inverse();
    
    // Unproject NDC to view space
    let near_ndc = Vec3::new(ndc.x, ndc.y, -1.0); // Near plane in NDC
    let far_ndc = Vec3::new(ndc.x, ndc.y, 1.0);   // Far plane in NDC
    
    // Convert from NDC to view space
    let near_view = inv_projection.project_point3(near_ndc);
    let far_view = inv_projection.project_point3(far_ndc);
    
    println!("[RAYCAST] 🔵 Near view: {:?}, Far view: {:?}", near_view, far_view);
    
    // Convert from view space to world space using camera transform
    let camera_affine = camera_transform.affine();
    let near_world = camera_affine.transform_point3(near_view);
    let far_world = camera_affine.transform_point3(far_view);
    
    println!("[RAYCAST] 🌍 Near world: {:?}, Far world: {:?}", near_world, far_world);
    
    // Calculate ray direction
    let direction = (far_world - near_world).normalize();
    
    println!("[RAYCAST] ➡️  Final ray: origin={:?}, dir={:?}", camera_pos, direction);
    
    // Use camera position as ray origin
    Ray3d::new(camera_pos, Dir3::new_unchecked(direction))
}

/// Test if ray hits a gizmo visual
fn test_ray_gizmo_hit(
    ray: &Ray3d,
    gizmo: &GizmoVisual,
    gizmo_transform: &GlobalTransform,
) -> Option<f32> {
    // Simple bounding sphere test
    let gizmo_pos = gizmo_transform.translation();
    let ray_to_gizmo = gizmo_pos - ray.origin;
    let projection = ray_to_gizmo.dot(*ray.direction);
    
    if projection > 0.0 {
        let closest_point = ray.origin + *ray.direction * projection;
        let dist_to_ray = closest_point.distance(gizmo_pos);
        
        // Gizmos are small - 0.3 unit hit radius
        if dist_to_ray < 0.3 {
            return Some(projection);
        }
    }
    
    None
}

/// Apply gizmo transformation to object
fn apply_gizmo_transformation(
    transform: &mut Transform,
    start_transform: &Transform,
    gizmo_type: GizmoType,
    axis: GizmoAxis,
    mouse_delta: Vec2,
    _camera_transform: &GlobalTransform,
) {
    let sensitivity = 0.01; // Adjust for comfortable manipulation
    
    match gizmo_type {
        GizmoType::Translate => {
            // Translate along axis
            let movement = match axis {
                GizmoAxis::X => Vec3::X * mouse_delta.x * sensitivity,
                GizmoAxis::Y => Vec3::Y * -mouse_delta.y * sensitivity, // Invert Y
                GizmoAxis::Z => Vec3::Z * mouse_delta.x * sensitivity,
                GizmoAxis::None => Vec3::ZERO,
            };
            transform.translation = start_transform.translation + movement;
        }
        GizmoType::Rotate => {
            // Rotate around axis
            let angle = mouse_delta.x * sensitivity;
            let rotation = match axis {
                GizmoAxis::X => Quat::from_axis_angle(Vec3::X, angle),
                GizmoAxis::Y => Quat::from_axis_angle(Vec3::Y, angle),
                GizmoAxis::Z => Quat::from_axis_angle(Vec3::Z, angle),
                GizmoAxis::None => Quat::IDENTITY,
            };
            transform.rotation = rotation * start_transform.rotation;
        }
        GizmoType::Scale => {
            // Scale along axis
            let scale_delta = 1.0 + mouse_delta.y * sensitivity;
            let new_scale = match axis {
                GizmoAxis::X => Vec3::new(start_transform.scale.x * scale_delta, start_transform.scale.y, start_transform.scale.z),
                GizmoAxis::Y => Vec3::new(start_transform.scale.x, start_transform.scale.y * scale_delta, start_transform.scale.z),
                GizmoAxis::Z => Vec3::new(start_transform.scale.x, start_transform.scale.y, start_transform.scale.z * scale_delta),
                GizmoAxis::None => start_transform.scale * scale_delta, // Uniform
            };
            transform.scale = new_scale.max(Vec3::splat(0.1)); // Prevent negative/zero scale
        }
        GizmoType::None => {}
    }
}
