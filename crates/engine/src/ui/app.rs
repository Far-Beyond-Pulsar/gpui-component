use gpui::{prelude::*, Animation, AnimationExt as _, SharedString, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dock::{DockArea, DockItem, Panel, PanelEvent, TabPanel},
    h_flex, v_flex, ActiveTheme as _, IconName, Sizable as _, StyledExt,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;
use std::{sync::Arc, time::Duration};

use super::{
    editors::EditorType,
    entry_screen::EntryScreen,
    file_manager_drawer::{FileManagerDrawer, FileSelected, FileType, PopoutFileManagerEvent},
    file_manager_window::FileManagerWindow,
    menu::AppTitleBar,
    panels::{BlueprintEditorPanel, DawEditorPanel, LevelEditorPanel, ScriptEditorPanel},
    problems_drawer::ProblemsDrawer,
    problems_window::ProblemsWindow,
    project_selector::ProjectSelected,
    rust_analyzer_manager::{AnalyzerEvent, AnalyzerStatus, RustAnalyzerManager},
    terminal_drawer::TerminalDrawer,
    terminal_window::TerminalWindow,
};

// Action to toggle the file manager drawer
#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = pulsar_app)]
pub struct ToggleFileManager;

// Action to toggle the problems drawer
#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = pulsar_app)]
pub struct ToggleProblems;

// Action to toggle the terminal
#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = pulsar_app)]
pub struct ToggleTerminal;

// Root wrapper that contains the titlebar, matching gpui-component storybook structure
pub struct PulsarRoot {
    title_bar: Entity<AppTitleBar>,
    app: Entity<PulsarApp>,
}

impl PulsarRoot {
    pub fn new(
        title: impl Into<SharedString>,
        app: Entity<PulsarApp>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let title_bar = cx.new(|cx| AppTitleBar::new(title, window, cx));
        Self { title_bar, app }
    }
}

impl Render for PulsarRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        use gpui_component::Root;
        
        let drawer_layer = Root::render_drawer_layer(window, cx);
        let modal_layer = Root::render_modal_layer(window, cx);
        let notification_layer = Root::render_notification_layer(window, cx);

        div()
            .size_full()
            .child(
                v_flex()
                    .size_full()
                    .child(self.title_bar.clone())
                    .child(div().flex_1().overflow_hidden().child(self.app.clone()))
            )
            .children(drawer_layer)
            .children(modal_layer)
            .children(notification_layer)
    }
}

pub struct PulsarApp {
    dock_area: Entity<DockArea>,
    project_path: Option<PathBuf>,
    entry_screen: Option<Entity<EntryScreen>>,
    file_manager_drawer: Entity<FileManagerDrawer>,
    drawer_open: bool,
    problems_drawer: Entity<ProblemsDrawer>,
    terminal_drawer: Entity<TerminalDrawer>,
    // Tab management
    center_tabs: Entity<TabPanel>,
    script_editor: Option<Entity<ScriptEditorPanel>>,
    blueprint_editors: Vec<Entity<BlueprintEditorPanel>>,
    daw_editors: Vec<Entity<DawEditorPanel>>,
    next_tab_id: usize,
    // Rust Analyzer
    rust_analyzer: Entity<RustAnalyzerManager>,
    analyzer_status_text: String,
}

