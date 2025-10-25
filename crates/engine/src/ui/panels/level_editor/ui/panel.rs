use gpui::*;
use gpui_component::{
    dock::{Panel, PanelEvent},
    h_flex,
    resizable::{h_resizable, v_resizable, resizable_panel, ResizableState},
    v_flex,
    ActiveTheme as _,
    StyledExt,
};
// Zero-copy Bevy viewport for 3D rendering
use gpui_component::bevy_viewport::{BevyViewport, BevyViewportState};

use crate::settings::EngineSettings;
// DEPRECATED: Old software renderers - replaced by Bevy
// use crate::ui::rainbow_engine_final::{RainbowRenderEngine, RainbowPattern};
// use crate::ui::wgpu_3d_renderer::Wgpu3DRenderer;
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
    ViewportPanel, ToolbarPanel, CameraMode, TransformTool,
};
use super::actions::*;
use crate::ui::panels::level_editor::scene_database::{
    SceneObjectData, ObjectType, Transform, MeshType, LightType,
};

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

    // Zero-copy Bevy viewport for 3D rendering
    viewport: Entity<BevyViewport>,
    viewport_state: Arc<parking_lot::RwLock<BevyViewportState>>,
    gpu_engine: Arc<Mutex<GpuRenderer>>, // Full GPU renderer from backend
    render_enabled: Arc<std::sync::atomic::AtomicBool>,
    
    // Game thread for object movement and game logic
    game_thread: Arc<GameThread>,
}

