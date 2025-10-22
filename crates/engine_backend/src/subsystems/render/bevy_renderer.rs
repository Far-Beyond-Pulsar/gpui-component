// Bevy Renderer with DIRECT rendering to DXGI shared textures
// Clean implementation - no complex buffer swapping, just render to shared GPU memory

use bevy::{
    prelude::*,
    core_pipeline::tonemapping::Tonemapping,
    render::{
        render_asset::RenderAssets,
        renderer::RenderDevice,
        texture::GpuImage,
        RenderPlugin, RenderApp, Render,
    },
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

#[cfg(target_os = "windows")]
use crate::subsystems::render::DxgiSharedTexture;

const RENDER_WIDTH: u32 = 1600;
const RENDER_HEIGHT: u32 = 900;

/// Rendering metrics
#[derive(Debug, Clone, Default)]
pub struct RenderMetrics {
    pub fps: f32,
    pub frame_time_ms: f32,
    pub draw_calls: u32,
    pub memory_usage_mb: f32,
    pub vertices_drawn: u64,
    pub frames_rendered: u64,
    pub bevy_fps: f32,
    pub pipeline_time_us: f32,
    pub gpu_time_us: f32,
    pub cpu_time_us: f32,
}

/// Camera controller
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

#[derive(Component)]
struct MainCamera;

/// Shared textures - double buffered for zero-copy rendering
#[derive(Clone)]
pub struct SharedGpuTextures {
    pub textures: Arc<[Handle<Image>; 2]>,
    pub native_handles: Arc<Mutex<Option<[crate::subsystems::render::NativeTextureHandle; 2]>>>,
    pub write_index: Arc<AtomicUsize>,
    pub read_index: Arc<AtomicUsize>,
    pub frame_number: Arc<AtomicU64>,
    pub width: u32,
    pub height: u32,
}

#[derive(Resource)]
struct SharedTexturesResource(Arc<Mutex<Option<SharedGpuTextures>>>);

#[derive(Resource, Clone)]
struct ShutdownFlag(Arc<AtomicBool>);

/// Renderer state
pub struct BevyRenderer {
    pub shared_textures: Arc<Mutex<Option<SharedGpuTextures>>>,
    pub camera_input: Arc<Mutex<CameraInput>>,
    shutdown: Arc<AtomicBool>,
    _render_thread: Option<std::thread::JoinHandle<()>>,
}

impl BevyRenderer {
    pub async fn new(_width: u32, _height: u32) -> Self {
        let shared_textures = Arc::new(Mutex::new(None));
        let camera_input = Arc::new(Mutex::new(CameraInput::new()));
        let shutdown = Arc::new(AtomicBool::new(false));

        let shared_textures_clone = shared_textures.clone();
        let camera_input_clone = camera_input.clone();
        let shutdown_clone = shutdown.clone();

        let render_thread = std::thread::Builder::new()
            .name("bevy-render".to_string())
            .spawn(move || {
                Self::run_bevy_renderer(
                    RENDER_WIDTH,
                    RENDER_HEIGHT,
                    shared_textures_clone,
                    camera_input_clone,
                    shutdown_clone,
                );
            })
            .expect("Failed to spawn Bevy render thread");

        tokio::time::sleep(Duration::from_millis(500)).await;

        Self {
            shared_textures,
            camera_input,
            shutdown,
            _render_thread: Some(render_thread),
        }
    }

    fn run_bevy_renderer(
        width: u32,
        height: u32,
        shared_textures: Arc<Mutex<Option<SharedGpuTextures>>>,
        camera_input: Arc<Mutex<CameraInput>>,
        shutdown: Arc<AtomicBool>,
    ) {
        println!("[BEVY] 🚀 Starting headless renderer {}x{}", width, height);

        let mut app = App::new();

        // Headless mode with DX12 backend (critical for DXGI shared resources!)
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

        app.add_plugins(bevy::app::ScheduleRunnerPlugin::run_loop(
            Duration::from_secs_f64(1.0 / 120.0),
        ));

        println!("[BEVY] ✅ Plugins configured");

        // Resources
        app.insert_resource(ClearColor(Color::srgb(0.1, 0.2, 0.3)))
            .insert_resource(camera_input.lock().unwrap().clone())
            .insert_resource(SharedTexturesResource(shared_textures.clone()))
            .insert_resource(ShutdownFlag(shutdown.clone()));
        
        // Insert shutdown resource
        
        // Main world systems - create textures FIRST, then setup scene
        app.add_systems(Startup, (create_shared_textures_startup, setup_scene).chain())
            .add_systems(Update, check_shutdown)
            .add_systems(Update, camera_movement_system) // Unreal-style camera controls
            .add_systems(Update, debug_rendering_system); // Add debug system

        // Render world systems
        if let Some(render_app) = app.get_sub_app_mut(RenderApp) {
            render_app.insert_resource(SharedTexturesResource(shared_textures.clone()));
            
            #[cfg(target_os = "windows")]
            {
                // Run shared texture creation ONCE on startup in the Render schedule
                // This runs BEFORE extraction, so the GpuImages are ready when camera extracts
                render_app.add_systems(
                    Render,
                    create_shared_textures
                        .run_if(|| {
                            static ONCE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
                            !ONCE.swap(true, std::sync::atomic::Ordering::Relaxed)
                        })
                        .before(bevy::render::RenderSystems::Render),
                );
            }

            // Extract native handles every frame (for debug/monitoring)
            render_app.add_systems(
                Render,
                extract_native_handles.in_set(bevy::render::RenderSystems::Render)
            );
        }

        println!("[BEVY] ✅ Starting render loop...");
        app.run();
    }

    pub fn update_camera_input(&mut self, input: CameraInput) {
        if let Ok(mut cam) = self.camera_input.lock() {
            *cam = input;
        }
    }

    pub fn get_shared_texture_handles(&self) -> Option<Vec<usize>> {
        // Read from global storage where Bevy stores the handles
        super::native_texture::get_shared_handles().map(|handles| {
            handles.iter().map(|h| {
                match h {
                    crate::subsystems::render::NativeTextureHandle::D3D11(ptr) => *ptr,
                    _ => 0,
                }
            }).collect()
        })
    }

    pub fn get_read_index(&self) -> usize {
        // Always use buffer 0 - we're using single-buffered rendering for now
        // TODO: Implement proper double-buffering with camera swapping
        0
    }

    pub fn get_current_native_handle(&self) -> Option<crate::subsystems::render::NativeTextureHandle> {
        let read_idx = self.get_read_index();
        if let Ok(lock) = self.shared_textures.lock() {
            if let Some(ref textures) = *lock {
                if let Ok(handles_lock) = textures.native_handles.lock() {
                    if let Some(ref handles) = *handles_lock {
                        return Some(handles[read_idx].clone());
                    }
                }
            }
        }
        None
    }

    pub fn get_metrics(&self) -> RenderMetrics {
        // Simple metrics - could be enhanced with actual FPS tracking
        RenderMetrics {
            fps: 60.0,
            frame_time_ms: 16.6,
            draw_calls: 10,
            memory_usage_mb: 256.0,
            vertices_drawn: 10000,
            frames_rendered: 1000,
            bevy_fps: 60.0,
            pipeline_time_us: 100.0,
            gpu_time_us: 500.0,
            cpu_time_us: 300.0,
        }
    }

    pub fn resize(&mut self, _width: u32, _height: u32) {
        // For now, resizing not supported with DXGI shared textures
        // Would require recreating the textures
        println!("[BEVY] ⚠️ Resize not yet implemented for DXGI shared textures");
    }

    pub fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
    }
}