impl PulsarApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_internal(None, None, true, window, cx)
    }

    pub fn new_with_project(
        project_path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        eprintln!(
            "DEBUG: PulsarApp::new_with_project called with path: {:?}",
            project_path
        );
        Self::new_internal(Some(project_path), None, true, window, cx)
    }
    
    /// Create a new PulsarApp with a pre-initialized rust analyzer
    /// This is used when loading from the loading screen to avoid re-initializing
    pub fn new_with_project_and_analyzer(
        project_path: PathBuf,
        rust_analyzer: Entity<RustAnalyzerManager>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        eprintln!(
            "DEBUG: PulsarApp::new_with_project_and_analyzer called with path: {:?}",
            project_path
        );
        Self::new_internal(Some(project_path), Some(rust_analyzer), true, window, cx)
    }

    /// Create a new window that shares the rust analyzer from an existing window
    /// This is used for detached windows and doesn't create a default Level Editor tab
    pub fn new_with_shared_analyzer(
        project_path: Option<PathBuf>,
        rust_analyzer: Entity<RustAnalyzerManager>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self::new_internal(project_path, Some(rust_analyzer), false, window, cx)
    }

    /// Get the global rust analyzer manager
    pub fn rust_analyzer(&self) -> &Entity<RustAnalyzerManager> {
        &self.rust_analyzer
    }

    /// Get the current workspace root
    pub fn workspace_root(&self) -> Option<&PathBuf> {
        self.project_path.as_ref()
    }

    /// Create a detached window with a panel, sharing the rust analyzer
    fn create_detached_window(
        &self,
        panel: Arc<dyn gpui_component::dock::PanelView>,
        position: gpui::Point<gpui::Pixels>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use gpui::{px, size, Bounds, Point, WindowBounds, WindowKind, WindowOptions};
        use gpui_component::Root;

        let window_size = size(px(800.), px(600.));
        let window_bounds = Bounds::new(
            Point {
                x: position.x - px(100.0),
                y: position.y - px(30.0),
            },
            window_size,
        );

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: Some(gpui::TitlebarOptions {
                title: None,
                appears_transparent: true,
                traffic_light_position: None, // No traffic lights
            }),
            window_min_size: Some(gpui::Size {
                width: px(400.),
                height: px(300.),
            }),
            kind: WindowKind::Normal,
            is_resizable: true,
            window_decorations: Some(gpui::WindowDecorations::Client),
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            ..Default::default()
        };

        let project_path = self.project_path.clone();
        let rust_analyzer = self.rust_analyzer.clone();

        let _ = cx.open_window(window_options, move |window, cx| {
            // Create PulsarApp with shared rust analyzer
            let app = cx.new(|cx| {
                let mut app = Self::new_with_shared_analyzer(
                    project_path.clone(),
                    rust_analyzer.clone(),
                    window,
                    cx,
                );

                // Add the panel to the center tabs
                app.center_tabs.update(cx, |tabs, cx| {
                    tabs.add_panel(panel.clone(), window, cx);
                });

                app
            });

            cx.new(|cx| Root::new(app.into(), window, cx))
        });
    }

    fn new_internal(
        project_path: Option<PathBuf>,
        shared_rust_analyzer: Option<Entity<RustAnalyzerManager>>,
        create_level_editor: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // Create the main dock area
        let dock_area = cx.new(|cx| DockArea::new("main-dock", Some(1), window, cx));
        let weak_dock = dock_area.downgrade();

        // Create center dock item with level editor tab if requested
        let center_dock_item = if create_level_editor {
            let level_editor = cx.new(|cx| LevelEditorPanel::new(window, cx));
            DockItem::tabs(
                vec![Arc::new(level_editor.clone())],
                Some(0),
                &weak_dock,
                window,
                cx,
            )
        } else {
            // Create empty tabs for detached windows
            DockItem::tabs(
                vec![],
                None,
                &weak_dock,
                window,
                cx,
            )
        };

        dock_area.update(cx, |dock, cx| {
            dock.set_center(center_dock_item, window, cx);
        });

        // Get the center TabPanel for dynamic tab management
        let center_tabs = if let DockItem::Tabs { view, .. } = dock_area.read(cx).items() {
            view.clone()
        } else {
            panic!("Expected tabs dock item");
        };

        // Initialize editor tracking
        let script_editor = None;
        let blueprint_editors = Vec::new();
        let daw_editors = Vec::new();

        // Create entry screen only if no project path is provided
        let entry_screen = if project_path.is_none() {
            let screen = cx.new(|cx| EntryScreen::new(window, cx));
            Some(screen)
        } else {
            None
        };

        // Create file manager drawer with the project path if provided
        let file_manager_drawer =
            cx.new(|cx| FileManagerDrawer::new(project_path.clone(), window, cx));
        cx.subscribe_in(&file_manager_drawer, window, Self::on_file_selected)
            .detach();
        cx.subscribe_in(&file_manager_drawer, window, Self::on_popout_file_manager)
            .detach();

        // Create problems drawer
        let problems_drawer = cx.new(|cx| ProblemsDrawer::new(window, cx));
        cx.subscribe_in(&problems_drawer, window, Self::on_navigate_to_diagnostic)
            .detach();

        // Create terminal drawer
        let terminal_drawer = cx.new(|cx| TerminalDrawer::new(window, cx));

        // Create rust analyzer manager or use shared one
        let rust_analyzer = if let Some(shared_analyzer) = shared_rust_analyzer {
            // Use the shared rust analyzer from another window
            shared_analyzer
        } else {
            // Create a new rust analyzer for this window
            let analyzer = cx.new(|cx| RustAnalyzerManager::new(window, cx));

            // Start rust analyzer if we have a project
            if let Some(ref project) = project_path {
                analyzer.update(cx, |analyzer, cx| {
                    analyzer.start(project.clone(), window, cx);
                });
            }

            analyzer
        };

        // Subscribe to analyzer events
        cx.subscribe_in(&rust_analyzer, window, Self::on_analyzer_event)
            .detach();

        // Subscribe to PanelEvent on center_tabs to handle tab close and cleanup
        cx.subscribe_in(&center_tabs, window, Self::on_tab_panel_event)
            .detach();

        // Subscribe to ProjectSelected events from entry screen or project selector
        if let Some(screen) = &entry_screen {
            cx.subscribe_in(screen, window, Self::on_project_selected)
                .detach();
        }

        Self {
            dock_area,
            project_path,
            entry_screen,
            file_manager_drawer,
            drawer_open: false,
            problems_drawer,
            terminal_drawer,
            center_tabs,
            script_editor,
            blueprint_editors,
            daw_editors,
            next_tab_id: 1,
            rust_analyzer,
            analyzer_status_text: "Idle".to_string(),
        }
    }

    fn on_analyzer_event(
        &mut self,
        _manager: &Entity<RustAnalyzerManager>,
        event: &AnalyzerEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            AnalyzerEvent::StatusChanged(status) => {
                self.analyzer_status_text = match status {
                    AnalyzerStatus::Idle => "Idle".to_string(),
                    AnalyzerStatus::Starting => "Starting...".to_string(),
                    AnalyzerStatus::Indexing { progress, message } => {
                        format!("Indexing: {} ({:.0}%)", message, progress * 100.0)
                    }
                    AnalyzerStatus::Ready => "Ready ✓".to_string(),
                    AnalyzerStatus::Error(e) => format!("Error: {}", e),
                    AnalyzerStatus::Stopped => "Stopped".to_string(),
                };
                cx.notify();
            }
            AnalyzerEvent::IndexingProgress { progress, message } => {
                self.analyzer_status_text =
                    format!("Indexing: {} ({:.0}%)", message, progress * 100.0);
                cx.notify();
            }
            AnalyzerEvent::Ready => {
                self.analyzer_status_text = "Ready ✓".to_string();
                cx.notify();
            }
            AnalyzerEvent::Error(e) => {
                self.analyzer_status_text = format!("Error: {}", e);
                cx.notify();
            }
            AnalyzerEvent::Diagnostics(diagnostics) => {
                // Forward diagnostics to the problems drawer
                self.problems_drawer.update(cx, |drawer, cx| {
                    drawer.set_diagnostics(diagnostics.clone(), cx);
                });
                cx.notify();
            }
        }
    }

    fn on_project_selected(
        &mut self,
        _selector: &Entity<EntryScreen>,
        event: &ProjectSelected,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.project_path = Some(event.path.clone());
        self.entry_screen = None; // Hide entry screen once project is loaded

        // Update file manager with project path
        self.file_manager_drawer.update(cx, |drawer, cx| {
            drawer.set_project_path(event.path.clone(), cx);
        });

        // Start rust analyzer for the project
        self.rust_analyzer.update(cx, |analyzer, cx| {
            analyzer.start(event.path.clone(), window, cx);
        });

        println!("Project selected: {:?}", event.path);
        cx.notify();
    }

    fn on_tab_panel_event(
        &mut self,
        _tabs: &Entity<TabPanel>,
        event: &PanelEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            PanelEvent::MoveToNewWindow(panel, position) => {
                // Create a new window with the same project and shared rust analyzer
                self.create_detached_window(panel.clone(), *position, window, cx);
            }
            PanelEvent::TabClosed(entity_id) => {
                self.blueprint_editors
                    .retain(|e| e.entity_id() != *entity_id);
                self.daw_editors
                    .retain(|e| e.entity_id() != *entity_id);
            }
            _ => {}
        }
    }

    fn on_file_selected(
        &mut self,
        _drawer: &Entity<FileManagerDrawer>,
        event: &FileSelected,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        eprintln!(
            "DEBUG: FileSelected event received - path: {:?}, type: {:?}",
            event.path, event.file_type
        );

        match event.file_type {
            FileType::Class => {
                eprintln!("DEBUG: Opening blueprint tab");
                self.open_blueprint_tab(event.path.clone(), window, cx);
            }
            FileType::Script => {
                eprintln!("DEBUG: Opening script tab");
                self.open_script_tab(event.path.clone(), window, cx);
            }
            FileType::DawProject => {
                eprintln!("DEBUG: Opening DAW tab for path: {:?}", event.path);
                self.open_daw_tab(event.path.clone(), window, cx);
            }
            _ => {
                eprintln!("DEBUG: Unknown file type, ignoring");
            }
        }

        // Close the drawer after opening a file
        self.drawer_open = false;
        cx.notify();
    }

    fn on_popout_file_manager(
        &mut self,
        _drawer: &Entity<FileManagerDrawer>,
        event: &PopoutFileManagerEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use gpui::{px, size, Bounds, Point, WindowBounds, WindowKind, WindowOptions};
        use gpui_component::Root;

        let project_path = event.project_path.clone();

        // Get a clone of self as Entity to pass to the file manager window
        let parent_app = cx.entity().clone();

        // Close the drawer when popping out
        self.drawer_open = false;
        cx.notify();

        // Open the file manager window
        let _ = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point {
                        x: px(100.0),
                        y: px(100.0),
                    },
                    size: size(px(1000.0), px(700.0)),
                })),
                titlebar: Some(gpui::TitlebarOptions {
                    title: None,
                    appears_transparent: true,
                    traffic_light_position: None, // No traffic lights
                }),
                kind: WindowKind::Normal,
                is_resizable: true,
                window_decorations: Some(gpui::WindowDecorations::Client),
                window_min_size: Some(gpui::Size {
                    width: px(600.),
                    height: px(400.),
                }),
                ..Default::default()
            },
            move |window, cx| {
                // Create a new file manager drawer for the window (use new_in_window)
                let new_drawer =
                    cx.new(|cx| FileManagerDrawer::new_in_window(project_path.clone(), window, cx));

                let file_manager_window =
                    cx.new(|cx| FileManagerWindow::new(new_drawer, parent_app.clone(), window, cx));

                cx.new(|cx| Root::new(file_manager_window.into(), window, cx))
            },
        );
    }

    fn toggle_drawer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.drawer_open = !self.drawer_open;
        cx.notify();
    }

    fn toggle_problems(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use gpui::{px, size, Bounds, Point, WindowBounds, WindowKind, WindowOptions};
        use gpui_component::Root;

        // Open problems in a separate window
        let problems_drawer = self.problems_drawer.clone();

        let _ = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point {
                        x: px(100.0),
                        y: px(100.0),
                    },
                    size: size(px(900.0), px(600.0)),
                })),
                titlebar: Some(gpui::TitlebarOptions {
                    title: None,
                    appears_transparent: true,
                    traffic_light_position: None, // No traffic lights
                }),
                kind: WindowKind::Normal,
                is_resizable: true,
                window_decorations: Some(gpui::WindowDecorations::Client),
                window_min_size: Some(gpui::Size {
                    width: px(500.),
                    height: px(300.),
                }),
                ..Default::default()
            },
            |window, cx| {
                let problems_window = cx.new(|cx| ProblemsWindow::new(problems_drawer, window, cx));

                cx.new(|cx| Root::new(problems_window.into(), window, cx))
            },
        );
    }

    fn toggle_terminal(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        use gpui::{px, size, Bounds, Point, WindowBounds, WindowKind, WindowOptions};
        use gpui_component::Root;

        // Open terminal in a separate window
        // Each window gets its own independent terminal instance
        let _ = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: Point {
                        x: px(150.0),
                        y: px(150.0),
                    },
                    size: size(px(1000.0), px(700.0)),
                })),
                titlebar: Some(gpui::TitlebarOptions {
                    title: None,
                    appears_transparent: true,
                    traffic_light_position: None, // No traffic lights
                }),
                kind: WindowKind::Normal,
                is_resizable: true,
                window_decorations: Some(gpui::WindowDecorations::Client),
                window_min_size: Some(gpui::Size {
                    width: px(600.),
                    height: px(400.),
                }),
                ..Default::default()
            },
            |window, cx| {
                // Create a NEW terminal drawer for this window (independent terminal session)
                let terminal_drawer = cx.new(|cx| TerminalDrawer::new(window, cx));
                let terminal_window = cx.new(|cx| TerminalWindow::new(terminal_drawer, window, cx));

                cx.new(|cx| Root::new(terminal_window.into(), window, cx))
            },
        );
    }

    fn on_toggle_file_manager(
        &mut self,
        _: &ToggleFileManager,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_drawer(window, cx);
    }

    fn on_toggle_problems(
        &mut self,
        _: &ToggleProblems,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_problems(window, cx);
    }

    fn on_toggle_terminal(
        &mut self,
        _: &ToggleTerminal,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.toggle_terminal(window, cx);
    }

    fn on_navigate_to_diagnostic(
        &mut self,
        _drawer: &Entity<ProblemsDrawer>,
        event: &super::problems_drawer::NavigateToDiagnostic,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        println!(
            "📂 Navigating to diagnostic: {:?} at line {}, column {}",
            event.file_path, event.line, event.column
        );

        // Open the file in the script editor
        self.open_script_tab(event.file_path.clone(), window, cx);

        // Navigate to the specific line and column
        if let Some(script_editor) = &self.script_editor {
            script_editor.update(cx, |editor, cx| {
                editor.go_to_line(event.line, event.column, window, cx);
            });
        }
    }

    /// Public handler for file selected events from external windows (like file manager window)
    /// This is called directly from the file manager window
    pub fn handle_file_selected_from_external_window(
        &mut self,
        event: &FileSelected,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        eprintln!(
            "DEBUG: File selected from external window - path: {:?}, type: {:?}",
            event.path, event.file_type
        );

        match event.file_type {
            FileType::Class => {
                eprintln!("DEBUG: Opening blueprint tab from external");
                self.open_blueprint_tab(event.path.clone(), window, cx);
            }
            FileType::Script => {
                eprintln!("DEBUG: Opening script tab from external");
                self.open_script_tab(event.path.clone(), window, cx);
            }
            FileType::DawProject => {
                eprintln!("DEBUG: Opening DAW tab from external: {:?}", event.path);
                self.open_daw_tab(event.path.clone(), window, cx);
            }
            _ => {
                eprintln!("DEBUG: Unknown file type from external, ignoring");
            }
        }
    }

    /// Open a blueprint editor tab for the given class path
    fn open_blueprint_tab(
        &mut self,
        class_path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Check if a blueprint editor for this class is already open
        let already_open = self
            .blueprint_editors
            .iter()
            .enumerate()
            .find_map(|(ix, editor)| {
                editor
                    .read(cx)
                    .current_class_path
                    .as_ref()
                    .map(|p| p == &class_path)
                    .unwrap_or(false)
                    .then_some(ix)
            });

        if let Some(ix) = already_open {
            // Focus the correct tab by matching entity_id in TabPanel using the public getter
            if let Some(editor_entity) = self.blueprint_editors.get(ix) {
                let target_id = editor_entity.entity_id();
                self.center_tabs.update(cx, |tabs, cx| {
                    if let Some(tab_ix) = tabs.index_of_panel_by_entity_id(target_id) {
                        tabs.set_active_tab(tab_ix, window, cx);
                    }
                });
            }
            return;
        }

        self.next_tab_id += 1;

        // Create a new blueprint editor panel and set its class path and tab title
        let class_name = class_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Blueprint")
            .to_string();

        let blueprint_editor = cx.new(|cx| {
            let mut panel = BlueprintEditorPanel::new(window, cx);
            panel.current_class_path = Some(class_path.clone());
            panel.tab_title = Some(class_name.clone());
            panel
        });

        // Subscribe to blueprint editor events (OpenEngineLibraryRequest)
        cx.subscribe_in(&blueprint_editor, window, Self::on_blueprint_editor_event)
            .detach();

        // Load the blueprint from the class path
        let graph_save_path = class_path.join("graph_save.json");
        if graph_save_path.exists() {
            blueprint_editor.update(cx, |editor, cx| {
                if let Err(e) = editor.load_blueprint(graph_save_path.to_str().unwrap(), window, cx)
                {
                    eprintln!("Failed to load blueprint: {}", e);
                }
            });
        }

        // Add the tab (Entity<BlueprintEditorPanel> implements all required traits)
        self.center_tabs.update(cx, |tabs, cx| {
            tabs.add_panel(Arc::new(blueprint_editor.clone()), window, cx);
        });

        // Store the blueprint editor reference
        self.blueprint_editors.push(blueprint_editor);
    }

    /// Handle events from blueprint editor panels
    fn on_blueprint_editor_event(
        &mut self,
        _editor: &Entity<BlueprintEditorPanel>,
        event: &crate::ui::panels::blueprint_editor2::OpenEngineLibraryRequest,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Blueprint editor is requesting to open an engine library
        self.open_engine_library_tab(
            event.library_id.clone(),
            event.library_name.clone(),
            window,
            cx,
        );

        // If a specific macro was requested, open it in the library tab
        if let (Some(macro_id), Some(macro_name)) = (&event.macro_id, &event.macro_name) {
            // Find the blueprint editor we just opened/focused for this library
            let library_tab_title = format!("📚 {} Library", event.library_name);
            if let Some(editor) = self.blueprint_editors.iter().find(|e| {
                e.read(cx).tab_title.as_ref().map(|t| t == &library_tab_title).unwrap_or(false)
            }) {
                // Open the macro in that editor
                editor.update(cx, |ed, cx| {
                    ed.open_global_macro(macro_id.clone(), macro_name.clone(), cx);
                });
            }
        }
    }

    /// Open a blueprint editor tab for an engine library (virtual blueprint class)
    /// This opens the library as if it were a blueprint, allowing users to browse and edit its macros
    pub fn open_engine_library_tab(
        &mut self,
        library_id: String,
        library_name: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Check if a blueprint editor for this library is already open
        // We use tab_title to identify library tabs since they don't have a class_path
        let already_open = self
            .blueprint_editors
            .iter()
            .enumerate()
            .find_map(|(ix, editor)| {
                let title = editor.read(cx).tab_title.clone();
                title
                    .map(|t| t == format!("📚 {} Library", library_name))
                    .unwrap_or(false)
                    .then_some(ix)
            });

        if let Some(ix) = already_open {
            // Focus the correct tab
            if let Some(editor_entity) = self.blueprint_editors.get(ix) {
                let target_id = editor_entity.entity_id();
                self.center_tabs.update(cx, |tabs, cx| {
                    if let Some(tab_ix) = tabs.index_of_panel_by_entity_id(target_id) {
                        tabs.set_active_tab(tab_ix, window, cx);
                    }
                });
            }
            return;
        }

        self.next_tab_id += 1;

        // Create a new blueprint editor for the library
        let blueprint_editor = cx.new(|cx| {
            BlueprintEditorPanel::new_for_library(library_id.clone(), library_name.clone(), window, cx)
        });

        // Subscribe to blueprint editor events
        cx.subscribe_in(&blueprint_editor, window, Self::on_blueprint_editor_event)
            .detach();

        // Add the tab
        self.center_tabs.update(cx, |tabs, cx| {
            tabs.add_panel(Arc::new(blueprint_editor.clone()), window, cx);
        });

        // Store the blueprint editor reference
        self.blueprint_editors.push(blueprint_editor);

        println!("📚 Opened engine library '{}' in main tab", library_name);
    }

    /// Open or focus the script editor tab
    fn open_script_tab(&mut self, file_path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        // Check if script editor already exists
        if let Some(script_editor) = &self.script_editor {
            // Script editor already exists, open the file in it
            script_editor.update(cx, |editor, cx| {
                editor.open_file(file_path, window, cx);
            });
            return;
        }

        // Create new script editor tab
        let script_editor = cx.new(|cx| ScriptEditorPanel::new(window, cx));

        // Wire up rust-analyzer to the script editor
        let analyzer = self.rust_analyzer.clone();
        script_editor.update(cx, |editor, cx| {
            editor.set_rust_analyzer(analyzer, cx);
        });

        // Load project in file explorer if we have a project path
        if let Some(ref project_path) = self.project_path {
            script_editor.update(cx, |editor, cx| {
                editor.set_project_path(project_path.clone(), window, cx);
            });
        }

        // Note: ScriptEditor now handles LSP notifications internally via set_rust_analyzer
        // We only subscribe here for non-LSP events (RunScriptRequested, etc.)
        cx.subscribe_in(&script_editor, window, Self::on_text_editor_event)
            .detach();

        // Open the specific file
        script_editor.update(cx, |editor, cx| {
            editor.open_file(file_path, window, cx);
        });

        // Add the tab to the tab panel
        self.center_tabs.update(cx, |tabs, cx| {
            tabs.add_panel(Arc::new(script_editor.clone()), window, cx);
        });

        // Store the script editor reference
        self.script_editor = Some(script_editor);
    }

    /// Open a DAW editor tab for the given project path
    fn open_daw_tab(&mut self, project_path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        eprintln!("DEBUG: open_daw_tab called with path: {:?}", project_path);

        // For now, just check if we have any DAW editors open
        // TODO: Improve tracking to match by project path
        if !self.daw_editors.is_empty() {
            eprintln!("DEBUG: DAW editor already exists, focusing first one");
            // Just focus the first DAW tab for now
            // In a more complete implementation, we'd track which tab corresponds to which editor
            return;
        }

        eprintln!("DEBUG: Creating new DAW editor panel");
        self.next_tab_id += 1;

        // Create new DAW editor tab
        let daw_editor =
            cx.new(|cx| DawEditorPanel::new_with_project(project_path.clone(), window, cx));

        // Extract project name for tab label
        let project_name = project_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("DAW")
            .to_string();

        eprintln!("DEBUG: Adding DAW editor to tab panel");
        // Add the tab to the tab panel
        self.center_tabs.update(cx, |tabs, cx| {
            tabs.add_panel(Arc::new(daw_editor.clone()), window, cx);
        });

        eprintln!("DEBUG: Storing DAW editor reference");
        // Store the DAW editor reference
        self.daw_editors.push(daw_editor);

        eprintln!("DEBUG: DAW tab opened successfully");
    }

    fn on_text_editor_event(
        &mut self,
        _editor: &Entity<ScriptEditorPanel>,
        event: &super::panels::TextEditorEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        use super::panels::TextEditorEvent;

        match event {
            // LSP notifications are now handled by ScriptEditor internally
            // We only handle app-level events here
            TextEditorEvent::FileOpened { .. } => {
                // No-op: ScriptEditor handles didOpen
            }
            TextEditorEvent::FileSaved { .. } => {
                // No-op: ScriptEditor handles didSave
            }
            TextEditorEvent::FileClosed { .. } => {
                // No-op: ScriptEditor handles didClose
            }
            _ => {
                // Handle other events (RunScriptRequested, etc.) if needed
            }
        }
    }

    /// Open a path in the appropriate editor
    pub fn open_path(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if path.is_dir() {
            // Check if it's a blueprint class (contains graph_save.json)
            if path.join("graph_save.json").exists() {
                self.open_blueprint_tab(path, window, cx);
            }
        } else if let Some(extension) = path.extension() {
            match extension.to_str() {
                Some("pdaw") => {
                    self.open_daw_tab(path, window, cx);
                }
                Some("rs") | Some("js") | Some("ts") | Some("py") | Some("lua") => {
                    self.open_script_tab(path, window, cx);
                }
                _ => {}
            }
        }
    }
}

