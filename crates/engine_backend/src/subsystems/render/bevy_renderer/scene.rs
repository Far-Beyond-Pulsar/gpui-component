//! Scene setup - spawns 3D objects, cameras, and lights

use bevy::prelude::*;
use bevy::core_pipeline::tonemapping::Tonemapping;
use super::components::{MainCamera, GameObjectId};
use super::resources::SharedTexturesResource;

/// Setup 3D scene - runs AFTER DXGI textures are created
pub fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    shared_textures: Res<SharedTexturesResource>,
) {
    println!("[BEVY] 🎬 Setting up scene...");

    // Get the render target handles that were created by create_shared_textures
    let render_target_0 = match shared_textures.0.lock().ok().and_then(|l| l.as_ref().map(|t| t.textures[0].clone())) {
        Some(handle) => {
            println!("[BEVY] ✅ Got render target handle: {:?}", handle);
            println!("[BEVY] 📍 Camera will render to asset ID: {:?}", handle.id());
            handle
        },
        None => {
            println!("[BEVY] ❌ No render targets available");
            return;
        }
    };

    // Camera rendering to shared DXGI texture with TONEMAPPING DISABLED
    println!("[BEVY] 📹 Creating camera targeting shared texture");
    println!("[BEVY] 🎯 Camera will render to buffer 0 (handle: {:?})", render_target_0.id());
    commands.spawn((
        Camera3d::default(),
        Camera {
            target: bevy::camera::RenderTarget::Image(render_target_0.into()),
            clear_color: bevy::prelude::ClearColorConfig::Custom(Color::srgb(0.2, 0.2, 0.3)), // Dark blue-grey background
            ..default()
        },
        Transform::from_xyz(-3.0, 3.0, 6.0).looking_at(Vec3::new(0.0, 0.5, 0.0), Vec3::Y),
        Tonemapping::None, // CRITICAL: Disable tonemapping for proper color reproduction
        MainCamera,
    ));
    println!("[BEVY] ✅ Camera spawned with tonemapping DISABLED - rendering to buffer 0");

    // Scene objects - SUPER BRIGHT AND OBVIOUS
    println!("[BEVY] 🎨 Spawning HIGH-VISIBILITY scene objects...");
    
    // Bright grey ground plane (concrete-like)
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.7, 0.7, 0.7),
            metallic: 0.0,
            perceptual_roughness: 0.8,
            reflectance: 0.1,
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    println!("[BEVY] ✅ Ground plane spawned");

    // Red metallic cube (left) - GAME OBJECT 1
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.9, 0.2, 0.2),
            metallic: 0.8,
            perceptual_roughness: 0.3,
            reflectance: 0.5,
            ..default()
        })),
        Transform::from_xyz(-2.0, 1.0, 0.0),
        GameObjectId(1), // Link to game thread object ID 1
    ));
    println!("[BEVY] ✅ Red metallic cube spawned (Game Object #1)");

    // Blue metallic sphere (right) - GAME OBJECT 2
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.5, 0.9),
            metallic: 0.9,
            perceptual_roughness: 0.1,
            reflectance: 0.9,
            ..default()
        })),
        Transform::from_xyz(2.0, 1.0, 0.0),
        GameObjectId(2), // Link to game thread object ID 2
    ));
    println!("[BEVY] ✅ Blue metallic sphere spawned (Game Object #2)");

    // Gold metallic sphere (top) - GAME OBJECT 3
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.843, 0.0),
            metallic: 0.95,
            perceptual_roughness: 0.2,
            reflectance: 0.8,
            ..default()
        })),
        Transform::from_xyz(0.0, 3.0, 0.0),
        GameObjectId(3), // Link to game thread object ID 3
    ));
    println!("[BEVY] ✅ Gold metallic sphere spawned (Game Object #3)");

    // Green metallic sphere (front) - GAME OBJECT 4
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.9, 0.3),
            metallic: 0.6,
            perceptual_roughness: 0.4,
            reflectance: 0.5,
            ..default()
        })),
        Transform::from_xyz(0.0, 1.0, 2.0),
        GameObjectId(4), // Link to game thread object ID 4
    ));
    println!("[BEVY] ✅ Green metallic sphere spawned (Game Object #4)");

    // Primary directional light (sun)
    commands.spawn((
        DirectionalLight {
            color: Color::WHITE,
            illuminance: 25000.0, // Bright sunlight
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    
    // Fill light (softer, from opposite side)
    commands.spawn((
        DirectionalLight {
            color: Color::srgb(0.9, 0.95, 1.0), // Slightly blue fill
            illuminance: 8000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(-4.0, 6.0, -4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    
    // Ambient light for overall scene brightness
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 500.0, // Subtle ambient
        affects_lightmapped_meshes: true,
    });
    
    println!("[BEVY] ✅ PBR lighting enabled with 2 directional lights + ambient");

    println!("[BEVY] ✅ Scene ready!");
    println!("[BEVY] 🎨 You should see:");
    println!("[BEVY] 🔵 Dark grey-blue background");
    println!("[BEVY] ⬜ Light grey ground plane");
    println!("[BEVY] 🔴 Red metallic cube (left)");
    println!("[BEVY] 🔵 Blue metallic sphere (right)");
    println!("[BEVY] 🟡 Gold metallic sphere (top)");
    println!("[BEVY] 🟢 Green metallic sphere (front)");
    println!("[BEVY] 💡 PBR lighting with 2-point lighting + ambient");
}

// Debug system to track rendering
pub fn debug_rendering_system(
    _query: Query<&Camera, With<MainCamera>>,
    mut _counter: Local<u32>,
) {
    // Any debug info can be printed here
}
