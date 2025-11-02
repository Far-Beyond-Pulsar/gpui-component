// Engine modules and imports
use crate::settings::EngineSettings;
use directories::ProjectDirs;
use gpui::Action;
use gpui::SharedString;
use gpui::*;
use gpui_component::scroll::ScrollbarShow;
use gpui_component::Root;
use serde::Deserialize;
use std::fs;
use std::path::Path;
use ui::app::ToggleCommandPalette;
use ui::entry_window::EntryWindow;

// Winit imports
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::event::{ElementState, MouseButton as WinitMouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window as WinitWindow, WindowId};
use winit::keyboard::{PhysicalKey, KeyCode};

#[cfg(target_os = "windows")]
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::{
            Direct3D::*,
            Direct3D11::*,
            Direct3D::Fxc::*,
            Dxgi::{Common::*, *},
        },
    },
};

// Engine modules
mod assets;
mod compiler;
mod graph;
mod recent_projects;
pub mod settings;
pub mod themes;
mod ui;
pub use assets::Assets;

// Engine constants
pub const ENGINE_NAME: &str = env!("CARGO_PKG_NAME");
pub const ENGINE_LICENSE: &str = env!("CARGO_PKG_LICENSE");
pub const ENGINE_AUTHORS: &str = env!("CARGO_PKG_AUTHORS");
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const ENGINE_HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");
pub const ENGINE_REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
pub const ENGINE_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
pub const ENGINE_LICENSE_FILE: &str = env!("CARGO_PKG_LICENSE_FILE");

// Engine actions
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectScrollbarShow(ScrollbarShow);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectLocale(SharedString);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectFont(usize);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectRadius(usize);

#[derive(Action, Clone, PartialEq, Eq)]
#[action(namespace = pulsar, no_json)]
pub struct OpenSettings;


// Helper function to convert Winit MouseButton to GPUI MouseButton
fn convert_mouse_button(button: WinitMouseButton) -> MouseButton {
    match button {
        WinitMouseButton::Left => MouseButton::Left,
        WinitMouseButton::Right => MouseButton::Right,
        WinitMouseButton::Middle => MouseButton::Middle,
        WinitMouseButton::Back => MouseButton::Navigate(NavigationDirection::Back),
        WinitMouseButton::Forward => MouseButton::Navigate(NavigationDirection::Forward),
        WinitMouseButton::Other(_) => MouseButton::Left, // Fallback to left for unknown buttons
    }
}

// Helper to convert winit modifiers to GPUI modifiers
fn convert_modifiers(winit_mods: &winit::keyboard::ModifiersState) -> Modifiers {
    Modifiers {
        control: winit_mods.control_key(),
        alt: winit_mods.alt_key(),
        shift: winit_mods.shift_key(),
        platform: winit_mods.super_key(), // Windows key on Windows, Command on Mac
        function: false, // Winit doesn't track function key separately
    }
}

// Simple click state tracking for double-click detection
#[derive(Debug, Clone)]
struct SimpleClickState {
    last_button: MouseButton,
    last_click_time: Instant,
    last_click_position: Point<Pixels>,
    current_count: usize,
    double_click_distance: f32, // pixels
    double_click_duration: std::time::Duration,
}

impl SimpleClickState {
    fn new() -> Self {
        Self {
            last_button: MouseButton::Left,
            last_click_time: Instant::now(),
            last_click_position: point(px(0.0), px(0.0)),
            current_count: 0,
            double_click_distance: 4.0, // Standard double-click tolerance
            double_click_duration: std::time::Duration::from_millis(500),
        }
    }

    fn update(&mut self, button: MouseButton, position: Point<Pixels>) -> usize {
        let now = Instant::now();
        // Calculate distance using pixel operations
        let dx = (position.x - self.last_click_position.x).abs();
        let dy = (position.y - self.last_click_position.y).abs();
        // Simple Manhattan distance to avoid accessing private fields
        let distance = dx.max(dy);
        
        if button == self.last_button 
            && now.duration_since(self.last_click_time) < self.double_click_duration
            && distance < px(self.double_click_distance) {
            self.current_count += 1;
        } else {
            self.current_count = 1;
        }
        
        self.last_button = button;
        self.last_click_time = now;
        self.last_click_position = position;
        self.current_count
    }
}

// Motion smoothing system for interpolated mouse movement  
// Like client-side prediction in multiplayer games
#[derive(Debug, Clone)]
struct MotionSmoother {
    interpolated_position: Point<Pixels>,
    target_position: Point<Pixels>,
    velocity: Point<Pixels>,
    last_update: Instant,
    smoothing_factor: f32,
    min_delta: f32,
    min_event_interval: Duration,
    last_event_time: Instant,
}

impl MotionSmoother {
    fn new() -> Self {
        Self {
            interpolated_position: point(px(0.0), px(0.0)),
            target_position: point(px(0.0), px(0.0)),
            velocity: point(px(0.0), px(0.0)),
            last_update: Instant::now(),
            smoothing_factor: 0.35,
            min_delta: 0.5,
            min_event_interval: Duration::from_micros(6944),
            last_event_time: Instant::now(),
        }
    }
    
    fn update_target(&mut self, new_position: Point<Pixels>) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32();
        
        if dt > 0.0 {
            let nx: f32 = new_position.x.into();
            let tx: f32 = self.target_position.x.into();
            let ny: f32 = new_position.y.into();
            let ty: f32 = self.target_position.y.into();
            
            self.velocity = point(
                px((nx - tx) / dt),
                px((ny - ty) / dt),
            );
        }
        
        self.target_position = new_position;
        self.last_update = now;
    }
    
    fn interpolate(&mut self) -> Point<Pixels> {
        let now = Instant::now();
        let dt = now.duration_since(self.last_update).as_secs_f32().min(0.1);
        
        let alpha = 1.0 - (1.0 - self.smoothing_factor).powf(dt * 60.0);
        
        let x: f32 = self.interpolated_position.x.into();
        let tx: f32 = self.target_position.x.into();
        self.interpolated_position.x = px(x + (tx - x) * alpha);
        
        let y: f32 = self.interpolated_position.y.into();
        let ty: f32 = self.target_position.y.into();
        self.interpolated_position.y = px(y + (ty - y) * alpha);
        
        self.interpolated_position
    }
    
    fn should_send_event(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_event_time);
        
        if elapsed < self.min_event_interval {
            return false;
        }
        
        let ix: f32 = self.interpolated_position.x.into();
        let tx: f32 = self.target_position.x.into();
        let iy: f32 = self.interpolated_position.y.into();
        let ty: f32 = self.target_position.y.into();
        
        let dx = (ix - tx).abs();
        let dy = (iy - ty).abs();
        
        if dx < self.min_delta && dy < self.min_delta {
            return false;
        }
        
        self.last_event_time = now;
        true
    }
}

