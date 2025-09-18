use gpui::*;
use gpui_component::{
    button::Button, dock::{Panel, PanelEvent}, h_flex, resizable::{h_resizable, resizable_panel, ResizableState}, v_flex, ActiveTheme as _, IconName, Selectable, StyledExt
};
use gpui_component::viewport::{Viewport, ViewportBuffers, RefreshHook, FramebufferFormat};

use crate::ui::shared::{Toolbar, ToolbarButton, ViewportControls, StatusBar};
use crate::ui::rainbow_engine::{RainbowRenderEngine, RainbowPattern};
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
    buffers: Arc<ViewportBuffers>,
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
        
        // Create viewport with proper GPUI background task pattern
        let (viewport_instance, buffers, refresh_hook) = Viewport::new(
            800, 
            600, 
            FramebufferFormat::Rgba8, 
            cx  // Pass context for GPUI background task setup
        );
        
        // Create the viewport entity
        let viewport = cx.new(|_cx| viewport_instance);
        
        // Create rainbow render engine
        let render_engine = Arc::new(Mutex::new(RainbowRenderEngine::new()));
        let buffers = Arc::new(buffers);
        let render_enabled = Arc::new(std::sync::atomic::AtomicBool::new(true));
        
        // Spawn render thread that uses the refresh hook to trigger GPUI reactive updates
        let engine_clone = render_engine.clone();
        let buffers_clone = buffers.clone();
        let hook_clone = refresh_hook.clone();
        let enabled_clone = render_enabled.clone();
        
        thread::spawn(move || {
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

    /// Controlled render thread with proper double buffering to prevent flickering
    fn render_thread_controlled(
        engine: Arc<Mutex<RainbowRenderEngine>>,
        buffers: Arc<ViewportBuffers>,
        refresh_hook: RefreshHook,
        render_enabled: Arc<std::sync::atomic::AtomicBool>,
    ) {
        // Target high FPS for smooth rendering, but limit UI refresh calls
        let target_frame_time = Duration::from_millis(8); // ~120 FPS rendering
        let mut frame_count = 0u64;
        let mut last_ui_refresh = std::time::Instant::now();
        let ui_refresh_interval = Duration::from_millis(16); // Limit UI refreshes to ~60fps
        
        while render_enabled.load(std::sync::atomic::Ordering::Relaxed) {
            let frame_start = std::time::Instant::now();
            
            // Step 1: Render to back buffer with exclusive access
            // This happens without any contention from the UI thread
            let render_successful = if let Ok(mut engine_guard) = engine.try_lock() {
                buffers.with_back_buffer(|back_buffer| {
                    engine_guard.render(back_buffer);
                }).is_some()
            } else {
                false
            };
            
            // Step 2: Only if render was successful, do atomic swap
            // This is the only point where buffers are briefly locked
            if render_successful {
                buffers.swap_buffers();
                
                // Step 3: Smart UI refresh - only call hook when enough time has passed
                // This reduces load on GPUI's reactive system while maintaining high render FPS
                let now = std::time::Instant::now();
                if now.duration_since(last_ui_refresh) >= ui_refresh_interval {
                    refresh_hook();
                    last_ui_refresh = now;
                }
                
                frame_count += 1;
            }
            
            // Step 4: Frame rate control
            let frame_time = frame_start.elapsed();
            let sleep_time = if frame_time < target_frame_time {
                target_frame_time - frame_time
            } else {
                Duration::from_millis(1) // Minimum yield
            };
            
            thread::sleep(sleep_time);
            
            // Periodic yield for better thread cooperation
            if frame_count % 30 == 0 {
                thread::yield_now();
            }
        }
    }

    pub fn toggle_rendering(&mut self) {
        let current = self.render_enabled.load(std::sync::atomic::Ordering::Relaxed);
        self.render_enabled.store(!current, std::sync::atomic::Ordering::Relaxed);
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
                ToolbarButton::new(IconName::Asterisk, "Select")
                    .tooltip("Select Tool (S)")
                    .active(true)
            )
            .add_button(
                ToolbarButton::new(IconName::Asterisk, "Move")
                    .tooltip("Move Tool (M)")
            )
            .add_button(
                ToolbarButton::new(IconName::Asterisk, "Rotate")
                    .tooltip("Rotate Tool (R)")
            )
            .add_button(
                ToolbarButton::new(IconName::Maximize, "Scale")
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
        let viewport_metrics = self.viewport.read(cx).metrics();
        
        // Get rainbow engine metrics
        let (engine_fps, frame_count, pattern_name) = if let Ok(engine) = self.render_engine.lock() {
            let fps = engine.get_fps();
            let frames = engine.get_frame_count();
            let pattern = format!("{:?}", self.current_pattern);
            (fps, frames, pattern)
        } else {
            (0.0, 0, "Unknown".to_string())
        };

        v_flex()
            .gap_1()
            .p_2()
            .bg(cx.theme().background.opacity(0.9))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .gap_2()
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
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().foreground)
                            .child(format!("Frames: {}", frame_count))
                    )
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
                            .child(format!("GPUI: {:.1} FPS", viewport_metrics.fps))
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Textures: {}", viewport_metrics.texture_updates))
                    )
            )
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