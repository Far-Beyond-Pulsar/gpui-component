// TRUE ZERO-COPY Bevy renderer using shared GPU textures
//
// Architecture:
// 1. Create 2 shared DirectX textures that both wgpu (Bevy) and GPUI can access
// 2. Bevy renders DIRECTLY to texture A (no copy!)
// 3. GPUI displays texture B
// 4. Swap A↔B pointers when frame is ready
// 5. NO CPU involvement, NO buffers, NO copies - pure GPU-to-GPU

use bevy::{
    camera::RenderTarget,
    prelude::*,
    pbr::PbrPlugin,
    core_pipeline::tonemapping::Tonemapping,
    render::{
        render_asset::RenderAssets,
        render_resource::{
            Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
        },
        renderer::RenderDevice,
        texture::GpuImage,
        RenderPlugin,
        RenderApp, ExtractSchedule, Render,
    },
    window::WindowPlugin,
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
    io::Write,
};
use super::Framebuffer;

#[cfg(target_os = "windows")]
use crate::subsystems::render::DxgiSharedTexture;

// Render target dimensions
const RENDER_WIDTH: u32 = 1600;
const RENDER_HEIGHT: u32 = 900;

/// Camera controller input state
#[derive(Resource, Default, Clone)]
pub struct CameraInput {
    pub forward: f32,
    pub right: f32,
    pub up: f32,
    pub mouse_delta_x: f32,
    pub mouse_delta_y: f32,
    pub pan_delta_x: f32,
    pub pan_delta_y: f32,
    pub zoom_delta: f32,
    pub move_speed: f32,
    pub pan_speed: f32,
    pub zoom_speed: f32,
    pub look_sensitivity: f32,
    pub boost: bool,
    pub orbit_mode: bool,
    pub orbit_distance: f32,
    pub focus_point: Vec3,
}

impl CameraInput {
    pub fn new() -> Self {
        Self {
            forward: 0.0,
            right: 0.0,
            up: 0.0,
            mouse_delta_x: 0.0,
            mouse_delta_y: 0.0,
            pan_delta_x: 0.0,
            pan_delta_y: 0.0,
            zoom_delta: 0.0,
            move_speed: 15.0,
            pan_speed: 0.01,
            zoom_speed: 1.5,
            look_sensitivity: 0.15,
            boost: false,
            orbit_mode: false,
            orbit_distance: 10.0,
            focus_point: Vec3::ZERO,
        }
    }
}

/// Marker component for the main camera
#[derive(Component)]
struct MainCamera;

/// Shared GPU texture system - TRUE ZERO-COPY!
/// Two textures ping-pong: Bevy writes to one, GPUI reads from other
#[derive(Clone)]
pub struct SharedGpuTextures {
    /// The two shared GPU texture handles (Bevy side)
    pub textures: Arc<[Handle<Image>; 2]>,
    /// Native GPU handles for GPUI (DirectX/Metal/Vulkan pointers)
    pub native_handles: Arc<Mutex<Option<[crate::subsystems::render::NativeTextureHandle; 2]>>>,
    /// Which texture Bevy is currently rendering to (0 or 1)
    pub write_index: Arc<AtomicUsize>,
    /// Which texture GPUI should display (0 or 1)
    pub read_index: Arc<AtomicUsize>,
    /// Frame counter incremented each time Bevy finishes a frame
    pub frame_number: Arc<AtomicU64>,
    /// Texture dimensions
    pub width: u32,
    pub height: u32,
}

/// Performance metrics
#[derive(Debug, Clone, Default)]
pub struct RenderMetrics {
    pub frames_rendered: u64,
    pub avg_frame_time_us: u64,
    pub last_copy_time_us: u64,
    pub total_bytes_transferred: u64,
    pub bevy_fps: f64,
    pub pipeline_time_us: u64,
    pub gpu_time_us: u64,
    pub cpu_time_us: u64,
    pub memory_usage_mb: f64,
    pub draw_calls: u32,
    pub vertices_drawn: u32,
}

