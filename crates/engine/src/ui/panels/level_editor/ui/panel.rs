use gpui::*;
use gpui_component::{
    dock::{Panel, PanelEvent},
    h_flex,
    resizable::{h_resizable, v_resizable, resizable_panel, ResizableState},
    v_flex,
    ActiveTheme as _,
    StyledExt,
};
use gpui_component::viewport_final::{Viewport, DoubleBuffer, RefreshHook, create_viewport_with_background_rendering};

use crate::ui::rainbow_engine_final::{RainbowRenderEngine, RainbowPattern};
use crate::ui::shared::StatusBar;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::{
    LevelEditorState, SceneBrowser, HierarchyPanel, PropertiesPanel,
    ViewportPanel, AssetBrowser, ToolbarPanel,
};

/// Main Level Editor Panel - Orchestrates all sub-components
pub struct LevelEditorPanel {
    focus_handle: FocusHandle,

    // Shared state
    state: LevelEditorState,

    // UI Components
    scene_browser: SceneBrowser,
    hierarchy: HierarchyPanel,
    properties: PropertiesPanel,
    viewport_panel: ViewportPanel,
    asset_browser: AssetBrowser,
    toolbar: ToolbarPanel,

    // Layout state
    horizontal_resizable_state: Entity<ResizableState>,
    vertical_resizable_state: Entity<ResizableState>,
    center_vertical_resizable_state: Entity<ResizableState>,

    // Viewport and rendering
    viewport: Entity<Viewport>,
    render_engine: Arc<Mutex<RainbowRenderEngine>>,
    buffers: Arc<DoubleBuffer>,
    refresh_hook: RefreshHook,
    current_pattern: RainbowPattern,
    render_speed: f32,
    render_enabled: Arc<std::sync::atomic::AtomicBool>,
}

