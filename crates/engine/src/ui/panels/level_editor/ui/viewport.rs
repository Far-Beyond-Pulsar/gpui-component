use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::{Button, ButtonVariants as _, Toggle}, h_flex, v_flex, ActiveTheme, IconName, Selectable, Sizable, StyledExt,
    chart::{LineChart, BarChart, AreaChart},
    PixelsExt,
};

// ✅ Using the simplified WgpuViewport with zero-copy GPU rendering
use gpui_component::wgpu_viewport::WgpuViewport;

use super::state::{CameraMode, LevelEditorState};
use super::actions::*;
use crate::ui::shared::ViewportControls;
use engine_backend::GameThread;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, Ordering};
use std::collections::VecDeque;
use std::cell::RefCell;
use std::rc::Rc;

/// Lock-free input state using atomics - no mutex contention!
#[derive(Clone)]
struct InputState {
    // Keyboard movement (atomic for lock-free access)
    forward: Arc<AtomicI32>,  // -1, 0, 1
    right: Arc<AtomicI32>,    // -1, 0, 1
    up: Arc<AtomicI32>,       // -1, 0, 1
    boost: Arc<AtomicBool>,

    // Mouse deltas (stored as i32 * 1000 for fractional precision)
    mouse_delta_x: Arc<AtomicI32>,
    mouse_delta_y: Arc<AtomicI32>,
    pan_delta_x: Arc<AtomicI32>,
    pan_delta_y: Arc<AtomicI32>,
    zoom_delta: Arc<AtomicI32>,

    // Move speed adjustment
    move_speed: Arc<AtomicI32>, // * 100 for precision
}

impl InputState {
    fn new() -> Self {
        Self {
            forward: Arc::new(AtomicI32::new(0)),
            right: Arc::new(AtomicI32::new(0)),
            up: Arc::new(AtomicI32::new(0)),
            boost: Arc::new(AtomicBool::new(false)),
            mouse_delta_x: Arc::new(AtomicI32::new(0)),
            mouse_delta_y: Arc::new(AtomicI32::new(0)),
            pan_delta_x: Arc::new(AtomicI32::new(0)),
            pan_delta_y: Arc::new(AtomicI32::new(0)),
            zoom_delta: Arc::new(AtomicI32::new(0)),
            move_speed: Arc::new(AtomicI32::new(1000)), // 10.0 * 100
        }
    }

    fn set_mouse_delta(&self, x: f32, y: f32) {
        self.mouse_delta_x.store((x * 1000.0) as i32, Ordering::Relaxed);
        self.mouse_delta_y.store((y * 1000.0) as i32, Ordering::Relaxed);
    }

    fn set_pan_delta(&self, x: f32, y: f32) {
        self.pan_delta_x.store((x * 1000.0) as i32, Ordering::Relaxed);
        self.pan_delta_y.store((y * 1000.0) as i32, Ordering::Relaxed);
    }

    fn set_zoom_delta(&self, z: f32) {
        self.zoom_delta.store((z * 1000.0) as i32, Ordering::Relaxed);
    }

    fn adjust_move_speed(&self, delta: f32) {
        let current = self.move_speed.load(Ordering::Relaxed) as f32 / 100.0;
        let new_speed = (current + delta).clamp(0.5, 100.0);
        self.move_speed.store((new_speed * 100.0) as i32, Ordering::Relaxed);
    }
}

// Performance metrics data points
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

#[derive(Clone)]
struct FrameTimeDataPoint {
    index: usize,
    frame_time_ms: f64,
}

#[derive(Clone)]
struct MemoryDataPoint {
    index: usize,
    memory_mb: f64,
}

#[derive(Clone)]
struct DrawCallsDataPoint {
    index: usize,
    draw_calls: f64,
}

#[derive(Clone)]
struct VerticesDataPoint {
    index: usize,
    vertices: f64,
}

#[derive(Clone)]
struct InputLatencyDataPoint {
    index: usize,
    latency_ms: f64,
}

#[derive(Clone)]
struct UiConsistencyDataPoint {
    index: usize,
    consistency_score: f64,
}