impl Drop for BevyRenderer {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// Unreal Engine-style camera movement system
/// Supports:
/// - WASD + QE for movement (with Shift for boost)
/// - Right mouse + drag for FPS rotation
/// - Middle mouse + drag for panning
/// - Mouse wheel for zoom (or move speed adjustment with right mouse held)
fn camera_movement_system(
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
        let distance = offset.length();
        
        // Rotate offset around focus point
        if camera_input.mouse_delta_x.abs() > 0.001 || camera_input.mouse_delta_y.abs() > 0.001 {
            // This would require converting to spherical coordinates and back
            // For now, keeping it simple with FPS rotation
        }
    }
}

/// Setup 3D scene - runs AFTER DXGI textures are created
fn setup_scene(
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
    
    // Bright magenta/purple ground plane
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20.0, 20.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 1.0), // BRIGHT MAGENTA
            unlit: true, // Make it fully bright
            ..default()
        })),
        Transform::from_xyz(0.0, 0.0, 0.0),
    ));
    println!("[BEVY] ✅ MAGENTA plane spawned");

    // Giant YELLOW cube
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(2.0, 2.0, 2.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 1.0, 0.0), // BRIGHT YELLOW
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(-2.0, 1.0, 0.0),
    ));
    println!("[BEVY] ✅ YELLOW cube spawned");

    // Giant CYAN sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.0, 1.0, 1.0), // BRIGHT CYAN
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(2.0, 1.0, 0.0),
    ));
    println!("[BEVY] ✅ CYAN sphere spawned");

    // Giant RED sphere at top
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.0, 0.0), // BRIGHT RED
            emissive: LinearRgba::rgb(1.0, 0.0, 0.0),
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, 3.0, 0.0),
    ));
    println!("[BEVY] ✅ RED sphere spawned");

    // Giant GREEN sphere
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.0, 1.0, 0.0), // BRIGHT GREEN
            emissive: LinearRgba::rgb(0.0, 1.0, 0.0),
            unlit: true,
            ..default()
        })),
        Transform::from_xyz(0.0, 1.0, 2.0),
    ));
    println!("[BEVY] ✅ GREEN sphere spawned");

    // Super bright directional light
    commands.spawn((
        DirectionalLight {
            illuminance: 50000.0,
            shadows_enabled: false,
            ..default()
        },
        Transform::from_xyz(4.0, 10.0, 4.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    println!("[BEVY] ✅ Light spawned");

    println!("[BEVY] ✅ Scene ready!");
    println!("[BEVY] 🎨 You should see:");
    println!("[BEVY] 🔵 CYAN/BLUE background");
    println!("[BEVY] 🟣 MAGENTA ground plane");
    println!("[BEVY] 🟡 YELLOW cube (left)");
    println!("[BEVY] 🔵 CYAN sphere (right)");
    println!("[BEVY] 🔴 RED sphere (top)");
    println!("[BEVY] 🟢 GREEN sphere (front)");
}

// Debug system to track rendering
fn debug_rendering_system(
    query: Query<&Camera, With<MainCamera>>,
    mut counter: Local<u32>,
) {
    *counter += 1;
    if *counter % 120 == 0 {
        for camera in query.iter() {
            println!("[BEVY] 🎥 Frame {} - Camera active, target: {:?}", *counter, camera.target);
        }
    }
}


/// Create DXGI shared textures BEFORE scene setup
/// This must run first so the Images exist when the camera is created
#[cfg(target_os = "windows")]
fn create_shared_textures_startup(
    shared_textures: Res<SharedTexturesResource>,
    mut images: ResMut<Assets<Image>>,
) {
    use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

    println!("[BEVY] 🔧 Creating DXGI shared textures...");

    // Check if already created
    if let Ok(lock) = shared_textures.0.lock() {
        if let Some(ref textures) = *lock {
            if let Ok(native_lock) = textures.native_handles.lock() {
                if native_lock.is_some() {
                    println!("[BEVY] ⚠️ Textures already created");
                    return;
                }
            }
        }
    }

    // Get D3D12 device - we need wgpu device for this
    // For now, create placeholder Images that will be replaced in render world
    let bytes_per_pixel = 4; // BGRA8
    let texture_size = (RENDER_WIDTH * RENDER_HEIGHT * bytes_per_pixel) as usize;
    
    let mut image_0 = Image {
        texture_descriptor: bevy::render::render_resource::TextureDescriptor {
            label: Some("DXGI Shared Render Target 0"),
            size: bevy::render::render_resource::Extent3d {
                width: RENDER_WIDTH,
                height: RENDER_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: bevy::render::render_resource::TextureDimension::D2,
            format: bevy::render::render_resource::TextureFormat::Bgra8UnormSrgb,
            usage: bevy::render::render_resource::TextureUsages::RENDER_ATTACHMENT | bevy::render::render_resource::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        ..default()
    };
    image_0.data = Some(vec![0u8; texture_size]); // Allocate proper buffer
    let render_target_0 = images.add(image_0);

    let mut image_1 = Image {
        texture_descriptor: bevy::render::render_resource::TextureDescriptor {
            label: Some("DXGI Shared Render Target 1"),
            size: bevy::render::render_resource::Extent3d {
                width: RENDER_WIDTH,
                height: RENDER_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: bevy::render::render_resource::TextureDimension::D2,
            format: bevy::render::render_resource::TextureFormat::Bgra8UnormSrgb,
            usage: bevy::render::render_resource::TextureUsages::RENDER_ATTACHMENT | bevy::render::render_resource::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        },
        ..default()
    };
    image_1.data = Some(vec![0u8; texture_size]); // Allocate proper buffer
    let render_target_1 = images.add(image_1);

    // Store handles - these will be replaced with DXGI-backed GpuImages in render world
    if let Ok(mut lock) = shared_textures.0.lock() {
        *lock = Some(SharedGpuTextures {
            textures: Arc::new([render_target_0.clone(), render_target_1.clone()]),
            native_handles: Arc::new(Mutex::new(None)),
            write_index: Arc::new(AtomicUsize::new(0)),
            read_index: Arc::new(AtomicUsize::new(1)),
            frame_number: Arc::new(AtomicU64::new(0)),
            width: RENDER_WIDTH,
            height: RENDER_HEIGHT,
        });
    }

    println!("[BEVY] ✅ Placeholder render target Images created");
}

/// Create DXGI shared textures and inject them into Bevy's render pipeline
/// This replaces the GPU backing texture of the render targets with DXGI shared textures
#[cfg(target_os = "windows")]
fn create_shared_textures(
    shared_textures: Res<SharedTexturesResource>,
    mut gpu_images: ResMut<RenderAssets<GpuImage>>,
    render_device: Res<RenderDevice>,
) {
    use wgpu_hal::api::Dx12;
    use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;

    println!("[BEVY] 🔧 Replacing render targets with DXGI shared textures...");

    let texture_handles = match shared_textures.0.lock().ok().and_then(|l| l.as_ref().map(|t| t.textures.clone())) {
        Some(handles) => handles,
        None => {
            println!("[BEVY] ❌ No texture handles available");
            return;
        }
    };
    
    // Check if we already have DXGI textures created
    if let Ok(lock) = shared_textures.0.lock() {
        if let Some(ref textures) = *lock {
            if let Ok(native_lock) = textures.native_handles.lock() {
                if native_lock.is_some() {
                    // Already created, don't recreate
                    return;
                }
            }
        }
    }

    // Get D3D12 device from wgpu
    let d3d12_device = unsafe {
        match render_device.wgpu_device().as_hal::<Dx12>() {
            Some(hal_device) => hal_device.raw_device().clone(),
            None => {
                println!("[BEVY] ❌ Failed to get D3D12 device");
                return;
            }
        }
    };

    // Create 2 DXGI shared textures
    let tex_0 = unsafe {
        match DxgiSharedTexture::create(&d3d12_device, RENDER_WIDTH, RENDER_HEIGHT, DXGI_FORMAT_B8G8R8A8_UNORM) {
            Ok(t) => t,
            Err(e) => {
                println!("[BEVY] ❌ Failed to create texture 0: {}", e);
                return;
            }
        }
    };

    let tex_1 = unsafe {
        match DxgiSharedTexture::create(&d3d12_device, RENDER_WIDTH, RENDER_HEIGHT, DXGI_FORMAT_B8G8R8A8_UNORM) {
            Ok(t) => t,
            Err(e) => {
                println!("[BEVY] ❌ Failed to create texture 1: {}", e);
                return;
            }
        }
    };

    let handle_0 = tex_0.handle_value();
    let handle_1 = tex_1.handle_value();

    println!("[BEVY] ✅ Created DXGI textures: 0x{:X}, 0x{:X}", handle_0, handle_1);

    // Store handles for GPUI
    super::native_texture::store_shared_handles(vec![handle_0, handle_1]);

    // Wrap D3D12 textures as wgpu textures and inject into Bevy
    unsafe {
        let hal_tex_0 = <Dx12 as wgpu_hal::Api>::Device::texture_from_raw(
            tex_0.dx12_resource.clone(),
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureDimension::D2,
            wgpu::Extent3d {
                width: RENDER_WIDTH,
                height: RENDER_HEIGHT,
                depth_or_array_layers: 1,
            },
            1, // mip_level_count
            1, // sample_count
        );

        let hal_tex_1 = <Dx12 as wgpu_hal::Api>::Device::texture_from_raw(
            tex_1.dx12_resource.clone(),
            wgpu::TextureFormat::Bgra8UnormSrgb,
            wgpu::TextureDimension::D2,
            wgpu::Extent3d {
                width: RENDER_WIDTH,
                height: RENDER_HEIGHT,
                depth_or_array_layers: 1,
            },
            1, // mip_level_count
            1, // sample_count
        );

        let wgpu_desc = wgpu::TextureDescriptor {
            label: Some("DXGI Shared Texture"),
            size: wgpu::Extent3d {
                width: RENDER_WIDTH,
                height: RENDER_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        let wgpu_tex_0 = render_device.wgpu_device().create_texture_from_hal::<Dx12>(hal_tex_0, &wgpu_desc);
        let mut desc1 = wgpu_desc.clone();
        desc1.label = Some("DXGI Shared Texture 1");
        let wgpu_tex_1 = render_device.wgpu_device().create_texture_from_hal::<Dx12>(hal_tex_1, &desc1);

        // Create texture views before moving textures
        let view_0 = wgpu_tex_0.create_view(&Default::default());
        let view_1 = wgpu_tex_1.create_view(&Default::default());

        // Create GpuImage and inject
        let gpu_img_0 = GpuImage {
            texture: bevy::render::render_resource::Texture::from(wgpu_tex_0),
            texture_view: bevy::render::render_resource::TextureView::from(view_0),
            texture_format: wgpu::TextureFormat::Bgra8UnormSrgb,
            sampler: render_device.create_sampler(&wgpu::SamplerDescriptor::default()),
            size: bevy::render::render_resource::Extent3d {
                width: RENDER_WIDTH,
                height: RENDER_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
        };

        let gpu_img_1 = GpuImage {
            texture: bevy::render::render_resource::Texture::from(wgpu_tex_1),
            texture_view: bevy::render::render_resource::TextureView::from(view_1),
            texture_format: wgpu::TextureFormat::Bgra8UnormSrgb,
            sampler: render_device.create_sampler(&wgpu::SamplerDescriptor::default()),
            size: bevy::render::render_resource::Extent3d {
                width: RENDER_WIDTH,
                height: RENDER_HEIGHT,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
        };

        // CRITICAL: Inject our textures into Bevy's render assets
        println!("[BEVY] 📍 Injecting DXGI texture into asset ID 0: {:?}", texture_handles[0].id());
        println!("[BEVY] 📍 Injecting DXGI texture into asset ID 1: {:?}", texture_handles[1].id());
        gpu_images.insert(&texture_handles[0], gpu_img_0);
        gpu_images.insert(&texture_handles[1], gpu_img_1);

        println!("[BEVY] ✅ Injected DXGI textures into Bevy - Rendering DIRECTLY to shared GPU memory!");

        // Keep textures alive
        std::mem::forget(tex_0);
        std::mem::forget(tex_1);
    }
}

/// Extract native GPU handles for GPUI
fn extract_native_handles(
    shared_textures: Res<SharedTexturesResource>,
    _gpu_images: Res<RenderAssets<GpuImage>>,
) {
    static FRAME: AtomicU64 = AtomicU64::new(0);
    let f = FRAME.fetch_add(1, Ordering::Relaxed);
    
    if f % 120 != 0 {
        return; // Extract once per second
    }

    println!("[BEVY] 🔍 Extracting handles (frame {})", f);

    let _texture_handles = match shared_textures.0.lock().ok().and_then(|l| l.as_ref().map(|t| t.textures.clone())) {
        Some(h) => h,
        None => return,
    };

    // TODO: Extract actual GPU texture handles from GpuImage if needed
    // For now, DXGI handles are already stored globally in create_shared_textures
}

fn check_shutdown(shutdown: Res<ShutdownFlag>, mut exit: MessageWriter<AppExit>) {
    if shutdown.0.load(Ordering::Acquire) {
        println!("[BEVY] 🛑 Shutdown requested");
        exit.write(AppExit::Success);
    }
}