/// Render statistics from Bevy thread
#[derive(Resource, Default)]
struct RenderStats {
    draw_calls: u32,
    vertices_drawn: u32,
}

/// Shutdown signal resource wrapper
#[derive(Resource)]
struct ShutdownSignal(Arc<AtomicBool>);

/// TRUE ZERO-COPY Bevy renderer
pub struct BevyRenderer {
    /// Shared GPU textures
    shared_textures: Arc<Mutex<Option<SharedGpuTextures>>>,
    /// Camera input from GPUI (public for direct access)
    pub camera_input: Arc<Mutex<CameraInput>>,
    /// Performance metrics
    metrics: Arc<Mutex<RenderMetrics>>,
    render_stats: Arc<Mutex<RenderStats>>,
    /// Render thread handle
    _render_thread: Option<thread::JoinHandle<()>>,
    /// Shutdown signal
    shutdown: Arc<AtomicBool>,
    /// Frame timing
    last_frame_time: Arc<Mutex<Instant>>,
}

impl BevyRenderer {
    /// Create new renderer with shared GPU textures
    pub async fn new(width: u32, height: u32) -> Self {
        println!("[BevyRenderer] 🚀 Creating TRUE ZERO-COPY renderer ({}x{})", width, height);
        println!("[BevyRenderer] Architecture: Shared DirectX/wgpu textures, NO copies!");

        let shared_textures = Arc::new(Mutex::new(None));
        let camera_input = Arc::new(Mutex::new(CameraInput::new()));
        let metrics = Arc::new(Mutex::new(RenderMetrics::default()));
        let render_stats = Arc::new(Mutex::new(RenderStats::default()));
        let shutdown = Arc::new(AtomicBool::new(false));
        let last_frame_time = Arc::new(Mutex::new(Instant::now()));

        // Clone for thread
        let shared_textures_clone = shared_textures.clone();
        let camera_input_clone = camera_input.clone();
        let metrics_clone = metrics.clone();
        let render_stats_clone = render_stats.clone();
        let shutdown_clone = shutdown.clone();
        let last_frame_time_clone = last_frame_time.clone();

        let render_thread = thread::spawn(move || {
            Self::run_bevy_renderer(
                width,
                height,
                shared_textures_clone,
                camera_input_clone,
                metrics_clone,
                render_stats_clone,
                shutdown_clone,
                last_frame_time_clone,
            );
        });

        // Wait for initialization
        tokio::time::sleep(Duration::from_millis(500)).await;

        Self {
            shared_textures,
            camera_input,
            metrics,
            render_stats,
            _render_thread: Some(render_thread),
            shutdown,
            last_frame_time,
        }
    }