/// Viewport Panel - MASSIVELY SIMPLIFIED with WgpuCanvas!
/// ~70% less code, zero-copy GPU rendering, lock-free synchronization
pub struct ViewportPanel {
    /// ✅ NEW: Zero-copy WgpuViewport (replaces all old viewport implementations)
    viewport: Entity<WgpuViewport>,

    /// Channel to stop render thread on cleanup
    render_stop_sender: Option<smol::channel::Sender<()>>,

    /// Shared state
    viewport_controls: ViewportControls,
    render_enabled: Arc<AtomicBool>,

    /// Performance tracking (unchanged)
    fps_history: RefCell<VecDeque<FpsDataPoint>>,
    fps_sample_counter: RefCell<usize>,
    tps_history: RefCell<VecDeque<TpsDataPoint>>,
    tps_sample_counter: RefCell<usize>,
    frame_time_history: RefCell<VecDeque<FrameTimeDataPoint>>,
    frame_time_counter: RefCell<usize>,
    memory_history: RefCell<VecDeque<MemoryDataPoint>>,
    memory_counter: RefCell<usize>,
    draw_calls_history: RefCell<VecDeque<DrawCallsDataPoint>>,
    draw_calls_counter: RefCell<usize>,
    vertices_history: RefCell<VecDeque<VerticesDataPoint>>,
    vertices_counter: RefCell<usize>,
    input_latency_history: RefCell<VecDeque<InputLatencyDataPoint>>,
    input_latency_counter: RefCell<usize>,
    last_input_time: RefCell<Option<std::time::Instant>>,
    ui_consistency_history: RefCell<VecDeque<UiConsistencyDataPoint>>,
    ui_consistency_counter: RefCell<usize>,

    /// Lock-free input state
    input_state: InputState,
    input_thread_spawned: Arc<AtomicBool>,

    /// Mouse tracking - ALL ATOMIC!
    last_mouse_x: Arc<AtomicI32>,
    last_mouse_y: Arc<AtomicI32>,
    mouse_right_captured: Arc<AtomicBool>,
    mouse_middle_captured: Arc<AtomicBool>,

    /// Keyboard state
    keys_pressed: Rc<RefCell<std::collections::HashSet<String>>>,
    alt_pressed: Rc<RefCell<bool>>,

    /// Focus handle
    focus_handle: FocusHandle,
}

