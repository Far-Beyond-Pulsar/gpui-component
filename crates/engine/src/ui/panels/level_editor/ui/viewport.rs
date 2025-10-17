use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::{Button, ButtonVariants as _, Toggle}, h_flex, v_flex, ActiveTheme, IconName, Selectable, Sizable, StyledExt,
    chart::{LineChart, BarChart, AreaChart},
    PixelsExt,
};
// OPTIMIZED: Using new zero-copy viewport
use gpui_component::viewport_optimized::OptimizedViewport;

use super::state::{CameraMode, LevelEditorState};
use super::actions::*;
use crate::ui::shared::ViewportControls;
use engine_backend::GameThread;
use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use std::cell::RefCell;
use std::rc::Rc;

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

#[derive(Clone, Copy, PartialEq)]
enum GraphType {
    Line,
    Bar,
}

/// Viewport Panel - OPTIMIZED 3D rendering viewport with PRO camera controls
/// Studio-quality navigation: FPS mode, Pan, Orbit, Zoom
pub struct ViewportPanel {
    viewport: Entity<OptimizedViewport>,
    viewport_controls: ViewportControls,
    render_enabled: Arc<std::sync::atomic::AtomicBool>,
    // FPS tracking for rolling graph - using RefCell for interior mutability
    fps_history: RefCell<VecDeque<FpsDataPoint>>,
    fps_sample_counter: RefCell<usize>,
    // TPS tracking for rolling graph
    tps_history: RefCell<VecDeque<TpsDataPoint>>,
    tps_sample_counter: RefCell<usize>,
    // Mouse tracking for camera control - Rc<RefCell<>> for shared mutable state across closures!
    last_mouse_pos: Rc<RefCell<Option<Point<Pixels>>>>,
    mouse_right_captured: Rc<RefCell<bool>>,  // Right-click for look
    mouse_middle_captured: Rc<RefCell<bool>>, // Middle-click for pan
    // Keyboard state for WASD + modifiers
    keys_pressed: Rc<RefCell<std::collections::HashSet<String>>>,
    alt_pressed: Rc<RefCell<bool>>,  // Alt for orbit mode
    // Focus handle for input
    focus_handle: FocusHandle,
}