impl Render for PulsarApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Update rust-analyzer progress if indexing
        self.rust_analyzer.update(cx, |analyzer, cx| {
            analyzer.update_progress_from_thread(cx);
        });

        // Show entry screen if no project is loaded
        if let Some(screen) = &self.entry_screen {
            return screen.clone().into_any_element();
        }

        let drawer_open = self.drawer_open;

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .on_action(cx.listener(Self::on_toggle_file_manager))
            .on_action(cx.listener(Self::on_toggle_problems))
            .on_action(cx.listener(Self::on_toggle_terminal))
            .child(
                // Menu bar
                {
                    let title_bar = cx.new(|cx| AppTitleBar::new("Pulsar Engine", window, cx));
                    title_bar.clone()
                },
            )
            .child(
                // Main dock area
                div()
                    .flex_1()
                    .relative()
                    .child(self.dock_area.clone())
                    .when(drawer_open, |this| {
                        this.child(
                            // Overlay background for file manager
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .size_full()
                                .bg(Hsla::black().opacity(0.3))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|app, _, window, cx| {
                                        app.drawer_open = false;
                                        cx.notify();
                                    }),
                                ),
                        )
                        .child(
                            // File manager drawer at bottom
                            div()
                                .absolute()
                                .bottom_0()
                                .left_0()
                                .right_0()
                                .h(px(300.))
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .child(self.file_manager_drawer.clone())
                                .with_animation(
                                    "slide-up",
                                    Animation::new(Duration::from_secs_f64(0.2)),
                                    |this, delta| this.bottom(px(-300.) + delta * px(300.)),
                                ),
                        )
                    }),
            )
            .child(
                // Footer with rust analyzer status and controls
                self.render_footer(drawer_open, cx),
            )
            .into_any_element()
    }
}

