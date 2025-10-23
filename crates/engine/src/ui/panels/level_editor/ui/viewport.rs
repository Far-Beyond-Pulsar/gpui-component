use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::{Button, ButtonVariants as _, Toggle}, h_flex, v_flex, ActiveTheme, IconName, Selectable, Sizable, StyledExt,
    chart::{LineChart, BarChart, AreaChart},
    PixelsExt,
};
// Zero-copy Bevy viewport for 3D rendering
use gpui_component::bevy_viewport::{BevyViewport, BevyViewportState};

use super::state::{CameraMode, LevelEditorState};
use super::actions::*;
use crate::ui::shared::ViewportControls;
use engine_backend::GameThread;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, Ordering};
use std::collections::VecDeque;
use std::cell::RefCell;
use std::rc::Rc;

// Raw input polling for viewport controls
use device_query::{DeviceQuery, DeviceState, Keycode, MouseState};

// Windows API for cursor management
#[cfg(target_os = "windows")]
use winapi::um::winuser::{ShowCursor, SetCursorPos, GetCursorPos, SetCursor};
#[cfg(target_os = "windows")]
use winapi::shared::windef::POINT;
#[cfg(target_os = "windows")]
use std::ptr;

/// Helper function to hide the Windows cursor
#[cfg(target_os = "windows")]
fn hide_cursor() {
    unsafe {
        // Set cursor to NULL to hide it completely
        SetCursor(ptr::null_mut());
    }
}

/// Helper function to show the Windows cursor  
#[cfg(target_os = "windows")]
fn show_cursor() {
    unsafe {
        // Load the standard arrow cursor
        let cursor = winapi::um::winuser::LoadCursorW(ptr::null_mut(), winapi::um::winuser::IDC_ARROW);
        SetCursor(cursor);
    }
}

/// Lock cursor to specific screen position (for camera rotation without cursor drift)
#[cfg(target_os = "windows")]
fn lock_cursor_position(x: i32, y: i32) {
    unsafe {
        SetCursorPos(x, y);
    }
}

/// Get current cursor position
#[cfg(target_os = "windows")]
fn get_cursor_position() -> (i32, i32) {
    unsafe {
        let mut point = POINT { x: 0, y: 0 };
        GetCursorPos(&mut point);
        (point.x, point.y)
    }
}

#[cfg(not(target_os = "windows"))]
fn hide_cursor() {
    // Placeholder for other platforms
}

#[cfg(not(target_os = "windows"))]
fn show_cursor() {
    // Placeholder for other platforms
}

#[cfg(not(target_os = "windows"))]
fn lock_cursor_position(_x: i32, _y: i32) {
    // Placeholder for other platforms
}

#[cfg(not(target_os = "windows"))]
fn get_cursor_position() -> (i32, i32) {
    (0, 0) // Placeholder for other platforms
}

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
    
    // Input latency tracking (measured on input thread)
    // Stores microseconds since last input was received, as i64
    input_latency_us: Arc<std::sync::atomic::AtomicU64>,
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
            input_latency_us: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }
    
    // Set mouse delta (converts f32 to i32 * 1000)
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
    consistency_score: f64, // Lower is better - represents stddev of FPS
}

#[derive(Clone, Copy, PartialEq)]
enum GraphType {
    Line,
    Bar,
}