impl ViewportPanel {
    pub fn new<V>(viewport: Entity<OptimizedViewport>, render_enabled: Arc<std::sync::atomic::AtomicBool>, cx: &mut Context<V>) -> Self 
    where
        V: 'static,
    {
        Self {
            viewport,
            viewport_controls: ViewportControls::new(),
            render_enabled,
            fps_history: RefCell::new(VecDeque::with_capacity(60)),
            fps_sample_counter: RefCell::new(0),
            tps_history: RefCell::new(VecDeque::with_capacity(60)),
            tps_sample_counter: RefCell::new(0),
            last_mouse_pos: Rc::new(RefCell::new(None)),
            mouse_right_captured: Rc::new(RefCell::new(false)),
            mouse_middle_captured: Rc::new(RefCell::new(false)),
            keys_pressed: Rc::new(RefCell::new(std::collections::HashSet::new())),
            alt_pressed: Rc::new(RefCell::new(false)),
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn render<V: 'static>(
        &mut self,
        state: &mut LevelEditorState,
        fps_graph_state: Rc<RefCell<bool>>,  // Shared state for the Switch
        gpu_engine: &Arc<Mutex<crate::ui::gpu_renderer::GpuRenderer>>,
        game_thread: &Arc<GameThread>,
        current_pattern: crate::ui::rainbow_engine_final::RainbowPattern,
        cx: &mut Context<V>,
    ) -> impl IntoElement
    where
        V: EventEmitter<gpui_component::dock::PanelEvent> + Render,
    {
        // Clone Arc/Rc for closures
        let gpu_clone_key_down = gpu_engine.clone();
        let gpu_clone_key_up = gpu_engine.clone();
        let gpu_clone_mouse = gpu_engine.clone();
        let gpu_clone_scroll = gpu_engine.clone();
        
        // Clone Rc<RefCell<>> - now these WILL be shared!
        let keys_pressed = self.keys_pressed.clone();
        let keys_pressed_up = self.keys_pressed.clone();
        
        let last_mouse_pos_down = self.last_mouse_pos.clone();
        let last_mouse_pos_move = self.last_mouse_pos.clone();
        
        let mouse_right_down = self.mouse_right_captured.clone();
        let mouse_right_move = self.mouse_right_captured.clone();
        let mouse_right_up = self.mouse_right_captured.clone();
        let mouse_right_scroll = self.mouse_right_captured.clone();
        
        let mouse_middle_down = self.mouse_middle_captured.clone();
        let mouse_middle_move = self.mouse_middle_captured.clone();
        let mouse_middle_up = self.mouse_middle_captured.clone();
        
        let alt_key = self.alt_pressed.clone();
        
        let mut viewport_div = div()
            .flex() // Enable flexbox
            .flex_col() // Column direction
            .flex_1() // Allow growing to fill parent
            .size_full() // Take full size
            .relative()
            .bg(cx.theme().muted.opacity(0.2))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(move |_this, event: &gpui::KeyDownEvent, _window, _cx| {
                let key = event.keystroke.key.to_lowercase();
                keys_pressed.borrow_mut().insert(key.clone());
                
                // Update camera input
                if let Ok(engine) = gpu_clone_key_down.lock() {
                    if let Some(ref bevy_renderer) = engine.bevy_renderer {
                        if let Ok(mut input) = bevy_renderer.camera_input.lock() {
                            let keys = keys_pressed.borrow();
                            input.forward = 0.0;
                            input.right = 0.0;
                            input.up = 0.0;
                            
                            if keys.contains("w") { input.forward += 1.0; }
                            if keys.contains("s") { input.forward -= 1.0; }
                            if keys.contains("d") { input.right += 1.0; }
                            if keys.contains("a") { input.right -= 1.0; }
                            if keys.contains("space") { input.up += 1.0; }
                            if keys.contains("shift") {
                                input.up -= 1.0;
                                input.boost = true;
                            } else {
                                input.boost = false;
                            }
                        }
                    }
                }
            }))
            .on_key_up(cx.listener(move |_this, event: &gpui::KeyUpEvent, _window, _cx| {
                let key = event.keystroke.key.to_lowercase();
                keys_pressed_up.borrow_mut().remove(&key);
                
                // Update camera input
                if let Ok(engine) = gpu_clone_key_up.lock() {
                    if let Some(ref bevy_renderer) = engine.bevy_renderer {
                        if let Ok(mut input) = bevy_renderer.camera_input.lock() {
                            let keys = keys_pressed_up.borrow();
                            input.forward = 0.0;
                            input.right = 0.0;
                            input.up = 0.0;
                            
                            if keys.contains("w") { input.forward += 1.0; }
                            if keys.contains("s") { input.forward -= 1.0; }
                            if keys.contains("d") { input.right += 1.0; }
                            if keys.contains("a") { input.right -= 1.0; }
                            if keys.contains("space") { input.up += 1.0; }
                            if keys.contains("shift") {
                                input.up -= 1.0;
                                input.boost = true;
                            } else {
                                input.boost = false;
                            }
                        }
                    }
                }
            }))
            .child(
                // Main viewport - should grow to fill space and handle mouse events
                div()
                    .flex() // Enable flex
                    .flex_1() // Grow to fill available space
                    .size_full() // Take full size
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(move |_this, event: &MouseDownEvent, _window, _cx| {
                            println!("[VIEWPORT-INNER] ⬅️ LEFT CLICK at position: x={:.2}, y={:.2}", 
                                event.position.x.as_f32(), event.position.y.as_f32());
                        }),
                    )
                    .on_mouse_down(
                        gpui::MouseButton::Right,
                        cx.listener(move |_this, event: &MouseDownEvent, _window, cx| {
                            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                            println!("[VIEWPORT] ➡️ RIGHT CLICK - Look Mode");
                            println!("Position: x={:.2}, y={:.2}", event.position.x.as_f32(), event.position.y.as_f32());
                            
                            // Right-click to capture mouse for camera look
                            *mouse_right_down.borrow_mut() = true;
                            *last_mouse_pos_down.borrow_mut() = Some(event.position);
                            
                            println!("Mouse look enabled: {}", *mouse_right_down.borrow());
                            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                            
                            cx.stop_propagation();
                        }),
                    )
                    .on_mouse_up(
                        gpui::MouseButton::Right,
                        cx.listener(move |_this, event: &MouseUpEvent, _window, cx| {
                            // Release mouse capture
                            println!("[VIEWPORT] ✅ RIGHT RELEASE - Look Mode Off at x={:.2}, y={:.2}", 
                                event.position.x.as_f32(), event.position.y.as_f32());
                            *mouse_right_up.borrow_mut() = false;
                            
                            cx.stop_propagation();
                        }),
                    )
                    .on_mouse_move(cx.listener(move |_this, event: &MouseMoveEvent, _window, _cx| {
                        let current_pos = event.position;
                        
                        // Check if mouse is captured for look or pan
                        let is_right_captured = *mouse_right_move.borrow();
                        let is_middle_captured = *mouse_middle_move.borrow();
                        
                        let mut mouse_delta = (0.0f32, 0.0f32);
                        
                        // Only calculate delta if we have a previous position AND mouse is captured
                        if (is_right_captured || is_middle_captured) {
                            if let Some(last_pos) = *last_mouse_pos_move.borrow() {
                                let dx: f32 = (current_pos.x - last_pos.x).into();
                                let dy: f32 = (current_pos.y - last_pos.y).into();
                                mouse_delta = (dx, dy);
                            }
                        }
                        
                        // Always update last position when captured
                        if is_right_captured || is_middle_captured {
                            *last_mouse_pos_move.borrow_mut() = Some(current_pos);
                        }
                        
                        // Update camera input with mouse delta - ACCUMULATE, don't replace!
                        if let Ok(engine) = gpu_clone_mouse.lock() {
                            if let Some(ref bevy_renderer) = engine.bevy_renderer {
                                if let Ok(mut input) = bevy_renderer.camera_input.lock() {
                                    // Look mode (right-click) - ACCUMULATE deltas for smoother input
                                    if is_right_captured && (mouse_delta.0.abs() > 0.01 || mouse_delta.1.abs() > 0.01) {
                                        input.mouse_delta_x += mouse_delta.0;
                                        input.mouse_delta_y += mouse_delta.1;
                                    }
                                    
                                    // Pan mode (middle-click) - ACCUMULATE deltas
                                    if is_middle_captured && (mouse_delta.0.abs() > 0.01 || mouse_delta.1.abs() > 0.01) {
                                        input.pan_delta_x += mouse_delta.0;
                                        input.pan_delta_y += mouse_delta.1;
                                    }
                                }
                            }
                        }
                    }))
                    .on_scroll_wheel(cx.listener(move |_this, event: &gpui::ScrollWheelEvent, _window, _cx| {
                        let scroll_delta: f32 = event.delta.pixel_delta(px(1.0)).y.into();
                        
                        if let Ok(engine) = gpu_clone_scroll.lock() {
                            if let Some(ref bevy_renderer) = engine.bevy_renderer {
                                if let Ok(mut input) = bevy_renderer.camera_input.lock() {
                                    // If right-click held: adjust movement speed
                                    if *mouse_right_scroll.borrow() {
                                        let speed_change = scroll_delta * 0.1;
                                        input.move_speed = (input.move_speed + speed_change).clamp(0.5, 50.0);
                                        println!("[VIEWPORT] 🎚️ Move Speed: {:.1} units/sec", input.move_speed);
                                    } 
                                    // Otherwise: zoom (dolly)
                                    else {
                                        input.zoom_delta = scroll_delta;
                                        println!("[VIEWPORT] 🔍 Zoom: {:.2}", scroll_delta);
                                    }
                                }
                            }
                        }
                    }))
                    .on_mouse_down(
                        gpui::MouseButton::Middle,
                        cx.listener(move |_this, event: &MouseDownEvent, _window, cx| {
                            println!("[VIEWPORT] 🖱️ MIDDLE CLICK - Pan Mode");
                            *mouse_middle_down.borrow_mut() = true;
                            cx.stop_propagation();
                        }),
                    )
                    .on_mouse_up(
                        gpui::MouseButton::Middle,
                        cx.listener(move |_this, _event: &MouseUpEvent, _window, cx| {
                            println!("[VIEWPORT] 🖱️ MIDDLE RELEASE - Pan Mode Off");
                            *mouse_middle_up.borrow_mut() = false;
                            cx.stop_propagation();
                        }),
                    )
                    .child(self.viewport.clone())
            )
            .when(state.show_viewport_controls, |viewport_div| {
                viewport_div.child(
                    // Viewport controls overlay (top-right)
                    div()
                        .absolute()
                        .top_4()
                        .right_4()
                        .w(px(200.0)) // Hardcoded width to prevent inheritance issues
                        .child(Self::render_viewport_controls_overlay(cx))
                )
            })
            .when(state.show_camera_mode_selector, |viewport_div| {
                viewport_div.child(
                    // Camera mode selector (bottom-left)
                    div()
                        .absolute()
                        .bottom_4()
                        .left_4()
                        .w(px(320.0)) // Hardcoded width to prevent inheritance issues
                        .child(Self::render_camera_mode_selector(state.camera_mode, cx))
                )
            })
            .when(state.show_viewport_options, |viewport_div| {
                viewport_div.child(
                    // Grid and rendering options (top-left)
                    div()
                        .absolute()
                        .top_4()
                        .left_4()
                        .w(px(400.0)) // Hardcoded width to prevent inheritance issues
                        .child(Self::render_viewport_options(state, cx))
                )
            });

