// Bevy Renderer with DIRECT rendering to DXGI shared textures
// Clean implementation - no complex buffer swapping, just render to shared GPU memory

use bevy::{
    prelude::*,
    core_pipeline::tonemapping::Tonemapping,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    render::{
        render_asset::RenderAssets,
        renderer::{RenderDevice, RenderQueue},
        diagnostic::RenderDiagnosticsPlugin,
        texture::GpuImage,
        RenderPlugin, RenderApp, Render,
    },
};
use std::{
    sync::{
        atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use wgpu_profiler::GpuProfiler;

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

/// Represents a single diagnostic metric for GPU profiling
#[derive(Debug, Clone)]
pub struct DiagnosticMetric {
    pub name: String,           // Human readable name like "Early Mesh Preprocessing"
    pub path: String,           // Full diagnostic path like "render/early_mesh_preprocessing/elapsed_gpu"
    pub value_ms: f32,          // Value in milliseconds
    pub percentage: f32,        // Percentage of total frame time
    pub is_gpu: bool,           // True if GPU timing, false if CPU timing
}

/// GPU Pipeline profiling data - like Unreal's "stat gpu"
/// Shows timing for each render pass/phase
#[derive(Debug, Clone, Default)]
pub struct GpuProfilerData {
    pub total_frame_ms: f32,
    pub fps: f32,
    pub frame_count: u64,
    pub total_gpu_ms: f32,
    
    // Dynamic collection of all available diagnostic metrics
    pub render_metrics: Vec<DiagnosticMetric>,
    
    // Legacy fields for backwards compatibility
    pub shadow_pass_ms: f32,
    pub shadow_pass_pct: f32,
    pub opaque_pass_ms: f32,
    pub opaque_pass_pct: f32,
    pub alpha_mask_pass_ms: f32,
    pub alpha_mask_pass_pct: f32,
    pub transparent_pass_ms: f32,
    pub transparent_pass_pct: f32,
    pub lighting_ms: f32,
    pub lighting_pct: f32,
    pub post_processing_ms: f32,
    pub post_processing_pct: f32,
    pub ui_pass_ms: f32,
    pub ui_pass_pct: f32,
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
            move_speed: 30.0,
            pan_speed: 0.01,
            zoom_speed: 1.5,
            look_sensitivity: 0.3,
            boost: false,
            orbit_mode: false,
            orbit_distance: 10.0,
            focus_point: Vec3::ZERO,
        }
    }
}

#[derive(Component)]
struct MainCamera;

/// Marker component linking a Bevy entity to a game thread object
#[derive(Component)]
struct GameObjectId(u64);

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

/// Resource containing reference to the game thread for syncing object positions
#[derive(Resource, Clone)]
struct GameThreadResource(Option<Arc<Mutex<crate::subsystems::game::GameState>>>);

/// Resource containing the shared camera input from the input thread
#[derive(Resource, Clone)]
struct CameraInputResource(Arc<Mutex<CameraInput>>);

/// Metrics tracking resource - shared between Bevy and the main thread
#[derive(Resource, Clone)]
struct MetricsResource {
    pub frames_rendered: Arc<AtomicU64>,
    pub last_frame_time: Arc<Mutex<std::time::Instant>>,
    pub fps: Arc<Mutex<f32>>,
    pub frame_time_ms: Arc<Mutex<f32>>,
    pub draw_calls: Arc<AtomicU32>,
    pub vertices_drawn: Arc<AtomicU64>,
    pub memory_usage_mb: Arc<Mutex<f32>>,
}

impl Default for MetricsResource {
    fn default() -> Self {
        Self {
            frames_rendered: Arc::new(AtomicU64::new(0)),
            last_frame_time: Arc::new(Mutex::new(std::time::Instant::now())),
            fps: Arc::new(Mutex::new(0.0)),
            frame_time_ms: Arc::new(Mutex::new(0.0)),
            draw_calls: Arc::new(AtomicU32::new(0)),
            vertices_drawn: Arc::new(AtomicU64::new(0)),
            memory_usage_mb: Arc::new(Mutex::new(0.0)),
        }
    }
}

/// GPU Profiler Resource - stores detailed render pipeline timing (shared with main thread)
#[derive(Resource, Clone)]
struct GpuProfilerResource {
    pub data: Arc<Mutex<GpuProfilerData>>,
}

impl Default for GpuProfilerResource {
    fn default() -> Self {
        Self {
            data: Arc::new(Mutex::new(GpuProfilerData::default())),
        }
    }
}