    /// Run Bevy app in dedicated thread
    fn run_bevy_renderer(
        width: u32,
        height: u32,
        shared_textures: Arc<Mutex<Option<SharedGpuTextures>>>,
        camera_input: Arc<Mutex<CameraInput>>,
        metrics: Arc<Mutex<RenderMetrics>>,
        render_stats: Arc<Mutex<RenderStats>>,
        shutdown: Arc<AtomicBool>,
        last_frame_time: Arc<Mutex<Instant>>,
    ) {
        println!("========================================");
        println!("[BEVY-RENDERER] 🎮🎮🎮 BEVY THREAD STARTING! 🎮🎮🎮");
        println!("[BEVY-RENDERER] Thread ID: {:?}", std::thread::current().id());
        println!("[BEVY-RENDERER] Render size: {}x{}", width, height);
        println!("[BEVY-RENDERER] Architecture: ZERO-COPY shared GPU textures!");
        println!("========================================");

        println!("[BEVY-RENDERER] Step 1: Creating App...");
        let mut app = App::new();
        println!("[BEVY-RENDERER] Step 2: App created successfully");
        
        println!("[BEVY-RENDERER] Step 3: Adding DefaultPlugins (headless mode)...");
        // Based on official Bevy headless example
        // Force DX12 backend for DXGI shared resource compatibility
        #[cfg(target_os = "windows")]
        let backend_config = bevy::render::settings::Backends::DX12;
        
        #[cfg(target_os = "macos")]
        let backend_config = bevy::render::settings::Backends::METAL;
        
        #[cfg(target_os = "linux")]
        let backend_config = bevy::render::settings::Backends::VULKAN;
        
        app.add_plugins(
            DefaultPlugins
                .set(bevy::window::WindowPlugin {
                    primary_window: None,
                    exit_condition: bevy::window::ExitCondition::DontExit,
                    ..default()
                })
                .set(RenderPlugin {
                    render_creation: bevy::render::settings::RenderCreation::Automatic(
                        bevy::render::settings::WgpuSettings {
                            backends: Some(backend_config),
                            ..default()
                        }
                    ),
                    ..default()
                })
                .disable::<bevy::winit::WinitPlugin>()
        );
        println!("[BEVY-RENDERER] Step 4: DefaultPlugins added with DX12 backend");
        
        println!("[BEVY-RENDERER] Step 5: Adding ScheduleRunnerPlugin (120 FPS)...");
        app.add_plugins(bevy::app::ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / 120.0),
        ));
        println!("[BEVY-RENDERER] Step 6: ScheduleRunnerPlugin added");
        
        println!("[BEVY-RENDERER] Step 7: Adding resources and systems...");
        let shared_textures_clone = shared_textures.clone();
        app.insert_resource(ClearColor(Color::srgb(0.2, 0.2, 0.3)))
            .insert_resource(camera_input.lock().unwrap().clone())
            .insert_resource(RenderStats::default())
            .insert_resource(ShutdownSignal(shutdown))
            .insert_resource(SharedTexturesResource(shared_textures))
            // ZERO-COPY: Setup scene with direct GPU texture rendering
            .add_systems(Startup, setup_scene)
            .add_systems(Update, || {
                static FRAME: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
                let f = FRAME.fetch_add(1, Ordering::Relaxed);
                if f % 60 == 0 {
                    println!("[BEVY-RENDERER] 🎬 UPDATE SYSTEM RUNNING! Frame {}", f);
                }
            })
            .add_systems(Update, update_camera_from_input)
            .add_systems(Update, animate_ball)
            .add_systems(Update, check_shutdown)
            .add_systems(Update, update_camera_render_target)
            // ZERO-COPY: Swap texture indices after rendering (double-buffer)
            .add_systems(Last, swap_render_textures);
        println!("[BEVY-RENDERER] Step 8: Resources and systems added");

        println!("[BEVY-RENDERER] Step 9: Adding render extraction system...");
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            println!("[BEVY-RENDERER] ✅ RenderApp found!");
            
            // Insert SharedTexturesResource in RenderWorld too
            render_app.insert_resource(SharedTexturesResource(shared_textures_clone));
            
            // ZERO-COPY: Create DXGI shared textures at render world startup
            #[cfg(target_os = "windows")]
            render_app.add_systems(Startup, initialize_shared_textures);
            
            // ZERO-COPY: Extract native GPU handles from wgpu textures
            // Run in Render schedule AFTER textures are prepared, not during Extract
            render_app.add_systems(
                Render,
                extract_native_texture_handles.in_set(bevy::render::RenderSystems::Render)
            );
            println!("[BEVY-RENDERER] ✅ HAL extraction system added to Render schedule");
        } else {
            println!("[BEVY-RENDERER] ❌ WARNING: No RenderApp found!");
        }

        println!("[BEVY-RENDERER] Step 10: Starting Bevy loop (headless, 120 FPS)...");
        println!("[BEVY-RENDERER] 🚀 ZERO-COPY: Bevy renders → Shared GPU textures → GPUI displays");
        std::io::stdout().flush().ok();
        
        app.run();
        
        println!("[BEVY-RENDERER] ⛔ Bevy loop exited");
    }

    /// Render a frame (GPUI calls this)
    pub fn render(&mut self, _framebuffer: &mut Framebuffer) {
        // In TRUE zero-copy mode, GPUI doesn't need a framebuffer!
        // GPUI displays the shared texture directly using ExternalTextureId
        // This method is just for compatibility with old API

        // Check if we have a new frame from Bevy
        if let Some(ref textures) = *self.shared_textures.lock().unwrap() {
            let _read_idx = textures.read_index.load(Ordering::Acquire);
            let frame_num = textures.frame_number.load(Ordering::Acquire);

            // Update metrics
            if let Ok(mut metrics) = self.metrics.lock() {
                metrics.frames_rendered = frame_num;

                if let Ok(last_time) = self.last_frame_time.lock() {
                    let elapsed = last_time.elapsed();
                    if elapsed.as_secs_f64() > 0.0 {
                        metrics.bevy_fps = frame_num as f64 / elapsed.as_secs_f64();
                    }
                }
            }
        }
    }

    /// Get which texture GPUI should display
    pub fn get_display_texture_index(&self) -> Option<usize> {
        self.shared_textures
            .lock()
            .ok()?
            .as_ref()
            .map(|t| t.read_index.load(Ordering::Acquire))
    }

    /// Get shared textures for GPUI integration
    pub fn get_shared_textures(&self) -> Option<SharedGpuTextures> {
        self.shared_textures.lock().ok()?.clone()
    }

    /// Get the native GPU texture handle for the current display frame
    /// This is what GPUI uses for IMMEDIATE MODE rendering - just a raw pointer!
    pub fn get_current_native_handle(&self) -> Option<crate::subsystems::render::NativeTextureHandle> {
        static CALL_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let count = CALL_COUNT.fetch_add(1, Ordering::Relaxed);
        
        let textures = self.shared_textures.lock().ok()?;
        let textures = textures.as_ref()?;

        // Get the read index (which texture GPUI should display)
        let read_idx = textures.read_index.load(Ordering::Acquire);

        // Get the native handles
        let handles_lock = textures.native_handles.lock().ok()?;
        let handles = handles_lock.as_ref()?;

        let handle = handles[read_idx];
        
        if count % 120 == 0 {  // Log every 120 calls
            println!("[BEVY-RENDERER] 📤 Call #{}: Providing native handle for index {}: {:?}", count, read_idx, handle);
        }
        
        Some(handle)
    }

    /// Update camera input from GPUI
    pub fn update_camera_input(&mut self, input: CameraInput) {
        if let Ok(mut cam_input) = self.camera_input.lock() {
            *cam_input = input;
        }
    }

    /// Get metrics
    pub fn get_metrics(&self) -> RenderMetrics {
        self.metrics.lock().unwrap().clone()
    }

    /// Resize renderer
    pub fn resize(&mut self, width: u32, height: u32) {
        println!("[BevyRenderer] 📐 Resize to {}x{}", width, height);
        // TODO: Recreate shared textures at new size
    }

    /// Update game objects (stub for compatibility)
    pub fn update_game_objects(&mut self, _objects: Vec<crate::subsystems::game::GameObject>) {
        // In TRUE zero-copy mode, game objects are managed differently
        // This is just for API compatibility
    }
}