        if state.show_performance_overlay {
            viewport_div = viewport_div.child(
                // Performance overlay (bottom-right)
                div()
                    .absolute()
                    .bottom_4()
                    .right_4()
                    .w(px(360.0)) // Expanded width for graph
                    .child(self.render_performance_overlay(state, fps_graph_state, gpu_engine, game_thread, current_pattern, cx))
            );
        }

        viewport_div
    }

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
                    .child(
                        Button::new("camera_perspective")
                            .child("Persp")
                            .xsmall()
                            .selected(matches!(camera_mode, CameraMode::Perspective))
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.dispatch_action(&PerspectiveView);
                            }))
                    )
                    .child(
                        Button::new("camera_orthographic")
                            .child("Ortho")
                            .xsmall()
                            .selected(matches!(camera_mode, CameraMode::Orthographic))
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.dispatch_action(&OrthographicView);
                            }))
                    )
                    .child(
                        Button::new("camera_top")
                            .child("Top")
                            .xsmall()
                            .selected(matches!(camera_mode, CameraMode::Top))
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.dispatch_action(&TopView);
                            }))
                    )
                    .child(
                        Button::new("camera_front")
                            .child("Front")
                            .xsmall()
                            .selected(matches!(camera_mode, CameraMode::Front))
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.dispatch_action(&FrontView);
                            }))
                    )
                    .child(
                        Button::new("camera_side")
                            .child("Side")
                            .xsmall()
                            .selected(matches!(camera_mode, CameraMode::Side))
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.dispatch_action(&SideView);
                            }))
                    )
            )
            .child(
                Button::new("close_camera_mode")
                    .icon(IconName::X)
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.dispatch_action(&ToggleCameraModeSelector);
                    }))
            )
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
                    .child(
                        Button::new("toggle_grid")
                            .child("Grid")
                            .xsmall()
                            .selected(state.show_grid)
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.dispatch_action(&ToggleGrid);
                            }))
                    )
                    .child(
                        Button::new("toggle_wireframe")
                            .child("Wireframe")
                            .xsmall()
                            .selected(state.show_wireframe)
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.dispatch_action(&ToggleWireframe);
                            }))
                    )
                    .child(
                        Button::new("toggle_lighting")
                            .child("Lighting")
                            .xsmall()
                            .selected(state.show_lighting)
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.dispatch_action(&ToggleLighting);
                            }))
                    )
                    .child(
                        Button::new("toggle_performance")
                            .child("Stats")
                            .xsmall()
                            .selected(state.show_performance_overlay)
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.dispatch_action(&TogglePerformanceOverlay);
                            }))
                    )
            )
            .child(
                Button::new("close_viewport_options")
                    .icon(IconName::X)
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.dispatch_action(&ToggleViewportOptions);
                    }))
            )
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
                    .child(
                        Button::new("grid")
                            .icon(IconName::LayoutDashboard)
                            .tooltip("Toggle Grid")
                            .xsmall()
                    )
                    .child(
                        Button::new("axes")
                            .icon(IconName::Axes)
                            .tooltip("Toggle Axes")
                            .xsmall()
                    )
                    .child(
                        Button::new("perspective")
                            .icon(IconName::Cube)
                            .tooltip("Toggle Perspective")
                            .xsmall()
                    )
            )
            .child(
                Button::new("close_viewport_controls")
                    .icon(IconName::X)
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.dispatch_action(&ToggleViewportControls);
                    }))
            )
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
        // Get comprehensive metrics
        let (ui_fps, bevy_fps, pipeline_us, gpu_us, cpu_us, render_metrics) = if let Ok(engine) = gpu_engine.lock() {
            let ui_fps = engine.get_fps();
            let bevy_fps = engine.get_bevy_fps();
            let pipeline = engine.get_pipeline_time_us();
            let gpu = engine.get_gpu_time_us();
            let cpu = engine.get_cpu_time_us();
            let metrics = engine.get_render_metrics();
            (ui_fps, bevy_fps, pipeline, gpu, cpu, metrics)
        } else {
            (0.0, 0.0, 0, 0, 0, None)
        };

        // Get game thread metrics
        let game_tps = game_thread.get_tps();
        let game_tick_count = game_thread.get_tick_count();
        let game_enabled = game_thread.is_enabled();

        // Update FPS history - track BOTH UI and Bevy FPS
        let mut fps_history = self.fps_history.borrow_mut();
        let mut fps_sample_counter = self.fps_sample_counter.borrow_mut();
        
        fps_history.push_back(FpsDataPoint {
            index: *fps_sample_counter,
            fps: ui_fps as f64,
        });
        *fps_sample_counter += 1;
        
        // Keep only last 120 samples for smoother graph
        if fps_history.len() > 120 {
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
        
        // Keep only last 120 samples
        if tps_history.len() > 120 {
            tps_history.pop_front();
        }

        // Prepare data for the TPS chart
        let tps_data: Vec<TpsDataPoint> = tps_history.iter().cloned().collect();
        drop(tps_history);
        drop(tps_sample_counter);

        v_flex()
            .gap_2()
            .p_2()
            .w_full()
            .bg(cx.theme().background.opacity(0.95))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            // HEADER ROW - FPS indicators
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
                            // UI FPS
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(if ui_fps > 300.0 {
                                        cx.theme().success
                                    } else if ui_fps > 144.0 {
                                        cx.theme().warning
                                    } else {
                                        cx.theme().danger
                                    })
                                    .child(format!("UI: {:.0} FPS", ui_fps))
                            )
                            // Bevy/Renderer FPS
                            .child(
                                div()
                                    .text_xs()
                                    .font_semibold()
                                    .text_color(if bevy_fps > 200.0 {
                                        cx.theme().success
                                    } else if bevy_fps > 60.0 {
                                        cx.theme().warning
                                    } else {
                                        cx.theme().danger
                                    })
                                    .child(format!("Render: {:.0} FPS", bevy_fps))
                            )
                            // Pipeline time
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("Pipeline: {:.2}ms", pipeline_us as f64 / 1000.0))
                            )
                            .child({
                                let enabled = self.render_enabled.clone();
                                Button::new("toggle_render")
                                    .child(if self.render_enabled.load(std::sync::atomic::Ordering::Relaxed) {
                                        "⏸"
                                    } else {
                                        "▶"
                                    })
                                    .xsmall()
                                    .on_click(move |_event, _window, _cx| {
                                        let current = enabled.load(std::sync::atomic::Ordering::Relaxed);
                                        enabled.store(!current, std::sync::atomic::Ordering::Relaxed);
                                    })
                            })
                    )
                    .child(
                        Button::new("close_performance")
                            .icon(IconName::X)
                            .ghost()
                            .xsmall()
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.dispatch_action(&TogglePerformanceOverlay);
                            }))
                    )
            )
            .when(!fps_data.is_empty(), |this| {
                this.child(
                    v_flex()
                        .w_full()
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .pt_2()
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .mb_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_semibold()
                                        .text_color(cx.theme().foreground)
                                        .child("FPS Graph")
                                )
                                .child({
                                    let fps_graph_clone = fps_graph_state.clone();
                                    
                                    gpui_component::switch::Switch::new("fps_graph_type")
                                        .checked(*fps_graph_state.borrow())
                                        .label("Line")
                                        .xsmall()
                                        .on_click(move |checked, _, _| {
                                            *fps_graph_clone.borrow_mut() = *checked;
                                        })
                                })
                        )
                        .child(
                            div()
                                .h(px(100.))
                                .w_full()
                                .child(if *fps_graph_state.borrow() {
                                    // Area chart with semi-transparent fill (line mode)
                                    let theme = cx.theme();
                                    let stroke_color = theme.chart_1;
                                    let fill_color = stroke_color.opacity(0.2);
                                    
                                    AreaChart::new(fps_data.clone())
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.fps)
                                        .stroke(stroke_color)
                                        .fill(fill_color)
                                        .linear()
                                        .tick_margin(10)
                                        .into_any_element()
                                } else {
                                    // Bar chart with color coding: Green (high), Yellow (mid), Red (low)
                                    let theme = cx.theme();
                                    let success_color = theme.success;
                                    let warning_color = theme.warning;
                                    let danger_color = theme.danger;
                                    
                                    BarChart::new(fps_data.clone())
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.fps)
                                        .fill(move |d| {
                                            // Color code based on FPS:
                                            // Green: > 120 FPS (high)
                                            // Yellow: 60-120 FPS (mid)
                                            // Red: < 60 FPS (low)
                                            if d.fps >= 120.0 {
                                                success_color
                                            } else if d.fps >= 60.0 {
                                                warning_color
                                            } else {
                                                danger_color
                                            }
                                        })
                                        .tick_margin(10)
                                        .into_any_element()
                                })
                        )
                )
            })
            .child(
                // TPS Row - Add game thread monitoring
                h_flex()
                    .gap_2()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(if game_tps > 220.0 {
                                cx.theme().success
                            } else if game_tps > 120.0 {
                                cx.theme().warning
                            } else {
                                cx.theme().danger
                            })
                            .child(format!("🎮 {:.0} TPS", game_tps))
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Target: 240 | Ticks: {}", game_tick_count))
                    )
            )
            .when(!tps_data.is_empty(), |this| {
                this.child(
                    v_flex()
                        .w_full()
                        .mt_2()
                        .child(
                            h_flex()
                                .w_full()
                                .items_center()
                                .justify_between()
                                .mb_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .font_semibold()
                                        .text_color(cx.theme().foreground)
                                        .child("TPS Graph (Game Thread)")
                                )
                        )
                        .child(
                            div()
                                .h(px(80.))
                                .w_full()
                                .child({
                                    // TPS Line Chart
                                    let theme = cx.theme();
                                    let stroke_color = theme.chart_2;
                                    let fill_color = stroke_color.opacity(0.2);
                                    
                                    AreaChart::new(tps_data.clone())
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.tps)
                                        .stroke(stroke_color)
                                        .fill(fill_color)
                                        .linear()
                                        .tick_margin(10)
                                        .into_any_element()
                                })
                        )
                )
            })
            // DETAILED METRICS SECTION
            .child(
                v_flex()
                    .w_full()
                    .gap_1()
                    .mt_2()
                    .pt_2()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("⚡ Performance Details")
                    )
                    .child(
                        h_flex()
                            .gap_4()
                            .w_full()
                            .child(
                                v_flex()
                                    .gap_1()
                                    .flex_1()
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("GPU Time:")
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    
                                                    .text_color(cx.theme().foreground)
                                                    .child(format!("{:.2}ms", gpu_us as f64 / 1000.0))
                                            )
                                    )
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("CPU Time:")
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    
                                                    .text_color(cx.theme().foreground)
                                                    .child(format!("{:.2}ms", cpu_us as f64 / 1000.0))
                                            )
                                    )
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .flex_1()
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("Frame Time:")
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    
                                                    .text_color(cx.theme().foreground)
                                                    .child(format!("{:.2}ms", if bevy_fps > 0.0 { 1000.0 / bevy_fps } else { 0.0 }))
                                            )
                                    )
                                    .when_some(render_metrics, |this, metrics| {
                                        this.child(
                                            h_flex()
                                                .justify_between()
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        .text_color(cx.theme().muted_foreground)
                                                        .child("Data Xfer:")
                                                )
                                                .child(
                                                    div()
                                                        .text_xs()
                                                        
                                                        .text_color(cx.theme().foreground)
                                                        .child(format!("{:.1}MB", metrics.total_bytes_transferred as f64 / 1_048_576.0))
                                                )
                                        )
                                    })
                            )
                    )
            )
    }
}