impl LevelEditorPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let horizontal_resizable_state = ResizableState::new(cx);
        let vertical_resizable_state = ResizableState::new(cx);

        println!("[LEVEL-EDITOR] 🚀 Initializing Bevy-powered viewport");
        println!("[LEVEL-EDITOR] 🔥 Zero-copy GPU texture sharing!");

        // Load engine settings for frame pacing configuration
        let settings = EngineSettings::default_path()
            .and_then(|path| Some(EngineSettings::load(&path)))
            .unwrap_or_default();

        let max_viewport_fps = settings.advanced.max_viewport_fps;
        println!("[LEVEL-EDITOR] 🎯 Frame pacing configured: {} FPS",
            if max_viewport_fps == 0 { "Unlimited".to_string() } else { max_viewport_fps.to_string() });

        // Create Bevy viewport with zero-copy shared textures
        let viewport = cx.new(|cx| BevyViewport::new(1600, 900, cx));
        let viewport_state = viewport.read(cx).shared_state();

        println!("[LEVEL-EDITOR] ✅ Bevy viewport created (1600x900)");
        
        // Create game thread but DON'T start it yet (editor starts in Edit mode)
        println!("[LEVEL-EDITOR] 🎮 Creating game thread (paused - Edit mode)...");
        let game_thread = Arc::new(GameThread::new(240.0));
        let game_state = game_thread.get_state();
        game_thread.set_enabled(false); // CRITICAL: Start disabled for Edit mode
        game_thread.start(); // Thread runs but does nothing while disabled
        println!("[LEVEL-EDITOR] ✅ Game thread created (paused for Edit mode)");
        
        // Create GPU render engine WITHOUT game thread link initially (Edit mode)
        println!("[LEVEL-EDITOR] 🎨 Creating GPU renderer (no game sync in Edit mode)...");
        let gpu_engine = Arc::new(Mutex::new(GpuRenderer::new(1600, 900))); // No game state initially
        let render_enabled = Arc::new(std::sync::atomic::AtomicBool::new(true));
        
        println!("[LEVEL-EDITOR] ✅ GPU renderer initialized (Edit mode)");
        println!("[LEVEL-EDITOR] 📝 Editor ready in EDIT MODE - objects frozen");

        // Wait a moment for Bevy to create shared textures, then initialize viewport
        let viewport_state_for_init = viewport_state.clone();
        let viewport_entity_for_init = viewport.clone();
        let gpu_engine_for_init = gpu_engine.clone();
        cx.spawn(async move |_this, mut cx| {
            // Try multiple times with increasing delays for Bevy to initialize
            for attempt in 1..=10 {
                cx.background_executor().timer(Duration::from_millis(200 * attempt)).await;
                
                println!("[LEVEL-EDITOR] 🔄 Attempt {} to initialize viewport with Bevy shared textures...", attempt);
                
                // Get native handles from Bevy renderer
                if let Ok(engine) = gpu_engine_for_init.lock() {
                    if let Some(ref bevy_renderer) = engine.bevy_renderer {
                        // Get both handles from Bevy's double buffer
                        if let Some(handles) = bevy_renderer.get_shared_texture_handles() {
                            if handles.len() < 2 {
                                println!("[LEVEL-EDITOR] ⚠️  Not enough handles");
                                return;
                            }
                            let handle0 = handles[0] as isize;
                            let handle1 = handles[1] as isize;
                            println!("[LEVEL-EDITOR] ✅ Got shared texture handles from Bevy on attempt {}", attempt);
                            
                            #[cfg(target_os = "windows")]
                            {
                                println!("[LEVEL-EDITOR] 📍 Initializing viewport with handles: 0x{:X}, 0x{:X}", handle0, handle1);
                                viewport_state_for_init.write().initialize_shared_textures(handle0, handle1, 1600, 900);
                                println!("[LEVEL-EDITOR] 🎉 Viewport initialized with shared textures!");
                                
                                // Notify the viewport element to refresh
                                let _ = cx.update(|cx| {
                                    viewport_entity_for_init.update(cx, |_viewport, cx| {
                                        cx.notify();
                                        println!("[LEVEL-EDITOR] ✅ Notified GPUI to refresh viewport UI");
                                    })
                                });

                                // Successfully initialized! The viewport will automatically refresh
                                // when Bevy renders new frames - we check in the render() method
                                println!("[LEVEL-EDITOR] 🎉 Viewport initialized with shared textures!");
                                println!("[LEVEL-EDITOR] ✅ Notified GPUI to refresh viewport UI");
                                
                                return; // Success!
                            }
                            
                            #[cfg(not(target_os = "windows"))]
                            {
                                println!("[LEVEL-EDITOR] ⚠️  Non-Windows platform not yet supported");
                                return;
                            }
                        } else {
                            println!("[LEVEL-EDITOR] ⚠️  Attempt {}: Shared texture handles not ready yet", attempt);
                        }
                    } else {
                        println!("[LEVEL-EDITOR] ⚠️  Attempt {}: Bevy renderer not available yet", attempt);
                    }
                } else {
                    println!("[LEVEL-EDITOR] ⚠️  Attempt {}: Could not lock GPU engine", attempt);
                }
            }
            
            println!("[LEVEL-EDITOR] ❌ Failed to initialize viewport after 10 attempts");
        }).detach();

        
        println!("[LEVEL-EDITOR] Modular level editor initialized");

        Self {
            focus_handle: cx.focus_handle(),
            state: LevelEditorState::new(),
            fps_graph_is_line: Rc::new(RefCell::new(true)),  // Default to line graph
            scene_browser: SceneBrowser::new(),
            hierarchy: HierarchyPanel::new(),
            properties: PropertiesPanel::new(),
            viewport_panel: ViewportPanel::new(viewport.clone(), render_enabled.clone(), window, cx),
            toolbar: ToolbarPanel::new(),
            horizontal_resizable_state,
            vertical_resizable_state,
            viewport,
            viewport_state,
            gpu_engine,
            render_enabled,
            game_thread,
        }
    }

    pub fn toggle_rendering(&mut self) {
        let current = self.render_enabled.load(std::sync::atomic::Ordering::Relaxed);
        self.render_enabled.store(!current, std::sync::atomic::Ordering::Relaxed);
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let objects_count = self.state.scene_objects().len();
        let selected_name = self.state.selected_object()
            .and_then(|id| self.state.scene_database.get_object(&id))
            .map(|obj| obj.name.clone())
            .unwrap_or_else(|| "None".to_string());
        
        StatusBar::new()
            .add_left_item(format!("Objects: {}", objects_count))
            .add_left_item(format!("Selected: {}", selected_name))
            .add_right_item(format!("{:?}", self.state.camera_mode))
            .add_right_item(format!("Grid: {}", if self.state.show_grid { "On" } else { "Off" }))
            .add_right_item(format!("Tool: {:?}", self.state.current_tool))
            .render(cx)
    }

    // Action handlers
    fn on_select_tool(&mut self, _: &SelectTool, _: &mut Window, cx: &mut Context<Self>) {
        self.state.set_tool(TransformTool::Select);
        
        // Update gizmo type in Bevy
        if let Ok(mut engine) = self.gpu_engine.lock() {
            if let Some(ref bevy_renderer) = engine.bevy_renderer {
                if let Ok(mut gizmo) = bevy_renderer.gizmo_state.lock() {
                    use engine_backend::subsystems::render::bevy_renderer::BevyGizmoType;
                    gizmo.gizmo_type = BevyGizmoType::None;
                    println!("[LEVEL-EDITOR] 🎯 Gizmo type: None (Select mode)");
                }
            }
        }
        
        cx.notify();
    }

    fn on_move_tool(&mut self, _: &MoveTool, _: &mut Window, cx: &mut Context<Self>) {
        self.state.set_tool(TransformTool::Move);
        
        // Update gizmo type in Bevy
        if let Ok(mut engine) = self.gpu_engine.lock() {
            if let Some(ref bevy_renderer) = engine.bevy_renderer {
                if let Ok(mut gizmo) = bevy_renderer.gizmo_state.lock() {
                    use engine_backend::subsystems::render::bevy_renderer::BevyGizmoType;
                    gizmo.gizmo_type = BevyGizmoType::Translate;
                    println!("[LEVEL-EDITOR] 🎯 Gizmo type: Translate");
                }
            }
        }
        
        cx.notify();
    }

    fn on_rotate_tool(&mut self, _: &RotateTool, _: &mut Window, cx: &mut Context<Self>) {
        self.state.set_tool(TransformTool::Rotate);
        
        // Update gizmo type in Bevy
        if let Ok(mut engine) = self.gpu_engine.lock() {
            if let Some(ref bevy_renderer) = engine.bevy_renderer {
                if let Ok(mut gizmo) = bevy_renderer.gizmo_state.lock() {
                    use engine_backend::subsystems::render::bevy_renderer::BevyGizmoType;
                    gizmo.gizmo_type = BevyGizmoType::Rotate;
                    println!("[LEVEL-EDITOR] 🎯 Gizmo type: Rotate");
                }
            }
        }
        
        cx.notify();
    }

    fn on_scale_tool(&mut self, _: &ScaleTool, _: &mut Window, cx: &mut Context<Self>) {
        self.state.set_tool(TransformTool::Scale);
        
        // Update gizmo type in Bevy
        if let Ok(mut engine) = self.gpu_engine.lock() {
            if let Some(ref bevy_renderer) = engine.bevy_renderer {
                if let Ok(mut gizmo) = bevy_renderer.gizmo_state.lock() {
                    use engine_backend::subsystems::render::bevy_renderer::BevyGizmoType;
                    gizmo.gizmo_type = BevyGizmoType::Scale;
                    println!("[LEVEL-EDITOR] 🎯 Gizmo type: Scale");
                }
            }
        }
        
        cx.notify();
    }

    fn on_add_object(&mut self, _: &AddObject, _: &mut Window, cx: &mut Context<Self>) {
        let objects_count = self.state.scene_objects().len();
        let new_object = SceneObjectData {
            id: format!("object_{}", objects_count + 1),
            name: "New Object".to_string(),
            object_type: ObjectType::Empty,
            transform: Transform::default(),
            visible: true,
            locked: false,
            parent: None,
            children: vec![],
            components: vec![],
        };
        self.state.scene_database.add_object(new_object, None);
        self.state.has_unsaved_changes = true;
        cx.notify();
    }

    fn on_add_object_of_type(&mut self, action: &AddObjectOfType, _: &mut Window, cx: &mut Context<Self>) {
        let object_type = match action.object_type.as_str() {
            "Mesh" => ObjectType::Mesh(MeshType::Cube),
            "Light" => ObjectType::Light(LightType::Directional),
            "Camera" => ObjectType::Camera,
            _ => ObjectType::Empty,
        };

        let objects_count = self.state.scene_objects().len();
        let new_object = SceneObjectData {
            id: format!("{}_{}", action.object_type.to_lowercase(), objects_count + 1),
            name: format!("New {}", action.object_type),
            object_type,
            transform: Transform::default(),
            visible: true,
            locked: false,
            parent: None,
            children: vec![],
            components: vec![],
        };
        self.state.scene_database.add_object(new_object, None);
        self.state.has_unsaved_changes = true;
        cx.notify();
    }

    fn on_delete_object(&mut self, _: &DeleteObject, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.state.selected_object() {
            self.state.scene_database.remove_object(&id);
            self.state.has_unsaved_changes = true;
            
            // Deselect after deletion
            self.state.select_object(None);
            
            // Clear gizmo selection in Bevy
            if let Ok(mut engine) = self.gpu_engine.lock() {
                if let Some(ref bevy_renderer) = engine.bevy_renderer {
                    if let Ok(mut gizmo) = bevy_renderer.gizmo_state.lock() {
                        gizmo.selected_object_id = None;
                        println!("[LEVEL-EDITOR] 🚫 Object deleted - Gizmo hidden");
                    }
                }
            }
        }
        cx.notify();
    }

    fn on_duplicate_object(&mut self, _: &DuplicateObject, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(id) = self.state.selected_object() {
            self.state.scene_database.duplicate_object(&id);
            self.state.has_unsaved_changes = true;
        }
        cx.notify();
    }

    fn on_select_object(&mut self, action: &SelectObject, _: &mut Window, cx: &mut Context<Self>) {
        self.state.select_object(Some(action.object_id.clone()));
        
        // Update gizmo position and selection in Bevy when object is selected
        if let Some(obj) = self.state.get_selected_object() {
            if let Ok(mut engine) = self.gpu_engine.lock() {
                if let Some(ref bevy_renderer) = engine.bevy_renderer {
                    if let Ok(mut gizmo) = bevy_renderer.gizmo_state.lock() {
                        // Set selected object ID (String)
                        gizmo.selected_object_id = Some(obj.id.clone());
                        
                        // Update position
                        gizmo.target_position.x = obj.transform.position[0];
                        gizmo.target_position.y = obj.transform.position[1];
                        gizmo.target_position.z = obj.transform.position[2];
                        
                        println!("[LEVEL-EDITOR] 🎯 Object '{}' selected - Gizmo at ({}, {}, {})",
                            obj.id,
                            gizmo.target_position.x, gizmo.target_position.y, gizmo.target_position.z);
                    }
                }
            }
        }
        
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

    fn on_play_scene(&mut self, _: &PlayScene, _: &mut Window, cx: &mut Context<Self>) {
        println!("[LEVEL-EDITOR] ▶️ PLAY button pressed");
        
        // Enter play mode (saves scene snapshot)
        self.state.enter_play_mode();
        
        // Enable game thread
        self.game_thread.set_enabled(true);
        println!("[LEVEL-EDITOR] ✅ Game thread enabled - objects will move");
        
        // Hide gizmos in Bevy (Play mode)
        if let Ok(mut engine) = self.gpu_engine.lock() {
            if let Some(ref bevy_renderer) = engine.bevy_renderer {
                if let Ok(mut gizmo) = bevy_renderer.gizmo_state.lock() {
                    gizmo.enabled = false;
                    println!("[LEVEL-EDITOR] 🚫 Gizmos hidden (Play mode)");
                }
            }
        }
        
        cx.notify();
    }

    fn on_stop_scene(&mut self, _: &StopScene, _: &mut Window, cx: &mut Context<Self>) {
        println!("[LEVEL-EDITOR] ⏹️ STOP button pressed");
        
        // Disable game thread
        self.game_thread.set_enabled(false);
        println!("[LEVEL-EDITOR] ⏸️ Game thread disabled");
        
        // Exit play mode (restores scene from snapshot)
        self.state.exit_play_mode();
        println!("[LEVEL-EDITOR] ✅ Scene restored to edit state");
        
        // Show gizmos in Bevy (Edit mode)
        if let Ok(mut engine) = self.gpu_engine.lock() {
            if let Some(ref bevy_renderer) = engine.bevy_renderer {
                if let Ok(mut gizmo) = bevy_renderer.gizmo_state.lock() {
                    gizmo.enabled = true;
                    println!("[LEVEL-EDITOR] ✅ Gizmos visible (Edit mode)");
                }
            }
        }
        
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
        // Sync viewport buffer index with Bevy's read index each frame
        if let Ok(engine_guard) = self.gpu_engine.try_lock() {
            if let Some(ref bevy_renderer) = engine_guard.bevy_renderer {
                let read_idx = bevy_renderer.get_read_index();
                let mut state = self.viewport_state.write();
                state.set_active_buffer(read_idx);
            }
        }

        // Request continuous animation frames to keep viewport and stats updating
        // This creates a render loop synchronized with the display refresh rate
        window.request_animation_frame();

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
            // Play/Edit mode
            .on_action(cx.listener(Self::on_play_scene))
            .on_action(cx.listener(Self::on_stop_scene))
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