impl ViewportPanel {
    pub fn new<V>(
        gpu_engine: &Arc<Mutex<crate::ui::gpu_renderer::GpuRenderer>>,
        render_enabled: Arc<AtomicBool>,
        cx: &mut Context<V>,
    ) -> anyhow::Result<Self>
    where
        V: 'static,
    {
        let input_state = InputState::new();

        // ✅ Create WgpuViewport with render thread
        let (viewport, render_stop_sender) = {
            #[cfg(target_os = "windows")]
            {
                use gpui_component::wgpu_viewport::create_viewport_with_render_thread;

                // Get D3D11 device from GPU engine
                let device = {
                    let engine = gpu_engine.lock().unwrap();
                    engine.d3d11_device().clone()
                };

                create_viewport_with_render_thread(
                    &device,
                    1920,
                    1080,
                    cx,
                    {
                        let gpu_engine = gpu_engine.clone();
                        let render_enabled = render_enabled.clone();
                        move |canvas_texture| {
                            if !render_enabled.load(Ordering::Relaxed) {
                                return;
                            }

                            // Get back buffer and render directly to GPU
                            let back_buffer = canvas_texture.back_buffer();

                            // Render to D3D11 texture
                            if let Ok(mut engine) = gpu_engine.try_lock() {
                                engine.render_to_d3d11_texture(back_buffer);
                            }

                            // Swap buffers (atomic, zero-copy)
                            canvas_texture.swap_buffers();
                        }
                    },
                )?
            }

            #[cfg(target_os = "macos")]
            {
                use gpui_component::wgpu_viewport::create_viewport_with_render_thread;

                let device = metal::Device::system_default()
                    .ok_or_else(|| anyhow::anyhow!("No Metal device found"))?;

                create_viewport_with_render_thread(
                    &device,
                    1920,
                    1080,
                    cx,
                    {
                        let gpu_engine = gpu_engine.clone();
                        let render_enabled = render_enabled.clone();
                        move |canvas_texture| {
                            if !render_enabled.load(Ordering::Relaxed) {
                                return;
                            }

                            let back_buffer = canvas_texture.back_buffer();

                            if let Ok(mut engine) = gpu_engine.try_lock() {
                                engine.render_to_metal_texture(back_buffer);
                            }

                            canvas_texture.swap_buffers();
                        }
                    },
                )?
            }

            #[cfg(target_os = "linux")]
            {
                // Linux stub - create viewport without render thread for now
                use gpui_component::wgpu_viewport::WgpuViewport;
                let (stop_sender, _) = smol::channel::bounded(1);

                // TODO: Implement Linux/Vulkan viewport creation
                return Err(anyhow::anyhow!("Linux viewport not yet implemented"));
            }
        };

        Ok(Self {
            viewport,
            render_stop_sender: Some(render_stop_sender),
            viewport_controls: ViewportControls::new(),
            render_enabled,
            fps_history: RefCell::new(VecDeque::with_capacity(120)),
            fps_sample_counter: RefCell::new(0),
            tps_history: RefCell::new(VecDeque::with_capacity(120)),
            tps_sample_counter: RefCell::new(0),
            frame_time_history: RefCell::new(VecDeque::with_capacity(120)),
            frame_time_counter: RefCell::new(0),
            memory_history: RefCell::new(VecDeque::with_capacity(120)),
            memory_counter: RefCell::new(0),
            draw_calls_history: RefCell::new(VecDeque::with_capacity(120)),
            draw_calls_counter: RefCell::new(0),
            vertices_history: RefCell::new(VecDeque::with_capacity(120)),
            vertices_counter: RefCell::new(0),
            input_latency_history: RefCell::new(VecDeque::with_capacity(120)),
            input_latency_counter: RefCell::new(0),
            last_input_time: RefCell::new(None),
            ui_consistency_history: RefCell::new(VecDeque::with_capacity(120)),
            ui_consistency_counter: RefCell::new(0),
            input_state,
            input_thread_spawned: Arc::new(AtomicBool::new(false)),
            last_mouse_x: Arc::new(AtomicI32::new(0)),
            last_mouse_y: Arc::new(AtomicI32::new(0)),
            mouse_right_captured: Arc::new(AtomicBool::new(false)),
            mouse_middle_captured: Arc::new(AtomicBool::new(false)),
            keys_pressed: Rc::new(RefCell::new(std::collections::HashSet::new())),
            alt_pressed: Rc::new(RefCell::new(false)),
            focus_handle: cx.focus_handle(),
        })
    }