impl LevelEditorPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let horizontal_resizable_state = ResizableState::new(cx);
        let vertical_resizable_state = ResizableState::new(cx);
        let center_vertical_resizable_state = ResizableState::new(cx);

        // Create viewport with zero-copy background rendering
        let (viewport, buffers, refresh_hook) = create_viewport_with_background_rendering(
            800,
            600,
            cx
        );

        // Create rainbow render engine
        let render_engine = Arc::new(Mutex::new(RainbowRenderEngine::new()));
        let render_enabled = Arc::new(std::sync::atomic::AtomicBool::new(true));

        // Spawn render thread
        let engine_clone = render_engine.clone();
        let buffers_clone = buffers.clone();
        let hook_clone = refresh_hook.clone();
        let enabled_clone = render_enabled.clone();

        thread::spawn(move || {
            Self::render_thread_controlled(engine_clone, buffers_clone, hook_clone, enabled_clone);
        });

        println!("[LEVEL-EDITOR] Modular level editor initialized");

        Self {
            focus_handle: cx.focus_handle(),
            state: LevelEditorState::new(),
            scene_browser: SceneBrowser::new(),
            hierarchy: HierarchyPanel::new(),
            properties: PropertiesPanel::new(),
            viewport_panel: ViewportPanel::new(viewport.clone()),
            asset_browser: AssetBrowser::new(),
            toolbar: ToolbarPanel::new(),
            horizontal_resizable_state,
            vertical_resizable_state,
            center_vertical_resizable_state,
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
        let base_frame_time = Duration::from_millis(8);
        let mut adaptive_frame_time = base_frame_time;
        let mut frame_count = 0u64;
        let mut consecutive_fast_frames = 0u32;
        let max_cpu_usage = 85;

        while render_enabled.load(std::sync::atomic::Ordering::Relaxed) {
            let frame_start = std::time::Instant::now();

            let render_successful = if let Ok(mut engine_guard) = engine.try_lock() {
                let back_buffer = buffers.get_back_buffer();
                let buffer_lock_result = back_buffer.try_lock();
                if let Ok(mut buffer_guard) = buffer_lock_result {
                    engine_guard.render_rgba8(&mut *buffer_guard);
                    true
                } else {
                    false
                }
            } else {
                false
            };

            if render_successful {
                buffers.swap_buffers();
                refresh_hook();
                frame_count += 1;
            }

            let frame_time = frame_start.elapsed();

            if frame_time < adaptive_frame_time.mul_f32(0.5) {
                consecutive_fast_frames += 1;
                if consecutive_fast_frames > 10 {
                    adaptive_frame_time = adaptive_frame_time.mul_f32(1.1).min(Duration::from_millis(16));
                    consecutive_fast_frames = 0;
                }
            } else {
                consecutive_fast_frames = 0;
                adaptive_frame_time = adaptive_frame_time.mul_f32(0.99).max(base_frame_time);
            }

            let target_cpu_usage = max_cpu_usage as f32 / 100.0;
            let work_time = frame_time.as_secs_f32();
            let total_frame_time = work_time / target_cpu_usage;
            let sleep_time = Duration::from_secs_f32(total_frame_time - work_time).max(Duration::from_millis(1));

            thread::sleep(sleep_time);

            if frame_count % 30 == 0 {
                thread::yield_now();
                thread::sleep(Duration::from_micros(100));
            }

            if frame_count % 120 == 0 {
                thread::sleep(Duration::from_millis(2));
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

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        StatusBar::new()
            .add_left_item(format!("Objects: {}", self.state.scene_objects.len()))
            .add_left_item(format!("Selected: {}",
                self.state.selected_object.as_ref()
                    .and_then(|id| self.state.get_selected_object())
                    .map(|obj| obj.name.as_str())
                    .unwrap_or("None")
            ))
            .add_right_item(format!("{:?}", self.state.camera_mode))
            .add_right_item(format!("Grid: {}", if self.state.show_grid { "On" } else { "Off" }))
            .add_right_item(format!("Tool: {:?}", self.state.current_tool))
            .render(cx)
    }
}

impl Panel for LevelEditorPanel {
    fn panel_name(&self) -> &'static str {
        "Level Editor"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        div()
            .child(
                if let Some(ref scene) = self.state.current_scene {
                    format!(
                        "Level Editor - {}{}",
                        scene.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Untitled"),
                        if self.state.has_unsaved_changes { " *" } else { "" }
                    )
                } else {
                    "Level Editor".to_string()
                }
            )
            .into_any_element()
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                // Toolbar at the top
                self.toolbar.render(&self.state, cx)
            )
            .child(
                // Main content area with resizable panels
                div()
                    .flex_1()
                    .child(
                        h_resizable("level-editor-main", self.horizontal_resizable_state.clone())
                            .child(
                                // Left sidebar: Scene Browser + Hierarchy
                                resizable_panel()
                                    .size(px(280.))
                                    .size_range(px(200.)..px(400.))
                                    .child(
                                        div()
                                            .size_full()
                                            .bg(cx.theme().sidebar)
                                            .border_r_1()
                                            .border_color(cx.theme().border)
                                            .child(
                                                v_resizable("level-editor-left", self.vertical_resizable_state.clone())
                                                    .child(
                                                        resizable_panel()
                                                            .size(px(200.))
                                                            .size_range(px(100.)..px(400.))
                                                            .child(self.scene_browser.render(cx))
                                                    )
                                                    .child(
                                                        resizable_panel()
                                                            .child(
                                                                div()
                                                                    .size_full()
                                                                    .p_2()
                                                                    .child(self.hierarchy.render(&self.state, cx))
                                                            )
                                                    )
                                            )
                                    )
                            )
                            .child(
                                // Center: Viewport + Asset Browser
                                resizable_panel()
                                    .child(
                                        div()
                                            .size_full()
                                            .child(
                                                v_resizable("level-editor-center", self.center_vertical_resizable_state.clone())
                                                    .child(
                                                        resizable_panel()
                                                            .child(
                                                                div()
                                                                    .size_full()
                                                                    .p_2()
                                                                    .child(
                                                                        self.viewport_panel.render(
                                                                            &self.state,
                                                                            &self.render_engine,
                                                                            &self.render_enabled,
                                                                            self.current_pattern,
                                                                            cx
                                                                        )
                                                                    )
                                                            )
                                                    )
                                                    .child(
                                                        resizable_panel()
                                                            .size(px(200.))
                                                            .size_range(px(150.)..px(400.))
                                                            .child(
                                                                div()
                                                                    .size_full()
                                                                    .bg(cx.theme().sidebar)
                                                                    .border_t_1()
                                                                    .border_color(cx.theme().border)
                                                                    .p_2()
                                                                    .child(self.asset_browser.render(cx))
                                                            )
                                                    )
                                            )
                                    )
                            )
                            .child(
                                // Right sidebar: Properties
                                resizable_panel()
                                    .size(px(320.))
                                    .size_range(px(250.)..px(500.))
                                    .child(
                                        div()
                                            .size_full()
                                            .bg(cx.theme().sidebar)
                                            .border_l_1()
                                            .border_color(cx.theme().border)
                                            .p_2()
                                            .child(self.properties.render(&self.state, cx))
                                    )
                            )
                    )
            )
            .child(
                // Status bar at the bottom
                self.render_status_bar(cx)
            )
    }
}
