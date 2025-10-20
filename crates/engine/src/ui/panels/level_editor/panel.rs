use gpui::*;
use gpui_component::{
    button::Button, dock::{Panel, PanelEvent}, h_flex, resizable::{h_resizable, resizable_panel, ResizableState}, v_flex, ActiveTheme as _, IconName, Selectable, StyledExt,
    chart::LineChart,
};
// use gpui_component::viewport_final::{Viewport, DoubleBuffer, RefreshHook, create_viewport_with_background_rendering};

use crate::ui::shared::{Toolbar, ToolbarButton, ViewportControls, StatusBar};
use crate::ui::rainbow_engine_final::{RainbowRenderEngine, RainbowPattern};
use engine_backend::{GameThread, GameState};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::collections::VecDeque;
use std::cell::RefCell;
// Stub types
type Viewport = ();
type DoubleBuffer = ();
type RefreshHook = fn();

#[derive(Clone)]
struct FpsDataPoint {
    index: usize,
    fps: f64,
}

#[derive(Clone)]
struct TpsDataPoint {
    index: usize,
    tps: f64,
}

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
    
    // Game thread state
    game_thread: Arc<GameThread>,
    
    // FPS tracking for rolling graph - using RefCell for interior mutability
    fps_history: RefCell<VecDeque<FpsDataPoint>>,
    fps_sample_counter: RefCell<usize>,
    
    // TPS tracking for rolling graph
    tps_history: RefCell<VecDeque<TpsDataPoint>>,
    tps_sample_counter: RefCell<usize>,
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
        println!("[LEVEL-EDITOR] ===== INITIALIZING LEVEL EDITOR PANEL =====");
        let resizable_state = ResizableState::new(cx);
        
        // Create viewport with zero-copy background rendering
        let (viewport, buffers, refresh_hook) = create_viewport_with_background_rendering(
            800, 
            600, 
            cx  // Context for background task setup
        );
        
        // Create rainbow render engine
        let render_engine = Arc::new(Mutex::new(RainbowRenderEngine::new()));
        let render_enabled = Arc::new(std::sync::atomic::AtomicBool::new(true));
        
        // Create and start game thread
        println!("[LEVEL-EDITOR] Creating game thread with target 60 TPS...");
        let game_thread = Arc::new(GameThread::new(60.0)); // Target 60 TPS
        println!("[LEVEL-EDITOR] Starting game thread...");
        game_thread.start();
        println!("[LEVEL-EDITOR] Game thread started successfully!");
        
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

        println!("[LEVEL-EDITOR] Created viewport and render engine, starting render thread...");
        println!("[LEVEL-EDITOR] ===== LEVEL EDITOR PANEL READY =====");

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
            game_thread,
            fps_history: RefCell::new(VecDeque::with_capacity(60)),
            fps_sample_counter: RefCell::new(0),
            tps_history: RefCell::new(VecDeque::with_capacity(60)),
            tps_sample_counter: RefCell::new(0),
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
        let max_cpu_usage = 85; // Don't use more than 85% of the dedicated core
        
        while render_enabled.load(std::sync::atomic::Ordering::Relaxed) {
            let frame_start = std::time::Instant::now();
            let timing_debug = frame_count % 60 == 0; // Debug every 60 frames (~1 sec)
            
            if frame_count == 0 {
                println!("[RENDER-THREAD] Starting render loop...");
            }
            
            // Step 1: Render to back buffer with exclusive access (DIRECT RGBA8)
            let render_start = std::time::Instant::now();
            let render_successful = if let Ok(mut engine_guard) = engine.try_lock() {
                let back_buffer = buffers.get_back_buffer();
                let buffer_lock_result = back_buffer.try_lock();
                if let Ok(mut buffer_guard) = buffer_lock_result {
                    // Render DIRECTLY in RGBA8 format - NO CONVERSION NEEDED
                    engine_guard.render_rgba8(&mut *buffer_guard);
                    if frame_count == 0 {
                        println!("[RENDER-THREAD] First frame rendered successfully to {}x{} buffer", 
                                buffer_guard.width, buffer_guard.height);
                    }
                    true
                } else {
                    if frame_count == 0 {
                        println!("[RENDER-THREAD] Failed to lock buffer");
                    }
                    false
                }
            } else {
                if frame_count == 0 {
                    println!("[RENDER-THREAD] Failed to lock render engine");
                }
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
                refresh_hook(); // Call refresh hook every successful frame
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
                // Gradually reduce throttling if we're not finishing too quickly
                adaptive_frame_time = adaptive_frame_time.mul_f32(0.99).max(base_frame_time);
            }
            
            // Calculate sleep time to maintain target CPU usage
            let throttle_start = std::time::Instant::now();
            let target_cpu_usage = max_cpu_usage as f32 / 100.0;
            let work_time = frame_time.as_secs_f32();
            let total_frame_time = work_time / target_cpu_usage;
            let sleep_time = Duration::from_secs_f32(total_frame_time - work_time).max(Duration::from_millis(1));
            
            thread::sleep(sleep_time);
            let throttle_time = throttle_start.elapsed();
            
            if timing_debug {
                println!("[RENDER-THREAD] Throttling: work_time={:.2}ms, sleep_time={:.2}ms, throttle_total={:.2}ms, adaptive_frame_time={:.2}ms", 
                    work_time * 1000.0, 
                    sleep_time.as_secs_f32() * 1000.0,
                    throttle_time.as_secs_f32() * 1000.0,
                    adaptive_frame_time.as_secs_f32() * 1000.0);
            }
            
            // Cooperative yielding every 30 frames for better system responsiveness
            if frame_count % 30 == 0 {
                thread::yield_now();
                // Extra yield for system responsiveness
                thread::sleep(Duration::from_micros(100));
            }
            
            // Periodic longer sleep to ensure we don't completely dominate the core
            if frame_count % 120 == 0 { // Every ~1 second at 120fps
                thread::sleep(Duration::from_millis(2));
            }
        }
    }

    pub fn toggle_rendering(&mut self) {
        let current = self.render_enabled.load(std::sync::atomic::Ordering::Relaxed);
        self.render_enabled.store(!current, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn toggle_game_thread(&mut self) {
        self.game_thread.toggle();
    }

    pub fn set_rainbow_pattern(&mut self, pattern: RainbowPattern) {
        self.current_pattern = pattern;
        if let Ok(mut engine) = self.render_engine.lock() {
            engine.set_pattern(pattern);
        }
    }

    pub fn set_render_speed(&mut self, speed: f32) {
        self.render_speed = speed;
        if let Ok(mut engine) = self.render_engine.lock() {
            engine.set_speed(speed);
        }
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Toolbar::new()
            .add_button(
                ToolbarButton::new(IconName::CursorPointer, "Select")
                    .tooltip("Select Tool (S)")
                    .active(true)
            )
            .add_button(
                ToolbarButton::new(IconName::Drag, "Move")
                    .tooltip("Move Tool (M)")
            )
            .add_button(
                ToolbarButton::new(IconName::RotateCameraRight, "Rotate")
                    .tooltip("Rotate Tool (R)")
            )
            .add_button(
                ToolbarButton::new(IconName::Enlarge, "Scale")
                    .tooltip("Scale Tool (T)")
            )
            .render(cx)
    }

    fn render_scene_hierarchy(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_2()
            .child(
                h_flex()
                    .w_full()
                    .p_2()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Scene Hierarchy")
                    )
                    .child(
                        Button::new("add_object")
                            .icon(IconName::Plus)
                            .tooltip("Add Object")
                    )
            )
            .child(
                div()
                    .flex_1()
                    .p_2()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .p_2()
                                    .rounded(cx.theme().radius)
                                    .bg(cx.theme().primary.opacity(0.1))
                                    .text_color(cx.theme().primary)
                                    .child("🎮 Main Camera")
                            )
                            .child(
                                div()
                                    .p_2()
                                    .rounded(cx.theme().radius)
                                    .hover(|style| style.bg(cx.theme().muted.opacity(0.5)))
                                    .child("☀️ Directional Light")
                            )
                            .child(
                                div()
                                    .p_2()
                                    .rounded(cx.theme().radius)
                                    .hover(|style| style.bg(cx.theme().muted.opacity(0.5)))
                                    .child("📦 Cube")
                            )
                            .child(
                                div()
                                    .p_2()
                                    .rounded(cx.theme().radius)
                                    .hover(|style| style.bg(cx.theme().muted.opacity(0.5)))
                                    .child("🟢 Sphere")
                            )
                    )
            )
    }

    fn render_viewport(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .bg(cx.theme().muted.opacity(0.2))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(div().size_full().child(self.viewport.clone()))
            .child(
                // Viewport controls overlay
                div()
                    .absolute()
                    .top_4()
                    .right_4()
                    .child(self.viewport_controls.render(cx))
            )
            .child(
                // Camera mode selector
                div()
                    .absolute()
                    .bottom_4()
                    .left_4()
                    .child(
                        h_flex()
                            .gap_2()
                            .p_2()
                            .bg(cx.theme().background.opacity(0.9))
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                Button::new("perspective")
                                    .child("Perspective")
                                    .selected(matches!(self.camera_mode, CameraMode::Perspective))
                            )
                            .child(
                                Button::new("orthographic")
                                    .child("Orthographic")
                                    .selected(matches!(self.camera_mode, CameraMode::Orthographic))
                            )
                    )
            )
            .child(
                // Performance overlay (debug)
                div()
                    .absolute()
                    .top_4()
                    .left_4()
                    .child(self.render_performance_overlay(cx))
            )
            .child(
                // Rainbow pattern controls
                div()
                    .absolute()
                    .bottom_4()
                    .right_4()
                    .child(
                        v_flex()
                            .gap_2()
                            .p_2()
                            .bg(cx.theme().background.opacity(0.9))
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("🌈 Rainbow Patterns")
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("waves")
                                            .child("Waves")
                                            .selected(matches!(self.current_pattern, RainbowPattern::Waves))
                                            .on_click(cx.listener(|this, _event, _window, _cx| {
                                                this.set_rainbow_pattern(RainbowPattern::Waves);
                                            }))
                                    )
                                    .child(
                                        Button::new("spiral")
                                            .child("Spiral")
                                            .selected(matches!(self.current_pattern, RainbowPattern::Spiral))
                                            .on_click(cx.listener(|this, _event, _window, _cx| {
                                                this.set_rainbow_pattern(RainbowPattern::Spiral);
                                            }))
                                    )
                                    .child(
                                        Button::new("plasma")
                                            .child("Plasma")
                                            .selected(matches!(self.current_pattern, RainbowPattern::Plasma))
                                            .on_click(cx.listener(|this, _event, _window, _cx| {
                                                this.set_rainbow_pattern(RainbowPattern::Plasma);
                                            }))
                                    )
                            )
                            .child(
                                h_flex()
                                    .gap_1()
                                    .child(
                                        Button::new("ripples")
                                            .child("Ripples")
                                            .selected(matches!(self.current_pattern, RainbowPattern::Ripples))
                                            .on_click(cx.listener(|this, _event, _window, _cx| {
                                                this.set_rainbow_pattern(RainbowPattern::Ripples);
                                            }))
                                    )
                                    .child(
                                        Button::new("matrix")
                                            .child("Matrix")
                                            .selected(matches!(self.current_pattern, RainbowPattern::Matrix))
                                            .on_click(cx.listener(|this, _event, _window, _cx| {
                                                this.set_rainbow_pattern(RainbowPattern::Matrix);
                                            }))
                                    )
                            )
                    )
            )
    }

    fn render_performance_overlay(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // Get rainbow engine metrics
        let (engine_fps, frame_count, pattern_name) = if let Ok(engine) = self.render_engine.lock() {
            let fps = engine.get_fps();
            let frames = engine.get_frame_count();
            let pattern = format!("{:?}", self.current_pattern);
            (fps, frames, pattern)
        } else {
            (0.0, 0, "Unknown".to_string())
        };

        // Get game thread metrics
        let game_tps = self.game_thread.get_tps();
        let game_tick_count = self.game_thread.get_tick_count();
        let game_enabled = self.game_thread.is_enabled();

        // Update FPS history for rolling graph using interior mutability
        let mut fps_history = self.fps_history.borrow_mut();
        let mut fps_sample_counter = self.fps_sample_counter.borrow_mut();
        
        fps_history.push_back(FpsDataPoint {
            index: *fps_sample_counter,
            fps: engine_fps as f64,
        });
        *fps_sample_counter += 1;
        
        // Keep only last 60 samples
        if fps_history.len() > 60 {
            fps_history.pop_front();
        }

        // Prepare data for the FPS chart
        let fps_data: Vec<FpsDataPoint> = fps_history.iter().cloned().collect();
        drop(fps_history);
        drop(fps_sample_counter);

        // Update TPS history for rolling graph
        let mut tps_history = self.tps_history.borrow_mut();
        let mut tps_sample_counter = self.tps_sample_counter.borrow_mut();
        
        tps_history.push_back(TpsDataPoint {
            index: *tps_sample_counter,
            tps: game_tps as f64,
        });
        *tps_sample_counter += 1;
        
        // Keep only last 60 samples
        if tps_history.len() > 60 {
            tps_history.pop_front();
        }

        // Prepare data for the TPS chart
        let tps_data: Vec<TpsDataPoint> = tps_history.iter().cloned().collect();
        drop(tps_history);
        drop(tps_sample_counter);

        v_flex()
            .gap_2()
            .p_3()
            .w(px(340.))
            .bg(cx.theme().background.opacity(0.95))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(if engine_fps > 200.0 { 
                                cx.theme().success 
                            } else if engine_fps > 120.0 { 
                                cx.theme().warning 
                            } else { 
                                cx.theme().accent 
                            })
                            .child(format!("🌈 {:.1} FPS", engine_fps))
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Target: 30"))
                    )
                    .child(
                        Button::new("toggle_render")
                            .child(if self.render_enabled.load(std::sync::atomic::Ordering::Relaxed) { "⏸" } else { "▶" })
                            .on_click(cx.listener(|this, _event, _window, _cx| {
                                this.toggle_rendering();
                            }))
                    )
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(if game_tps > 55.0 { 
                                cx.theme().success 
                            } else if game_tps > 30.0 { 
                                cx.theme().warning 
                            } else { 
                                cx.theme().destructive 
                            })
                            .child(format!("🎮 {:.1} TPS", game_tps))
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Target: 60"))
                    )
                    .child(
                        Button::new("toggle_game")
                            .child(if game_enabled { "⏸" } else { "▶" })
                            .on_click(cx.listener(|this, _event, _window, _cx| {
                                this.toggle_game_thread();
                            }))
                    )
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().foreground)
                            .child(format!("Frames: {} | Ticks: {}", frame_count, game_tick_count))
                    )
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Pattern: {}", pattern_name))
                    )
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("GPUI: Zero-copy"))
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Mode: Final"))
                    )
            )
            .when(!fps_data.is_empty(), |this| {
                this.child(
                    v_flex()
                        .w_full()
                        .mt_2()
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .pt_2()
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .mb_1()
                                .child("FPS Graph")
                        )
                        .child(
                            div()
                                .h(px(100.))
                                .w_full()
                                .child(
                                    LineChart::new(fps_data)
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.fps)
                                        .linear()
                                        .tick_margin(10)
                                )
                        )
                )
            })
            .when(!tps_data.is_empty(), |this| {
                this.child(
                    v_flex()
                        .w_full()
                        .mt_2()
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .pt_2()
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .mb_1()
                                .child("TPS Graph")
                        )
                        .child(
                            div()
                                .h(px(100.))
                                .w_full()
                                .child(
                                    LineChart::new(tps_data)
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.tps)
                                        .linear()
                                        .tick_margin(10)
                                )
                        )
                )
            })
    }
    
    fn render_properties(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_2()
            .child(
                h_flex()
                    .w_full()
                    .p_2()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Properties")
                    )
            )
            .child(
                div()
                    .flex_1()
                    .p_3()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(
                        if self.selected_object.is_some() {
                            v_flex()
                                .gap_3()
                                .child(
                                    div()
                                        .text_lg()
                                        .font_semibold()
                                        .text_color(cx.theme().foreground)
                                        .child("Cube")
                                )
                                .child(
                                    v_flex()
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_medium()
                                                .text_color(cx.theme().foreground)
                                                .child("Transform")
                                        )
                                        .child(self.render_transform_section(cx))
                                )
                                .child(
                                    v_flex()
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_medium()
                                                .text_color(cx.theme().foreground)
                                                .child("Material")
                                        )
                                        .child(self.render_material_section(cx))
                                )
                                .into_any_element()
                        } else {
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(cx.theme().muted_foreground)
                                .child("No object selected")
                                .into_any_element()
                        }
                    )
            )
    }

    fn render_transform_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(self.render_vector3_field("Position", (0.0, 0.0, 0.0), cx))
            .child(self.render_vector3_field("Rotation", (0.0, 0.0, 0.0), cx))
            .child(self.render_vector3_field("Scale", (1.0, 1.0, 1.0), cx))
    }

    fn render_material_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child("Material:")
                    )
                    .child(
                        Button::new("select_material")
                            .child("Default Material")
                    )
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child("Color:")
                    )
                    .child(
                        div()
                            .size_8()
                            .bg(cx.theme().primary)
                            .rounded(px(4.0))
                            .border_1()
                            .border_color(cx.theme().border)
                    )
            )
    }

    fn render_vector3_field(&self, label: &str, values: (f32, f32, f32), cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .items_center()
            .child(
                div()
                    .w_16()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(format!("{}:", label))
            )
            .child(
                h_flex()
                    .gap_1()
                    .child(self.render_float_input("X", values.0, cx))
                    .child(self.render_float_input("Y", values.1, cx))
                    .child(self.render_float_input("Z", values.2, cx))
            )
    }

    fn render_float_input(&self, axis: &str, value: f32, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_1()
            .items_center()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(axis.to_string())
            )
            .child(
                div()
                    .w_16()
                    .px_2()
                    .py_1()
                    .bg(cx.theme().input)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(px(4.0))
                    .text_xs()
                    .text_color(cx.theme().foreground)
                    .child(format!("{:.2}", value))
            )
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        StatusBar::new()
            .add_left_item(format!("Objects: {}", 4))
            .add_left_item(format!("Vertices: {}", 24))
            .add_left_item(format!("Triangles: {}", 12))
            .add_right_item("FPS: 60")
            .add_right_item("Perspective")
            .add_right_item("Grid: On")
            .render(cx)
    }
}