fn main() {
    println!("{}", ENGINE_NAME);
    println!("Version: {}", ENGINE_VERSION);
    println!("Authors: {}", ENGINE_AUTHORS);
    println!("Description: {}", ENGINE_DESCRIPTION);
    println!("🚀 Starting Pulsar Engine with Winit + GPUI Zero-Copy Composition\n");

    // Determine app data directory
    let proj_dirs = ProjectDirs::from("com", "Pulsar", "Pulsar_Engine")
        .expect("Could not determine app data directory");
    let appdata_dir = proj_dirs.data_dir();
    let themes_dir = appdata_dir.join("themes");
    let config_dir = appdata_dir.join("configs");
    let config_file = config_dir.join("engine.toml");

    println!("App data directory: {:?}", appdata_dir);
    println!("Themes directory: {:?}", themes_dir);
    println!("Config directory: {:?}", config_dir);
    println!("Config file: {:?}", config_file);

    // Extract bundled themes if not present
    if !themes_dir.exists() {
        if let Err(e) = fs::create_dir_all(&themes_dir) {
            eprintln!("Failed to create themes directory: {e}");
        } else {
            // Copy all themes from project themes/ to appdata_dir/themes/
            let project_themes_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("themes");
            if let Ok(entries) = fs::read_dir(&project_themes_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(name) = path.file_name() {
                            let dest = themes_dir.join(name);
                            let _ = fs::copy(&path, &dest);
                        }
                    }
                }
            }
        }
    }

    // Create default config if not present
    if !config_file.exists() {
        if let Err(e) = fs::create_dir_all(&config_dir) {
            eprintln!("Failed to create config directory: {e}");
        }
        let default_settings = EngineSettings::default();
        default_settings.save(&config_file);
    }

    // Load settings
    println!("Loading engine settings from {:?}", config_file);
    let _engine_settings = EngineSettings::load(&config_file);

    // Initialize Tokio runtime for engine backend
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(8)
        .thread_name("PulsarEngineRuntime")
        .enable_all()
        .build()
        .unwrap();

    // Init the Game engine backend (subsystems, etc)
    rt.block_on(engine_backend::EngineBackend::init());

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    // Use Wait mode for event-driven rendering (only render when needed)
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut app = WinitGpuiApp::new();
    event_loop.run_app(&mut app).expect("Failed to run event loop");
}

struct WinitGpuiApp {
    winit_window: Option<Arc<WinitWindow>>,
    gpui_app: Option<Application>,
    gpui_window: Option<WindowHandle<Root>>,
    gpui_window_initialized: bool,
    needs_render: bool, // Flag to track if GPUI needs rendering
    
    // State tracking for proper event forwarding to GPUI
    last_cursor_position: Point<Pixels>,
    motion_smoother: MotionSmoother,
    current_modifiers: Modifiers,
    pressed_mouse_buttons: HashSet<MouseButton>,
    click_state: SimpleClickState,
    
    #[cfg(target_os = "windows")]
    d3d_device: Option<ID3D11Device>,
    #[cfg(target_os = "windows")]
    d3d_context: Option<ID3D11DeviceContext>,
    #[cfg(target_os = "windows")]
    shared_texture: Option<ID3D11Texture2D>,
    #[cfg(target_os = "windows")]
    shared_texture_initialized: bool,
    #[cfg(target_os = "windows")]
    swap_chain: Option<IDXGISwapChain1>,
    #[cfg(target_os = "windows")]
    blend_state: Option<ID3D11BlendState>,
    #[cfg(target_os = "windows")]
    render_target_view: Option<ID3D11RenderTargetView>,
    #[cfg(target_os = "windows")]
    vertex_shader: Option<ID3D11VertexShader>,
    #[cfg(target_os = "windows")]
    pixel_shader: Option<ID3D11PixelShader>,
    #[cfg(target_os = "windows")]
    vertex_buffer: Option<ID3D11Buffer>,
    #[cfg(target_os = "windows")]
    input_layout: Option<ID3D11InputLayout>,
    #[cfg(target_os = "windows")]
    sampler_state: Option<ID3D11SamplerState>,
    #[cfg(target_os = "windows")]
    persistent_gpui_texture: Option<ID3D11Texture2D>, // Our copy of GPUI's texture that persists
    #[cfg(target_os = "windows")]
    persistent_gpui_srv: Option<ID3D11ShaderResourceView>, // Cached SRV for persistent texture (no per-frame alloc)
}

impl WinitGpuiApp {
    fn new() -> Self {
        Self {
            winit_window: None,
            gpui_app: None,
            gpui_window: None,
            gpui_window_initialized: false,
            needs_render: true, // Start with true to render initial frame
            last_cursor_position: point(px(0.0), px(0.0)),
            motion_smoother: MotionSmoother::new(),
            current_modifiers: Modifiers::default(),
            pressed_mouse_buttons: HashSet::new(),
            click_state: SimpleClickState::new(),
            #[cfg(target_os = "windows")]
            d3d_device: None,
            #[cfg(target_os = "windows")]
            d3d_context: None,
            #[cfg(target_os = "windows")]
            shared_texture: None,
            #[cfg(target_os = "windows")]
            shared_texture_initialized: false,
            #[cfg(target_os = "windows")]
            swap_chain: None,
            #[cfg(target_os = "windows")]
            blend_state: None,
            #[cfg(target_os = "windows")]
            render_target_view: None,
            #[cfg(target_os = "windows")]
            vertex_shader: None,
            #[cfg(target_os = "windows")]
            pixel_shader: None,
            #[cfg(target_os = "windows")]
            vertex_buffer: None,
            #[cfg(target_os = "windows")]
            input_layout: None,
            #[cfg(target_os = "windows")]
            sampler_state: None,
            #[cfg(target_os = "windows")]
            persistent_gpui_texture: None,
            #[cfg(target_os = "windows")]
            persistent_gpui_srv: None,
        }
    }
}

