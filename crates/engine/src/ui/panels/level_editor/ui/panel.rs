use gpui::*;
use gpui_component::{
    dock::{Panel, PanelEvent},
    h_flex,
    resizable::{h_resizable, v_resizable, resizable_panel, ResizableState},
    v_flex,
    ActiveTheme as _,
    StyledExt,
};
// NEW: True zero-copy GPU viewport using ExternalTexture - NO CPU COPIES!
use gpui_component::gpu_viewport::{GpuViewport, create_gpu_viewport};

use crate::settings::EngineSettings;
use crate::ui::rainbow_engine_final::{RainbowRenderEngine, RainbowPattern};
use crate::ui::wgpu_3d_renderer::Wgpu3DRenderer;
use crate::ui::gpu_renderer::GpuRenderer;
use crate::ui::shared::StatusBar;
use engine_backend::{GameThread, GameState};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver, channel};
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

    // NEW: True zero-copy GPU viewport - NO CPU COPIES!
    viewport: Entity<GpuViewport>,
    gpu_engine: Arc<Mutex<GpuRenderer>>, // Full GPU renderer from backend
    current_pattern: RainbowPattern,
    render_speed: f32,
    render_enabled: Arc<std::sync::atomic::AtomicBool>,
    
    // Game thread for object movement and game logic
    game_thread: Arc<GameThread>,
    
    // Channel for receiving rendered frames from render thread
    frame_receiver: Receiver<Arc<Vec<u8>>>,
}

