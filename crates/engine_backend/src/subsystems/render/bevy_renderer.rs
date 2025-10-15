// Bevy headless renderer - following EXACT official example pattern
// This is the ONLY way that works reliably with Bevy 0.17.2

use bevy::{
    app::ScheduleRunnerPlugin,
    camera::RenderTarget,
    core_pipeline::tonemapping::Tonemapping,
    image::TextureFormatPixelInfo,
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
    thread,
    time::Duration,
};
use super::Framebuffer;

/// Bevy renderer that provides frames to external viewport
/// Uses official Bevy pattern - app.run() blocks, so we run it on a thread
pub struct BevyRenderer {
    frame_receiver: Receiver<Vec<u8>>,
    running: Arc<AtomicBool>,
    width: u32,
    height: u32,
    frame_count: u64,
    last_frame: Option<Vec<u8>>,
}

impl BevyRenderer {
    pub async fn new(width: u32, height: u32) -> Self {
        println!("[BevyRenderer] Creating renderer {}x{}", width, height);
        
        let (frame_sender, frame_receiver) = crossbeam_channel::unbounded();
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = running.clone();
        
        // Spawn Bevy app on dedicated thread - this is required because app.run() blocks
        thread::spawn(move || {
            run_bevy_app(width, height, frame_sender, running_clone);
        });
        
        // Wait a bit for initialization
        tokio::time::sleep(Duration::from_millis(500)).await;
        
        println!("[BevyRenderer] Renderer initialized");
        
        Self {
            frame_receiver,
            running,
            width,
            height,
            frame_count: 0,
            last_frame: None,
        }
    }
    
    pub fn render(&mut self, framebuffer: &mut Framebuffer) {
        self.frame_count += 1;
        
        // Drain all pending frames, keep the latest
        let mut got_frame = false;
        while let Ok(frame) = self.frame_receiver.try_recv() {
            got_frame = true;
            self.last_frame = Some(frame);
        }
        
        // Copy latest frame to framebuffer with alignment handling
        if let Some(ref frame_data) = self.last_frame {
            // Calculate expected sizes
            let row_bytes = self.width as usize * 4;
            let aligned_row_bytes = RenderDevice::align_copy_bytes_per_row(row_bytes);
            let expected_size = aligned_row_bytes * self.height as usize;
            
            if frame_data.len() == expected_size {
                // Handle row alignment - copy row by row, stripping padding
                for y in 0..self.height as usize {
                    let src_offset = y * aligned_row_bytes;
                    let dst_offset = y * framebuffer.width as usize * 4;
                    let copy_len = (self.width as usize * 4).min(framebuffer.width as usize * 4);
                    
                    if src_offset + row_bytes <= frame_data.len() 
                        && dst_offset + copy_len <= framebuffer.buffer.len() {
                        framebuffer.buffer[dst_offset..dst_offset + copy_len]
                            .copy_from_slice(&frame_data[src_offset..src_offset + copy_len]);
                    }
                }
                
                if self.frame_count % 60 == 0 {
                    println!("[BevyRenderer] Frame {} - copied with alignment ({}x{} → {}x{})", 
                             self.frame_count, self.width, self.height, framebuffer.width, framebuffer.height);
                }
            } else {
                if self.frame_count % 60 == 0 {
                    println!("[BevyRenderer] Frame {} - size mismatch: {} vs {} expected", 
                             self.frame_count, frame_data.len(), expected_size);
                }
            }
        } else {
            if self.frame_count % 60 == 0 {
                println!("[BevyRenderer] Frame {} - no frame data yet", self.frame_count);
            }
        }
        
        if got_frame && self.frame_count % 60 == 0 {
            println!("[BevyRenderer] Received new frame from Bevy!");
        }
    }
    