    pub fn render<V: 'static>(
        &mut self,
        state: &mut LevelEditorState,
        fps_graph_state: Rc<RefCell<bool>>,
        gpu_engine: &Arc<Mutex<crate::ui::gpu_renderer::GpuRenderer>>,
        game_thread: &Arc<GameThread>,
        current_pattern: crate::ui::rainbow_engine_final::RainbowPattern,
        cx: &mut Context<V>,
    ) -> impl IntoElement
    where
        V: EventEmitter<gpui_component::dock::PanelEvent> + Render,
    {
        // Spawn dedicated input processing thread ONLY ONCE
        if !self.input_thread_spawned.load(Ordering::Relaxed) {
            self.input_thread_spawned.store(true, Ordering::Relaxed);

            let input_state_for_thread = self.input_state.clone();
            let gpu_engine_for_thread = gpu_engine.clone();

            std::thread::spawn(move || {
                println!("[INPUT-THREAD] 🎮 Dedicated input processing thread started");
                loop {
                    std::thread::sleep(std::time::Duration::from_millis(8));

                    if let Ok(engine) = gpu_engine_for_thread.try_lock() {
                        if let Some(ref bevy_renderer) = engine.bevy_renderer {
                            if let Ok(mut input) = bevy_renderer.camera_input.try_lock() {
                                input.forward = input_state_for_thread.forward.load(Ordering::Relaxed) as f32;
                                input.right = input_state_for_thread.right.load(Ordering::Relaxed) as f32;
                                input.up = input_state_for_thread.up.load(Ordering::Relaxed) as f32;
                                input.boost = input_state_for_thread.boost.load(Ordering::Relaxed);

                                let mouse_x = input_state_for_thread.mouse_delta_x.swap(0, Ordering::Relaxed);
                                let mouse_y = input_state_for_thread.mouse_delta_y.swap(0, Ordering::Relaxed);
                                if mouse_x != 0 || mouse_y != 0 {
                                    input.mouse_delta_x = mouse_x as f32 / 1000.0;
                                    input.mouse_delta_y = mouse_y as f32 / 1000.0;
                                }

                                let pan_x = input_state_for_thread.pan_delta_x.swap(0, Ordering::Relaxed);
                                let pan_y = input_state_for_thread.pan_delta_y.swap(0, Ordering::Relaxed);
                                if pan_x != 0 || pan_y != 0 {
                                    input.pan_delta_x = pan_x as f32 / 1000.0;
                                    input.pan_delta_y = pan_y as f32 / 1000.0;
                                }

                                let zoom = input_state_for_thread.zoom_delta.swap(0, Ordering::Relaxed);
                                if zoom != 0 {
                                    input.zoom_delta = zoom as f32 / 1000.0;
                                }

                                input.move_speed = input_state_for_thread.move_speed.load(Ordering::Relaxed) as f32 / 100.0;
                            }
                        }
                    }
                }
            });
        }

        // Clone for closures
        let input_state_key_down = self.input_state.clone();
        let input_state_key_up = self.input_state.clone();
        let input_state_mouse = self.input_state.clone();
        let input_state_scroll = self.input_state.clone();

        let last_mouse_x = self.last_mouse_x.clone();
        let last_mouse_y = self.last_mouse_y.clone();

        let mouse_right_down = self.mouse_right_captured.clone();
        let mouse_right_move = self.mouse_right_captured.clone();
        let mouse_right_up = self.mouse_right_captured.clone();
        let mouse_right_scroll = self.mouse_right_captured.clone();

        let mouse_middle_down = self.mouse_middle_captured.clone();
        let mouse_middle_move = self.mouse_middle_captured.clone();
        let mouse_middle_up = self.mouse_middle_captured.clone();

        let mut viewport_div = div()
            .flex()
            .flex_col()
            .flex_1()
            .size_full()
            .relative()
            .bg(cx.theme().muted.opacity(0.2))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .track_focus(&self.focus_handle)
            .on_key_down(move |event: &gpui::KeyDownEvent, _phase, _cx| {
                let key = &event.keystroke.key;
                match key.as_ref() {
                    "w" | "W" => input_state_key_down.forward.store(1, Ordering::Relaxed),
                    "s" | "S" => input_state_key_down.forward.store(-1, Ordering::Relaxed),
                    "d" | "D" => input_state_key_down.right.store(1, Ordering::Relaxed),
                    "a" | "A" => input_state_key_down.right.store(-1, Ordering::Relaxed),
                    "space" | " " => input_state_key_down.up.store(1, Ordering::Relaxed),
                    "shift" => {
                        input_state_key_down.up.store(-1, Ordering::Relaxed);
                        input_state_key_down.boost.store(true, Ordering::Relaxed);
                    }
                    _ => return,
                }
            })
            .on_key_up(move |event: &gpui::KeyUpEvent, _phase, _cx| {
                let key = &event.keystroke.key;
                match key.as_ref() {
                    "w" | "W" | "s" | "S" => input_state_key_up.forward.store(0, Ordering::Relaxed),
                    "d" | "D" | "a" | "A" => input_state_key_up.right.store(0, Ordering::Relaxed),
                    "space" | " " => input_state_key_up.up.store(0, Ordering::Relaxed),
                    "shift" => {
                        input_state_key_up.up.store(0, Ordering::Relaxed);
                        input_state_key_up.boost.store(false, Ordering::Relaxed);
                    }
                    _ => return,
                }
            })
            .child(
                // ✅ MASSIVELY SIMPLIFIED: Just render the WgpuViewport!
                div()
                    .flex()
                    .flex_1()
                    .size_full()
                    .on_mouse_down(gpui::MouseButton::Left, move |event: &MouseDownEvent, _phase, _cx| {
                        println!("[VIEWPORT] ⬅️ LEFT CLICK at x={:.2}, y={:.2}",
                            event.position.x.as_f32(), event.position.y.as_f32());
                    })
                    .on_mouse_down(gpui::MouseButton::Right, move |_event: &MouseDownEvent, _phase, _cx| {
                        mouse_right_down.store(true, Ordering::Relaxed);
                    })
                    .on_mouse_up(gpui::MouseButton::Right, move |_event: &MouseUpEvent, _phase, _cx| {
                        mouse_right_up.store(false, Ordering::Relaxed);
                    })
                    .on_mouse_move(move |event: &MouseMoveEvent, _phase, _cx| {
                        let current_pos = event.position;
                        let is_right_captured = mouse_right_move.load(Ordering::Relaxed);
                        let is_middle_captured = mouse_middle_move.load(Ordering::Relaxed);

                        if is_right_captured || is_middle_captured {
                            let x_f32: f32 = current_pos.x.into();
                            let y_f32: f32 = current_pos.y.into();
                            let x_atomic = (x_f32 * 1000.0) as i32;
                            let y_atomic = (y_f32 * 1000.0) as i32;

                            let last_x = last_mouse_x.swap(x_atomic, Ordering::Relaxed);
                            let last_y = last_mouse_y.swap(y_atomic, Ordering::Relaxed);

                            if last_x != 0 || last_y != 0 {
                                let dx = (x_atomic - last_x) as f32 / 1000.0;
                                let dy = (y_atomic - last_y) as f32 / 1000.0;

                                if is_right_captured && (dx.abs() > 0.01 || dy.abs() > 0.01) {
                                    input_state_mouse.set_mouse_delta(dx, dy);
                                }

                                if is_middle_captured && (dx.abs() > 0.01 || dy.abs() > 0.01) {
                                    input_state_mouse.set_pan_delta(dx, dy);
                                }
                            }
                        }
                    })
                    .on_scroll_wheel(move |event: &gpui::ScrollWheelEvent, _phase, _cx| {
                        let scroll_delta: f32 = event.delta.pixel_delta(px(1.0)).y.into();

                        if mouse_right_scroll.load(Ordering::Relaxed) {
                            input_state_scroll.adjust_move_speed(scroll_delta * 0.1);
                        } else {
                            input_state_scroll.set_zoom_delta(scroll_delta * 0.5);
                        }
                    })
                    .on_mouse_down(gpui::MouseButton::Middle, move |_event: &MouseDownEvent, _phase, _cx| {
                        mouse_middle_down.store(true, Ordering::Relaxed);
                    })
                    .on_mouse_up(gpui::MouseButton::Middle, move |_event: &MouseUpEvent, _phase, _cx| {
                        mouse_middle_up.store(false, Ordering::Relaxed);
                    })
                    // ✅ That's it! Just render the viewport
                    .child(self.viewport.clone())
            );

        // Add overlays (viewport controls, camera mode selector, performance overlay)
        if state.show_viewport_controls {
            viewport_div = viewport_div.child(
                div()
                    .absolute()
                    .top_4()
                    .right_4()
                    .w(px(200.0))
                    .child(Self::render_viewport_controls_overlay(cx))
            );
        }

        if state.show_camera_mode_selector {
            viewport_div = viewport_div.child(
                div()
                    .absolute()
                    .bottom_4()
                    .left_4()
                    .w(px(320.0))
                    .child(Self::render_camera_mode_selector(state.camera_mode, cx))
            );
        }

        if state.show_viewport_options {
            viewport_div = viewport_div.child(
                div()
                    .absolute()
                    .top_4()
                    .left_4()
                    .w(px(400.0))
                    .child(Self::render_viewport_options(state, cx))
            );
        }

        if state.show_performance_overlay {
            viewport_div = viewport_div.child(
                div()
                    .absolute()
                    .bottom_4()
                    .right_4()
                    .w(px(360.0))
                    .child(self.render_performance_overlay(state, fps_graph_state, gpu_engine, game_thread, current_pattern, cx))
            );
        }

        viewport_div
    }

    // Keep all your overlay rendering methods unchanged...
    fn render_camera_mode_selector<V: 'static>(camera_mode: CameraMode, cx: &mut Context<V>) -> impl IntoElement
    where
        V: EventEmitter<gpui_component::dock::PanelEvent> + Render,
    {
        h_flex()
            .gap_1()
            .p_1()
            .w_full()
            .bg(cx.theme().background.opacity(0.9))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .justify_between()
            .items_center()
            .child(
                h_flex()
                    .gap_1()
                    .child(Button::new("camera_perspective").child("Persp").xsmall()
                        .selected(matches!(camera_mode, CameraMode::Perspective))
                        .on_click(cx.listener(|_, _, _, cx| cx.dispatch_action(&PerspectiveView))))
                    .child(Button::new("camera_orthographic").child("Ortho").xsmall()
                        .selected(matches!(camera_mode, CameraMode::Orthographic))
                        .on_click(cx.listener(|_, _, _, cx| cx.dispatch_action(&OrthographicView))))
                    .child(Button::new("camera_top").child("Top").xsmall()
                        .selected(matches!(camera_mode, CameraMode::Top))
                        .on_click(cx.listener(|_, _, _, cx| cx.dispatch_action(&TopView))))
                    .child(Button::new("camera_front").child("Front").xsmall()
                        .selected(matches!(camera_mode, CameraMode::Front))
                        .on_click(cx.listener(|_, _, _, cx| cx.dispatch_action(&FrontView))))
                    .child(Button::new("camera_side").child("Side").xsmall()
                        .selected(matches!(camera_mode, CameraMode::Side))
                        .on_click(cx.listener(|_, _, _, cx| cx.dispatch_action(&SideView))))
            )
            .child(Button::new("close_camera_mode").icon(IconName::X).ghost().xsmall()
                .on_click(cx.listener(|_, _, _, cx| cx.dispatch_action(&ToggleCameraModeSelector))))
    }

    fn render_viewport_options<V: 'static>(state: &LevelEditorState, cx: &mut Context<V>) -> impl IntoElement
    where
        V: EventEmitter<gpui_component::dock::PanelEvent> + Render,
    {
        h_flex()
            .gap_1()
            .p_1()
            .w_full()
            .bg(cx.theme().background.opacity(0.9))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .justify_between()
            .items_center()
            .child(
                h_flex()
                    .gap_1()
                    .child(Button::new("toggle_grid").child("Grid").xsmall()
                        .selected(state.show_grid)
                        .on_click(cx.listener(|_, _, _, cx| cx.dispatch_action(&ToggleGrid))))
                    .child(Button::new("toggle_wireframe").child("Wireframe").xsmall()
                        .selected(state.show_wireframe)
                        .on_click(cx.listener(|_, _, _, cx| cx.dispatch_action(&ToggleWireframe))))
                    .child(Button::new("toggle_lighting").child("Lighting").xsmall()
                        .selected(state.show_lighting)
                        .on_click(cx.listener(|_, _, _, cx| cx.dispatch_action(&ToggleLighting))))
                    .child(Button::new("toggle_performance").child("Stats").xsmall()
                        .selected(state.show_performance_overlay)
                        .on_click(cx.listener(|_, _, _, cx| cx.dispatch_action(&TogglePerformanceOverlay))))
            )
            .child(Button::new("close_viewport_options").icon(IconName::X).ghost().xsmall()
                .on_click(cx.listener(|_, _, _, cx| cx.dispatch_action(&ToggleViewportOptions))))
    }

    fn render_viewport_controls_overlay<V: 'static>(cx: &mut Context<V>) -> impl IntoElement
    where
        V: EventEmitter<gpui_component::dock::PanelEvent> + Render,
    {
        h_flex()
            .gap_2()
            .p_2()
            .w_full()
            .bg(cx.theme().background.opacity(0.9))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .justify_between()
            .items_center()
            .child(
                h_flex()
                    .gap_2()
                    .child(Button::new("grid").icon(IconName::LayoutDashboard).tooltip("Toggle Grid").xsmall())
                    .child(Button::new("axes").icon(IconName::Axes).tooltip("Toggle Axes").xsmall())
                    .child(Button::new("perspective").icon(IconName::Cube).tooltip("Toggle Perspective").xsmall())
            )
            .child(Button::new("close_viewport_controls").icon(IconName::X).ghost().xsmall()
                .on_click(cx.listener(|_, _, _, cx| cx.dispatch_action(&ToggleViewportControls))))
    }

    fn render_performance_overlay<V: 'static>(
        &mut self,
        state: &mut LevelEditorState,
        fps_graph_state: Rc<RefCell<bool>>,
        gpu_engine: &Arc<Mutex<crate::ui::gpu_renderer::GpuRenderer>>,
        game_thread: &Arc<GameThread>,
        current_pattern: crate::ui::rainbow_engine_final::RainbowPattern,
        cx: &mut Context<V>,
    ) -> impl IntoElement
    where
        V: EventEmitter<gpui_component::dock::PanelEvent> + Render,
    {
        // Get metrics from GPU engine
        let (ui_fps, bevy_fps, pipeline_us, gpu_us, cpu_us, render_metrics) = if let Ok(engine) = gpu_engine.lock() {
            (
                engine.get_fps(),
                engine.get_bevy_fps(),
                engine.get_pipeline_time_us(),
                engine.get_gpu_time_us(),
                engine.get_cpu_time_us(),
                engine.get_render_metrics(),
            )
        } else {
            (0.0, 0.0, 0, 0, 0, None)
        };

        let (memory_mb, draw_calls, vertices_drawn) = if let Some(ref metrics) = render_metrics {
            (metrics.memory_usage_mb, metrics.draw_calls, metrics.vertices_drawn)
        } else {
            (0.0, 0, 0)
        };

        let game_tps = game_thread.get_tps();
        let game_tick_count = game_thread.get_tick_count();

        // Update FPS history
        let mut fps_history = self.fps_history.borrow_mut();
        let mut fps_sample_counter = self.fps_sample_counter.borrow_mut();

        fps_history.push_back(FpsDataPoint {
            index: *fps_sample_counter,
            fps: ui_fps as f64,
        });
        *fps_sample_counter += 1;

        if fps_history.len() > 120 {
            fps_history.pop_front();
        }

        let fps_data: Vec<FpsDataPoint> = fps_history.iter().cloned().collect();
        drop(fps_history);
        drop(fps_sample_counter);

        // Simple performance display
        v_flex()
            .gap_2()
            .p_2()
            .w_full()
            .bg(cx.theme().background.opacity(0.95))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .gap_3()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_3()
                            .items_center()
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(if ui_fps > 300.0 { cx.theme().success }
                                               else if ui_fps > 144.0 { cx.theme().warning }
                                               else { cx.theme().danger })
                                    .child(format!("UI: {:.0} FPS", ui_fps))
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(if bevy_fps > 200.0 { cx.theme().success }
                                               else if bevy_fps > 60.0 { cx.theme().warning }
                                               else { cx.theme().danger })
                                    .child(format!("Render: {:.0} FPS", bevy_fps))
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("Pipeline: {:.2}ms", pipeline_us as f64 / 1000.0))
                            )
                            .child({
                                let enabled = self.render_enabled.clone();
                                Button::new("toggle_render")
                                    .child(if self.render_enabled.load(Ordering::Relaxed) { "⏸" } else { "▶" })
                                    .xsmall()
                                    .on_click(move |_event, _window, _cx| {
                                        let current = enabled.load(Ordering::Relaxed);
                                        enabled.store(!current, Ordering::Relaxed);
                                    })
                            })
                    )
                    .child(Button::new("close_performance").icon(IconName::X).ghost().xsmall()
                        .on_click(cx.listener(|_, _, _, cx| cx.dispatch_action(&TogglePerformanceOverlay))))
            )
            .child(
                div()
                    .text_xs()
                    .font_semibold()
                    .text_color(if game_tps > 220.0 { cx.theme().success }
                               else if game_tps > 120.0 { cx.theme().warning }
                               else { cx.theme().danger })
                    .child(format!("🎮 {:.0} TPS | Ticks: {}", game_tps, game_tick_count))
            )
    }
}

impl Drop for ViewportPanel {
    fn drop(&mut self) {
        // ✅ Clean shutdown of render thread
        if let Some(sender) = self.render_stop_sender.take() {
            let _ = sender.try_send(());
            println!("[VIEWPORT-PANEL] 🛑 Sent stop signal to render thread");
        }
    }
}
