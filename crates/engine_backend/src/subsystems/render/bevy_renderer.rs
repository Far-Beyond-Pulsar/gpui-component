use bevy::{
    app::ScheduleRunnerPlugin,
    camera::RenderTarget,
    core_pipeline::tonemapping::Tonemapping,
    prelude::*,
    render::{
        render_asset::RenderAssets,
        render_graph::{self, NodeRunError, RenderGraph, RenderGraphContext, RenderLabel},
        render_resource::{
            Buffer, BufferDescriptor, BufferUsages, CommandEncoderDescriptor, Extent3d, MapMode,
            PollType, TexelCopyBufferInfo, TexelCopyBufferLayout, TextureFormat, TextureUsages,
        },
        renderer::{RenderContext, RenderDevice, RenderQueue},
        Extract, Render, RenderApp, RenderSystems,
    },
    window::ExitCondition,
    winit::WinitPlugin,
};
use crossbeam_channel::{Receiver, Sender};
use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use super::Framebuffer;

/// Bevy-based GPU renderer that runs on a separate thread with zero-copy rendering
pub struct BevyRenderer {
    receiver: Receiver<Vec<u8>>,
    thread_handle: Option<JoinHandle<()>>,
    width: u32,
    height: u32,
    frame_count: u64,
    running: Arc<AtomicBool>,
}

impl BevyRenderer {
    pub async fn new(width: u32, height: u32) -> Self {
        println!("[BEVY-RENDERER] Starting Bevy renderer on render thread {}x{}", width, height);
        
        let (frame_sender, frame_receiver) = crossbeam_channel::unbounded::<Vec<u8>>();
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        // Spawn Bevy on a dedicated render thread
        let thread_handle = thread::spawn(move || {
            run_bevy_render_thread(width, height, frame_sender, running_clone);
        });

        // Give Bevy time to initialize
        tokio::time::sleep(Duration::from_millis(500)).await;

        println!("[BEVY-RENDERER] Bevy renderer initialized on thread!");

        Self {
            receiver: frame_receiver,
            thread_handle: Some(thread_handle),
            width,
            height,
            frame_count: 0,
            running,
        }
    }

    /// Render and copy the result to framebuffer (zero-copy when possible)
    pub fn render(&mut self, framebuffer: &mut Framebuffer) {
        self.frame_count += 1;
        
        // Try to receive the latest frame (non-blocking)
        if let Ok(image_data) = self.receiver.try_recv() {
            if image_data.len() == framebuffer.buffer.len() {
                // Direct copy - zero-copy from Bevy thread to viewport
                framebuffer.buffer.copy_from_slice(&image_data);
            } else if !image_data.is_empty() {
                // Handle row alignment if needed
                self.copy_with_alignment(&image_data, framebuffer);
            }
        }
        
        if self.frame_count % 60 == 0 {
            println!("[BEVY-RENDERER] Frame {} rendered", self.frame_count);
        }
    }

    fn copy_with_alignment(&self, image_data: &[u8], framebuffer: &mut Framebuffer) {
        let row_bytes = self.width as usize * 4;
        let aligned_row_bytes = RenderDevice::align_copy_bytes_per_row(row_bytes);
        
        if row_bytes == aligned_row_bytes {
            if image_data.len() == framebuffer.buffer.len() {
                framebuffer.buffer.copy_from_slice(image_data);
            }
        } else {
            // Copy row by row, skipping padding
            for y in 0..self.height as usize {
                let src_offset = y * aligned_row_bytes;
                let dst_offset = y * row_bytes;
                if src_offset + row_bytes <= image_data.len() 
                    && dst_offset + row_bytes <= framebuffer.buffer.len() {
                    framebuffer.buffer[dst_offset..dst_offset + row_bytes]
                        .copy_from_slice(&image_data[src_offset..src_offset + row_bytes]);
                }
            }
        }
    }

    pub fn get_frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        // TODO: Send resize command to render thread
        println!("[BEVY-RENDERER] Resize requested: {}x{}", width, height);
    }
}