impl PulsarApp {
    fn render_footer(&self, drawer_open: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let analyzer = self.rust_analyzer.read(cx);
        let status = analyzer.status();
        let is_running = analyzer.is_running();

        let error_count = self
            .problems_drawer
            .read(cx)
            .count_by_severity(crate::ui::problems_drawer::DiagnosticSeverity::Error);
        let warning_count = self
            .problems_drawer
            .read(cx)
            .count_by_severity(crate::ui::problems_drawer::DiagnosticSeverity::Warning);

        // STUDIO-QUALITY STATUS BAR
        h_flex()
            .w_full()
            .h(px(34.))
            .px_3()
            .items_center()
            .gap_4()
            .bg(cx.theme().secondary)
            .border_t_2()
            .border_color(cx.theme().border)
            .child(
                // LEFT SECTION - Actions
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        // Project Files Toggle
                        Button::new("toggle-drawer")
                            .ghost()
                            .icon(IconName::Folder)
                            .label("Files")
                            .when(drawer_open, |btn| btn.primary())
                            .tooltip("Toggle Project Files (Ctrl+B)")
                            .on_click(cx.listener(|app, _, window, cx| {
                                app.toggle_drawer(window, cx);
                            })),
                    )
                    .child(
                        // Problems Window Button - with smart styling
                        Button::new("open-problems")
                            .ghost()
                            .when(error_count > 0, |btn| {
                                btn.with_variant(gpui_component::button::ButtonVariant::Danger)
                            })
                            .when(error_count == 0 && warning_count > 0, |btn| {
                                btn.with_variant(gpui_component::button::ButtonVariant::Warning)
                            })
                            .icon(if error_count > 0 {
                                IconName::Close
                            } else if warning_count > 0 {
                                IconName::TriangleAlert
                            } else {
                                IconName::CheckCircle
                            })
                            .label(if error_count + warning_count > 0 {
                                format!(
                                    "{} {}",
                                    error_count + warning_count,
                                    if error_count > 0 {
                                        "Problems"
                                    } else {
                                        "Warnings"
                                    }
                                )
                            } else {
                                "No Problems".to_string()
                            })
                            .tooltip("Open Problems Window")
                            .on_click(cx.listener(|app, _, window, cx| {
                                app.toggle_problems(window, cx);
                            })),
                    )
                    .child(
                        // Terminal Window Button
                        Button::new("open-terminal")
                            .ghost()
                            .icon(IconName::Terminal)
                            .label("Terminal")
                            .tooltip("Open Terminal Window")
                            .on_click(cx.listener(|app, _, window, cx| {
                                app.toggle_terminal(window, cx);
                            })),
                    ),
            )
            .child(
                // CENTER SECTION - Rust Analyzer Status
                h_flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .gap_3()
                    .child(
                        // Professional status indicator
                        h_flex()
                            .items_center()
                            .gap_2()
                            .px_3()
                            .py_1p5()
                            .rounded(px(6.0))
                            .bg(cx.theme().background.opacity(0.5))
                            .border_1()
                            .border_color(cx.theme().border.opacity(0.5))
                            .child(
                                // Animated status dot
                                div()
                                    .w(px(10.))
                                    .h(px(10.))
                                    .rounded_full()
                                    .bg(match status {
                                        AnalyzerStatus::Ready => cx.theme().success,
                                        AnalyzerStatus::Indexing { .. }
                                        | AnalyzerStatus::Starting => cx.theme().warning,
                                        AnalyzerStatus::Error(_) => cx.theme().danger,
                                        _ => cx.theme().muted_foreground,
                                    })
                                    .shadow_sm(),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child("rust-analyzer"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("·"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(match status {
                                        AnalyzerStatus::Ready => cx.theme().success,
                                        AnalyzerStatus::Indexing { .. } => cx.theme().warning,
                                        AnalyzerStatus::Error(_) => cx.theme().danger,
                                        _ => cx.theme().muted_foreground,
                                    })
                                    .child(self.analyzer_status_text.clone()),
                            ),
                    )
                    .child(
                        // Analyzer controls
                        h_flex()
                            .gap_1()
                            .items_center()
                            .when(is_running, |this| {
                                this.child(
                                    Button::new("stop-analyzer")
                                        .ghost()
                                        .icon(IconName::Close)
                                        .tooltip("Stop rust-analyzer")
                                        .xsmall()
                                        .on_click(cx.listener(|app, _, window, cx| {
                                            app.rust_analyzer.update(cx, |analyzer, cx| {
                                                analyzer.stop(window, cx);
                                            });
                                        })),
                                )
                            })
                            .child(
                                Button::new("restart-analyzer")
                                    .ghost()
                                    .icon(IconName::Undo)
                                    .tooltip(if is_running {
                                        "Restart rust-analyzer"
                                    } else {
                                        "Start rust-analyzer"
                                    })
                                    .xsmall()
                                    .on_click(cx.listener(move |app, _, window, cx| {
                                        if let Some(project) = app.project_path.clone() {
                                            app.rust_analyzer.update(cx, |analyzer, cx| {
                                                if is_running {
                                                    analyzer.restart(window, cx);
                                                } else {
                                                    analyzer.start(project, window, cx);
                                                }
                                            });
                                        }
                                    })),
                            ),
                    ),
            )
            .child(
                // RIGHT SECTION - Project Path
                h_flex()
                    .items_center()
                    .px_3()
                    .py_1p5()
                    .rounded(px(6.0))
                    .bg(cx.theme().background.opacity(0.3))
                    .child(
                        div()
                            .text_xs()
                            .font_family("JetBrainsMono-Regular")
                            .text_color(cx.theme().muted_foreground)
                            .children(
                                self.project_path
                                    .as_ref()
                                    .and_then(|path| path.file_name())
                                    .map(|name| name.to_string_lossy().to_string())
                                    .or_else(|| {
                                        self.project_path
                                            .as_ref()
                                            .map(|path| path.display().to_string())
                                    }),
                            ),
                    ),
            )
    }
}

pub struct EditorPanel {
    editor_type: EditorType,
    focus_handle: FocusHandle,
}

impl EditorPanel {
    pub fn new(editor_type: EditorType, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            editor_type,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(editor_type: EditorType, window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(editor_type, window, cx))
    }
}

impl Panel for EditorPanel {
    fn panel_name(&self) -> &'static str {
        self.editor_type.display_name()
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        div()
            .child(self.editor_type.display_name())
            .into_any_element()
    }

