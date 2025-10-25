//! Game object synchronization between game thread and render thread

use bevy::prelude::*;
use super::components::GameObjectId;
use super::resources::GameThreadResource;

/// Sync game thread object positions/rotations to Bevy entities
/// This system reads from the game thread state and updates matching Bevy transforms
pub fn sync_game_objects_system(
    game_thread: Res<GameThreadResource>,
    mut query: Query<(&GameObjectId, &mut Transform)>,
) {
    // Get game state if available
    let Some(ref game_state_arc) = game_thread.0 else {
        return; // No game thread connected
    };

    // Try to lock game state (non-blocking)
    let Ok(game_state) = game_state_arc.try_lock() else {
        return; // Game thread busy, skip this frame
    };

    // Update all entities that have a GameObjectId
    for (game_obj_id, mut transform) in query.iter_mut() {
        if let Some(game_obj) = game_state.get_object(game_obj_id.0) {
            // Sync position
            transform.translation = Vec3::new(
                game_obj.position[0],
                game_obj.position[1],
                game_obj.position[2],
            );

            // Sync rotation (convert degrees to radians)
            transform.rotation = Quat::from_euler(
                EulerRot::XYZ,
                game_obj.rotation[0].to_radians(),
                game_obj.rotation[1].to_radians(),
                game_obj.rotation[2].to_radians(),
            );

            // Sync scale
            transform.scale = Vec3::new(
                game_obj.scale[0],
                game_obj.scale[1],
                game_obj.scale[2],
            );
        }
    }
}