impl Drop for BevyRenderer {
    fn drop(&mut self) {
        println!("[BEVY-RENDERER] Shutting down render thread...");
        self.running.store(false, Ordering::Relaxed);
        
        if let Some(handle) = self.thread_handle.take() {
            let _ = handle.join();
        }
    }
}

/// Run Bevy app on dedicated render thread
fn run_bevy_render_thread(
    width: u32,
    height: u32,
    frame_sender: Sender<Vec<u8>>,
    running: Arc<AtomicBool>,
) {
    println!("[BEVY-THREAD] Initializing Bevy app on render thread...");

    let mut app = App::new();

    // Configure for headless rendering
    app.insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.15)))
        .add_plugins(
            DefaultPlugins
                .set(ImagePlugin::default_nearest())
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: ExitCondition::DontExit,
                    ..default()
                })
                .disable::<WinitPlugin>(),
        )
        .add_plugins(ImageCopyPlugin { sender: frame_sender })
        .add_plugins(ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / 60.0),
        ));

    // Setup scene
    app.add_systems(Startup, move |mut commands: Commands,
                                    mut meshes: ResMut<Assets<Mesh>>,
                                    mut materials: ResMut<Assets<StandardMaterial>>,
                                    mut images: ResMut<Assets<Image>>,
                                    render_device: Res<RenderDevice>| {
        setup_render_target(&mut commands, &mut images, &render_device, width, height);
        setup_scene(&mut commands, &mut meshes, &mut materials);
    });
    
    app.add_systems(Update, rotate_cube);

    println!("[BEVY-THREAD] Bevy app configured, entering render loop...");

    // Main render loop
    while running.load(Ordering::Relaxed) {
        app.update();
    }

    println!("[BEVY-THREAD] Render thread exiting");
}

/// Setup render target for offscreen rendering
fn setup_render_target(
    commands: &mut Commands,
    images: &mut ResMut<Assets<Image>>,
    render_device: &Res<RenderDevice>,
    width: u32,
    height: u32,
) {
    let size = Extent3d {
        width,
        height,
        ..Default::default()
    };

    // This is the texture that will be rendered to
    let mut render_target_image =
        Image::new_target_texture(size.width, size.height, TextureFormat::bevy_default());
    render_target_image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let render_target_image_handle = images.add(render_target_image);

    // Create image copier component
    commands.spawn(ImageCopier::new(
        render_target_image_handle.clone(),
        size,
        render_device,
    ));

    // Setup camera
    commands.spawn((
        Camera3d::default(),
        Camera {
            target: RenderTarget::Image(render_target_image_handle.into()),
            ..default()
        },
        Tonemapping::None,
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Setup a 3D scene with cube, ground, and axes
fn setup_scene(
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<StandardMaterial>>,
) {
    println!("[BEVY-SCENE] Setting up 3D scene...");

    // Ground plane (circular base)
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));

    // Cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb_u8(124, 144, 255))),
        Transform::from_xyz(0.0, 0.5, 0.0),
        RotatingCube,
    ));

    // Point light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    println!("[BEVY-SCENE] Scene setup complete!");
}

#[derive(Component)]
struct RotatingCube;

fn rotate_cube(time: Res<Time>, mut query: Query<&mut Transform, With<RotatingCube>>) {
    for mut transform in query.iter_mut() {
        transform.rotation = Quat::from_rotation_y(time.elapsed_secs() * 0.5);
    }
}

/// Plugin for Render world part of work
pub struct ImageCopyPlugin {
    sender: Sender<Vec<u8>>,
}

impl Plugin for ImageCopyPlugin {
    fn build(&self, app: &mut App) {
        let (s, r) = crossbeam_channel::unbounded();
        
        let render_app = app
            .insert_resource(MainWorldReceiver(r))
            .sub_app_mut(RenderApp);
        
        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        graph.add_node(ImageCopy, ImageCopyDriver);
        graph.add_node_edge(bevy::render::graph::CameraDriverLabel, ImageCopy);
        
        render_app
            .insert_resource(RenderWorldSender(self.sender.clone()))
            .add_systems(ExtractSchedule, image_copy_extract)
            .add_systems(Render, receive_image_from_buffer.after(RenderSystems::Render));
    }
}