/// Render world resource that holds the actual wgpu-profiler instance
#[derive(Resource)]
struct WgpuProfilerResource {
    pub profiler: GpuProfiler,
}

/// Renderer state
pub struct BevyRenderer {
    pub shared_textures: Arc<Mutex<Option<SharedGpuTextures>>>,
    pub camera_input: Arc<Mutex<CameraInput>>,
    pub metrics: Arc<MetricsResource>,
    pub gpu_profiler: Arc<Mutex<GpuProfilerData>>,
    shutdown: Arc<AtomicBool>,
    _render_thread: Option<std::thread::JoinHandle<()>>,
}

impl BevyRenderer {
    pub async fn new(width: u32, height: u32) -> Self {
        Self::new_with_game_thread(width, height, None).await
    }

    pub async fn new_with_game_thread(
        _width: u32, 
        _height: u32,
        game_thread_state: Option<Arc<Mutex<crate::subsystems::game::GameState>>>,
    ) -> Self {
        let shared_textures = Arc::new(Mutex::new(None));
        let camera_input = Arc::new(Mutex::new(CameraInput::new()));
        let metrics = Arc::new(MetricsResource::default());
        let gpu_profiler = Arc::new(Mutex::new(GpuProfilerData::default()));
        let shutdown = Arc::new(AtomicBool::new(false));

        let shared_textures_clone = shared_textures.clone();
        let camera_input_clone = camera_input.clone();
        let metrics_clone = metrics.clone();
        let gpu_profiler_clone = gpu_profiler.clone();
        let shutdown_clone = shutdown.clone();
        let game_thread_clone = game_thread_state.clone();

        let render_thread = std::thread::Builder::new()
            .name("bevy-render".to_string())
            .spawn(move || {
                Self::run_bevy_renderer(
                    RENDER_WIDTH,
                    RENDER_HEIGHT,
                    shared_textures_clone,
                    camera_input_clone,
                    metrics_clone,
                    gpu_profiler_clone,
                    shutdown_clone,
                    game_thread_clone,
                );
            })
            .expect("Failed to spawn Bevy render thread");

        // tokio::time::sleep(Duration::from_millis(500)).await;

        Self {
            shared_textures,
            camera_input,
            metrics,
            gpu_profiler,
            shutdown,
            _render_thread: Some(render_thread),
        }
    }

