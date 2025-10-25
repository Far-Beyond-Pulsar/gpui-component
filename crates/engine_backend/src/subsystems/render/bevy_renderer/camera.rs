//! Camera input synchronization and movement systems

use bevy::prelude::*;
use super::types::CameraInput;
use super::resources::CameraInputResource;
use super::components::MainCamera;

/// Sync camera input from the input thread to the Bevy ECS resource
/// This system reads from the shared Arc<Mutex<CameraInput>> that the input thread updates
/// and copies it to the Bevy ECS CameraInput resource that camera_movement_system uses
pub fn sync_camera_input_system(
    camera_input_resource: Res<CameraInputResource>,
    mut camera_input: ResMut<CameraInput>,
) {
    // Try to lock the shared camera input without blocking
    if let Ok(mut shared_input) = camera_input_resource.0.try_lock() {
        // Copy the input from the input thread to the Bevy ECS resource
        *camera_input = shared_input.clone();
        
        // IMPORTANT: Clear the delta values in the shared input after copying
        // so they don't get re-applied on the next frame
        // The input thread will set new deltas if there's actual mouse movement
        shared_input.mouse_delta_x = 0.0;
        shared_input.mouse_delta_y = 0.0;
        shared_input.pan_delta_x = 0.0;
        shared_input.pan_delta_y = 0.0;
        shared_input.zoom_delta = 0.0;
    }
    // If lock fails, skip this frame - no blocking!
}

/// Unreal Engine-style camera movement system
/// Supports:
/// - WASD + QE for movement (with Shift for boost)
/// - Right mouse + drag for FPS rotation
/// - Middle mouse + drag for panning
/// - Mouse wheel for zoom (or move speed adjustment with right mouse held)
pub fn camera_movement_system(
    time: Res<Time>,
    mut camera_input: ResMut<CameraInput>,
    mut query: Query<&mut Transform, With<MainCamera>>,
) {
    let Ok(mut transform) = query.single_mut() else {
        return;
    };

    let delta_time = time.delta_secs();
    
    // Calculate effective move speed (with boost)
    let effective_speed = if camera_input.boost {
        camera_input.move_speed * 3.0
    } else {
        camera_input.move_speed
    };
    
    // === FPS-STYLE MOVEMENT (Right mouse + WASD) ===
    // Forward/backward movement (local Z axis)
    if camera_input.forward.abs() > 0.001 {
        let forward = transform.forward();
        transform.translation += forward.as_vec3() * camera_input.forward * effective_speed * delta_time;
    }
    
    // Left/right strafe (local X axis)
    if camera_input.right.abs() > 0.001 {
        let right = transform.right();
        transform.translation += right.as_vec3() * camera_input.right * effective_speed * delta_time;
    }
    
    // Up/down movement (world Y axis)
    if camera_input.up.abs() > 0.001 {
        transform.translation.y += camera_input.up * effective_speed * delta_time;
    }
    
    // === ROTATION (Right mouse + drag) ===
    if camera_input.mouse_delta_x.abs() > 0.001 || camera_input.mouse_delta_y.abs() > 0.001 {
        // Yaw (rotate around world Y axis)
        let yaw_delta = -camera_input.mouse_delta_x * camera_input.look_sensitivity * delta_time;
        transform.rotate_y(yaw_delta);
        
        // Pitch (rotate around local X axis)
        let pitch_delta = -camera_input.mouse_delta_y * camera_input.look_sensitivity * delta_time;
        transform.rotate_local_x(pitch_delta);
        
        // Clear mouse deltas after use
        camera_input.mouse_delta_x = 0.0;
        camera_input.mouse_delta_y = 0.0;
    }
    
    // === PANNING (Middle mouse + drag) ===
    if camera_input.pan_delta_x.abs() > 0.001 || camera_input.pan_delta_y.abs() > 0.001 {
        // Pan along camera's local axes
        let right = transform.right();
        let up = transform.up();
        
        transform.translation -= right.as_vec3() * camera_input.pan_delta_x * camera_input.pan_speed;
        transform.translation += up.as_vec3() * camera_input.pan_delta_y * camera_input.pan_speed;
        
        // Clear pan deltas after use
        camera_input.pan_delta_x = 0.0;
        camera_input.pan_delta_y = 0.0;
    }
    
    // === ZOOM (Mouse wheel) ===
    if camera_input.zoom_delta.abs() > 0.001 {
        let forward = transform.forward();
        transform.translation += forward.as_vec3() * camera_input.zoom_delta * camera_input.zoom_speed * delta_time;
        
        // Clear zoom delta after use
        camera_input.zoom_delta = 0.0;
    }
    
    // === ORBIT MODE (Alt + Left mouse - future enhancement) ===
    if camera_input.orbit_mode {
        // Calculate camera position relative to focus point
        let offset = transform.translation - camera_input.focus_point;
        let _distance = offset.length();
        
        // Rotate offset around focus point
        if camera_input.mouse_delta_x.abs() > 0.001 || camera_input.mouse_delta_y.abs() > 0.001 {
            // This would require converting to spherical coordinates and back
            // For now, keeping it simple with FPS rotation
        }
    }
}