#[derive(Resource, Deref)]
struct MainWorldReceiver(Receiver<Vec<u8>>);

#[derive(Resource, Deref)]
struct RenderWorldSender(Sender<Vec<u8>>);

#[derive(Clone, Component)]
struct ImageCopier {
    buffer: Buffer,
    enabled: Arc<AtomicBool>,
    src_image: Handle<Image>,
}

impl ImageCopier {
    fn new(src_image: Handle<Image>, size: Extent3d, render_device: &RenderDevice) -> Self {
        let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row((size.width) as usize) * 4;
        
        let cpu_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("bevy_renderer_copy_buffer"),
            size: padded_bytes_per_row as u64 * size.height as u64,
            usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        
        Self {
            buffer: cpu_buffer,
            src_image,
            enabled: Arc::new(AtomicBool::new(true)),
        }
    }
    
    fn enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed)
    }
}

#[derive(Clone, Default, Resource, Deref, DerefMut)]
struct ImageCopiers(Vec<ImageCopier>);

fn image_copy_extract(mut commands: Commands, image_copiers: Extract<Query<&ImageCopier>>) {
    commands.insert_resource(ImageCopiers(
        image_copiers.iter().cloned().collect(),
    ));
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, RenderLabel)]
struct ImageCopy;

#[derive(Default)]
struct ImageCopyDriver;

impl render_graph::Node for ImageCopyDriver {
    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let image_copiers = world.get_resource::<ImageCopiers>().unwrap();
        let gpu_images = world
            .get_resource::<RenderAssets<bevy::render::texture::GpuImage>>()
            .unwrap();
        
        for image_copier in image_copiers.iter() {
            if !image_copier.enabled() {
                continue;
            }
            
            let src_image = gpu_images.get(&image_copier.src_image).unwrap();
            let mut encoder = render_context
                .render_device()
                .create_command_encoder(&CommandEncoderDescriptor::default());
            
            let block_dimensions = src_image.texture_format.block_dimensions();
            let block_size = src_image.texture_format.block_copy_size(None).unwrap();
            let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(
                (src_image.size.width as usize / block_dimensions.0 as usize) * block_size as usize,
            );
            
            encoder.copy_texture_to_buffer(
                src_image.texture.as_image_copy(),
                TexelCopyBufferInfo {
                    buffer: &image_copier.buffer,
                    layout: TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(
                            std::num::NonZero::<u32>::new(padded_bytes_per_row as u32)
                                .unwrap()
                                .into(),
                        ),
                        rows_per_image: None,
                    },
                },
                src_image.size,
            );
            
            let render_queue = world.get_resource::<RenderQueue>().unwrap();
            render_queue.submit(std::iter::once(encoder.finish()));
        }
        
        Ok(())
    }
}

fn receive_image_from_buffer(
    image_copiers: Res<ImageCopiers>,
    render_device: Res<RenderDevice>,
    sender: Res<RenderWorldSender>,
) {
    for image_copier in image_copiers.iter() {
        if !image_copier.enabled() {
            continue;
        }
        
        let buffer_slice = image_copier.buffer.slice(..);
        let (_s, r) = crossbeam_channel::bounded(1);
        
        buffer_slice.map_async(MapMode::Read, move |result| match result {
            Ok(_) => _s.send(()).expect("Failed to send map update"),
            Err(err) => panic!("Failed to map buffer {err}"),
        });
        
        render_device
            .poll(PollType::Wait)
            .expect("Failed to poll device");
        
        r.recv().expect("Failed to receive the map_async message");
        let _ = sender.send(buffer_slice.get_mapped_range().to_vec());
        image_copier.buffer.unmap();
    }
}