impl ApplicationHandler for WinitGpuiApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.winit_window.is_some() {
            return;
        }

        println!("✅ Creating Winit window...");

        let window_attributes = WinitWindow::default_attributes()
            .with_title("Winit + GPUI Zero-Copy Demo")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
            .with_transparent(false);

        let winit_window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("Failed to create window"),
        );

        println!("✅ Winit window created");
        println!("✅ Initializing GPUI...");

        let app = Application::new().with_assets(Assets);

        self.winit_window = Some(winit_window);
        self.gpui_app = Some(app);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // Only handle events for our winit window
        if let Some(winit_window) = &self.winit_window {
            if winit_window.id() != window_id {
                return;
            }

            match event {
                WindowEvent::CloseRequested => {
                    println!("\n👋 Closing application...");
                    event_loop.exit();
                }
                WindowEvent::RedrawRequested => {
                    #[cfg(target_os = "windows")]
                    unsafe {
                        // Only render if GPUI requested it or we need to render
                        if self.needs_render && self.gpui_app.is_some() {
                            let gpui_app = self.gpui_app.as_mut().unwrap();

                            // First refresh windows (marks windows as dirty)
                            let _ = gpui_app.update(|app| {
                                app.refresh_windows();
                            });
                            // After update finishes, effects are flushed
                            // Now manually trigger drawing
                            let _ = gpui_app.update(|app| {
                                app.draw_windows();
                            });

                            // Reset the flag after rendering
                            self.needs_render = false;
                        }

                        // Lazy initialization of shared texture on first render
                        if !self.shared_texture_initialized && self.gpui_window.is_some() && self.gpui_app.is_some() && self.d3d_device.is_some() {
                            let gpui_window = self.gpui_window.as_ref().unwrap();
                            let gpui_app = self.gpui_app.as_mut().unwrap();
                            let device = self.d3d_device.as_ref().unwrap();

                            // Get the shared texture handle from GPUI using the new update method
                            let handle_result = gpui_app.update(|app| {
                                gpui_window.update(app, |_view, window, _cx| {
                                    window.get_shared_texture_handle()
                                })
                            });

                            if let Ok(handle_ptr) = handle_result {
                                if let Some(handle_ptr) = handle_ptr {
                                    println!("✅ Got shared texture handle from GPUI: {:p}", handle_ptr);

                                    // Open the shared texture using OpenSharedResource (legacy API)
                                    // GPUI uses GetSharedHandle() which requires the legacy API
                                    let mut texture: Option<ID3D11Texture2D> = None;
                                    let result = device.OpenSharedResource(
                                        HANDLE(handle_ptr as _),
                                        &mut texture
                                    );

                                    match result {
                                        Ok(_) => {
                                            if let Some(shared_texture) = texture {
                                                // Get texture description to create our persistent copy
                                                let mut desc = D3D11_TEXTURE2D_DESC::default();
                                                shared_texture.GetDesc(&mut desc);

                                                // Create persistent texture (not shared, just ours)
                                                desc.MiscFlags = D3D11_RESOURCE_MISC_FLAG(0).0 as u32; // Remove shared flag
                                                desc.Usage = D3D11_USAGE_DEFAULT;
                                                desc.BindFlags = D3D11_BIND_SHADER_RESOURCE.0 as u32;

                                                let mut persistent_texture: Option<ID3D11Texture2D> = None;
                                                let create_result = device.CreateTexture2D(&desc, None, Some(&mut persistent_texture));

                                                if create_result.is_ok() && persistent_texture.is_some() {
                                                    let tex = persistent_texture.as_ref().unwrap();

                                                    // CRITICAL: Create SRV once here, not per-frame
                                                    // This prevents memory leaks from allocating SRV every frame
                                                    let mut srv: Option<ID3D11ShaderResourceView> = None;
                                                    let srv_result = device.CreateShaderResourceView(tex, None, Some(&mut srv));

                                                    if srv_result.is_ok() && srv.is_some() {
                                                        self.persistent_gpui_srv = srv;
                                                        println!("✅ Created cached SRV for persistent texture (no per-frame alloc)");
                                                    } else {
                                                        eprintln!("❌ Failed to create SRV: {:?}", srv_result);
                                                    }

                                                    self.persistent_gpui_texture = persistent_texture;
                                                    println!("✅ Created persistent GPUI texture buffer!");
                                                } else {
                                                    eprintln!("❌ Failed to create persistent texture: {:?}", create_result);
                                                }

                                                self.shared_texture = Some(shared_texture);
                                                self.shared_texture_initialized = true;
                                                println!("✅ Opened shared texture in winit D3D11 device!");
                                            }
                                        }
                                        Err(e) => {
                                            println!("❌ Failed to open shared texture: {:?}", e);
                                            self.shared_texture_initialized = true;
                                        }
                                    }
                                } else {
                                    println!("⚠️  GPUI hasn't created shared texture yet, will retry next frame");
                                }
                            }
                        }

                        // Note: We don't present here - we'll present once after compositing GPUI on top

                        // GPU-side zero-copy composition: Winit renders green, then GPUI texture on top
                        // CRITICAL: Only present frames when we have valid GPUI content to avoid flickering
                        if let (Some(context), Some(shared_texture), Some(persistent_texture), Some(srv), Some(swap_chain), Some(render_target_view), Some(blend_state), Some(vertex_shader), Some(pixel_shader), Some(vertex_buffer), Some(input_layout), Some(sampler_state)) =
                            (&self.d3d_context, &self.shared_texture, &self.persistent_gpui_texture, &self.persistent_gpui_srv, &self.swap_chain, &self.render_target_view, &self.blend_state, &self.vertex_shader, &self.pixel_shader, &self.vertex_buffer, &self.input_layout, &self.sampler_state) {

                            // Copy from GPUI's shared texture to our persistent buffer
                            // This preserves the last rendered frame even if GPUI doesn't re-render
                            context.CopyResource(persistent_texture, shared_texture);

                            // Clear to green (bottom layer) - immediate mode background
                            let green = [0.0f32, 1.0, 0.0, 1.0];
                            context.ClearRenderTargetView(render_target_view, &green);

                            // Set render target
                            context.OMSetRenderTargets(Some(&[Some(render_target_view.clone())]), None);

                            // Set blend state for alpha blending (top layer)
                            let blend_factor = [0.0f32, 0.0, 0.0, 0.0];
                            context.OMSetBlendState(Some(blend_state), Some(&blend_factor), 0xffffffff);

                            // Use cached SRV (no per-frame allocation!)
                            {
                                static mut FRAME_COUNT: u32 = 0;
                                FRAME_COUNT += 1;
                                if FRAME_COUNT % 60 == 1 {
                                    eprintln!("🎨 Compositing GPUI texture (frame {})", FRAME_COUNT);
                                }

                                // Set shaders
                                context.VSSetShader(vertex_shader, None);
                                context.PSSetShader(pixel_shader, None);

                                // Set input layout
                                context.IASetInputLayout(input_layout);

                                // Set vertex buffer (fullscreen quad)
                                let stride = 16u32; // 2 floats pos + 2 floats tex = 16 bytes
                                let offset = 0u32;
                                context.IASetVertexBuffers(0, 1, Some(&Some(vertex_buffer.clone())), Some(&stride), Some(&offset));

                                // Set topology
                                context.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);

                                // Set GPUI texture and sampler
                                context.PSSetShaderResources(0, Some(&[Some(srv.clone())]));
                                context.PSSetSamplers(0, Some(&[Some(sampler_state.clone())]));

                                // Set viewport - must use physical pixels
                                // inner_size() already returns physical pixels (logical × scale factor)
                                let size = winit_window.inner_size();
                                let viewport = D3D11_VIEWPORT {
                                    TopLeftX: 0.0,
                                    TopLeftY: 0.0,
                                    Width: size.width as f32,
                                    Height: size.height as f32,
                                    MinDepth: 0.0,
                                    MaxDepth: 1.0,
                                };
                                context.RSSetViewports(Some(&[viewport]));

                                // Draw fullscreen quad with GPUI texture on top of green
                                context.Draw(4, 0);

                                // ONLY present when we successfully composited GPUI content
                                // This prevents flickering of green-only frames
                                let _ = swap_chain.Present(1, DXGI_PRESENT(0));
                            }
                        } else {
                            // Don't present if we don't have GPUI texture ready yet
                            // This shows the last valid frame instead of flickering
                            static mut SKIP_COUNT: u32 = 0;
                            SKIP_COUNT += 1;
                            if SKIP_COUNT <= 3 || SKIP_COUNT % 60 == 0 {
                                eprintln!("⏭️  Skipping frame {} - waiting for GPUI texture to be ready", SKIP_COUNT);
                            }
                        }
                    }

                    // Don't continuously request redraws - only render when events occur or GPUI requests it
                }
                // Handle keyboard events - request redraw
                WindowEvent::KeyboardInput { event, .. } => {
                    // Forward keyboard events to GPUI
                    if let (Some(gpui_window), Some(gpui_app)) = (&self.gpui_window, &mut self.gpui_app) {
                        // Store event and create keystroke before borrowing
                        let current_modifiers = self.current_modifiers;
                        
                        // Get the key string
                        let keystroke_opt = match &event.physical_key {
                            PhysicalKey::Code(code) => {
                                if let Some(key) = Self::keycode_to_string_static(*code) {
                                    let key_char = match &event.text {
                                        Some(text) if !text.is_empty() => Some(text.chars().next().map(|c| c.to_string()).unwrap_or_default()),
                                        _ => None,
                                    };
                                    
                                    Some(Keystroke {
                                        modifiers: current_modifiers,
                                        key,
                                        key_char,
                                    })
                                } else {
                                    None
                                }
                            }
                            PhysicalKey::Unidentified(_) => None,
                        };
                        
                        if let Some(keystroke) = keystroke_opt {
                            let gpui_event = match event.state {
                                ElementState::Pressed => {
                                    PlatformInput::KeyDown(KeyDownEvent {
                                        keystroke,
                                        is_held: event.repeat,
                                    })
                                }
                                ElementState::Released => {
                                    PlatformInput::KeyUp(KeyUpEvent { keystroke })
                                }
                            };

                            let _ = gpui_app.update(|cx| gpui_window.inject_input_event(cx, gpui_event));
                        }
                    }
                    
                    self.needs_render = true;
                    if let Some(window) = &self.winit_window {
                        window.request_redraw();
                    }
                }
                WindowEvent::ModifiersChanged(new_modifiers) => {
                    // Update tracked modifiers
                    self.current_modifiers = convert_modifiers(&new_modifiers.state());
                    
                    // Forward modifier changes to GPUI
                    if let (Some(gpui_window), Some(gpui_app)) = (&self.gpui_window, &mut self.gpui_app) {
                        let gpui_event = PlatformInput::ModifiersChanged(ModifiersChangedEvent {
                            modifiers: self.current_modifiers,
                            capslock: Capslock { on: false }, // TODO: Track capslock state
                        });

                        let _ = gpui_app.update(|cx| gpui_window.inject_input_event(cx, gpui_event));
                    }
                    
                    self.needs_render = true;
                    if let Some(window) = &self.winit_window {
                        window.request_redraw();
                    }
                }
                // Handle window resize - resize GPUI renderer and request redraw
                WindowEvent::Resized(new_size) => {
                    // Tell GPUI to resize its internal rendering buffers AND update logical size
                    if let (Some(gpui_app), Some(gpui_window), Some(winit_window)) =
                        (&mut self.gpui_app, &self.gpui_window, &self.winit_window) {

                        let scale_factor = winit_window.scale_factor() as f32;

                        // Physical pixels for renderer (what GPU renders at)
                        let physical_size = gpui::size(
                            gpui::DevicePixels(new_size.width as i32),
                            gpui::DevicePixels(new_size.height as i32),
                        );

                        // Logical pixels for GPUI layout (physical / scale)
                        let logical_size = gpui::size(
                            gpui::px(new_size.width as f32 / scale_factor),
                            gpui::px(new_size.height as f32 / scale_factor),
                        );

                        let _ = gpui_app.update(|app| {
                            let _ = gpui_window.update(app, |_view, window, _cx| {
                                #[cfg(target_os = "windows")]
                                {
                                    // Resize renderer (GPU buffers)
                                    if let Err(e) = window.resize_renderer(physical_size) {
                                        eprintln!("❌ Failed to resize GPUI renderer: {:?}", e);
                                    } else {
                                        println!("✅ Resized GPUI renderer to {:?}", physical_size);

                                        // CRITICAL: GPUI recreates its texture when resizing, so we need to re-obtain the shared handle
                                        // Mark for re-initialization on next frame
                                        self.shared_texture_initialized = false;
                                        self.shared_texture = None;
                                        self.persistent_gpui_texture = None;
                                        self.persistent_gpui_srv = None; // Also clear cached SRV
                                        println!("🔄 Marked shared texture for re-initialization after GPUI resize");
                                    }

                                    // Update logical size (for UI layout)
                                    window.update_logical_size(logical_size);
                                    println!("✅ Updated GPUI logical size to {:?} (scale {})", logical_size, scale_factor);

                                    // CRITICAL: Mark window as dirty to trigger UI re-layout
                                    // This is what GPUI's internal windows do in bounds_changed()
                                    window.refresh();
                                    println!("🎨 Marked window for refresh/re-layout");
                                }
                            });
                        });
                    }

                    // CRITICAL: Resize the swap chain to match the new window size
                    // This is why the green background was stuck at the original size!
                    if let Some(swap_chain) = &self.swap_chain {
                        unsafe {
                            // Release the render target view before resizing
                            self.render_target_view = None;

                            // Resize swap chain buffers
                            let resize_result = swap_chain.ResizeBuffers(
                                0,  // Keep current buffer count
                                new_size.width,
                                new_size.height,
                                DXGI_FORMAT_UNKNOWN,  // Keep current format
                                DXGI_SWAP_CHAIN_FLAG(0),  // No flags
                            );

                            if let Err(e) = resize_result {
                                eprintln!("❌ Failed to resize swap chain: {:?}", e);
                            } else {
                                println!("✅ Resized swap chain to {}x{}", new_size.width, new_size.height);

                                // Recreate render target view
                                if let Some(device) = &self.d3d_device {
                                    let back_buffer: Option<ID3D11Texture2D> = swap_chain.GetBuffer(0).ok();
                                    if let Some(back_buffer) = back_buffer {
                                        let mut rtv: Option<ID3D11RenderTargetView> = None;
                                        let result = device.CreateRenderTargetView(&back_buffer, None, Some(&mut rtv));
                                        if result.is_ok() {
                                            self.render_target_view = rtv;
                                            println!("✅ Recreated render target view");
                                        } else {
                                            eprintln!("❌ Failed to create render target view: {:?}", result);
                                        }
                                    } else {
                                        eprintln!("❌ Failed to get back buffer after resize");
                                    }
                                }
                            }
                        }
                    }

                    self.needs_render = true;
                    if let Some(window) = &self.winit_window {
                        window.request_redraw();
                    }
                }
                WindowEvent::CursorMoved { position, .. } => {
                    // Update cursor position tracking
                    if let Some(winit_window) = &self.winit_window {
                        let scale_factor = winit_window.scale_factor() as f32;
                        let logical_x = position.x as f32 / scale_factor;
                        let logical_y = position.y as f32 / scale_factor;
                        self.last_cursor_position = point(px(logical_x), px(logical_y));
                        
                        // Debug output
                        eprintln!("🖱️ CursorMoved: physical ({}, {}), logical ({}, {})", 
                            position.x, position.y, logical_x, logical_y);
                    }
                    
                    // Forward mouse move events to GPUI using inject_input_event
                    if let (Some(gpui_window), Some(gpui_app)) = (&self.gpui_window, &mut self.gpui_app) {
                        let winit_window = self.winit_window.as_ref().unwrap();
                        let scale_factor = winit_window.scale_factor() as f32;

                        // Convert physical position to logical position
                        let logical_x = position.x as f32 / scale_factor;
                        let logical_y = position.y as f32 / scale_factor;

                        // Determine which button is pressed (if any)
                        let pressed_button = if self.pressed_mouse_buttons.contains(&MouseButton::Left) {
                            Some(MouseButton::Left)
                        } else if self.pressed_mouse_buttons.contains(&MouseButton::Right) {
                            Some(MouseButton::Right)
                        } else if self.pressed_mouse_buttons.contains(&MouseButton::Middle) {
                            Some(MouseButton::Middle)
                        } else {
                            None
                        };

                        let gpui_event = PlatformInput::MouseMove(MouseMoveEvent {
                            position: point(px(logical_x), px(logical_y)),
                            pressed_button,
                            modifiers: self.current_modifiers,
                        });

                        eprintln!("📤 Injecting MouseMove event...");
                        let result = gpui_app.update(|cx| gpui_window.inject_input_event(cx, gpui_event));
                        eprintln!("📥 MouseMove result: {:?}", result);

                        // Request redraw for cursor updates
                        self.needs_render = true;
                        winit_window.request_redraw();
                    }
                }
                WindowEvent::MouseInput { state, button, .. } => {
                    // Forward mouse button events to GPUI
                    if let (Some(gpui_window), Some(gpui_app)) = (&self.gpui_window, &mut self.gpui_app) {
                        let gpui_button = convert_mouse_button(button);
                        // Use actual cursor position for clicks, not smoothed position!
                        let position = self.last_cursor_position;

                        match state {
                            ElementState::Pressed => {
                                eprintln!("🖱️ MouseInput PRESSED: {:?} at {:?}", button, position);
                                
                                // Track pressed button
                                self.pressed_mouse_buttons.insert(gpui_button);
                                
                                // Update click count
                                let click_count = self.click_state.update(gpui_button, position);
                                
                                let gpui_event = PlatformInput::MouseDown(MouseDownEvent {
                                    button: gpui_button,
                                    position,
                                    modifiers: self.current_modifiers,
                                    click_count,
                                    first_mouse: false,
                                });

                                eprintln!("📤 Injecting MouseDown event...");
                                let result = gpui_app.update(|cx| gpui_window.inject_input_event(cx, gpui_event));
                                eprintln!("📥 MouseDown result: {:?}", result);
                            }
                            ElementState::Released => {
                                eprintln!("🖱️ MouseInput RELEASED: {:?}", button);
                                
                                // Remove pressed button
                                self.pressed_mouse_buttons.remove(&gpui_button);
                                
                                let gpui_event = PlatformInput::MouseUp(MouseUpEvent {
                                    button: gpui_button,
                                    position,
                                    modifiers: self.current_modifiers,
                                    click_count: self.click_state.current_count,
                                });

                                eprintln!("📤 Injecting MouseUp event...");
                                let result = gpui_app.update(|cx| gpui_window.inject_input_event(cx, gpui_event));
                                eprintln!("📥 MouseUp result: {:?}", result);
                            }
                        }

                        // Request redraw for click feedback
                        self.needs_render = true;
                        if let Some(window) = &self.winit_window {
                            window.request_redraw();
                        }
                    }
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    // Forward mouse wheel events to GPUI
                    if let (Some(gpui_window), Some(gpui_app)) = (&self.gpui_window, &mut self.gpui_app) {
                        let winit_window = self.winit_window.as_ref().unwrap();

                        // Convert delta
                        let scroll_delta = match delta {
                            winit::event::MouseScrollDelta::LineDelta(x, y) => {
                                ScrollDelta::Lines(point(x, y))
                            }
                            winit::event::MouseScrollDelta::PixelDelta(pos) => {
                                let scale_factor = winit_window.scale_factor() as f32;
                                ScrollDelta::Pixels(point(
                                    px(pos.x as f32 / scale_factor),
                                    px(pos.y as f32 / scale_factor),
                                ))
                            }
                        };

                        // Use actual cursor position for scroll events
                        let position = self.last_cursor_position;

                        let gpui_event = PlatformInput::ScrollWheel(ScrollWheelEvent {
                            position,
                            delta: scroll_delta,
                            modifiers: self.current_modifiers,
                            touch_phase: TouchPhase::Moved,
                        });

                        let _ = gpui_app.update(|cx| gpui_window.inject_input_event(cx, gpui_event));

                        // Request redraw for scroll updates
                        self.needs_render = true;
                        winit_window.request_redraw();
                    }
                }
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Initialize GPUI window ONCE using external window API
        if !self.gpui_window_initialized && self.winit_window.is_some() && self.gpui_app.is_some() {
            let winit_window = self.winit_window.as_ref().unwrap().clone();
            let scale_factor = winit_window.scale_factor() as f32;
            let size = winit_window.inner_size();

            // GPUI bounds must be in LOGICAL pixels (physical / scale)
            // inner_size() returns physical pixels
            let logical_width = size.width as f32 / scale_factor;
            let logical_height = size.height as f32 / scale_factor;

            let bounds = Bounds {
                origin: point(px(0.0), px(0.0)),
                size: gpui::size(px(logical_width), px(logical_height)),
            };

            println!("🎯 Creating GPUI window: physical {}x{}, scale {}, logical {}x{}",
                size.width, size.height, scale_factor, logical_width, logical_height);

            let gpui_raw_handle = winit_window
                .window_handle()
                .expect("Failed to get window handle")
                .as_raw();

            let external_handle = ExternalWindowHandle {
                raw_handle: gpui_raw_handle,
                bounds,
                scale_factor,
                surface_handle: None,
            };

            println!("✅ Opening GPUI window on external winit window...");

            // Initialize GPUI components (fonts, themes, keybindings)
            let app = self.gpui_app.as_mut().unwrap();

            // Load custom fonts
            app.update(|app| {
                if let Some(font_data) = Assets::get("fonts/JetBrainsMono-Regular.ttf") {
                    match app.text_system().add_fonts(vec![font_data.data]) {
                        Ok(_) => println!("Successfully loaded JetBrains Mono font"),
                        Err(e) => println!("Failed to load JetBrains Mono font: {:?}", e),
                    }
                } else {
                    println!("Could not find JetBrains Mono font file");
                }

                // Initialize GPUI components
                gpui_component::init(app);
                crate::themes::init(app);
                crate::ui::terminal::init(app);

                // Setup keybindings
                app.bind_keys([
                    KeyBinding::new("ctrl-,", OpenSettings, None),
                    KeyBinding::new("ctrl-space", ToggleCommandPalette, None),
                ]);

                app.on_action(|_: &OpenSettings, app_cx| {
                    // TODO: Implement settings window opening for Winit
                    println!("Settings window requested (not yet implemented for Winit)");
                });

                app.activate(true);
            });

            println!("✅ GPUI components initialized");

            // Open GPUI window using external window API
            let gpui_window = app.open_window_external(external_handle.clone(), |window, cx| {
                let entry_view = cx.new(|cx| EntryWindow::new(window, cx));
                cx.new(|cx| Root::new(entry_view.clone().into(), window, cx))
            }).expect("Failed to open GPUI window");

            self.gpui_window = Some(gpui_window);

            // Initialize D3D11 for blitting on Windows
            #[cfg(target_os = "windows")]
            unsafe {
                println!("✅ Initializing D3D11 for GPU blitting...");

                let mut device = None;
                let mut context = None;
                let mut feature_level = Default::default();

                let result = D3D11CreateDevice(
                    None,
                    D3D_DRIVER_TYPE_HARDWARE,
                    HMODULE(std::ptr::null_mut()),
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    None,
                    D3D11_SDK_VERSION,
                    Some(&mut device),
                    Some(&mut feature_level),
                    Some(&mut context),
                );

                if result.is_ok() && device.is_some() {
                    self.d3d_device = device.clone();
                    self.d3d_context = context;
                    println!("✅ D3D11 device created successfully!");

                    // Create swap chain for the winit window
                    let parent_raw = winit_window.window_handle().unwrap().as_raw();
                    let hwnd = match parent_raw {
                        RawWindowHandle::Win32(h) => HWND(h.hwnd.get() as isize as *mut _),
                        _ => {
                            println!("❌ Failed to get HWND");
                            return;
                        }
                    };

                    let dxgi_device: IDXGIDevice = device.as_ref().unwrap().cast().unwrap();
                    let adapter = dxgi_device.GetAdapter().unwrap();
                    let dxgi_factory: IDXGIFactory2 = adapter.GetParent().unwrap();

                    // Swap chain must use physical pixels
                    // inner_size() already returns physical pixels (logical × scale factor)
                    let physical_width = size.width;
                    let physical_height = size.height;
                    println!("🎯 Creating swap chain: physical {}x{}, scale {}",
                        physical_width, physical_height, winit_window.scale_factor());

                    let swap_chain_desc = DXGI_SWAP_CHAIN_DESC1 {
                        Width: physical_width,
                        Height: physical_height,
                        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        Stereo: FALSE,
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                        BufferCount: 2,
                        Scaling: DXGI_SCALING_NONE,
                        SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                        AlphaMode: DXGI_ALPHA_MODE_IGNORE,  // Ignore alpha for solid window
                        Flags: 0,
                    };

                    let swap_chain = dxgi_factory.CreateSwapChainForHwnd(
                        device.as_ref().unwrap(),
                        hwnd,
                        &swap_chain_desc,
                        None,
                        None,
                    );
                    if let Ok(swap_chain) = swap_chain {
                        self.swap_chain = Some(swap_chain.clone());
                        println!("✅ Swap chain created for winit window!");

                        // Create render target view from swap chain back buffer
                        if let Ok(back_buffer) = swap_chain.GetBuffer::<ID3D11Texture2D>(0) {
                            let mut rtv: Option<ID3D11RenderTargetView> = None;
                            if device.as_ref().unwrap().CreateRenderTargetView(&back_buffer, None, Some(&mut rtv as *mut _)).is_ok() {
                                self.render_target_view = rtv;
                                println!("✅ Render target view created!");
                            }
                        }

                        // Create blend state for alpha blending
                        let blend_desc = D3D11_BLEND_DESC {
                            AlphaToCoverageEnable: FALSE,
                            IndependentBlendEnable: FALSE,
                            RenderTarget: [
                                D3D11_RENDER_TARGET_BLEND_DESC {
                                    BlendEnable: TRUE,
                                    SrcBlend: D3D11_BLEND_SRC_ALPHA,
                                    DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                                    BlendOp: D3D11_BLEND_OP_ADD,
                                    SrcBlendAlpha: D3D11_BLEND_ONE,
                                    DestBlendAlpha: D3D11_BLEND_ZERO,
                                    BlendOpAlpha: D3D11_BLEND_OP_ADD,
                                    RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
                                },
                                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                            ],
                        };

                        let mut blend_state = None;
                        if device.as_ref().unwrap().CreateBlendState(&blend_desc, Some(&mut blend_state as *mut _)).is_ok() {
                            self.blend_state = blend_state;
                            println!("✅ Blend state created for alpha composition!");
                        }

                        // Create shaders for GPU alpha blending by compiling HLSL at runtime
                        println!("🔨 Compiling shaders at runtime...");

                        // Vertex shader source: passthrough with position and texcoord
                        let vs_source = r#"
struct VS_INPUT {
    float2 pos : POSITION;
    float2 tex : TEXCOORD0;
};

struct PS_INPUT {
    float4 pos : SV_POSITION;
    float2 tex : TEXCOORD0;
};

PS_INPUT main(VS_INPUT input) {
    PS_INPUT output;
    output.pos = float4(input.pos, 0.0f, 1.0f);
    output.tex = input.tex;
    return output;
}
"#;

                        // Pixel shader source: sample texture with alpha
                        let ps_source = r#"
Texture2D gpuiTexture : register(t0);
SamplerState samplerState : register(s0);

struct PS_INPUT {
    float4 pos : SV_POSITION;
    float2 tex : TEXCOORD0;
};

float4 main(PS_INPUT input) : SV_TARGET {
    return gpuiTexture.Sample(samplerState, input.tex);
}
"#;

                        // Compile vertex shader
                        let vs_bytecode_blob = {
                            let mut blob: Option<ID3DBlob> = None;
                            let mut error_blob: Option<ID3DBlob> = None;
                            let result = D3DCompile(
                                vs_source.as_ptr() as *const _,
                                vs_source.len(),
                                None,
                                None,
                                None,
                                s!("main"),
                                s!("vs_5_0"),
                                0,
                                0,
                                &mut blob,
                                Some(&mut error_blob),
                            );

                            if result.is_err() {
                                if let Some(err) = error_blob {
                                    let err_msg = std::slice::from_raw_parts(
                                        err.GetBufferPointer() as *const u8,
                                        err.GetBufferSize(),
                                    );
                                    println!("❌ VS compile error: {}", String::from_utf8_lossy(err_msg));
                                }
                            }
                            blob
                        };

                        // Compile pixel shader
                        let ps_bytecode_blob = {
                            let mut blob: Option<ID3DBlob> = None;
                            let mut error_blob: Option<ID3DBlob> = None;
                            let result = D3DCompile(
                                ps_source.as_ptr() as *const _,
                                ps_source.len(),
                                None,
                                None,
                                None,
                                s!("main"),
                                s!("ps_5_0"),
                                0,
                                0,
                                &mut blob,
                                Some(&mut error_blob),
                            );

                            if result.is_err() {
                                if let Some(err) = error_blob {
                                    let err_msg = std::slice::from_raw_parts(
                                        err.GetBufferPointer() as *const u8,
                                        err.GetBufferSize(),
                                    );
                                    println!("❌ PS compile error: {}", String::from_utf8_lossy(err_msg));
                                }
                            }
                            blob
                        };

                        let vs_bytecode = if let Some(blob) = &vs_bytecode_blob {
                            std::slice::from_raw_parts(
                                blob.GetBufferPointer() as *const u8,
                                blob.GetBufferSize(),
                            )
                        } else {
                            &[] as &[u8]
                        };

                        let ps_bytecode = if let Some(blob) = &ps_bytecode_blob {
                            std::slice::from_raw_parts(
                                blob.GetBufferPointer() as *const u8,
                                blob.GetBufferSize(),
                            )
                        } else {
                            &[] as &[u8]
                        };

                        if vs_bytecode.is_empty() || ps_bytecode.is_empty() {
                            println!("❌ Shader compilation failed!");
                        }

                        // Create D3D11 shader objects from compiled bytecode
                        let mut vertex_shader = None;
                        let mut pixel_shader = None;

                        let vs_result = if !vs_bytecode.is_empty() {
                            device.as_ref().unwrap().CreateVertexShader(vs_bytecode, None, Some(&mut vertex_shader as *mut _))
                        } else {
                            Err(Error::from(E_FAIL))
                        };

                        let ps_result = if !ps_bytecode.is_empty() {
                            device.as_ref().unwrap().CreatePixelShader(ps_bytecode, None, Some(&mut pixel_shader as *mut _))
                        } else {
                            Err(Error::from(E_FAIL))
                        };

                        if vs_result.is_ok() && ps_result.is_ok() {
                            self.vertex_shader = vertex_shader;
                            self.pixel_shader = pixel_shader;
                            println!("✅ Shaders created from bytecode!");
                        } else {
                            println!("❌ Failed to create shaders - VS: {:?}, PS: {:?}", vs_result, ps_result);
                        }

                        if self.vertex_shader.is_some() && self.pixel_shader.is_some() {

                            // Create input layout that matches the vertex shader
                            // VS_INPUT has: float2 pos : POSITION; float2 tex : TEXCOORD0;
                            let layout = [
                                D3D11_INPUT_ELEMENT_DESC {
                                    SemanticName: s!("POSITION"),
                                    SemanticIndex: 0,
                                    Format: DXGI_FORMAT_R32G32_FLOAT,
                                    InputSlot: 0,
                                    AlignedByteOffset: 0,
                                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                                    InstanceDataStepRate: 0,
                                },
                                D3D11_INPUT_ELEMENT_DESC {
                                    SemanticName: s!("TEXCOORD"),
                                    SemanticIndex: 0,
                                    Format: DXGI_FORMAT_R32G32_FLOAT,
                                    InputSlot: 0,
                                    AlignedByteOffset: 8,
                                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                                    InstanceDataStepRate: 0,
                                },
                            ];

                            let mut input_layout = None;
                            if device.as_ref().unwrap().CreateInputLayout(&layout, vs_bytecode, Some(&mut input_layout as *mut _)).is_ok() {
                                self.input_layout = input_layout;
                                println!("✅ Input layout created!");
                            } else {
                                println!("❌ Failed to create input layout");
                            }
                        }

                        // Create vertex buffer for fullscreen quad
                        #[repr(C)]
                        struct Vertex {
                            pos: [f32; 2],
                            tex: [f32; 2],
                        }

                        let vertices = [
                            Vertex { pos: [-1.0, -1.0], tex: [0.0, 1.0] },
                            Vertex { pos: [-1.0,  1.0], tex: [0.0, 0.0] },
                            Vertex { pos: [ 1.0, -1.0], tex: [1.0, 1.0] },
                            Vertex { pos: [ 1.0,  1.0], tex: [1.0, 0.0] },
                        ];

                        let vb_desc = D3D11_BUFFER_DESC {
                            ByteWidth: std::mem::size_of_val(&vertices) as u32,
                            Usage: D3D11_USAGE_DEFAULT,
                            BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
                            CPUAccessFlags: 0,
                            MiscFlags: 0,
                            StructureByteStride: 0,
                        };

                        let vb_data = D3D11_SUBRESOURCE_DATA {
                            pSysMem: vertices.as_ptr() as *const _,
                            SysMemPitch: 0,
                            SysMemSlicePitch: 0,
                        };

                        let mut vertex_buffer = None;
                        if device.as_ref().unwrap().CreateBuffer(&vb_desc, Some(&vb_data), Some(&mut vertex_buffer as *mut _)).is_ok() {
                            self.vertex_buffer = vertex_buffer;
                            println!("✅ Vertex buffer created!");
                        }

                        // Create sampler state
                        let sampler_desc = D3D11_SAMPLER_DESC {
                            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
                            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
                            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
                            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
                            MipLODBias: 0.0,
                            MaxAnisotropy: 1,
                            ComparisonFunc: D3D11_COMPARISON_NEVER,
                            BorderColor: [0.0, 0.0, 0.0, 0.0],
                            MinLOD: 0.0,
                            MaxLOD: f32::MAX,
                        };

                        let mut sampler_state = None;
                        if device.as_ref().unwrap().CreateSamplerState(&sampler_desc, Some(&mut sampler_state as *mut _)).is_ok() {
                            self.sampler_state = sampler_state;
                            println!("✅ Sampler state created!");
                        }
                    } else {
                        println!("❌ Failed to create swap chain");
                    }

                    // Note: We'll get the shared texture handle lazily on first render
                    // GPUI creates the shared texture during its first draw call
                    println!("💡 Shared texture will be retrieved on first render");
                } else {
                    println!("❌ Failed to create D3D11 device: {:?}", result);
                }
            }

            self.gpui_window_initialized = true;
            println!("✅ GPUI window opened! Ready for GPU composition!\n");
        }
    }
}