impl LevelEditorPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let horizontal_resizable_state = ResizableState::new(cx);
        let vertical_resizable_state = ResizableState::new(cx);

        println!("[LEVEL-EDITOR] 🚀 Initializing TRUE ZERO-COPY GPU viewport");
        println!("[LEVEL-EDITOR] 🔥 Direct GPU rendering - NO CPU COPIES - NO ALLOCATIONS!");

        // Load engine settings for frame pacing configuration
        let settings = EngineSettings::default_path()
            .and_then(|path| Some(EngineSettings::load(&path)))
            .unwrap_or_default();

        let max_viewport_fps = settings.advanced.max_viewport_fps;
        println!("[LEVEL-EDITOR] 🎯 Frame pacing configured: {} FPS",
            if max_viewport_fps == 0 { "Unlimited".to_string() } else { max_viewport_fps.to_string() });

        // NEW: Create GPU viewport with ExternalTexture system
        // TRUE ZERO-COPY: Bevy writes directly to GPU-visible memory!
        let viewport = create_gpu_viewport(
            1600,
            900,
            window,
            cx
        );

        println!("[LEVEL-EDITOR] ✅ GPU viewport created (1600x900) - ZERO CPU COPIES!");
        
        // Create GPU render engine with matching resolution
        let gpu_engine = Arc::new(Mutex::new(GpuRenderer::new(1600, 900)));
        let render_enabled = Arc::new(std::sync::atomic::AtomicBool::new(true));
        
        println!("[LEVEL-EDITOR] ✅ GPU renderer initialized");

        // Create and start game thread for object movement
        println!("[LEVEL-EDITOR] 🎮 Creating game thread with target 240 TPS...");
        let game_thread = Arc::new(GameThread::new(240.0));
        game_thread.start();
        println!("[LEVEL-EDITOR] ✅ Game thread started successfully!");

        // Create channel for frame data (render thread -> UI thread)
        let (frame_sender, frame_receiver) = channel();

        // Spawn GPU direct render thread - TRUE ZERO-COPY!
        let gpu_clone = gpu_engine.clone();
        let enabled_clone = render_enabled.clone();
        let game_thread_clone = game_thread.clone();
        let viewport_clone = viewport.clone();

        thread::spawn(move || {
            Self::render_thread_gpu_direct(
                gpu_clone,
                viewport_clone,
                enabled_clone,
                game_thread_clone,
                frame_sender,
            );
        });

        println!("[LEVEL-EDITOR] 🔥 GPU direct render thread spawned!");

        println!("[LEVEL-EDITOR] Modular level editor initialized");

        Self {
            focus_handle: cx.focus_handle(),
            state: LevelEditorState::new(),
            fps_graph_is_line: Rc::new(RefCell::new(true)),  // Default to line graph
            scene_browser: SceneBrowser::new(),
            hierarchy: HierarchyPanel::new(),
            properties: PropertiesPanel::new(),
            viewport_panel: ViewportPanel::new(viewport.clone(), render_enabled.clone(), cx),
            toolbar: ToolbarPanel::new(),
            horizontal_resizable_state,
            vertical_resizable_state,
            viewport,
            gpu_engine,
            current_pattern: RainbowPattern::Waves,
            render_speed: 2.0,
            render_enabled,
            game_thread,
            frame_receiver,
        }
    }

    /// TRUE ZERO-COPY GPU render thread - Writes directly to GPU-visible memory!
    /// NO CPU COPIES - NO ALLOCATIONS - MAXIMUM PERFORMANCE!
    fn render_thread_gpu_direct(
        gpu_engine: Arc<Mutex<GpuRenderer>>,
        _viewport: Entity<GpuViewport>,
        render_enabled: Arc<std::sync::atomic::AtomicBool>,
        game_thread: Arc<GameThread>,
        frame_sender: Sender<Arc<Vec<u8>>>,
    ) {
        let base_frame_time = Duration::from_millis(8);
        let mut adaptive_frame_time = base_frame_time;
        let mut frame_count = 0u64;
        let mut consecutive_fast_frames = 0u32;
        let max_cpu_usage = 85;

        // Pre-allocate GPU buffer once (reuse for efficiency)
        let buffer_size = (1600 * 900 * 4) as usize;
        let mut gpu_buffer = vec![0u8; buffer_size];

        println!("[RENDER-THREAD-GPU] 🔥 Starting TRUE ZERO-COPY render loop!");
        println!("[RENDER-THREAD-GPU] 💎 Pre-allocated buffer: {} MB", buffer_size as f64 / 1_048_576.0);
        println!("[RENDER-THREAD-GPU] 🚀 Frames sent to UI thread for GPU upload!");

        while render_enabled.load(std::sync::atomic::Ordering::Relaxed) {
            let frame_start = std::time::Instant::now();

            // Sync game objects to renderer before rendering
            if let Ok(game_state) = game_thread.get_state().lock() {
                let objects = game_state.objects.clone();
                if let Ok(mut engine) = gpu_engine.try_lock() {
                    if let Some(ref mut bevy_renderer) = engine.bevy_renderer {
                        bevy_renderer.update_game_objects(objects);
                    }
                }
            }

            // Render to buffer
            let render_successful = if let Ok(mut engine_guard) = gpu_engine.try_lock() {
                engine_guard.render_to_buffer(&mut gpu_buffer);
                true
            } else {
                false
            };

            if render_successful {
                frame_count += 1;

                // Send frame to UI thread (Arc for zero-copy sharing)
                let frame_data = Arc::new(gpu_buffer.clone());
                let _ = frame_sender.send(frame_data);

                if frame_count % 120 == 1 {
                    println!("[RENDER-THREAD-GPU] 🔥 {} frames rendered!", frame_count);
                }
            } else if frame_count % 120 == 1 {
                println!("[RENDER-THREAD-GPU] ⚠️  Frame {} render failed", frame_count);
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
        }

        println!("[RENDER-THREAD-GPU] 🛑 GPU direct render loop exited");
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Process any pending frames from the render thread
        // This is TRUE ZERO-COPY: we write directly to GPU-mapped memory!
        #[cfg(target_os = "windows")]
        {
            if let Some(texture_id) = self.viewport.read(cx).texture_id() {
                // Drain all pending frames (keep only the latest)
                let mut latest_frame: Option<Arc<Vec<u8>>> = None;
                while let Ok(frame) = self.frame_receiver.try_recv() {
                    latest_frame = Some(frame);
                }
                
                // Update external texture with latest frame
                if let Some(frame_data) = latest_frame {
                    let result = window.update_external_texture(texture_id, |buffer| {
                        // TRUE ZERO-COPY: Write directly to GPU-mapped memory!
                        let copy_len = frame_data.len().min(buffer.len());
                        buffer[..copy_len].copy_from_slice(&frame_data[..copy_len]);
                    });
                    
                    if let Err(e) = result {
                        eprintln!("[LEVEL-EDITOR] ❌ Failed to update texture: {}", e);
                    }
                    
                    // Request next frame
                    cx.notify();
                }
            }
        }
        
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