/// Viewport Panel - TRUE ZERO-COPY GPU 3D rendering viewport with PRO camera controls
/// Studio-quality navigation: FPS mode, Pan, Orbit, Zoom
/// Direct GPU rendering - NO CPU COPIES!
/// Input is processed on a dedicated input thread with latency tracking
pub struct ViewportPanel {
    viewport: Entity<BevyViewport>,
    viewport_controls: ViewportControls,
    render_enabled: Arc<std::sync::atomic::AtomicBool>,
    // FPS tracking for rolling graph - using RefCell for interior mutability
    fps_history: RefCell<VecDeque<FpsDataPoint>>,
    fps_sample_counter: RefCell<usize>,
    // TPS tracking for rolling graph
    tps_history: RefCell<VecDeque<TpsDataPoint>>,
    tps_sample_counter: RefCell<usize>,
    // Frame time tracking for jitter analysis
    frame_time_history: RefCell<VecDeque<FrameTimeDataPoint>>,
    frame_time_counter: RefCell<usize>,
    // Memory usage tracking
    memory_history: RefCell<VecDeque<MemoryDataPoint>>,
    memory_counter: RefCell<usize>,
    // Draw calls tracking
    draw_calls_history: RefCell<VecDeque<DrawCallsDataPoint>>,
    draw_calls_counter: RefCell<usize>,
    // Vertices tracking
    vertices_history: RefCell<VecDeque<VerticesDataPoint>>,
    vertices_counter: RefCell<usize>,
    // Input latency tracking (measured on input thread)
    input_latency_history: RefCell<VecDeque<InputLatencyDataPoint>>,
    input_latency_counter: RefCell<usize>,
    // UI refresh consistency tracking (tracks FPS variance over time)
    ui_consistency_history: RefCell<VecDeque<UiConsistencyDataPoint>>,
    ui_consistency_counter: RefCell<usize>,
    // Lock-free input state - no mutex contention on UI thread!
    input_state: InputState,
    // Track if input thread has been spawned
    input_thread_spawned: Arc<AtomicBool>,
    // Track if viewport is hovered/focused - only process input when true
    viewport_hovered: Arc<AtomicBool>,
    // Mouse tracking - ALL ATOMIC! No RefCell!
    last_mouse_x: Arc<AtomicI32>,
    last_mouse_y: Arc<AtomicI32>,
    mouse_right_captured: Arc<AtomicBool>,
    mouse_middle_captured: Arc<AtomicBool>,
    // Locked cursor position for infinite mouse movement during drag
    locked_cursor_x: Arc<AtomicI32>,
    locked_cursor_y: Arc<AtomicI32>,
    // Keyboard state for WASD + modifiers - NOT NEEDED ANYMORE, using atomics directly!
    keys_pressed: Rc<RefCell<std::collections::HashSet<String>>>,
    alt_pressed: Rc<RefCell<bool>>,
    // Focus handle for input
    focus_handle: FocusHandle,
}