impl Panel for LevelEditorPanel {
    fn panel_name(&self) -> &'static str {
        "Level Editor"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        div().child("Level Editor").into_any_element()
    }

    fn dump(&self, _cx: &App) -> gpui_component::dock::PanelState {
        gpui_component::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}

impl Focusable for LevelEditorPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for LevelEditorPanel {}

impl Render for LevelEditorPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_toolbar(cx))
            .child(
                div()
                    .flex_1()
                    .child(
                        h_resizable("level-editor-panels", self.resizable_state.clone())
                            .child(
                                resizable_panel()
                                    .size(px(280.))
                                    .size_range(px(200.)..px(400.))
                                    .child(
                                        div()
                                            .size_full()
                                            .bg(cx.theme().sidebar)
                                            .border_1()
                                            .border_color(cx.theme().border)
                                            .rounded(cx.theme().radius)
                                            .p_2()
                                            .child(self.render_scene_hierarchy(cx))
                                    )
                            )
                            .child(
                                resizable_panel()
                                    .child(
                                        div()
                                            .size_full()
                                            .p_2()
                                            .child(self.render_viewport(window, cx))
                                    )
                            )
                            .child(
                                resizable_panel()
                                    .size(px(320.))
                                    .size_range(px(250.)..px(500.))
                                    .child(
                                        div()
                                            .size_full()
                                            .bg(cx.theme().sidebar)
                                            .border_1()
                                            .border_color(cx.theme().border)
                                            .rounded(cx.theme().radius)
                                            .p_2()
                                            .child(self.render_properties(cx))
                                    )
                            )
                    )
            )
            .child(self.render_status_bar(cx))
    }
}