    pub fn get_frame_count(&self) -> u64 {
        self.frame_count
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
}

impl Drop for BevyRenderer {
    fn drop(&mut self) {
        println!("[BevyRenderer] Shutting down");
        self.running.store(false, Ordering::Relaxed);
    }
}

// This follows the EXACT structure of the official Bevy headless_renderer example
fn run_bevy_app(width: u32, height: u32, frame_sender: Sender<Vec<u8>>, running: Arc<AtomicBool>) {
    println!("[BevyApp] Starting Bevy app");
    
    let mut app = App::new();
    
    app.insert_resource(FrameConfig { width, height })
        .insert_resource(ClearColor(Color::srgb(0.1, 0.1, 0.15)))
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
        .add_plugins(ScheduleRunnerPlugin::run_loop(Duration::from_secs_f64(1.0 / 60.0)))
        .add_systems(Startup, setup);
    
    // Run the app - this blocks until stopped
    println!("[BevyApp] Running app loop");
    
    // Since app.run() blocks forever, we can't check the running flag
    // The app will just keep running until the process exits
    app.run();
}

#[derive(Resource)]
struct FrameConfig {
    width: u32,
    height: u32,
}

// Setup function - called during Startup
// This is where we can access RenderDevice because Bevy has initialized everything
fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    config: Res<FrameConfig>,
    render_device: Res<RenderDevice>,
) {
    println!("[BevyApp] Setup called");
    
    let size = Extent3d {
        width: config.width,
        height: config.height,
        ..Default::default()
    };
    
    // Create render target
    let mut render_target_image =
        Image::new_target_texture(size.width, size.height, TextureFormat::bevy_default());
    render_target_image.texture_descriptor.usage |= TextureUsages::COPY_SRC;
    let render_target_handle = images.add(render_target_image);
    
    // Create image copier
    commands.spawn(ImageCopier::new(
        render_target_handle.clone(),
        size,
        &render_device,
    ));
    
    // Create camera
    commands.spawn((
        Camera2d,
        Camera {
            target: RenderTarget::Image(render_target_handle.into()),
            ..default()
        },
    ));
    
    println!("[BevyApp] Setup complete");
}

// Image copy plugin - extracts frames from render world
struct ImageCopyPlugin {
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
        let padded_bytes_per_row = RenderDevice::align_copy_bytes_per_row(size.width as usize) * 4;
        
        let cpu_buffer = render_device.create_buffer(&BufferDescriptor {
            label: Some("frame_copy_buffer"),
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
        
        println!("[ImageCopyDriver] Running - {} copiers", image_copiers.len());
        
        for image_copier in image_copiers.iter() {
            if !image_copier.enabled() {
                println!("[ImageCopyDriver] Copier disabled, skipping");
                continue;
            }
            
            let Some(src_image) = gpu_images.get(&image_copier.src_image) else {
                println!("[ImageCopyDriver] Source image not found in GPU");
                continue;
            };
            
            println!("[ImageCopyDriver] Copying {}x{} texture", 
                     src_image.size.width, src_image.size.height);
            
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
            
            println!("[ImageCopyDriver] Copy command submitted");
        }
        
        Ok(())
    }
}

fn receive_image_from_buffer(
    image_copiers: Res<ImageCopiers>,
    render_device: Res<RenderDevice>,
    sender: Res<RenderWorldSender>,
) {
    for image_copier in image_copiers.0.iter() {
        if !image_copier.enabled() {
            continue;
        }
        
        let buffer_slice = image_copier.buffer.slice(..);
        let (s, r) = crossbeam_channel::bounded(1);
        
        buffer_slice.map_async(MapMode::Read, move |result| match result {
            Ok(_) => { 
                println!("[BevyRender] Buffer mapped successfully");
                let _ = s.send(()); 
            },
            Err(err) => eprintln!("[BevyRenderer] Buffer map failed: {}", err),
        });
        
        match render_device.poll(PollType::Wait) {
            Ok(_) => {},
            Err(e) => {
                eprintln!("[BevyRenderer] Poll failed: {:?}", e);
                continue;
            }
        }
        
        if r.recv().is_ok() {
            let data = buffer_slice.get_mapped_range().to_vec();
            println!("[BevyRender] Sending {} bytes to main thread", data.len());
            let _ = sender.send(data);
            image_copier.buffer.unmap();
        } else {
            eprintln!("[BevyRenderer] Failed to receive map completion");
        }
    }
}