    fn run_bevy_renderer(
        width: u32,
        height: u32,
        shared_textures: Arc<Mutex<Option<SharedGpuTextures>>>,
        camera_input: Arc<Mutex<CameraInput>>,
        metrics: Arc<MetricsResource>,
        gpu_profiler: Arc<Mutex<GpuProfilerData>>,
        shutdown: Arc<AtomicBool>,
        game_thread_state: Option<Arc<Mutex<crate::subsystems::game::GameState>>>,
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
            Duration::from_secs_f64(1.0 / 3000.0),
        ));

        println!("[BEVY] ✅ Plugins configured");

        // Resources
        app.insert_resource(ClearColor(Color::srgb(0.1, 0.2, 0.3)))
            .insert_resource(camera_input.lock().unwrap().clone())
            .insert_resource(CameraInputResource(camera_input.clone())) // Shared input from input thread
            .insert_resource(SharedTexturesResource(shared_textures.clone()))
            .insert_resource(metrics.as_ref().clone())
            .insert_resource(GpuProfilerResource { data: gpu_profiler.clone() }) // GPU profiler data
            .insert_resource(ShutdownFlag(shutdown.clone()))
            .insert_resource(GameThreadResource(game_thread_state))
            .add_plugins(FrameTimeDiagnosticsPlugin::default()) // Bevy frame time diagnostics
            .add_plugins(RenderDiagnosticsPlugin::default()); // Bevy GPU render diagnostics
        
        // Insert shutdown resource
        
        // Main world systems - create textures FIRST, then setup scene
        app.add_systems(Startup, (create_shared_textures_startup, setup_scene).chain())
            .add_systems(Update, check_shutdown)
            .add_systems(Update, sync_camera_input_system) // NEW: Sync input thread camera input to Bevy ECS
            .add_systems(Update, camera_movement_system) // Unreal-style camera controls
            .add_systems(Update, sync_game_objects_system) // NEW: Sync game thread to Bevy
            .add_systems(Update, update_metrics_system) // Track FPS and frame times
            .add_systems(Update, update_gpu_profiler_system) // Extract GPU profiler data from Bevy diagnostics
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
        // Get actual metrics from the tracking resource
        let frames_rendered = self.metrics.frames_rendered.load(Ordering::Relaxed);

        let bevy_fps = self.metrics.fps.lock().ok()
            .map(|f| *f)
            .unwrap_or(0.0);

        let frame_time_ms = self.metrics.frame_time_ms.lock().ok()
            .map(|f| *f)
            .unwrap_or(0.0);

        // Calculate pipeline time from frame time
        let pipeline_time_us = frame_time_ms * 1000.0;

        // Estimate GPU and CPU times (rough estimates based on frame time)
        // GPU typically takes ~60-70% of frame time, CPU ~30-40%
        let gpu_time_us = pipeline_time_us * 0.65;
        let cpu_time_us = pipeline_time_us * 0.35;

        // Calculate dynamic render stats based on scene complexity
        // These are estimates until we can hook into Bevy's actual render stats
        let draw_calls = self.metrics.draw_calls.load(Ordering::Relaxed);
        let vertices_drawn = self.metrics.vertices_drawn.load(Ordering::Relaxed);
        let memory_usage_mb = self.metrics.memory_usage_mb.lock().ok()
            .map(|m| *m)
            .unwrap_or(0.0);

        RenderMetrics {
            fps: bevy_fps,
            frame_time_ms,
            draw_calls,
            memory_usage_mb,
            vertices_drawn,
            frames_rendered,
            bevy_fps,
            pipeline_time_us,
            gpu_time_us,
            cpu_time_us,
        }
    }
    
    /// Get GPU profiler data - detailed render pipeline timing (like Unreal's "stat gpu")
    pub fn get_gpu_profiler_data(&self) -> Option<GpuProfilerData> {
        self.gpu_profiler.lock().ok().map(|data| data.clone())
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

/// Sync camera input from the input thread to the Bevy ECS resource
/// This system reads from the shared Arc<Mutex<CameraInput>> that the input thread updates
/// and copies it to the Bevy ECS CameraInput resource that camera_movement_system uses
fn sync_camera_input_system(
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
        let _distance = offset.length();
        
        // Rotate offset around focus point
        if camera_input.mouse_delta_x.abs() > 0.001 || camera_input.mouse_delta_y.abs() > 0.001 {
            // This would require converting to spherical coordinates and back
            // For now, keeping it simple with FPS rotation
        }
    }
}

/// Update performance metrics system
/// Tracks FPS, frame time, and frame count
fn update_metrics_system(
    _time: Res<Time>,
    mut metrics: ResMut<MetricsResource>,
    meshes: Res<Assets<Mesh>>,
    materials: Res<Assets<StandardMaterial>>,
    cameras: Query<&Camera>,
) {
    // Increment frame count
    let _frame_count = metrics.frames_rendered.fetch_add(1, Ordering::Relaxed);

    // Calculate FPS and frame time every frame
    if let Ok(mut last_frame_time) = metrics.last_frame_time.lock() {
        let now = std::time::Instant::now();
        let frame_duration = now.duration_since(*last_frame_time);
        let frame_time_ms = frame_duration.as_secs_f32() * 1000.0;

        // Update frame time
        if let Ok(mut stored_frame_time) = metrics.frame_time_ms.lock() {
            *stored_frame_time = frame_time_ms;
        }

        // Calculate FPS (1 / frame_time in seconds)
        let fps = if frame_time_ms > 0.0 {
            1000.0 / frame_time_ms
        } else {
            0.0
        };

        if let Ok(mut stored_fps) = metrics.fps.lock() {
            // Smooth FPS with exponential moving average
            *stored_fps = (*stored_fps * 0.9) + (fps * 0.1);
        }

        *last_frame_time = now;
    }

    // Calculate draw calls estimate based on active cameras and scene complexity
    // Each camera typically has multiple render passes
    let num_cameras = cameras.iter().count();
    let num_meshes = meshes.len();
    let num_materials = materials.len();
    
    // Estimate: cameras * (base passes + mesh instances + material batches)
    // Base passes: shadow + main + post-processing ~= 3
    let estimated_draw_calls = (num_cameras * 3) + num_meshes.min(100) + num_materials.min(50);
    metrics.draw_calls.store(estimated_draw_calls as u32, Ordering::Relaxed);

    // Calculate vertices estimate based on meshes
    let mut total_vertices = 0u64;
    for mesh in meshes.iter() {
        // count_vertices() returns usize, not Option<usize>
        let vertex_count = mesh.1.count_vertices();
        total_vertices += vertex_count as u64;
    }
    metrics.vertices_drawn.store(total_vertices, Ordering::Relaxed);

    // Estimate memory usage (very rough)
    // Textures + meshes + materials + buffers
    let mesh_memory_mb = (total_vertices * 32) as f32 / (1024.0 * 1024.0); // ~32 bytes per vertex
    let material_memory_mb = (num_materials * 1024) as f32 / (1024.0 * 1024.0); // ~1KB per material
    let total_memory_mb = mesh_memory_mb + material_memory_mb + 64.0; // +64MB base overhead
    
    if let Ok(mut memory) = metrics.memory_usage_mb.lock() {
        *memory = total_memory_mb;
    }
}

