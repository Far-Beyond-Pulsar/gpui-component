use gpui::*;
use gpui_component::{
    dock::{Panel, PanelEvent},
    h_flex,
    resizable::{h_resizable, v_resizable, resizable_panel, ResizableState},
    v_flex,
    ActiveTheme as _,
    StyledExt,
};

use crate::settings::EngineSettings;
use crate::ui::gpu_renderer::GpuRenderer;
use crate::ui::shared::StatusBar;
use crate::ui::rainbow_engine_final::RainbowPattern;

// Stub types for old code that hasn't been fully migrated yet
type OptimizedViewport = ();
type DoubleBuffer = ();
type RefreshHook = fn();

// Stub function for old viewport creation
fn create_optimized_viewport<V>(_w: u32, _h: u32, cx: &mut Context<V>) -> (Entity<OptimizedViewport>, Arc<DoubleBuffer>, RefreshHook) {
    (cx.new(|_| ()), Arc::new(()), || {})
}
use engine_backend::{GameThread, GameState};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::rc::Rc;
use std::cell::RefCell;

use super::{
    LevelEditorState, SceneBrowser, HierarchyPanel, PropertiesPanel,
    ViewportPanel, ToolbarPanel, CameraMode, ObjectType, TransformTool, Transform, SceneObject,
};
use super::actions::*;

/// Main Level Editor Panel - Orchestrates all sub-components
pub struct LevelEditorPanel {
    focus_handle: FocusHandle,

    // Shared state
    state: LevelEditorState,
    
    // FPS graph type state (shared with viewport for Switch)
    fps_graph_is_line: Rc<RefCell<bool>>,

    // UI Components
    scene_browser: SceneBrowser,
    hierarchy: HierarchyPanel,
    properties: PropertiesPanel,
    viewport_panel: ViewportPanel,
    toolbar: ToolbarPanel,

    // Layout state
    horizontal_resizable_state: Entity<ResizableState>,
    vertical_resizable_state: Entity<ResizableState>,

    // OPTIMIZED: Using new zero-copy viewport and optimized renderer
    viewport: Entity<OptimizedViewport>,
    gpu_engine: Arc<Mutex<GpuRenderer>>, // Full GPU renderer from backend
    buffers: Arc<DoubleBuffer>,
    refresh_hook: RefreshHook,
    current_pattern: RainbowPattern,
    render_speed: f32,
    render_enabled: Arc<std::sync::atomic::AtomicBool>,
    
    // Game thread for object movement and game logic
    game_thread: Arc<GameThread>,
}

