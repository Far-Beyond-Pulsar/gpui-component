use gpui::*;
use gpui_component::{
    button::Button, dock::{Panel, PanelEvent}, h_flex, resizable::{h_resizable, resizable_panel, ResizableState}, v_flex, ActiveTheme as _, IconName, Selectable, StyledExt
};
use gpui_component::viewport_final::{Viewport, DoubleBuffer, RefreshHook, create_viewport_with_background_rendering};

use crate::ui::shared::{Toolbar, ToolbarButton, ViewportControls, StatusBar};
use crate::ui::rainbow_engine_final::{RainbowRenderEngine, RainbowPattern};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

pub struct LevelEditorPanel {
    focus_handle: FocusHandle,
    selected_object: Option<String>,
    viewport_controls: ViewportControls,
    show_wireframe: bool,
    show_lighting: bool,
    camera_mode: CameraMode,
    resizable_state: Entity<ResizableState>,
    viewport: Entity<Viewport>,
    
    // Rainbow engine state
    render_engine: Arc<Mutex<RainbowRenderEngine>>,
    buffers: Arc<DoubleBuffer>,
    refresh_hook: RefreshHook,
    current_pattern: RainbowPattern,
    render_speed: f32,
    render_enabled: Arc<std::sync::atomic::AtomicBool>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CameraMode {
    Perspective,
    Orthographic,
    Top,
    Front,
    Side,
}

impl LevelEditorPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let resizable_state = ResizableState::new(cx);
        
        // Create viewport with zero-copy background rendering
        let (viewport_instance, buffers, refresh_hook) = create_viewport_with_background_rendering(
            800, 
            600, 
            cx  // Context for background task setup
        );
        
        let viewport = cx.new_entity(viewport_instance);
        
        // Initialize rainbow render engine
        let render_engine = Arc::new(Mutex::new(RainbowRenderEngine::new()));
        let render_enabled = Arc::new(std::sync::atomic::AtomicBool::new(true));
        
        // Spawn render thread that uses the refresh hook to trigger GPUI reactive updates
        let engine_clone = render_engine.clone();
        let buffers_clone = buffers.clone();
        let hook_clone = refresh_hook.clone();
        let enabled_clone = render_enabled.clone();
        
        thread::spawn(move || {
            // Set thread priority and core affinity for dedicated rendering
            #[cfg(target_os = "windows")]
            {
                let handle = unsafe { windows::Win32::System::Threading::GetCurrentThread() };
                
                // Set to high priority for smooth rendering
                unsafe {
                    windows::Win32::System::Threading::SetThreadPriority(
                        handle, 
                        windows::Win32::System::Threading::THREAD_PRIORITY_ABOVE_NORMAL
                    );
                }
                
                // Try to pin to a dedicated core (last available core)
                let system_info = unsafe {
                    let mut si = std::mem::zeroed();
                    windows::Win32::System::SystemInformation::GetSystemInfo(&mut si);
                    si
                };
                
                // Use the last core for rendering to avoid main thread interference
                let num_cores = system_info.dwNumberOfProcessors;
                if num_cores > 1 {
                    let render_core_mask = 1usize << (num_cores - 1);
                    unsafe {
                        windows::Win32::System::Threading::SetThreadAffinityMask(handle, render_core_mask);
                    }
                    println!("[RENDER] Pinned render thread to core {}", num_cores - 1);
                }
            }
            
            Self::render_thread_controlled(engine_clone, buffers_clone, hook_clone, enabled_clone);
        });