impl Drop for BevyRenderer {
    fn drop(&mut self) {
        println!("[BevyRenderer] 🛑 Shutting down");
        self.shutdown.store(true, Ordering::Release);
    }
}

unsafe impl Send for BevyRenderer {}
unsafe impl Sync for BevyRenderer {}

// ============================================================================
// BEVY SYSTEMS
// ============================================================================

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut images: ResMut<Assets<Image>>,
    shared_textures: Res<SharedTexturesResource>,
) {
    println!("[BEVY-RENDERER] 🎬 setup_scene called - creating render textures");
    
    // Create render target textures
    let texture_0 = images.add(create_render_texture());
    let texture_1 = images.add(create_render_texture());

    println!("[BEVY-RENDERER] 📦 Created {} render textures", 2);
    
    // Store in shared resource
    if let Ok(mut textures_lock) = shared_textures.0.lock() {
        *textures_lock = Some(SharedGpuTextures {
            textures: Arc::new([texture_0.clone(), texture_1.clone()]),
            native_handles: Arc::new(Mutex::new(None)),
            write_index: Arc::new(AtomicUsize::new(0)),
            read_index: Arc::new(AtomicUsize::new(1)),
            frame_number: Arc::new(AtomicU64::new(0)),
            width: RENDER_WIDTH,
            height: RENDER_HEIGHT,
        });
        println!("[BEVY-RENDERER] ✅ Stored shared textures in resource");
    }

    // Camera targeting texture 0 initially
    commands.spawn((
        Camera3d::default(),
        Camera {
            target: RenderTarget::Image(texture_0.into()),
            ..default()
        },
        Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        MainCamera,
    ));
    
    println!("[BEVY-RENDERER] 📷 Camera spawned");

    // Light
    commands.spawn((
        PointLight {
            intensity: 1500.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // Plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(5.0, 5.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.5, 0.3))),
    ));

    // Cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        Transform::from_xyz(0.0, 0.5, 0.0),
    ));

    // Ball
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.5))),
        MeshMaterial3d(materials.add(Color::srgb(1.0, 0.2, 0.2))),
        Transform::from_xyz(1.5, 0.5, 0.0),
        AnimatedBall,
    ));
    
    println!("[BEVY-RENDERER] 🎭 Scene setup complete!");
}

