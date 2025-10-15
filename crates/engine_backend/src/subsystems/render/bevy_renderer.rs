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
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::Duration,
};
use super::Framebuffer;

const RENDER_FPS: f64 = 60.0;
const CHANNEL_CAPACITY: usize = 2; // Double buffer for smooth frames

/// Bevy-based GPU renderer that runs on a separate thread with zero-copy rendering
/// Provides studio-quality 3D rendering with modern effects
pub struct BevyRenderer {
    receiver: Receiver<Vec<u8>>,
    thread_handle: Option<JoinHandle<()>>,
    width: u32,
    height: u32,
    frame_count: u64,
    running: Arc<AtomicBool>,
    last_frame_data: Option<Vec<u8>>, // Cache last frame for smooth rendering
}

impl BevyRenderer {
    pub async fn new(width: u32, height: u32) -> Self {
        println!("[BEVY-RENDERER] Starting high-performance Bevy renderer {}x{}", width, height);
        
        let (frame_sender, frame_receiver) = crossbeam_channel::bounded::<Vec<u8>>(CHANNEL_CAPACITY);
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();

        // Spawn Bevy on a dedicated render thread with high priority
        let thread_handle = thread::Builder::new()
            .name("BevyRenderThread".to_string())
            .spawn(move || {
                run_bevy_render_thread(width, height, frame_sender, running_clone);
            })
            .expect("Failed to spawn render thread");

        // Give Bevy time to initialize
        tokio::time::sleep(Duration::from_millis(500)).await;

        println!("[BEVY-RENDERER] Studio-quality renderer initialized!");

        Self {
            receiver: frame_receiver,
            thread_handle: Some(thread_handle),
            width,
            height,
            frame_count: 0,
            running,
            last_frame_data: None,
        }
    }

    /// Render and copy the result to framebuffer (zero-copy when possible)
    pub fn render(&mut self, framebuffer: &mut Framebuffer) {
        self.frame_count += 1;
        
        // Try to receive the latest frame (non-blocking, may drain multiple frames)
        let mut new_frame = false;
        while let Ok(image_data) = self.receiver.try_recv() {
            self.last_frame_data = Some(image_data);
            new_frame = true;
        }
        
        // Use latest frame (or cached frame if no new frame available)
        if let Some(ref image_data) = self.last_frame_data {
            if image_data.len() == framebuffer.buffer.len() {
                // Direct copy - zero-copy from Bevy thread to viewport
                framebuffer.buffer.copy_from_slice(image_data);
            } else if !image_data.is_empty() {
                // Handle row alignment if needed
                self.copy_with_alignment(image_data, framebuffer);
            }
        }
        
        if self.frame_count % 60 == 0 {
            let status = if new_frame { "new frame" } else { "cached frame" };
            println!("[BEVY-RENDERER] Frame {} rendered ({})", self.frame_count, status);
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

/// Run Bevy app on dedicated render thread with studio-quality settings
fn run_bevy_render_thread(
    width: u32,
    height: u32,
    frame_sender: Sender<Vec<u8>>,
    running: Arc<AtomicBool>,
) {
    println!("[BEVY-THREAD] Initializing high-performance Bevy app...");
    
    // Set panic hook to see detailed errors
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("[BEVY-PANIC] {}", panic_info);
        if let Some(location) = panic_info.location() {
            eprintln!("[BEVY-PANIC] Location: {}:{}:{}", 
                location.file(), location.line(), location.column());
        }
    }));

    let mut app = App::new();