        Self {
            focus_handle: cx.focus_handle(),
            selected_object: None,
            viewport_controls: ViewportControls::new(),
            show_wireframe: false,
            show_lighting: true,
            camera_mode: CameraMode::Perspective,
            resizable_state,
            viewport,
            render_engine,
            buffers,
            refresh_hook,
            current_pattern: RainbowPattern::Waves,
            render_speed: 2.0,
            render_enabled,
        }
    }

    /// Controlled render thread with proper double buffering and CPU throttling
    fn render_thread_controlled(
        engine: Arc<Mutex<RainbowRenderEngine>>,
        buffers: Arc<DoubleBuffer>,
        refresh_hook: RefreshHook,
        render_enabled: Arc<std::sync::atomic::AtomicBool>,
    ) {
        // Adaptive frame timing to prevent CPU pegging
        let base_frame_time = Duration::from_millis(8); // ~120 FPS baseline
        let mut adaptive_frame_time = base_frame_time;
        let mut frame_count = 0u64;
        let mut last_ui_refresh = std::time::Instant::now();
        let ui_refresh_interval = Duration::from_millis(16); // Limit UI refreshes to ~60fps
        
        // CPU throttling parameters
        let mut consecutive_fast_frames = 0u32;
        let _max_cpu_usage = 85; // Don't use more than 85% of the dedicated core
        
        while render_enabled.load(std::sync::atomic::Ordering::Relaxed) {
            let frame_start = std::time::Instant::now();
            let timing_debug = frame_count % 60 == 0; // Debug every 60 frames (~1 sec)
            
            // Step 1: Render to back buffer with exclusive access (DIRECT RGBA8)
            let render_start = std::time::Instant::now();
            let render_successful = if let Ok(mut engine_guard) = engine.try_lock() {
                let back_buffer = buffers.get_back_buffer();
                if let Ok(mut buffer_guard) = back_buffer.try_lock() {
                    // Render DIRECTLY in RGBA8 format - NO CONVERSION NEEDED
                    engine_guard.render_rgba8(&mut *buffer_guard);
                    true
                } else {
                    false
                }
            } else {
                false
            };
            let render_time = render_start.elapsed();
            
            // Step 2: Only if render was successful, do atomic swap (zero-copy)
            if render_successful {
                let swap_start = std::time::Instant::now();
                buffers.swap_buffers(); // Atomic pointer swap - zero operations
                let swap_time = swap_start.elapsed();
                
                // Step 3: Smart UI refresh with throttling
                let refresh_start = std::time::Instant::now();
                let now = std::time::Instant::now();
                if now.duration_since(last_ui_refresh) >= ui_refresh_interval {
                    refresh_hook();
                    last_ui_refresh = now;
                }
                let refresh_time = refresh_start.elapsed();
                
                if timing_debug {
                    println!("[RENDER-THREAD] Frame {}: render={}μs, swap={}μs, refresh={}μs", 
                        frame_count, 
                        render_time.as_micros(), 
                        swap_time.as_micros(), 
                        refresh_time.as_micros());
                }
                
                frame_count += 1;
            }
            
            let frame_time = frame_start.elapsed();
            
            // Adaptive CPU throttling to prevent core pegging
            if frame_time < adaptive_frame_time.mul_f32(0.5) {
                consecutive_fast_frames += 1;
                // If we're consistently finishing frames too quickly, throttle more
                if consecutive_fast_frames > 10 {
                    adaptive_frame_time = adaptive_frame_time.mul_f32(1.1).min(Duration::from_millis(16));
                    consecutive_fast_frames = 0;
                }
            } else {
                consecutive_fast_frames = 0;
                // If frames are taking longer, reduce throttling slightly
                adaptive_frame_time = adaptive_frame_time.mul_f32(0.99).max(Duration::from_millis(4));
            }
            
            // Apply CPU throttling with adaptive timing
            let work_time = frame_time;
            let sleep_time = if adaptive_frame_time > work_time {
                adaptive_frame_time - work_time
            } else {
                Duration::from_millis(1) // Minimum sleep to prevent 100% CPU
            };
            
            let throttle_start = std::time::Instant::now();
            std::thread::sleep(sleep_time);
            let throttle_total = throttle_start.elapsed();
            
            if timing_debug {
                println!("[RENDER-THREAD] Throttling: work_time={:.2}ms, sleep_time={:.2}ms, throttle_total={:.2}ms, adaptive_frame_time={:.2}ms",
                    work_time.as_secs_f64() * 1000.0,
                    sleep_time.as_secs_f64() * 1000.0,
                    throttle_total.as_secs_f64() * 1000.0,
                    adaptive_frame_time.as_secs_f64() * 1000.0);
            }
        }
    }

    fn get_engine_info(&self) -> String {
        if let Ok(engine) = self.render_engine.try_lock() {
            format!(
                "FPS: {:.1} | Frame: {} | Pattern: {:?} | Speed: {:.1}x",
                engine.get_fps(),
                engine.get_frame_count(),
                self.current_pattern,
                self.render_speed
            )
        } else {
            "Engine busy...".to_string()
        }
    }
}

impl Focusable for LevelEditorPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for LevelEditorPanel {}

impl Panel for LevelEditorPanel {
    fn panel_name(&self) -> &'static str {
        "Level Editor"
    }

    fn title(&self, _cx: &App) -> gpui::AnyElement {
        h_flex()
            .items_center()
            .child(
                h_flex()
                    .gap_2()
                    .child(IconName::Hammer.into_any_element())
                    .child("Level Editor")
            )
            .into_any_element()
    }
}