// ============================================================================
// GPU Profiler - Uses Bevy's built-in RenderDiagnosticsPlugin
// ============================================================================

/// Update GPU Profiler system - extracts REAL GPU timing from Bevy's RenderDiagnosticsPlugin
/// Like Unreal's "stat gpu" - uses actual measured timings from GPU render passes
fn update_gpu_profiler_system(
    diagnostics: Res<DiagnosticsStore>,
    profiler: Res<GpuProfilerResource>,
) {
    use bevy::diagnostic::DiagnosticPath;
    
    // Debug: Print all available diagnostic paths (only once per second)
    use std::sync::atomic::{AtomicU64, Ordering};
    static LAST_DEBUG_PRINT: AtomicU64 = AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // Helper to get GPU timing from diagnostic path
    let get_gpu_timing = |path_str: &str| -> f32 {
        let path = DiagnosticPath::from_components(path_str.split('/'));
        diagnostics
            .get(&path)
            .and_then(|d| d.smoothed())
            .unwrap_or(0.0) as f32
    };
    
    // Helper to get CPU timing from diagnostic path
    let get_cpu_timing = |path_str: &str| -> f32 {
        let path = DiagnosticPath::from_components(path_str.split('/'));
        diagnostics
            .get(&path)
            .and_then(|d| d.smoothed())
            .unwrap_or(0.0) as f32
    };
    
    // Get frame time as baseline
    let frame_time_ms = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|d| d.smoothed())
        .unwrap_or(16.67) as f32;
    
    // Extract REAL GPU timings from Bevy's RenderDiagnosticsPlugin
    // These are measured using GPU timestamp queries, not estimates
    // Paths follow format: "render/{pass_name}/elapsed_gpu" (in milliseconds)
    
    // Core render passes
    let shadow_ms = get_gpu_timing("render/shadow_pass/elapsed_gpu");
    let opaque_ms = get_gpu_timing("render/main_opaque_pass/elapsed_gpu") 
        + get_gpu_timing("render/opaque_pass/elapsed_gpu");
    let alpha_mask_ms = get_gpu_timing("render/alpha_mask_pass/elapsed_gpu");
    let transparent_ms = get_gpu_timing("render/main_transparent_pass/elapsed_gpu")
        + get_gpu_timing("render/transparent_pass/elapsed_gpu");
    
    // Lighting and post-processing
    let lighting_ms = get_gpu_timing("render/lighting/elapsed_gpu")
        + get_gpu_timing("render/pbr_lighting/elapsed_gpu");
    let post_ms = get_gpu_timing("render/post_processing/elapsed_gpu")
        + get_gpu_timing("render/tonemapping/elapsed_gpu")
        + get_gpu_timing("render/fxaa/elapsed_gpu")
        + get_gpu_timing("render/bloom/elapsed_gpu");
    
    // UI rendering
    let ui_ms = get_gpu_timing("render/ui_pass/elapsed_gpu");
    
    // Calculate total GPU time from all measured passes
    let total_gpu_ms = shadow_ms + opaque_ms + alpha_mask_ms + transparent_ms 
        + lighting_ms + post_ms + ui_ms;
    
    // Use total_gpu_ms if available, otherwise fall back to frame_time_ms
    let baseline_ms = if total_gpu_ms > 0.1 { total_gpu_ms } else { frame_time_ms };
    
    let calc_pct = |ms: f32| {
        if baseline_ms > 0.0 {
            (ms / baseline_ms * 100.0).max(0.0).min(100.0)
        } else {
            0.0
        }
    };
    
    // Collect dynamic diagnostic metrics
    let mut render_metrics = Vec::new();
    
    // Helper function to create human-readable names from diagnostic paths
    let path_to_name = |path: &str| -> String {
        if path == "frame_time" {
            return "Frame Time".to_string();
        }
        if path == "fps" {
            return "FPS".to_string();
        }
        if path == "frame_count" {
            return "Frame Count".to_string();
        }
        
        // Extract the render pass name from paths like "render/early_mesh_preprocessing/elapsed_gpu"
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() >= 2 && parts[0] == "render" {
            let pass_name = parts[1];
            let timing_type = parts.get(2).unwrap_or(&"");
            
            // Convert snake_case to Title Case
            let title_case = pass_name
                .split('_')
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + &chars.collect::<String>(),
                    }
                })
                .collect::<Vec<String>>()
                .join(" ");
                
            // Add timing type suffix
            match *timing_type {
                "elapsed_gpu" => format!("{} (GPU)", title_case),
                "elapsed_cpu" => format!("{} (CPU)", title_case),
                _ => title_case,
            }
        } else {
            // Fallback: just capitalize the path
            path.replace('_', " ").split_whitespace()
                .map(|word| {
                    let mut chars = word.chars();
                    match chars.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().collect::<String>() + &chars.collect::<String>(),
                    }
                })
                .collect::<Vec<String>>()
                .join(" ")
        }
    };
    
    // Get FPS and frame count
    let fps = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|d| d.smoothed())
        .unwrap_or(0.0) as f32;
        
    let frame_count = diagnostics
        .get(&FrameTimeDiagnosticsPlugin::FRAME_COUNT)
        .and_then(|d| d.value())
        .unwrap_or(0.0) as u64;
    
    // Iterate through all available diagnostics and collect render-related ones
    for diagnostic in diagnostics.iter() {
        let path_str = diagnostic.path().to_string();
        
        // Only include render passes, fps, and frame_time
        if path_str.starts_with("render/") || path_str == "fps" || path_str == "frame_time" || path_str == "frame_count" {
            if let Some(value) = diagnostic.smoothed() {
                let value_ms = value as f32;
                let is_gpu = path_str.contains("elapsed_gpu");
                let percentage = calc_pct(value_ms);
                
                render_metrics.push(DiagnosticMetric {
                    name: path_to_name(&path_str),
                    path: path_str,
                    value_ms,
                    percentage,
                    is_gpu,
                });
            }
        }
    }
    
    // Sort metrics by GPU/CPU type, then by value (descending)
    render_metrics.sort_by(|a, b| {
        // First sort by type (GPU first, then CPU)
        match (a.is_gpu, b.is_gpu) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                // Then by value (descending)
                b.value_ms.partial_cmp(&a.value_ms).unwrap_or(std::cmp::Ordering::Equal)
            }
        }
    });

    // Update profiler data with REAL GPU timings from hardware queries
    if let Ok(mut data) = profiler.data.lock() {
        data.total_frame_ms = frame_time_ms;
        data.fps = fps;
        data.frame_count = frame_count;
        data.render_metrics = render_metrics;
        data.shadow_pass_ms = shadow_ms;
        data.shadow_pass_pct = calc_pct(shadow_ms);
        data.opaque_pass_ms = opaque_ms;
        data.opaque_pass_pct = calc_pct(opaque_ms);
        data.alpha_mask_pass_ms = alpha_mask_ms;
        data.alpha_mask_pass_pct = calc_pct(alpha_mask_ms);
        data.transparent_pass_ms = transparent_ms;
        data.transparent_pass_pct = calc_pct(transparent_ms);
        data.lighting_ms = lighting_ms;
        data.lighting_pct = calc_pct(lighting_ms);
        data.post_processing_ms = post_ms;
        data.post_processing_pct = calc_pct(post_ms);
        data.ui_pass_ms = ui_ms;
        data.ui_pass_pct = calc_pct(ui_ms);
        data.total_gpu_ms = total_gpu_ms;
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
fn debug_rendering_system(
    query: Query<&Camera, With<MainCamera>>,
    mut counter: Local<u32>,
) {
    // Any debug info can be printed here
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

    let _texture_handles = match shared_textures.0.lock().ok().and_then(|l| l.as_ref().map(|t| t.textures.clone())) {
        Some(h) => h,
        None => return,
    };

    // TODO: Extract actual GPU texture handles from GpuImage if needed
    // For now, DXGI handles are already stored globally in create_shared_textures
}

/// Sync game thread object positions/rotations to Bevy entities
/// This system reads from the game thread state and updates matching Bevy transforms
fn sync_game_objects_system(
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

fn check_shutdown(shutdown: Res<ShutdownFlag>, mut exit: MessageWriter<AppExit>) {
    if shutdown.0.load(Ordering::Acquire) {
        println!("[BEVY] 🛑 Shutdown requested");
        exit.write(AppExit::Success);
    }
}