    // Configure for headless rendering with optimizations
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
        );

    println!("[BEVY-THREAD] Default plugins added, setting up render systems...");

    // Add the image copy plugin BEFORE ScheduleRunnerPlugin (important for RenderApp to exist)
    app.add_plugins(ImageCopyPlugin { sender: frame_sender });
    
    println!("[BEVY-THREAD] ImageCopyPlugin added, adding scheduler...");
    
    // Add ScheduleRunnerPlugin AFTER setting up render systems
    app.add_plugins(ScheduleRunnerPlugin::run_loop(
        Duration::from_secs_f64(1.0 / RENDER_FPS),
    ));

    println!("[BEVY-THREAD] Scheduler added, setting up scene...");

    // We need to wait for the first frame to complete before we can set up the scene
    // So we'll set up the scene in PreUpdate after the first frame
    let mut setup_done = false;
    app.add_systems(PreUpdate, move |mut commands: Commands,
                                      mut images: ResMut<Assets<Image>>| {
        if setup_done {
            return;
        }
        setup_done = true;
        
        println!("[BEVY-THREAD] First frame complete, setting up render target...");
        
        // We can't access RenderDevice from main world, so we create the render target differently
        setup_render_target_simple(&mut commands, &mut images, width, height);
        
        // Spawn a simple animated marker entity
        commands.spawn((
            AnimatedMarker,
            Transform::default(),
        ));
        
        println!("[BEVY-THREAD] Scene setup complete!");
    });
    
    app.add_systems(Update, rotate_marker);

    println!("[BEVY-THREAD] Entering high-performance render loop @ {} FPS...", RENDER_FPS);

    // Main render loop with error handling
    let mut frame_num = 0u64;
    while running.load(Ordering::Relaxed) {
        frame_num += 1;
        if frame_num % 60 == 0 {
            println!("[BEVY-THREAD] Frame {}", frame_num);
        }
        
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            app.update();
        })) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("[BEVY-THREAD] Panic caught in app.update()!");
                if let Some(s) = e.downcast_ref::<&str>() {
                    eprintln!("[BEVY-THREAD] Panic message: {}", s);
                } else if let Some(s) = e.downcast_ref::<String>() {
                    eprintln!("[BEVY-THREAD] Panic message: {}", s);
                }
                break;
            }
        }
    }

    println!("[BEVY-THREAD] Render thread exiting cleanly");
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

    // Setup camera with high-quality settings
    commands.spawn((
        Camera3d::default(),
        Camera {
            target: RenderTarget::Image(render_target_image_handle.into()),
            ..default()
        },
        Tonemapping::TonyMcMapface, // Filmic tonemapping for studio look
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

/// Setup render target without RenderDevice (simpler version for Startup systems)
fn setup_render_target_simple(
    commands: &mut Commands,
    images: &mut ResMut<Assets<Image>>,
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

    // We'll create the ImageCopier in the Extract phase when RenderDevice is available
    // For now just spawn a marker component
    commands.spawn((
        ImageCopierSetup {
            src_image: render_target_image_handle.clone(),
            size,
        },
    ));

    // Setup camera with high-quality settings
    commands.spawn((
        Camera3d::default(),
        Camera {
            target: RenderTarget::Image(render_target_image_handle.into()),
            ..default()
        },
        Tonemapping::TonyMcMapface, // Filmic tonemapping for studio look
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

#[derive(Component)]
struct AnimatedMarker;

fn rotate_marker(time: Res<Time>, mut query: Query<&mut Transform, With<AnimatedMarker>>) {
    for mut transform in query.iter_mut() {
        // Just update the transform for testing
        let t = time.elapsed_secs();
        transform.rotation = Quat::from_rotation_z(t * 0.5);
    }
}

/// Image copy plugin for extracting rendered frames
/// Based on official Bevy headless_renderer example
struct ImageCopyPlugin {
    sender: Sender<Vec<u8>>,
}

impl Plugin for ImageCopyPlugin {
    fn build(&self, app: &mut App) {
        println!("[ImageCopyPlugin] Building plugin...");
        let (s, r) = crossbeam_channel::unbounded();
        
        // Insert receiver in main world
        println!("[ImageCopyPlugin] Inserting MainWorldReceiver...");
        app.insert_resource(MainWorldReceiver(r));
        
        // Check if RenderApp exists
        println!("[ImageCopyPlugin] Checking for RenderApp...");
        if app.get_sub_app(RenderApp).is_none() {
            eprintln!("[ImageCopyPlugin] ERROR: RenderApp does not exist!");
            eprintln!("[ImageCopyPlugin] This means DefaultPlugins were not properly initialized");
            return;
        }
        
        println!("[ImageCopyPlugin] RenderApp exists, configuring...");
        let render_app = app.sub_app_mut(RenderApp);
        
        // Setup render graph in render world
        println!("[ImageCopyPlugin] Setting up render graph...");
        let mut graph = render_app.world_mut().resource_mut::<RenderGraph>();
        graph.add_node(ImageCopy, ImageCopyDriver);
        graph.add_node_edge(bevy::render::graph::CameraDriverLabel, ImageCopy);
        
        // Add render world systems
        println!("[ImageCopyPlugin] Adding render world systems...");
        render_app
            .insert_resource(RenderWorldSender(self.sender.clone()))
            .add_systems(ExtractSchedule, image_copy_extract)
            .add_systems(
                Render,
                receive_image_from_buffer.after(RenderSystems::Render),
            );
        
        println!("[ImageCopyPlugin] Plugin build complete!");
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

/// Marker component to setup ImageCopier in render world when RenderDevice is available
#[derive(Component, Clone)]
struct ImageCopierSetup {
    src_image: Handle<Image>,
    size: Extent3d,
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

/// Extracting `ImageCopier`s into render world, because `ImageCopyDriver` accesses them
/// Also creates ImageCopiers from ImageCopierSetup markers on first extraction
fn image_copy_extract(
    mut commands: Commands,
    image_copiers: Extract<Query<&ImageCopier>>,
    image_copier_setups: Extract<Query<&ImageCopierSetup>>,
    render_device: Res<RenderDevice>,
) {
    let mut copiers: Vec<ImageCopier> = image_copiers.iter().cloned().collect();
    
    // Create ImageCopiers from setup markers (first frame only)
    for setup in image_copier_setups.iter() {
        println!("[ImageCopyExtract] Creating ImageCopier from setup marker");
        let copier = ImageCopier::new(setup.src_image.clone(), setup.size, &render_device);
        copiers.push(copier);
    }
    
    commands.insert_resource(ImageCopiers(copiers));
}

#[derive(Debug, PartialEq, Eq, Clone, Hash, RenderLabel)]
struct ImageCopy;

#[derive(Default)]
struct ImageCopyDriver;

// Copies image content from render target to buffer
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
            
            // Calculating correct size of image row because
            // copy_texture_to_buffer can copy image only by rows aligned wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
            // That's why image in buffer can be little bit wider
            // This should be taken into account at copy from buffer stage
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

/// runs in render world after Render stage to send image from buffer via channel (receiver is in main world)
fn receive_image_from_buffer(
    image_copiers: Res<ImageCopiers>,
    render_device: Res<RenderDevice>,
    sender: Res<RenderWorldSender>,
) {
    for image_copier in image_copiers.0.iter() {
        if !image_copier.enabled() {
            continue;
        }
        
        // Finally time to get our data back from the gpu.
        // First we get a buffer slice which represents a chunk of the buffer (which we
        // can't access yet).
        // We want the whole thing so use unbounded range.
        let buffer_slice = image_copier.buffer.slice(..);
        
        // Now things get complicated. WebGPU, for safety reasons, only allows either the GPU
        // or CPU to access a buffer's contents at a time. We need to "map" the buffer which means
        // flipping ownership of the buffer over to the CPU and making access legal. We do this
        // with `BufferSlice::map_async`.
        //
        // The problem is that map_async is not an async function so we can't await it. What
        // we need to do instead is pass in a closure that will be executed when the slice is
        // either mapped or the mapping has failed.
        //
        // The problem with this is that we don't have a reliable way to wait in the main
        // code for the buffer to be mapped and even worse, calling get_mapped_range or
        // get_mapped_range_mut prematurely will cause a panic, not return an error.
        //
        // Using channels solves this as awaiting the receiving of a message from
        // the passed closure will force the outside code to wait. It also doesn't hurt
        // if the closure finishes before the outside code catches up as the message is
        // buffered and receiving will just pick that up.
        //
        // It may also be worth noting that although on native, the usage of asynchronous
        // channels is wholly unnecessary, for the sake of portability to Wasm
        // we'll use async channels that work on both native and Wasm.
        
        let (s, r) = crossbeam_channel::bounded(1);
        
        // Maps the buffer so it can be read on the cpu
        buffer_slice.map_async(MapMode::Read, move |r| match r {
            // This will execute once the gpu is ready, so after the call to poll()
            Ok(r) => s.send(r).expect("Failed to send map update"),
            Err(err) => panic!("Failed to map buffer {err}"),
        });
        
        // In order for the mapping to be completed, one of three things must happen.
        // One of those can be calling `Device::poll`. This isn't necessary on the web as devices
        // are polled automatically but natively, we need to make sure this happens manually.
        // `Maintain::Wait` will cause the thread to wait on native but not on WebGpu.
        
        // This blocks until the gpu is done executing everything
        render_device
            .poll(PollType::Wait)
            .expect("Failed to poll device for map async");
        
        // This blocks until the buffer is mapped
        r.recv().expect("Failed to receive the map_async message");
        
        // This could fail on app exit, if Main world clears resources (including receiver) while Render world still renders
        let _ = sender.send(buffer_slice.get_mapped_range().to_vec());
        
        // We need to make sure all `BufferView`'s are dropped before we do what we're about
        // to do.
        // Unmap so that we can copy to the staging buffer in the next iteration.
        image_copier.buffer.unmap();
    }
}