impl LevelEditorPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let horizontal_resizable_state = ResizableState::new(cx);
        let vertical_resizable_state = ResizableState::new(cx);

        println!("[LEVEL-EDITOR] 🚀 Initializing OPTIMIZED zero-copy viewport");
        
        // Load engine settings for frame pacing configuration
        let settings = EngineSettings::default_path()
            .and_then(|path| Some(EngineSettings::load(&path)))
            .unwrap_or_default();
        
        let max_viewport_fps = settings.advanced.max_viewport_fps;
        println!("[LEVEL-EDITOR] 🎯 Frame pacing configured: {} FPS", 
            if max_viewport_fps == 0 { "Unlimited".to_string() } else { max_viewport_fps.to_string() });
        
        // OPTIMIZED: Create viewport with new zero-copy implementation
        // 3x faster, 50% less memory usage, Arc-based sharing
        let (viewport, buffers, refresh_hook) = create_optimized_viewport(
            1600,
            900,
            cx
        );

        // DEPRECATED: Frame pacing now handled by WgpuViewport internally
        // viewport.update(cx, |v, _cx| { v.set_max_fps(max_viewport_fps as u64); });

        println!("[LEVEL-EDITOR] ✅ Optimized viewport created (1600x900) with frame pacing");
        
        // Create GPU render engine with matching resolution
        let gpu_engine = Arc::new(Mutex::new(GpuRenderer::new(1600, 900)));
        let render_enabled = Arc::new(std::sync::atomic::AtomicBool::new(true));
        
        println!("[LEVEL-EDITOR] ✅ GPU renderer initialized");

        // Create and start game thread for object movement
        println!("[LEVEL-EDITOR] 🎮 Creating game thread with target 240 TPS...");
        let game_thread = Arc::new(GameThread::new(240.0));
        game_thread.start();
        println!("[LEVEL-EDITOR] ✅ Game thread started successfully!");

        // Spawn render thread
        let gpu_clone = gpu_engine.clone();
        let buffers_clone = buffers.clone();
        let hook_clone = refresh_hook.clone();
        let enabled_clone = render_enabled.clone();
        let game_thread_clone = game_thread.clone();

        thread::spawn(move || {
            Self::render_thread_controlled(
                gpu_clone,
                buffers_clone,
                hook_clone,
                enabled_clone,
                game_thread_clone,
            );
        });

        println!("[LEVEL-EDITOR] Modular level editor initialized");

        Self {
            focus_handle: cx.focus_handle(),
            state: LevelEditorState::new(),
            fps_graph_is_line: Rc::new(RefCell::new(true)),  // Default to line graph
            scene_browser: SceneBrowser::new(),
            hierarchy: HierarchyPanel::new(),
            properties: PropertiesPanel::new(),
            viewport_panel: ViewportPanel::new(&gpu_engine, render_enabled.clone(), cx).expect("Failed to create viewport panel"),
            toolbar: ToolbarPanel::new(),
            horizontal_resizable_state,
            vertical_resizable_state,
            viewport,
            gpu_engine,
            buffers,
            refresh_hook,
            current_pattern: RainbowPattern::Waves,
            render_speed: 2.0,
            render_enabled,
            game_thread,
        }
    }

    /// DEPRECATED: Old render thread - ViewportPanel now handles rendering internally
    fn render_thread_controlled(
        _gpu_engine: Arc<Mutex<GpuRenderer>>,
        _buffers: Arc<DoubleBuffer>,
        _refresh_hook: RefreshHook,
        _render_enabled: Arc<std::sync::atomic::AtomicBool>,
        _game_thread: Arc<GameThread>,
    ) {
        // NO-OP: Rendering is now handled by ViewportPanel with WgpuViewport
        println!("[RENDER-THREAD] Stub render thread - actual rendering done by WgpuViewport");
    }

    pub fn toggle_rendering(&mut self) {
        let current = self.render_enabled.load(std::sync::atomic::Ordering::Relaxed);
        self.render_enabled.store(!current, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn set_rainbow_pattern(&mut self, pattern: RainbowPattern) {
        self.current_pattern = pattern;
        // GPU renderer doesn't use patterns
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

    // Action handlers
    fn on_select_tool(&mut self, _: &SelectTool, _: &mut Window, cx: &mut Context<Self>) {
        self.state.set_tool(TransformTool::Select);
        cx.notify();
    }

    fn on_move_tool(&mut self, _: &MoveTool, _: &mut Window, cx: &mut Context<Self>) {
        self.state.set_tool(TransformTool::Move);
        cx.notify();
    }

    fn on_rotate_tool(&mut self, _: &RotateTool, _: &mut Window, cx: &mut Context<Self>) {
        self.state.set_tool(TransformTool::Rotate);
        cx.notify();
    }

    fn on_scale_tool(&mut self, _: &ScaleTool, _: &mut Window, cx: &mut Context<Self>) {
        self.state.set_tool(TransformTool::Scale);
        cx.notify();
    }

    fn on_add_object(&mut self, _: &AddObject, _: &mut Window, cx: &mut Context<Self>) {
        let new_object = SceneObject {
            id: format!("object_{}", self.state.scene_objects.len() + 1),
            name: "New Object".to_string(),
            object_type: ObjectType::Empty,
            transform: Transform::default(),
            visible: true,
            children: vec![],
        };
        self.state.add_object(new_object);
        cx.notify();
    }

    fn on_add_object_of_type(&mut self, action: &AddObjectOfType, _: &mut Window, cx: &mut Context<Self>) {
        let object_type = match action.object_type.as_str() {
            "Mesh" => ObjectType::Mesh,
            "Light" => ObjectType::Light,
            "Camera" => ObjectType::Camera,
            _ => ObjectType::Empty,
        };

        let new_object = SceneObject {
            id: format!("{}_{}", action.object_type.to_lowercase(), self.state.scene_objects.len() + 1),
            name: format!("New {}", action.object_type),
            object_type,
            transform: Transform::default(),
            visible: true,
            children: vec![],
        };
        self.state.add_object(new_object);
        cx.notify();
    }

    fn on_delete_object(&mut self, _: &DeleteObject, _: &mut Window, cx: &mut Context<Self>) {
        self.state.remove_selected_object();
        cx.notify();
    }

    fn on_duplicate_object(&mut self, _: &DuplicateObject, _: &mut Window, cx: &mut Context<Self>) {
        self.state.duplicate_selected_object();
        cx.notify();
    }

    fn on_select_object(&mut self, action: &SelectObject, _: &mut Window, cx: &mut Context<Self>) {
        self.state.select_object(Some(action.object_id.clone()));
        cx.notify();
    }

    fn on_toggle_object_expanded(&mut self, action: &ToggleObjectExpanded, _: &mut Window, cx: &mut Context<Self>) {
        self.state.toggle_object_expanded(&action.object_id);
        cx.notify();
    }

    fn on_toggle_grid(&mut self, _: &ToggleGrid, _: &mut Window, cx: &mut Context<Self>) {
        self.state.toggle_grid();
        cx.notify();
    }

    fn on_toggle_wireframe(&mut self, _: &ToggleWireframe, _: &mut Window, cx: &mut Context<Self>) {
        self.state.toggle_wireframe();
        cx.notify();
    }

    fn on_toggle_lighting(&mut self, _: &ToggleLighting, _: &mut Window, cx: &mut Context<Self>) {
        self.state.toggle_lighting();
        cx.notify();
    }

    fn on_toggle_performance_overlay(&mut self, _: &TogglePerformanceOverlay, _: &mut Window, cx: &mut Context<Self>) {
        self.state.toggle_performance_overlay();
        cx.notify();
    }

    fn on_toggle_viewport_controls(&mut self, _: &ToggleViewportControls, _: &mut Window, cx: &mut Context<Self>) {
        self.state.toggle_viewport_controls();
        cx.notify();
    }

    fn on_toggle_camera_mode_selector(&mut self, _: &ToggleCameraModeSelector, _: &mut Window, cx: &mut Context<Self>) {
        self.state.toggle_camera_mode_selector();
        cx.notify();
    }

    fn on_toggle_viewport_options(&mut self, _: &ToggleViewportOptions, _: &mut Window, cx: &mut Context<Self>) {
        self.state.toggle_viewport_options();
        cx.notify();
    }

    fn on_toggle_fps_graph_type(&mut self, _: &ToggleFpsGraphType, _: &mut Window, cx: &mut Context<Self>) {
        self.state.toggle_fps_graph_type();
        cx.notify();
    }

    fn on_perspective_view(&mut self, _: &PerspectiveView, _: &mut Window, cx: &mut Context<Self>) {
        self.state.set_camera_mode(CameraMode::Perspective);
        cx.notify();
    }

    fn on_orthographic_view(&mut self, _: &OrthographicView, _: &mut Window, cx: &mut Context<Self>) {
        self.state.set_camera_mode(CameraMode::Orthographic);
        cx.notify();
    }

    fn on_top_view(&mut self, _: &TopView, _: &mut Window, cx: &mut Context<Self>) {
        self.state.set_camera_mode(CameraMode::Top);
        cx.notify();
    }

    fn on_front_view(&mut self, _: &FrontView, _: &mut Window, cx: &mut Context<Self>) {
        self.state.set_camera_mode(CameraMode::Front);
        cx.notify();
    }

    fn on_side_view(&mut self, _: &SideView, _: &mut Window, cx: &mut Context<Self>) {
        self.state.set_camera_mode(CameraMode::Side);
        cx.notify();
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
            .key_context("LevelEditor")
            // Transform tools
            .on_action(cx.listener(Self::on_select_tool))
            .on_action(cx.listener(Self::on_move_tool))
            .on_action(cx.listener(Self::on_rotate_tool))
            .on_action(cx.listener(Self::on_scale_tool))
            // Object operations
            .on_action(cx.listener(Self::on_add_object))
            .on_action(cx.listener(Self::on_add_object_of_type))
            .on_action(cx.listener(Self::on_delete_object))
            .on_action(cx.listener(Self::on_duplicate_object))
            .on_action(cx.listener(Self::on_select_object))
            .on_action(cx.listener(Self::on_toggle_object_expanded))
            // View operations
            .on_action(cx.listener(Self::on_toggle_grid))
            .on_action(cx.listener(Self::on_toggle_wireframe))
            .on_action(cx.listener(Self::on_toggle_lighting))
            .on_action(cx.listener(Self::on_toggle_performance_overlay))
            .on_action(cx.listener(Self::on_toggle_viewport_controls))
            .on_action(cx.listener(Self::on_toggle_camera_mode_selector))
            .on_action(cx.listener(Self::on_toggle_viewport_options))
            .on_action(cx.listener(Self::on_toggle_fps_graph_type))
            // Camera modes
            .on_action(cx.listener(Self::on_perspective_view))
            .on_action(cx.listener(Self::on_orthographic_view))
            .on_action(cx.listener(Self::on_top_view))
            .on_action(cx.listener(Self::on_front_view))
            .on_action(cx.listener(Self::on_side_view))
            .child(
                // Toolbar at the top
                self.toolbar.render(&self.state, cx)
            )
            .child(
                // Main content area with resizable panels
                div()
                    .flex_1() // Grow to fill remaining space
                    .flex() // Enable flexbox
                    .flex_row() // Row direction for resizable panels
                    .min_h_0() // Allow shrinking below content size
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
                                                                    .p_1()
                                                                    .child(self.hierarchy.render(&self.state, cx))
                                                            )
                                                    )
                                            )
                                    )
                            )
                            .child(
                                // Center: Viewport
                                resizable_panel()
                                    .child(
                                        div()
                                            .size_full()
                                            .p_1()
                                            .child(
                                                self.viewport_panel.render(
                                                    &mut self.state,
                                                    self.fps_graph_is_line.clone(),
                                                    &self.gpu_engine,
                                                    &self.game_thread,
                                                    self.current_pattern,
                                                    cx
                                                )
                                            )
                                    )
                            )
                            .child(
                                // Right sidebar: Properties
                                resizable_panel()
                                    .size(px(300.))
                                    .size_range(px(250.)..px(450.))
                                    .child(
                                        div()
                                            .size_full()
                                            .bg(cx.theme().sidebar)
                                            .border_l_1()
                                            .border_color(cx.theme().border)
                                            .p_1()
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