    fn dump(&self, _cx: &App) -> gpui_component::dock::PanelState {
        gpui_component::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}

impl Focusable for EditorPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for EditorPanel {}

impl Render for EditorPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_editor_content(cx)
    }
}

impl EditorPanel {
    fn render_editor_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                // Header
                h_flex()
                    .w_full()
                    .p_4()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .justify_between()
                    .items_center()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_lg()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child(self.editor_type.display_name()),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(self.editor_type.description()),
                            ),
                    ),
            )
            .child(
                // Content
                div()
                    .flex_1()
                    .p_4()
                    .overflow_hidden()
                    .child(self.render_specific_editor(cx)),
            )
    }

    fn render_specific_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        match self.editor_type {
            EditorType::Level => self.render_level_editor(cx).into_any_element(),
            EditorType::Script => self.render_script_editor(cx).into_any_element(),
            EditorType::Blueprint => self.render_blueprint_editor(cx).into_any_element(),
            _ => self.render_placeholder_editor(cx).into_any_element(),
        }
    }

    fn render_level_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .size_full()
            .gap_4()
            .child(
                // Left panel - Scene Hierarchy
                div()
                    .w_64()
                    .h_full()
                    .bg(cx.theme().sidebar)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .p_3()
                    .child("Scene Hierarchy"),
            )
            .child(
                // Center - 3D Viewport
                div()
                    .flex_1()
                    .h_full()
                    .bg(cx.theme().muted.opacity(0.2))
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        v_flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .size_16()
                                    .bg(cx.theme().primary.opacity(0.2))
                                    .rounded_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child("🎮"),
                            )
                            .child(
                                div()
                                    .text_lg()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("3D Viewport"),
                            ),
                    ),
            )
            .child(
                // Right panel - Properties
                div()
                    .w_64()
                    .h_full()
                    .bg(cx.theme().sidebar)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .p_3()
                    .child("Properties"),
            )
    }

    fn render_script_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .size_full()
            .gap_4()
            .child(
                // Left panel - File Explorer
                div()
                    .w_64()
                    .h_full()
                    .bg(cx.theme().sidebar)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .p_3()
                    .child("File Explorer"),
            )
            .child(
                // Center - Code Editor
                div()
                    .flex_1()
                    .h_full()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .p_4()
                    .child("Code Editor Area"),
            )
            .child(
                // Right panel - Output/Terminal
                div()
                    .w_64()
                    .h_full()
                    .bg(cx.theme().sidebar)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .p_3()
                    .child("Terminal"),
            )
    }

    fn render_blueprint_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .size_full()
            .gap_4()
            .child(
                div()
                    .w_64()
                    .h_full()
                    .bg(cx.theme().sidebar)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .p_3()
                    .child("Node Library"),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .bg(cx.theme().muted.opacity(0.2))
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child("Visual Node Graph"),
            )
    }

    fn render_placeholder_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div().flex_1().flex().items_center().justify_center().child(
            v_flex()
                .items_center()
                .gap_4()
                .child(
                    div()
                        .text_lg()
                        .font_semibold()
                        .text_color(cx.theme().foreground)
                        .child(format!("{} Editor", self.editor_type.display_name())),
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("Coming soon..."),
                ),
        )
    }
}