impl WinitGpuiApp {
    // Helper to convert KeyCode to string (static so it can be used without &self borrow)
    fn keycode_to_string_static(code: KeyCode) -> Option<String> {
        use KeyCode::*;
        Some(match code {
            // Letters
            KeyA => "a",
            KeyB => "b",
            KeyC => "c",
            KeyD => "d",
            KeyE => "e",
            KeyF => "f",
            KeyG => "g",
            KeyH => "h",
            KeyI => "i",
            KeyJ => "j",
            KeyK => "k",
            KeyL => "l",
            KeyM => "m",
            KeyN => "n",
            KeyO => "o",
            KeyP => "p",
            KeyQ => "q",
            KeyR => "r",
            KeyS => "s",
            KeyT => "t",
            KeyU => "u",
            KeyV => "v",
            KeyW => "w",
            KeyX => "x",
            KeyY => "y",
            KeyZ => "z",
            
            // Numbers
            Digit0 => "0",
            Digit1 => "1",
            Digit2 => "2",
            Digit3 => "3",
            Digit4 => "4",
            Digit5 => "5",
            Digit6 => "6",
            Digit7 => "7",
            Digit8 => "8",
            Digit9 => "9",
            
            // Special keys
            Space => "space",
            Enter => "enter",
            Tab => "tab",
            Backspace => "backspace",
            Escape => "escape",
            Delete => "delete",
            Insert => "insert",
            Home => "home",
            End => "end",
            PageUp => "pageup",
            PageDown => "pagedown",
            
            // Arrow keys
            ArrowUp => "up",
            ArrowDown => "down",
            ArrowLeft => "left",
            ArrowRight => "right",
            
            // Function keys
            F1 => "f1",
            F2 => "f2",
            F3 => "f3",
            F4 => "f4",
            F5 => "f5",
            F6 => "f6",
            F7 => "f7",
            F8 => "f8",
            F9 => "f9",
            F10 => "f10",
            F11 => "f11",
            F12 => "f12",
            
            // Punctuation and symbols
            Minus => "-",
            Equal => "=",
            BracketLeft => "[",
            BracketRight => "]",
            Backslash => "\\",
            Semicolon => ";",
            Quote => "'",
            Comma => ",",
            Period => ".",
            Slash => "/",
            Backquote => "`",
            
            _ => return None,
        }.to_string())
    }
}

struct DemoView {
    counter: usize,
}

impl DemoView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { counter: 0 }
    }
}

impl Render for DemoView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.counter += 1;

        // Transparent background - let Winit's green show through
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap_4()
            .child(
                // Small blue square to show GPUI is rendering
                div()
                    .size(px(200.0))
                    .bg(rgb(0x4A90E2))
                    .rounded_lg()
                    .shadow_lg()
                    .border_2()
                    .border_color(rgb(0xFFFFFF))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(rgb(0xFFFFFF))
                            .child("GPUI"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(0x333333))
                            .child(format!("Frame: {}", self.counter)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x666666))
                            .child("✅ GPUI rendering on Winit window!"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x666666))
                            .child("🎨 Zero-copy GPU composition"),
                    ),
            )
    }
}
