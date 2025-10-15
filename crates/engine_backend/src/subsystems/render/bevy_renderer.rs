use bevy::{
    prelude::*,
    render::{
        camera::RenderTarget,
        render_asset::RenderAssetUsages,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        view::RenderLayers,
    },
    window::{PresentMode, WindowMode},
};
use std::sync::{Arc, Mutex};
use wgpu::TextureView;
use super::Framebuffer;

/// Bevy-based GPU renderer that renders to a texture we can copy to framebuffer
pub struct BevyRenderer {
    app: App,
    render_texture_handle: Handle<Image>,
    width: u32,
    height: u32,
    frame_count: u64,
}

impl BevyRenderer {
    pub async fn new(width: u32, height: u32) -> Self {
        println!("[BEVY-RENDERER] Initializing Bevy app {}x{}", width, height);
        
        let mut app = App::new();

        // Add minimal Bevy plugins for headless rendering
        app.add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "Bevy Renderer (Offscreen)".to_string(),
                resolution: (width as f32, height as f32).into(),
                present_mode: PresentMode::AutoNoVsync,
                mode: WindowMode::Windowed,
                visible: false, // Hidden window for offscreen rendering
                ..default()
            }),
            ..default()
        }));

        println!("[BEVY-RENDERER] Setting up 3D scene...");
        
        // Setup the 3D scene
        app.add_systems(Startup, setup_scene);
        app.add_systems(Update, rotate_cube);

        // Create render texture
        let mut images = app.world_mut().resource_mut::<Assets<Image>>();
        
        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };

        let mut render_texture = Image {
            texture_descriptor: TextureDescriptor {
                label: None,
                size,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                mip_level_count: 1,
                sample_count: 1,
                usage: TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST
                    | TextureUsages::RENDER_ATTACHMENT
                    | TextureUsages::COPY_SRC,
                view_formats: &[],
            },
            ..default()
        };

        render_texture.resize(size);
        let render_texture_handle = images.add(render_texture);

        println!("[BEVY-RENDERER] Bevy renderer initialized!");

        Self {
            app,
            render_texture_handle,
            width,
            height,
            frame_count: 0,
        }
    }

    /// Update the Bevy app and render a frame
    pub fn render(&mut self, framebuffer: &mut Framebuffer) {
        self.frame_count += 1;
        
        if self.frame_count % 60 == 0 {
            println!("[BEVY-RENDERER] Frame {} - Updating Bevy app", self.frame_count);
        }
        
        // Update Bevy
        self.app.update();

        // TODO: Extract texture data from render world
        // For now, render a placeholder showing Bevy is working
        self.render_placeholder(framebuffer);
    }

    fn render_placeholder(&self, framebuffer: &mut Framebuffer) {
        // Render a distinct pattern showing this is Bevy
        let time = (self.frame_count as f32) * 0.016;
        
        for y in 0..framebuffer.height {
            for x in 0..framebuffer.width {
                let idx = ((y * framebuffer.width + x) * 4) as usize;
                
                let u = x as f32 / framebuffer.width as f32;
                let v = y as f32 / framebuffer.height as f32;
                
                // Bevy bird colors! (distinctive pattern)
                let r = ((u * 3.0 + time).sin() * 100.0 + 155.0) as u8;
                let g = ((v * 3.0 + time * 1.5).cos() * 100.0 + 155.0) as u8;
                let b = (((u + v) * 3.0 + time * 0.8).sin() * 100.0 + 155.0) as u8;
                
                framebuffer.buffer[idx] = r;
                framebuffer.buffer[idx + 1] = g;
                framebuffer.buffer[idx + 2] = b;
                framebuffer.buffer[idx + 3] = 255;
            }
        }
    }

    pub fn get_frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        
        // TODO: Recreate render texture with new dimensions
    }
}

/// Setup a basic 3D scene in Bevy
fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 2.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });

    // Light
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            intensity: 10_000_000.0,
            range: 100.0,
            ..default()
        },
        transform: Transform::from_xyz(4.0, 8.0, 4.0),
        ..default()
    });

    // Ground plane
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(10.0, 10.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.5, 0.3),
            perceptual_roughness: 0.8,
            ..default()
        }),
        ..default()
    });

    // Cube
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(1.0, 1.0, 1.0)),
            material: materials.add(StandardMaterial {
                base_color: Color::srgb(0.8, 0.4, 0.2),
                metallic: 0.2,
                perceptual_roughness: 0.5,
                ..default()
            }),
            transform: Transform::from_xyz(0.0, 0.5, 0.0),
            ..default()
        },
        RotatingCube,
    ));

    // X axis (red)
    commands.spawn(PbrBundle {
        mesh: meshes.add(Capsule3d::new(0.05, 2.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 0.0),
            emissive: LinearRgba::rgb(1.0, 0.0, 0.0),
            ..default()
        }),
        transform: Transform::from_xyz(1.0, 0.0, 0.0)
            .with_rotation(Quat::from_rotation_z(std::f32::consts::PI / 2.0)),
        ..default()
    });

    // Y axis (green)
    commands.spawn(PbrBundle {
        mesh: meshes.add(Capsule3d::new(0.05, 2.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.0, 1.0, 0.0),
            emissive: LinearRgba::rgb(0.0, 1.0, 0.0),
            ..default()
        }),
        transform: Transform::from_xyz(0.0, 1.0, 0.0),
        ..default()
    });

    // Z axis (blue)
    commands.spawn(PbrBundle {
        mesh: meshes.add(Capsule3d::new(0.05, 2.0)),
        material: materials.add(StandardMaterial {
            base_color: Color::srgb(0.0, 0.0, 1.0),
            emissive: LinearRgba::rgb(0.0, 0.0, 1.0),
            ..default()
        }),
        transform: Transform::from_xyz(0.0, 0.0, 1.0)
            .with_rotation(Quat::from_rotation_x(std::f32::consts::PI / 2.0)),
        ..default()
    });
}

#[derive(Component)]
struct RotatingCube;

fn rotate_cube(time: Res<Time>, mut query: Query<&mut Transform, With<RotatingCube>>) {
    for mut transform in query.iter_mut() {
        transform.rotation = Quat::from_rotation_y(time.elapsed_seconds() * 0.5);
    }
}