impl ViewportPanel {
    pub fn new<V>(viewport: Entity<BevyViewport>, render_enabled: Arc<std::sync::atomic::AtomicBool>, cx: &mut Context<V>) -> Self
    where
        V: 'static,
    {
        let input_state = InputState::new();
        
        Self {
            viewport,
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
            ui_consistency_history: RefCell::new(VecDeque::with_capacity(120)),
            ui_consistency_counter: RefCell::new(0),
            input_state,
            input_thread_spawned: Arc::new(AtomicBool::new(false)),
            viewport_hovered: Arc::new(AtomicBool::new(false)),
            last_mouse_x: Arc::new(AtomicI32::new(0)),
            last_mouse_y: Arc::new(AtomicI32::new(0)),
            mouse_right_captured: Arc::new(AtomicBool::new(false)),
            mouse_middle_captured: Arc::new(AtomicBool::new(false)),
            locked_cursor_x: Arc::new(AtomicI32::new(0)),
            locked_cursor_y: Arc::new(AtomicI32::new(0)),
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
        cx: &mut Context<V>,
    ) -> impl IntoElement
    where
        V: EventEmitter<gpui_component::dock::PanelEvent> + Render,
    {
        // Note: We can't check focus here because we don't have Window reference in this context
        // Instead, the viewport div will set focus when clicked (via track_focus)
        // and the input thread will check if right button is pressed to determine activity
        
        // Spawn dedicated input processing thread ONLY ONCE (not every frame!)
        if !self.input_thread_spawned.load(Ordering::Relaxed) {
            self.input_thread_spawned.store(true, Ordering::Relaxed);
            
            let input_state_for_thread = self.input_state.clone();
            let gpu_engine_for_thread = gpu_engine.clone();
            let mouse_right_captured = self.mouse_right_captured.clone();
            let mouse_middle_captured = self.mouse_middle_captured.clone();
            let locked_cursor_x = self.locked_cursor_x.clone();
            let locked_cursor_y = self.locked_cursor_y.clone();
            let viewport_hovered = self.viewport_hovered.clone();
            let focus_handle = self.focus_handle.clone();
            
            std::thread::spawn(move || {
                println!("[INPUT-THREAD] 🚀 Dedicated RAW INPUT processing thread started");
                let device_state = DeviceState::new();
                let mut last_mouse_pos: Option<(i32, i32)> = None;
                let mut right_was_pressed = false;
                let mut is_rotating = false; // Track if we're in rotation mode (Shift + Right)
                let mut is_panning = false;  // Track if we're in pan mode (Right without Shift)
                
                loop {
                    // Mark when we start processing input
                    let input_start = std::time::Instant::now();
                    
                    // Sleep for ~8ms (~120Hz processing rate)
                    std::thread::sleep(std::time::Duration::from_millis(8));
                    
                    // ONLY process input if viewport is FOCUSED (checked in render via is_focused)
                    let is_focused = viewport_hovered.load(Ordering::Relaxed);
                    if !is_focused && !is_rotating && !is_panning {
                        // Not focused and not currently in a drag operation - input thread does NOTHING
                        continue;
                    }
                    
                    // Poll mouse and keyboard state (ONLY when focused!)
                    let mouse: MouseState = device_state.get_mouse();
                    let keys: Vec<Keycode> = device_state.get_keys();
                    let right_pressed = mouse.button_pressed[1]; // Right button
                    let shift_pressed = keys.contains(&Keycode::LShift) || keys.contains(&Keycode::RShift);
                    
                    // NEW BINDINGS:
                    // Right click alone = Rotate camera (standard FPS controls)
                    // Shift + Right click = Pan camera (modifier for alternate mode)
                    
                    // Check if right button state changed
                    if right_pressed && !right_was_pressed {
                        // Right button just pressed
                        println!("[INPUT-THREAD] ========== RIGHT BUTTON PRESSED (viewport focused) ==========");
                        let (x, y) = get_cursor_position();
                        locked_cursor_x.store(x, Ordering::Relaxed);
                        locked_cursor_y.store(y, Ordering::Relaxed);
                        last_mouse_pos = Some((x, y));
                        
                        if shift_pressed {
                            // Shift + Right = Panning
                            is_panning = true;
                            is_rotating = false;
                            mouse_middle_captured.store(true, Ordering::Relaxed);
                            println!("[INPUT-THREAD] Shift+Right pressed - PAN mode, locked cursor at ({}, {})", x, y);
                        } else {
                            // Right alone = Rotation (standard behavior)
                            is_rotating = true;
                            is_panning = false;
                            mouse_right_captured.store(true, Ordering::Relaxed);
                            println!("[INPUT-THREAD] Right pressed - ROTATION mode, locked cursor at ({}, {})", x, y);
                        }
                        println!("[INPUT-THREAD] Calling hide_cursor()...");
                        hide_cursor();
                        println!("[INPUT-THREAD] hide_cursor() completed");
                        right_was_pressed = true;
                    } else if !right_pressed && right_was_pressed {
                        // Right button just released
                        println!("[INPUT-THREAD] ========== RIGHT BUTTON RELEASED ==========");
                        if is_rotating {
                            mouse_right_captured.store(false, Ordering::Relaxed);
                            is_rotating = false;
                            println!("[INPUT-THREAD] Rotation released");
                        }
                        if is_panning {
                            mouse_middle_captured.store(false, Ordering::Relaxed);
                            is_panning = false;
                            println!("[INPUT-THREAD] Pan released");
                        }
                        
                        let lock_x = locked_cursor_x.load(Ordering::Relaxed);
                        let lock_y = locked_cursor_y.load(Ordering::Relaxed);
                        lock_cursor_position(lock_x, lock_y);
                        println!("[INPUT-THREAD] Calling show_cursor()...");
                        show_cursor();
                        println!("[INPUT-THREAD] show_cursor() completed");
                        last_mouse_pos = None;
                        right_was_pressed = false;
                    }
                    
                    // If button is held, calculate mouse delta and reset cursor
                    if is_rotating || is_panning {
                        let (current_x, current_y) = get_cursor_position();
                        
                        if let Some((last_x, last_y)) = last_mouse_pos {
                            let dx = current_x - last_x;
                            let dy = current_y - last_y;
                            
                            if dx != 0 || dy != 0 {
                                // Store deltas in atomics based on mode
                                if is_rotating {
                                    // Right alone = Rotation (standard FPS camera)
                                    input_state_for_thread.mouse_delta_x.fetch_add((dx as f32 * 1000.0) as i32, Ordering::Relaxed);
                                    input_state_for_thread.mouse_delta_y.fetch_add((dy as f32 * 1000.0) as i32, Ordering::Relaxed);
                                }
                                if is_panning {
                                    // Shift + Right = Panning
                                    input_state_for_thread.pan_delta_x.fetch_add((dx as f32 * 1000.0) as i32, Ordering::Relaxed);
                                    input_state_for_thread.pan_delta_y.fetch_add((dy as f32 * 1000.0) as i32, Ordering::Relaxed);
                                }
                                
                                // Reset cursor to locked position for infinite movement
                                let lock_x = locked_cursor_x.load(Ordering::Relaxed);
                                let lock_y = locked_cursor_y.load(Ordering::Relaxed);
                                lock_cursor_position(lock_x, lock_y);
                                // Keep last_mouse_pos at the locked position
                                last_mouse_pos = Some((lock_x, lock_y));
                            }
                        }
                    }
                    
                    // Try to acquire GPU engine lock without blocking
                    if let Ok(engine) = gpu_engine_for_thread.try_lock() {
                        if let Some(ref bevy_renderer) = engine.bevy_renderer {
                            if let Ok(mut input) = bevy_renderer.camera_input.try_lock() {
                                // Read atomic values (no blocking!)
                                input.forward = input_state_for_thread.forward.load(Ordering::Relaxed) as f32;
                                input.right = input_state_for_thread.right.load(Ordering::Relaxed) as f32;
                                input.up = input_state_for_thread.up.load(Ordering::Relaxed) as f32;
                                input.boost = input_state_for_thread.boost.load(Ordering::Relaxed);
                                
                                // Read and CLEAR mouse deltas (always swap to 0, even if they're 0)
                                let mouse_x = input_state_for_thread.mouse_delta_x.swap(0, Ordering::Relaxed);
                                let mouse_y = input_state_for_thread.mouse_delta_y.swap(0, Ordering::Relaxed);
                                // Set to the CameraInput (including 0 to clear previous frame's deltas)
                                input.mouse_delta_x = mouse_x as f32 / 1000.0;
                                input.mouse_delta_y = mouse_y as f32 / 1000.0;
                                
                                // Read and CLEAR pan deltas (always swap to 0, even if they're 0)
                                let pan_x = input_state_for_thread.pan_delta_x.swap(0, Ordering::Relaxed);
                                let pan_y = input_state_for_thread.pan_delta_y.swap(0, Ordering::Relaxed);
                                // Set to the CameraInput (including 0 to clear previous frame's deltas)
                                input.pan_delta_x = pan_x as f32 / 1000.0;
                                input.pan_delta_y = pan_y as f32 / 1000.0;
                                
                                // Read and CLEAR zoom delta (always swap to 0, even if it's 0)
                                let zoom = input_state_for_thread.zoom_delta.swap(0, Ordering::Relaxed);
                                // Set to the CameraInput (including 0 to clear previous frame's delta)
                                input.zoom_delta = zoom as f32 / 1000.0;
                                
                                // Update move speed
                                input.move_speed = input_state_for_thread.move_speed.load(Ordering::Relaxed) as f32 / 100.0;
                                
                                // Calculate and store input latency (time from input start to GPU update)
                                let input_latency = input_start.elapsed().as_micros() as u64;
                                input_state_for_thread.input_latency_us.store(input_latency, Ordering::Relaxed);
                            }
                        }
                    }
                    // If lock fails, skip this cycle - no blocking!
                }
            });
        }
        
        // Clone input state for closures (lock-free!)
        let input_state_key_down = self.input_state.clone();
        let input_state_key_up = self.input_state.clone();
        let input_state_mouse = self.input_state.clone();
        let input_state_scroll = self.input_state.clone();
        
        // Clone atomics for mouse tracking - NO RefCell!
        let last_mouse_x = self.last_mouse_x.clone();
        let last_mouse_x_down = self.last_mouse_x.clone();
        let last_mouse_x_up = self.last_mouse_x.clone();
        let last_mouse_y = self.last_mouse_y.clone();
        let last_mouse_y_down = self.last_mouse_y.clone();
        let last_mouse_y_up = self.last_mouse_y.clone();
        
        let mouse_right_down = self.mouse_right_captured.clone();
        let mouse_right_move = self.mouse_right_captured.clone();
        let mouse_right_up = self.mouse_right_captured.clone();
        let mouse_right_scroll = self.mouse_right_captured.clone();
        
        let mouse_middle_down = self.mouse_middle_captured.clone();
        let mouse_middle_move = self.mouse_middle_captured.clone();
        let mouse_middle_up = self.mouse_middle_captured.clone();
        let mouse_middle_down_last_x = self.last_mouse_x.clone();
        let mouse_middle_down_last_y = self.last_mouse_y.clone();
        let mouse_middle_up_last_x = self.last_mouse_x.clone();
        let mouse_middle_up_last_y = self.last_mouse_y.clone();
        
        // Clone locked cursor position atomics
        let locked_cursor_x_down = self.locked_cursor_x.clone();
        let locked_cursor_y_down = self.locked_cursor_y.clone();
        let locked_cursor_x_up = self.locked_cursor_x.clone();
        let locked_cursor_y_up = self.locked_cursor_y.clone();
        let locked_cursor_x_move = self.locked_cursor_x.clone();
        let locked_cursor_y_move = self.locked_cursor_y.clone();
        let locked_cursor_x_middle_down = self.locked_cursor_x.clone();
        let locked_cursor_y_middle_down = self.locked_cursor_y.clone();
        let locked_cursor_x_middle_up = self.locked_cursor_x.clone();
        let locked_cursor_y_middle_up = self.locked_cursor_y.clone();
        let locked_cursor_x_middle_move = self.locked_cursor_x.clone();
        let locked_cursor_y_middle_move = self.locked_cursor_y.clone();
        // Clone viewport hovered flag for click detection
        let viewport_clicked = self.viewport_hovered.clone();
        
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
            .on_mouse_down(gpui::MouseButton::Left, move |_, _, _| {
                // When viewport is clicked, enable input thread
                viewport_clicked.store(true, Ordering::Relaxed);
                println!("[VIEWPORT] Clicked - input thread enabled");
            })
            .on_key_down(move |event: &gpui::KeyDownEvent, _phase, _cx| {
                // GPUI automatically filters key events to focused elements via track_focus
                // ULTRA FAST PATH: Update atomics directly, no allocations, no hashing, no RefCell!
                let key = &event.keystroke.key;
                match key.as_ref() {
                    "w" | "W" => input_state_key_down.forward.store(1, Ordering::Relaxed),
                    "s" | "S" => input_state_key_down.forward.store(-1, Ordering::Relaxed),
                    "d" | "D" => input_state_key_down.right.store(1, Ordering::Relaxed),
                    "a" | "A" => input_state_key_down.right.store(-1, Ordering::Relaxed),
                    "e" | "E" | "space" | " " => input_state_key_down.up.store(1, Ordering::Relaxed), // Up: E or Space
                    "q" | "Q" | "shift" => input_state_key_down.up.store(-1, Ordering::Relaxed), // Down: Q or Shift
                    _ => return, // Ignore all other keys instantly
                }
            })
            .on_key_up(move |event: &gpui::KeyUpEvent, _phase, _cx| {
                // GPUI automatically filters key events to focused elements via track_focus
                // ULTRA FAST PATH: Update atomics directly, no allocations, no hashing, no RefCell!
                let key = &event.keystroke.key;
                match key.as_ref() {
                    "w" | "W" | "s" | "S" => input_state_key_up.forward.store(0, Ordering::Relaxed),
                    "d" | "D" | "a" | "A" => input_state_key_up.right.store(0, Ordering::Relaxed),
                    "e" | "E" | "q" | "Q" | "space" | " " | "shift" => input_state_key_up.up.store(0, Ordering::Relaxed),
                    _ => return, // Ignore all other keys instantly
                }
            })
            .child(
                // Main viewport - input thread handles ALL mouse/keyboard when focused
                div()
                    .flex() // Enable flex
                    .flex_1() // Grow to fill available space
                    .size_full() // Take full size
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        move |event: &MouseDownEvent, _phase, _cx| {
                            println!("[VIEWPORT-INNER] ⬅️ LEFT CLICK at position: x={:.2}, y={:.2}", 
                                event.position.x.as_f32(), event.position.y.as_f32());
                        },
                    )
                    .on_scroll_wheel(move |event: &gpui::ScrollWheelEvent, _phase, _cx| {
                        let scroll_delta: f32 = event.delta.pixel_delta(px(1.0)).y.into();
                        
                        // Pure atomic operations - no RefCell!
                        if mouse_right_scroll.load(Ordering::Relaxed) {
                            input_state_scroll.adjust_move_speed(scroll_delta * 0.1);
                        } else {
                            input_state_scroll.set_zoom_delta(scroll_delta * 0.5);
                        }
                    })
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
                    .child(self.render_performance_overlay(state, fps_graph_state, gpu_engine, game_thread, cx))
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

        // Extract additional metrics
        let (memory_mb, draw_calls, vertices_drawn) = if let Some(ref metrics) = render_metrics {
            (metrics.memory_usage_mb, metrics.draw_calls, metrics.vertices_drawn)
        } else {
            (0.0, 0, 0)
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

        // Calculate UI refresh consistency (FPS variance/stddev) for consistency tracking
        let ui_consistency_score = if fps_data.len() >= 10 {
            // Calculate standard deviation of last 30 FPS samples (or all if less)
            let sample_size = fps_data.len().min(30);
            let recent_fps: Vec<f64> = fps_data.iter()
                .rev()
                .take(sample_size)
                .map(|d| d.fps)
                .collect();
            
            let mean = recent_fps.iter().sum::<f64>() / recent_fps.len() as f64;
            let variance = recent_fps.iter()
                .map(|fps| (fps - mean).powi(2))
                .sum::<f64>() / recent_fps.len() as f64;
            variance.sqrt() // Standard deviation - lower is more consistent
        } else {
            0.0
        };

        // Update UI consistency history
        let mut ui_consistency_history = self.ui_consistency_history.borrow_mut();
        let mut ui_consistency_counter = self.ui_consistency_counter.borrow_mut();
        
        ui_consistency_history.push_back(UiConsistencyDataPoint {
            index: *ui_consistency_counter,
            consistency_score: ui_consistency_score,
        });
        *ui_consistency_counter += 1;
        
        if ui_consistency_history.len() > 120 {
            ui_consistency_history.pop_front();
        }
        
        let ui_consistency_data: Vec<UiConsistencyDataPoint> = ui_consistency_history.iter().cloned().collect();
        drop(ui_consistency_history);
        drop(ui_consistency_counter);

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

        // Update Frame Time history for jitter analysis
        let mut frame_time_history = self.frame_time_history.borrow_mut();
        let mut frame_time_counter = self.frame_time_counter.borrow_mut();
        
        let frame_time_ms = pipeline_us as f64 / 1000.0;
        frame_time_history.push_back(FrameTimeDataPoint {
            index: *frame_time_counter,
            frame_time_ms,
        });
        *frame_time_counter += 1;
        
        if frame_time_history.len() > 120 {
            frame_time_history.pop_front();
        }
        
        let frame_time_data: Vec<FrameTimeDataPoint> = frame_time_history.iter().cloned().collect();
        drop(frame_time_history);
        drop(frame_time_counter);

        // Update Memory history
        let mut memory_history = self.memory_history.borrow_mut();
        let mut memory_counter = self.memory_counter.borrow_mut();
        
        memory_history.push_back(MemoryDataPoint {
            index: *memory_counter,
            memory_mb: memory_mb as f64,
        });
        *memory_counter += 1;
        
        if memory_history.len() > 120 {
            memory_history.pop_front();
        }
        
        let memory_data: Vec<MemoryDataPoint> = memory_history.iter().cloned().collect();
        drop(memory_history);
        drop(memory_counter);

        // Update Draw Calls history
        let mut draw_calls_history = self.draw_calls_history.borrow_mut();
        let mut draw_calls_counter = self.draw_calls_counter.borrow_mut();
        
        draw_calls_history.push_back(DrawCallsDataPoint {
            index: *draw_calls_counter,
            draw_calls: draw_calls as f64,
        });
        *draw_calls_counter += 1;
        
        if draw_calls_history.len() > 120 {
            draw_calls_history.pop_front();
        }
        
        let draw_calls_data: Vec<DrawCallsDataPoint> = draw_calls_history.iter().cloned().collect();
        drop(draw_calls_history);
        drop(draw_calls_counter);

        // Update Vertices history
        let mut vertices_history = self.vertices_history.borrow_mut();
        let mut vertices_counter = self.vertices_counter.borrow_mut();
        
        vertices_history.push_back(VerticesDataPoint {
            index: *vertices_counter,
            vertices: vertices_drawn as f64,
        });
        *vertices_counter += 1;
        
        if vertices_history.len() > 120 {
            vertices_history.pop_front();
        }
        
        let vertices_data: Vec<VerticesDataPoint> = vertices_history.iter().cloned().collect();
        drop(vertices_history);
        drop(vertices_counter);

        // Track input latency from the input thread (measured in microseconds, stored atomically)
        let input_latency_ms = self.input_state.input_latency_us.load(Ordering::Relaxed) as f64 / 1000.0;

        let mut input_latency_history = self.input_latency_history.borrow_mut();
        let mut input_latency_counter = self.input_latency_counter.borrow_mut();
        
        input_latency_history.push_back(InputLatencyDataPoint {
            index: *input_latency_counter,
            latency_ms: input_latency_ms,
        });
        *input_latency_counter += 1;
        
        if input_latency_history.len() > 120 {
            input_latency_history.pop_front();
        }
        
        let input_latency_data: Vec<InputLatencyDataPoint> = input_latency_history.iter().cloned().collect();
        drop(input_latency_history);
        drop(input_latency_counter);

        // Calculate frame time variance for jitter detection
        let frame_time_variance = if !frame_time_data.is_empty() {
            let mean = frame_time_data.iter().map(|d| d.frame_time_ms).sum::<f64>() / frame_time_data.len() as f64;
            let variance = frame_time_data.iter()
                .map(|d| (d.frame_time_ms - mean).powi(2))
                .sum::<f64>() / frame_time_data.len() as f64;
            variance.sqrt() // Standard deviation
        } else {
            0.0
        };

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
            // UI REFRESH CONSISTENCY GRAPH - Shows FPS variance over time (lower is better/more consistent)
            .when(!ui_consistency_data.is_empty(), |this| {
                this.child(
                    v_flex()
                        .w_full()
                        .mt_2()
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .child("📊 UI Refresh Consistency (StdDev) - Lower is smoother")
                        )
                        .child(
                            div()
                                .h(px(80.))
                                .w_full()
                                .child(if *fps_graph_state.borrow() {
                                    // Line mode - Area chart
                                    let theme = cx.theme();
                                    let stroke_color = theme.chart_2;
                                    let fill_color = stroke_color.opacity(0.2);

                                    AreaChart::new(ui_consistency_data.clone())
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.consistency_score)
                                        .stroke(stroke_color)
                                        .fill(fill_color)
                                        .linear()
                                        .tick_margin(10)
                                        .into_any_element()
                                } else {
                                    // Bar mode with color coding: Green (consistent), Yellow (moderate), Red (inconsistent)
                                    let theme = cx.theme();
                                    let success_color = theme.success;
                                    let warning_color = theme.warning;
                                    let danger_color = theme.danger;

                                    BarChart::new(ui_consistency_data.clone())
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.consistency_score)
                                        .fill(move |d| {
                                            // Color code based on consistency score (stddev):
                                            // Green: < 2.0 (very consistent)
                                            // Yellow: 2.0-5.0 (moderate jitter)
                                            // Red: > 5.0 (inconsistent/hitchy)
                                            if d.consistency_score < 2.0 {
                                                success_color
                                            } else if d.consistency_score < 5.0 {
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
                                .child(if *fps_graph_state.borrow() {
                                    // Line mode - Area chart
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
                                } else {
                                    // Bar mode
                                    let theme = cx.theme();
                                    let success_color = theme.success;
                                    let warning_color = theme.warning;
                                    let danger_color = theme.danger;
                                    
                                    BarChart::new(tps_data.clone())
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.tps)
                                        .fill(move |d| {
                                            // Color code based on TPS:
                                            // Green: > 220 TPS (high)
                                            // Yellow: 120-220 TPS (mid)
                                            // Red: < 120 TPS (low)
                                            if d.tps >= 220.0 {
                                                success_color
                                            } else if d.tps >= 120.0 {
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
                                                    .text_color(if gpu_us < 8000 {
                                                        cx.theme().success
                                                    } else if gpu_us < 16000 {
                                                        cx.theme().warning
                                                    } else {
                                                        cx.theme().danger
                                                    })
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
                                                    .text_color(if cpu_us < 2000 {
                                                        cx.theme().success
                                                    } else if cpu_us < 5000 {
                                                        cx.theme().warning
                                                    } else {
                                                        cx.theme().danger
                                                    })
                                                    .child(format!("{:.2}ms", cpu_us as f64 / 1000.0))
                                            )
                                    )
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("Frame Jitter:")
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(if frame_time_variance < 1.0 {
                                                        cx.theme().success
                                                    } else if frame_time_variance < 3.0 {
                                                        cx.theme().warning
                                                    } else {
                                                        cx.theme().danger
                                                    })
                                                    .child(format!("±{:.2}ms", frame_time_variance))
                                            )
                                    )
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("Input Thread:")
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(if input_latency_ms < 5.0 {
                                                        cx.theme().success
                                                    } else if input_latency_ms < 15.0 {
                                                        cx.theme().warning
                                                    } else {
                                                        cx.theme().danger
                                                    })
                                                    .child(format!("{:.1}ms", input_latency_ms))
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
                                                    .child("GPU Memory:")
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().foreground)
                                                    .child(format!("{:.1}MB", memory_mb))
                                            )
                                    )
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("Draw Calls:")
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().foreground)
                                                    .child(format!("{}", draw_calls))
                                            )
                                    )
                                    .child(
                                        h_flex()
                                            .justify_between()
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("Vertices:")
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().foreground)
                                                    .child(format!("{}", vertices_drawn))
                                            )
                                    )
                            )
                    )
            )
            // FRAME TIME JITTER GRAPH - Critical for finding stutters!
            .when(!frame_time_data.is_empty(), |this| {
                this.child(
                    v_flex()
                        .w_full()
                        .mt_2()
                        .pt_2()
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .child("📊 Frame Time (ms) - Spikes indicate stutters")
                        )
                        .child(
                            div()
                                .h(px(80.))
                                .w_full()
                                .child(if *fps_graph_state.borrow() {
                                    // Line mode - Area chart
                                    let theme = cx.theme();
                                    let stroke_color = theme.chart_3;
                                    let fill_color = stroke_color.opacity(0.2);
                                    
                                    AreaChart::new(frame_time_data.clone())
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.frame_time_ms)
                                        .stroke(stroke_color)
                                        .fill(fill_color)
                                        .linear()
                                        .tick_margin(10)
                                        .into_any_element()
                                } else {
                                    // Bar mode
                                    let theme = cx.theme();
                                    let chart_color = theme.chart_3;
                                    
                                    BarChart::new(frame_time_data.clone())
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.frame_time_ms)
                                        .fill(move |_d| chart_color)
                                        .tick_margin(10)
                                        .into_any_element()
                                })
                        )
                )
            })
            // GPU MEMORY USAGE GRAPH
            .when(!memory_data.is_empty(), |this| {
                this.child(
                    v_flex()
                        .w_full()
                        .mt_2()
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .child("💾 GPU Memory Usage (MB)")
                        )
                        .child(
                            div()
                                .h(px(60.))
                                .w_full()
                                .child(if *fps_graph_state.borrow() {
                                    // Line mode - Area chart
                                    let theme = cx.theme();
                                    let stroke_color = theme.chart_4;
                                    let fill_color = stroke_color.opacity(0.2);
                                    
                                    AreaChart::new(memory_data.clone())
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.memory_mb)
                                        .stroke(stroke_color)
                                        .fill(fill_color)
                                        .linear()
                                        .tick_margin(10)
                                        .into_any_element()
                                } else {
                                    // Bar mode
                                    let theme = cx.theme();
                                    let chart_color = theme.chart_4;
                                    
                                    BarChart::new(memory_data.clone())
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.memory_mb)
                                        .fill(move |_d| chart_color)
                                        .tick_margin(10)
                                        .into_any_element()
                                })
                        )
                )
            })
            // DRAW CALLS GRAPH
            .when(!draw_calls_data.is_empty(), |this| {
                this.child(
                    v_flex()
                        .w_full()
                        .mt_2()
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .child("🎨 Draw Calls per Frame")
                        )
                        .child(
                            div()
                                .h(px(60.))
                                .w_full()
                                .child(if *fps_graph_state.borrow() {
                                    // Line mode - Area chart
                                    let theme = cx.theme();
                                    let stroke_color = theme.chart_5;
                                    let fill_color = stroke_color.opacity(0.2);
                                    
                                    AreaChart::new(draw_calls_data.clone())
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.draw_calls)
                                        .stroke(stroke_color)
                                        .fill(fill_color)
                                        .linear()
                                        .tick_margin(10)
                                        .into_any_element()
                                } else {
                                    // Bar mode
                                    let theme = cx.theme();
                                    let chart_color = theme.chart_5;
                                    
                                    BarChart::new(draw_calls_data.clone())
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.draw_calls)
                                        .fill(move |_d| chart_color)
                                        .tick_margin(10)
                                        .into_any_element()
                                })
                        )
                )
            })
            // VERTICES GRAPH
            .when(!vertices_data.is_empty(), |this| {
                this.child(
                    v_flex()
                        .w_full()
                        .mt_2()
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .child("🔺 Vertices Rendered")
                        )
                        .child(
                            div()
                                .h(px(60.))
                                .w_full()
                                .child(if *fps_graph_state.borrow() {
                                    // Line mode - Area chart
                                    let theme = cx.theme();
                                    let stroke_color = theme.chart_1;
                                    let fill_color = stroke_color.opacity(0.2);
                                    
                                    AreaChart::new(vertices_data.clone())
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.vertices)
                                        .stroke(stroke_color)
                                        .fill(fill_color)
                                        .linear()
                                        .tick_margin(10)
                                        .into_any_element()
                                } else {
                                    // Bar mode
                                    let theme = cx.theme();
                                    let chart_color = theme.chart_1;
                                    
                                    BarChart::new(vertices_data.clone())
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.vertices)
                                        .fill(move |_d| chart_color)
                                        .tick_margin(10)
                                        .into_any_element()
                                })
                        )
                )
            })
            // INPUT LATENCY GRAPH - Critical for responsive controls! (Measured on input thread)
            .when(!input_latency_data.is_empty(), |this| {
                this.child(
                    v_flex()
                        .w_full()
                        .mt_2()
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().foreground)
                                .child("⚡ Input Thread Latency (ms) - Time to send input to GPU")
                        )
                        .child(
                            div()
                                .h(px(60.))
                                .w_full()
                                .child(if *fps_graph_state.borrow() {
                                    // Line mode - Area chart
                                    let theme = cx.theme();
                                    let stroke_color = theme.warning;
                                    let fill_color = stroke_color.opacity(0.2);
                                    
                                    AreaChart::new(input_latency_data.clone())
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.latency_ms)
                                        .stroke(stroke_color)
                                        .fill(fill_color)
                                        .linear()
                                        .tick_margin(10)
                                        .into_any_element()
                                } else {
                                    // Bar mode
                                    let theme = cx.theme();
                                    let chart_color = theme.warning;
                                    
                                    BarChart::new(input_latency_data.clone())
                                        .x(|d| SharedString::from(format!("{}", d.index)))
                                        .y(|d| d.latency_ms)
                                        .fill(move |_d| chart_color)
                                        .tick_margin(10)
                                        .into_any_element()
                                })
                        )
                )
            })
    }
}