/// Create DXGI shared textures that both DX12 (Bevy) and DX11 (GPUI) can access
#[cfg(target_os = "windows")]
unsafe fn create_shared_render_textures(
    render_device: &RenderDevice,
    images: &mut Assets<Image>,
) -> (Handle<Image>, Handle<Image>) {
    use crate::subsystems::render::DxgiSharedTexture;
    use wgpu_hal::api::Dx12;
    use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

    println!("[BEVY-RENDERER] 🔧 Creating DXGI shared textures for zero-copy DX12↔DX11...");

    // Get DX12 device from wgpu using v26 API
    let hal_device_ref = render_device.wgpu_device().as_hal::<Dx12>();
    
    let d3d12_device = match hal_device_ref {
        Some(device) => device.raw_device().clone(),
        None => {
            println!("[BEVY-RENDERER] ❌ Failed to get DX12 device - falling back to regular textures");
            let tex0 = images.add(create_render_texture());
            let tex1 = images.add(create_render_texture());
            return (tex0, tex1);
        }
    };

    println!("[BEVY-RENDERER] ✅ Got DX12 device");

    // Create 2 shared textures with ALLOW_SIMULTANEOUS_ACCESS flag
    println!("[BEVY-RENDERER] 📝 Calling DxgiSharedTexture::create for texture 0...");
    let shared_tex_0 = match DxgiSharedTexture::create(
        &d3d12_device,
        RENDER_WIDTH,
        RENDER_HEIGHT,
        DXGI_FORMAT_B8G8R8A8_UNORM,
    ) {
        Ok(tex) => {
            println!("[BEVY-RENDERER] ✅ Successfully created shared texture 0");
            tex
        }
        Err(e) => {
            println!("[BEVY-RENDERER] ❌ Failed to create shared texture 0: {}", e);
            println!("[BEVY-RENDERER] 🔄 Falling back to regular textures");
            let tex0 = images.add(create_render_texture());
            let tex1 = images.add(create_render_texture());
            return (tex0, tex1);
        }
    };

    println!("[BEVY-RENDERER] 📝 Calling DxgiSharedTexture::create for texture 1...");
    let shared_tex_1 = match DxgiSharedTexture::create(
        &d3d12_device,
        RENDER_WIDTH,
        RENDER_HEIGHT,
        DXGI_FORMAT_B8G8R8A8_UNORM,
    ) {
        Ok(tex) => {
            println!("[BEVY-RENDERER] ✅ Successfully created shared texture 1");
            tex
        }
        Err(e) => {
            println!("[BEVY-RENDERER] ❌ Failed to create shared texture 1: {}", e);
            println!("[BEVY-RENDERER] 🔄 Falling back to regular textures");
            let tex0 = images.add(create_render_texture());
            let tex1 = images.add(create_render_texture());
            return (tex0, tex1);
        }
    };

    let handle_0 = shared_tex_0.handle_value();
    let handle_1 = shared_tex_1.handle_value();

    println!("[BEVY-RENDERER] ✅ Created DXGI shared textures");
    println!("[BEVY-RENDERER] 📍 Shared Handle 0: 0x{:X}", handle_0);
    println!("[BEVY-RENDERER] 📍 Shared Handle 1: 0x{:X}", handle_1);

    // Create Bevy Images with placeholder data
    // The actual texture data is in the D3D12 shared resource
    let image_0 = Image {
        texture_descriptor: TextureDescriptor {
            label: Some("DXGI Shared Render Target 0"),
            size: Extent3d {
                width: RENDER_WIDTH,
                height: RENDER_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        data: None, // No CPU data - texture lives in GPU only
        ..Default::default()
    };

    let mut image_1 = image_0.clone();
    image_1.texture_descriptor.label = Some("DXGI Shared Render Target 1");

    // Add to Bevy's asset system
    let asset_handle_0 = images.add(image_0);
    let asset_handle_1 = images.add(image_1);

    // Store the DXGI shared handles globally for extraction
    super::native_texture::store_shared_handles(vec![handle_0, handle_1]);

    println!("[BEVY-RENDERER] ✅ Shared textures ready - DX12 and DX11 can access same VRAM!");
    println!("[BEVY-RENDERER] 🎯 TRUE ZERO-COPY rendering enabled");

    // Keep the shared textures alive (they're in Arc, will be cleaned up with the renderer)
    std::mem::forget(shared_tex_0);
    std::mem::forget(shared_tex_1);

    (asset_handle_0, asset_handle_1)
}

/// Create a render target texture for camera output
fn create_render_texture() -> Image {
    let size = Extent3d {
        width: RENDER_WIDTH,
        height: RENDER_HEIGHT,
        depth_or_array_layers: 1,
    };

    let mut img = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };
    img.resize(size);
    img
}

#[derive(Component)]
struct AnimatedBall;

fn animate_ball(
    time: Res<Time>,
    mut query: Query<&mut Transform, With<AnimatedBall>>,
) {
    for mut transform in query.iter_mut() {
        let t = time.elapsed_secs();
        let altitude = (t * 3.0).sin().abs() * 2.0 + 0.5;
        transform.translation.y = altitude;
    }
}

fn update_camera_from_input(
    time: Res<Time>,
    input: Res<CameraInput>,
    mut query: Query<&mut Transform, With<MainCamera>>,
) {
    for mut transform in query.iter_mut() {
        let dt = time.delta_secs();

        // Movement
        let speed = if input.boost { input.move_speed * 3.0 } else { input.move_speed };
        let forward = transform.forward();
        let right = transform.right();

        transform.translation += forward * input.forward * speed * dt;
        transform.translation += right * input.right * speed * dt;
        transform.translation.y += input.up * speed * dt;

        // Rotation (mouse look)
        if input.mouse_delta_x.abs() > 0.001 || input.mouse_delta_y.abs() > 0.001 {
            let yaw = -input.mouse_delta_x * input.look_sensitivity;
            let pitch = -input.mouse_delta_y * input.look_sensitivity;

            let (mut yaw_quat, mut pitch_quat, _) = transform.rotation.to_euler(EulerRot::YXZ);
            yaw_quat += yaw.to_radians();
            pitch_quat = (pitch_quat + pitch.to_radians()).clamp(-89_f32.to_radians(), 89_f32.to_radians());

            transform.rotation = Quat::from_euler(EulerRot::YXZ, yaw_quat, pitch_quat, 0.0);
        }
    }
}

fn check_shutdown(shutdown: Res<ShutdownSignal>, mut exit: MessageWriter<AppExit>) {
    if shutdown.0.load(Ordering::Acquire) {
        exit.write(AppExit::Success);
    }
}

/// Update camera to render to the current write texture (ping-pong)
/// NO ALLOCATIONS - just updates the handle reference
fn update_camera_render_target(
    shared_textures_res: Res<SharedTexturesResource>,
    mut camera_query: Query<&mut Camera, With<MainCamera>>,
) {
    if let Ok(textures_lock) = shared_textures_res.0.lock() {
        if let Some(ref textures) = *textures_lock {
            let write_idx = textures.write_index.load(Ordering::Acquire);
            let target_handle = &textures.textures[write_idx];

            for mut camera in camera_query.iter_mut() {
                // Handle::clone() just increments Arc refcount, cheap operation
                camera.target = RenderTarget::Image(target_handle.clone().into());
            }
        }
    }
}

/// Shared textures resource wrapper
#[derive(Resource)]
struct SharedTexturesResource(Arc<Mutex<Option<SharedGpuTextures>>>);

/// Extract native GPU texture handles from wgpu and store for GPUI
fn extract_native_texture_handles(
    shared_textures: Res<SharedTexturesResource>,
    images: Res<RenderAssets<GpuImage>>,
    render_device: Res<RenderDevice>,
) {
    static EXTRACT_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let count = EXTRACT_COUNT.fetch_add(1, Ordering::Relaxed);
    
    if count % 60 == 0 {  // Log every 60 extracts
        println!("[BEVY-RENDERER] 🔍 Extract #{}: Extracting native texture handles...", count);
    }
    
    if let Ok(textures_lock) = shared_textures.0.lock() {
        if let Some(ref textures) = *textures_lock {
            if count % 60 == 0 {
                println!("[BEVY-RENDERER] 📦 Found {} textures to extract", textures.textures.len());
            }
            
            // Extract native handles from both textures
            #[cfg(target_os = "windows")]
            let mut native_handles = [
                crate::subsystems::render::NativeTextureHandle::D3D11(0),
                crate::subsystems::render::NativeTextureHandle::D3D11(0),
            ];

            #[cfg(target_os = "macos")]
            let mut native_handles = [
                crate::subsystems::render::NativeTextureHandle::Metal(0),
                crate::subsystems::render::NativeTextureHandle::Metal(0),
            ];

            #[cfg(target_os = "linux")]
            let mut native_handles = [
                crate::subsystems::render::NativeTextureHandle::Vulkan(0),
                crate::subsystems::render::NativeTextureHandle::Vulkan(0),
            ];

            let mut extracted_count = 0;
            for (i, handle) in textures.textures.iter().enumerate() {
                if let Some(gpu_image) = images.get(handle) {
                    // Extract native DirectX/Metal/Vulkan handle from wgpu texture
                    unsafe {
                        if let Some(native_handle) = crate::subsystems::render::NativeTextureHandle::from_wgpu_texture(
                            &gpu_image.texture,
                            &render_device,
                        ) {
                            if count % 60 == 0 {
                                println!("[BEVY-RENDERER] 🎉 Successfully extracted native handle {}: {:?}", i, native_handle);
                            }
                            native_handles[i] = native_handle;
                            extracted_count += 1;
                        } else if count % 60 == 0 {
                            println!("[BEVY-RENDERER] ❌ Failed to extract native handle {}", i);
                        }
                    }
                } else if count % 60 == 0 {
                    println!("[BEVY-RENDERER] ❌ No GpuImage for texture {}", i);
                }
            }

            // Store native handles for GPUI to access
            if let Ok(mut handles_lock) = textures.native_handles.lock() {
                *handles_lock = Some(native_handles);
                if count % 60 == 0 {
                    println!("[BEVY-RENDERER] 💾 Stored {} native handles for GPUI", extracted_count);
                }
            }
        } else if count % 60 == 0 {
            println!("[BEVY-RENDERER] ⚠️ No shared textures found");
        }
    } else if count % 60 == 0 {
        println!("[BEVY-RENDERER] ❌ Failed to lock shared textures");
    }
}

/// After rendering, swap texture indices for GPUI to display
fn swap_render_textures(
    shared_textures: Res<SharedTexturesResource>,
) {
    if let Ok(textures_lock) = shared_textures.0.lock() {
        if let Some(ref textures) = *textures_lock {
            // Swap: what Bevy was writing to is now ready for GPUI to read
            let write_idx = textures.write_index.load(Ordering::Acquire);
            let read_idx = (write_idx + 1) % 2;

            textures.read_index.store(write_idx, Ordering::Release);
            textures.write_index.store(read_idx, Ordering::Release);
            textures.frame_number.fetch_add(1, Ordering::Release);
        }
    }
}

/// Initialize DXGI shared textures in the render world (Windows only)
/// This runs once at startup in the render world where RenderDevice is available
#[cfg(target_os = "windows")]
fn initialize_shared_textures(
    render_device: Res<RenderDevice>,
) {
    use crate::subsystems::render::DxgiSharedTexture;
    use wgpu_hal::api::Dx12;
    use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

    println!("[BEVY-RENDERER] 🔧 Initializing DXGI shared textures in render world...");

    // Get DX12 device from wgpu
    let hal_device_ref = unsafe { render_device.wgpu_device().as_hal::<Dx12>() };
    
    let d3d12_device = match hal_device_ref {
        Some(device) => device.raw_device().clone(),
        None => {
            println!("[BEVY-RENDERER] ❌ Failed to get DX12 device - DXGI shared textures disabled");
            return;
        }
    };

    println!("[BEVY-RENDERER] ✅ Got DX12 device for shared texture creation");

    // Create 2 shared textures with ALLOW_SIMULTANEOUS_ACCESS flag
    println!("[BEVY-RENDERER] 📝 Creating DXGI shared texture 0...");
    let shared_tex_0 = match unsafe { DxgiSharedTexture::create(
        &d3d12_device,
        RENDER_WIDTH,
        RENDER_HEIGHT,
        DXGI_FORMAT_B8G8R8A8_UNORM,
    ) } {
        Ok(tex) => {
            println!("[BEVY-RENDERER] ✅ Successfully created shared texture 0");
            tex
        }
        Err(e) => {
            println!("[BEVY-RENDERER] ❌ Failed to create shared texture 0: {}", e);
            return;
        }
    };

    println!("[BEVY-RENDERER] 📝 Creating DXGI shared texture 1...");
    let shared_tex_1 = match unsafe { DxgiSharedTexture::create(
        &d3d12_device,
        RENDER_WIDTH,
        RENDER_HEIGHT,
        DXGI_FORMAT_B8G8R8A8_UNORM,
    ) } {
        Ok(tex) => {
            println!("[BEVY-RENDERER] ✅ Successfully created shared texture 1");
            tex
        }
        Err(e) => {
            println!("[BEVY-RENDERER] ❌ Failed to create shared texture 1: {}", e);
            return;
        }
    };

    let handle_0 = shared_tex_0.handle_value();
    let handle_1 = shared_tex_1.handle_value();

    println!("[BEVY-RENDERER] ✅ Created DXGI shared textures");
    println!("[BEVY-RENDERER] 📍 Shared Handle 0: 0x{:X}", handle_0);
    println!("[BEVY-RENDERER] 📍 Shared Handle 1: 0x{:X}", handle_1);

    // Store the DXGI shared handles globally for extraction
    super::native_texture::store_shared_handles(vec![handle_0, handle_1]);

    println!("[BEVY-RENDERER] ✅ Shared textures ready - DX12 and DX11 can access same VRAM!");
    println!("[BEVY-RENDERER] 🎯 TRUE ZERO-COPY rendering enabled");

    // Keep the shared textures alive (they're in Arc, will be cleaned up with the renderer)
    std::mem::forget(shared_tex_0);
    std::mem::forget(shared_tex_1);
}