impl Render for LevelEditorPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let engine_info = self.get_engine_info();
        
        v_flex()
            .size_full()
            .child(
                // Top toolbar
                Toolbar::new()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                ToolbarButton::new("add_cube", IconName::Plus)
                                    .tooltip("Add Cube")
                                    .on_click(cx.listener(|this, _event, _cx| {
                                        this.selected_object = Some("New Cube".to_string());
                                    }))
                            )
                            .child(
                                ToolbarButton::new("select", IconName::MousePointer)
                                    .tooltip("Select Tool")
                                    .selected(true)
                            )
                            .child(
                                ToolbarButton::new("move", IconName::Move)
                                    .tooltip("Move Tool")
                            )
                            .child(
                                ToolbarButton::new("rotate", IconName::RotateCw)
                                    .tooltip("Rotate Tool")
                            )
                            .child(
                                ToolbarButton::new("scale", IconName::Maximize)
                                    .tooltip("Scale Tool")
                            )
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("wireframe")
                                    .label("Wireframe")
                                    .selected(self.show_wireframe)
                                    .on_click(cx.listener(|this, _event, _cx| {
                                        this.show_wireframe = !this.show_wireframe;
                                    }))
                            )
                            .child(
                                Button::new("lighting")
                                    .label("Lighting")
                                    .selected(self.show_lighting)
                                    .on_click(cx.listener(|this, _event, _cx| {
                                        this.show_lighting = !this.show_lighting;
                                    }))
                            )
                    )
            )
            .child(
                // Main content area with resizable panels
                h_resizable(
                    self.resizable_state.clone(),
                    resizable_panel()
                        .id("viewport")
                        .child(
                            v_flex()
                                .size_full()
                                .child(self.viewport_controls.clone())
                                .child(
                                    div()
                                        .flex_1()
                                        .size_full()
                                        .bg(cx.theme().colors().surface)
                                        .border_1()
                                        .border_color(cx.theme().colors().border)
                                        .child(self.viewport.clone())
                                )
                        ),
                    resizable_panel()
                        .id("properties")
                        .child(
                            v_flex()
                                .size_full()
                                .p_4()
                                .gap_4()
                                .child(
                                    v_flex()
                                        .gap_2()
                                        .child("Object Properties")
                                        .child(
                                            div()
                                                .p_2()
                                                .bg(cx.theme().colors().surface)
                                                .border_1()
                                                .border_color(cx.theme().colors().border)
                                                .child(
                                                    if let Some(ref obj) = self.selected_object {
                                                        format!("Selected: {}", obj)
                                                    } else {
                                                        "No object selected".to_string()
                                                    }
                                                )
                                        )
                                )
                                .child(
                                    v_flex()
                                        .gap_2()
                                        .child("Rainbow Engine")
                                        .child(
                                            v_flex()
                                                .gap_2()
                                                .child(
                                                    Button::new("pattern_waves")
                                                        .label("Waves")
                                                        .selected(matches!(self.current_pattern, RainbowPattern::Waves))
                                                        .on_click(cx.listener(|this, _event, _cx| {
                                                            this.current_pattern = RainbowPattern::Waves;
                                                            if let Ok(mut engine) = this.render_engine.try_lock() {
                                                                engine.set_pattern(RainbowPattern::Waves);
                                                            }
                                                        }))
                                                )
                                                .child(
                                                    Button::new("pattern_spiral")
                                                        .label("Spiral")
                                                        .selected(matches!(self.current_pattern, RainbowPattern::Spiral))
                                                        .on_click(cx.listener(|this, _event, _cx| {
                                                            this.current_pattern = RainbowPattern::Spiral;
                                                            if let Ok(mut engine) = this.render_engine.try_lock() {
                                                                engine.set_pattern(RainbowPattern::Spiral);
                                                            }
                                                        }))
                                                )
                                                .child(
                                                    Button::new("pattern_plasma")
                                                        .label("Plasma")
                                                        .selected(matches!(self.current_pattern, RainbowPattern::Plasma))
                                                        .on_click(cx.listener(|this, _event, _cx| {
                                                            this.current_pattern = RainbowPattern::Plasma;
                                                            if let Ok(mut engine) = this.render_engine.try_lock() {
                                                                engine.set_pattern(RainbowPattern::Plasma);
                                                            }
                                                        }))
                                                )
                                                .child(
                                                    Button::new("pattern_matrix")
                                                        .label("Matrix")
                                                        .selected(matches!(self.current_pattern, RainbowPattern::Matrix))
                                                        .on_click(cx.listener(|this, _event, _cx| {
                                                            this.current_pattern = RainbowPattern::Matrix;
                                                            if let Ok(mut engine) = this.render_engine.try_lock() {
                                                                engine.set_pattern(RainbowPattern::Matrix);
                                                            }
                                                        }))
                                                )
                                        )
                                )
                        )
                )
            )
            .child(
                // Bottom status bar
                StatusBar::new(&engine_info)
            )
            .focusable()
            .focus(self.focus_handle.clone())
    }
}
