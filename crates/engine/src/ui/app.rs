use gpui::{prelude::*, Animation, AnimationExt as _, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dock::{DockArea, DockItem, Panel, PanelEvent, TabPanel},
    h_flex, v_flex, ActiveTheme as _, IconName, StyledExt, Sizable as _,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;
use std::{sync::Arc, time::Duration};

use super::{
    editors::EditorType,
    entry_screen::EntryScreen,
    project_selector::ProjectSelected,
    file_manager_drawer::{FileManagerDrawer, FileSelected, FileType},
    menu::AppTitleBar,
    panels::{BlueprintEditorPanel, LevelEditorPanel, ScriptEditorPanel},
    problems_drawer::ProblemsDrawer,
    rust_analyzer_manager::{RustAnalyzerManager, AnalyzerEvent, AnalyzerStatus},
};

// Action to toggle the file manager drawer
#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = pulsar_app)]
pub struct ToggleFileManager;

// Action to toggle the problems drawer
#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = pulsar_app)]
pub struct ToggleProblems;

pub struct PulsarApp {
    dock_area: Entity<DockArea>,
    project_path: Option<PathBuf>,
    entry_screen: Option<Entity<EntryScreen>>,
    file_manager_drawer: Entity<FileManagerDrawer>,
    drawer_open: bool,
    problems_drawer: Entity<ProblemsDrawer>,
    problems_drawer_open: bool,
    // Tab management
    center_tabs: Entity<TabPanel>,
    script_editor: Option<Entity<ScriptEditorPanel>>,
    blueprint_editors: Vec<Entity<BlueprintEditorPanel>>,
    next_tab_id: usize,
    // Rust Analyzer
    rust_analyzer: Entity<RustAnalyzerManager>,
    analyzer_status_text: String,
}

impl PulsarApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::new_internal(None, window, cx)
    }

    pub fn new_with_project(
        project_path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        eprintln!("DEBUG: PulsarApp::new_with_project called with path: {:?}", project_path);
        Self::new_internal(Some(project_path), window, cx)
    }

    /// Get the global rust analyzer manager
    pub fn rust_analyzer(&self) -> &Entity<RustAnalyzerManager> {
        &self.rust_analyzer
    }

    /// Get the current workspace root
    pub fn workspace_root(&self) -> Option<&PathBuf> {
        self.project_path.as_ref()
    }

    fn new_internal(
        project_path: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // TODO: Load last opened tabs if editor settings allow
        // TODO: Dynamic layout allowing tabs to be split screened
        let dock_area = cx.new(|cx| DockArea::new("main-dock", Some(1), window, cx));
        let weak_dock = dock_area.downgrade();

        // Only create level editor tab by default
        let level_editor = cx.new(|cx| LevelEditorPanel::new(window, cx));

        // Set up the dock area with the level editor tab
        let center_dock_item = DockItem::tabs(
            vec![Arc::new(level_editor.clone())],
            Some(0),
            &weak_dock,
            window,
            cx,
        );

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

        // Create problems drawer
        let problems_drawer = cx.new(|cx| ProblemsDrawer::new(window, cx));
        cx.subscribe_in(&problems_drawer, window, Self::on_navigate_to_diagnostic).detach();

        // Create rust analyzer manager
        let rust_analyzer = cx.new(|cx| RustAnalyzerManager::new(window, cx));
        cx.subscribe_in(&rust_analyzer, window, Self::on_analyzer_event).detach();

        // Start rust analyzer if we have a project
        if let Some(ref project) = project_path {
            rust_analyzer.update(cx, |analyzer, cx| {
                analyzer.start(project.clone(), window, cx);
            });
        }

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
            problems_drawer_open: false,
            center_tabs,
            script_editor,
            blueprint_editors,
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
                self.analyzer_status_text = format!("Indexing: {} ({:.0}%)", message, progress * 100.0);
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
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
        match event {
            PanelEvent::TabClosed(entity_id) => {
                self.blueprint_editors
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
        match event.file_type {
            FileType::Class => {
                self.open_blueprint_tab(event.path.clone(), window, cx);
            }
            FileType::Script => {
                self.open_script_tab(event.path.clone(), window, cx);
            }
            _ => {}
        }

        // Close the drawer after opening a file
        self.drawer_open = false;
        cx.notify();
    }

    fn toggle_drawer(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.drawer_open = !self.drawer_open;
        cx.notify();
    }

    fn toggle_problems(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.problems_drawer_open = !self.problems_drawer_open;
        cx.notify();
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

    fn on_navigate_to_diagnostic(
        &mut self,
        _drawer: &Entity<ProblemsDrawer>,
        event: &super::problems_drawer::NavigateToDiagnostic,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        println!("📂 Navigating to diagnostic: {:?} at line {}, column {}", event.file_path, event.line, event.column);
        
        // Open the file in the script editor
        self.open_script_tab(event.file_path.clone(), window, cx);
        
        // Navigate to the specific line and column
        if let Some(script_editor) = &self.script_editor {
            script_editor.update(cx, |editor, cx| {
                editor.go_to_line(event.line, event.column, window, cx);
            });
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

        // Note: ScriptEditor now handles LSP notifications internally via set_rust_analyzer
        // We only subscribe here for non-LSP events (RunScriptRequested, etc.)
        cx.subscribe_in(&script_editor, window, Self::on_text_editor_event).detach();

        // Open the specific file
        script_editor.update(cx, |editor, cx| {
            editor.open_file(file_path, window, cx);
        });

        // Add the tab
        self.center_tabs.update(cx, |tabs, cx| {
            tabs.add_panel(Arc::new(script_editor.clone()), window, cx);
        });

        // Store the script editor reference
        self.script_editor = Some(script_editor);
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
                // TODO: Create a separate asset editor for assets like .png, .jpg, .wav, .mp3, etc.
                // TODO: Create a material editor for .material files
                // TODO: Create a level editor open level feature for .usd files
                // TODO: Make a new file that has a helper fn for all these extensions
                // TODO: There is probably a crate that has all these extensions defined
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
        let problems_open = self.problems_drawer_open;

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .on_action(cx.listener(Self::on_toggle_file_manager))
            .on_action(cx.listener(Self::on_toggle_problems))
            .child(
                // Menu bar
                {
                    let title_bar = cx.new(|cx| AppTitleBar::new("Pulsar Engine", window, cx));
                    title_bar.clone()
                },
            )
            .child(
                // Main dock area with overlays
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
                    })
                    .when(problems_open, |this| {
                        this.child(
                            // Overlay background for problems
                            div()
                                .absolute()
                                .top_0()
                                .left_0()
                                .size_full()
                                .bg(Hsla::black().opacity(0.3))
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(|app, _, _window, cx| {
                                        app.problems_drawer_open = false;
                                        cx.notify();
                                    }),
                                ),
                        )
                        .child(
                            // Problems drawer at bottom
                            div()
                                .absolute()
                                .bottom_0()
                                .left_0()
                                .right_0()
                                .h(px(300.))
                                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                .child(self.problems_drawer.clone())
                                .with_animation(
                                    "slide-up-problems",
                                    Animation::new(Duration::from_secs_f64(0.2)),
                                    |this, delta| this.bottom(px(-300.) + delta * px(300.)),
                                ),
                        )
                    }),
            )
            .child(
                // Footer with rust analyzer status and controls
                self.render_footer(drawer_open, problems_open, cx)
            )
            .into_any_element()
    }
}

impl PulsarApp {
    fn render_footer(&self, drawer_open: bool, problems_open: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let analyzer = self.rust_analyzer.read(cx);
        let status = analyzer.status();
        let is_running = analyzer.is_running();
        
        let error_count = self.problems_drawer.read(cx).count_by_severity(
            crate::ui::problems_drawer::DiagnosticSeverity::Error
        );
        let warning_count = self.problems_drawer.read(cx).count_by_severity(
            crate::ui::problems_drawer::DiagnosticSeverity::Warning
        );
        
        h_flex()
            .w_full()
            .h(px(32.))
            .px_2()
            .items_center()
            .gap_3()
            .bg(cx.theme().sidebar)
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                // Left side - drawer toggles
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("toggle-drawer")
                            .ghost()
                            .icon(if drawer_open {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronUp
                            })
                            .label("Project Files")
                            .on_click(cx.listener(|app, _, window, cx| {
                                app.toggle_drawer(window, cx);
                            })),
                    )
                    .child(
                        Button::new("toggle-problems")
                            .ghost()
                            .when(error_count > 0, |btn| {
                                btn.with_variant(gpui_component::button::ButtonVariant::Danger)
                            })
                            .when(error_count == 0 && warning_count > 0, |btn| {
                                btn.with_variant(gpui_component::button::ButtonVariant::Primary)
                            })
                            .icon(if error_count > 0 {
                                IconName::Close
                            } else if warning_count > 0 {
                                IconName::Anytype_dark
                            } else {
                                IconName::Info
                            })
                            .label(format!(
                                "Problems ({})",
                                error_count + warning_count
                            ))
                            .on_click(cx.listener(|app, _, window, cx| {
                                app.toggle_problems(window, cx);
                            })),
                    )
            )
            .child(
                // Center - Rust Analyzer Section
                h_flex()
                    .flex_1()
                    .items_center()
                    .gap_2()
                    .child(
                        // Analyzer status indicator
                        div()
                            .flex()
                            .items_center()
                            .gap_1p5()
                            .child(
                                // Status icon/spinner
                                div()
                                    .w(px(8.))
                                    .h(px(8.))
                                    .rounded_full()
                                    .bg(match status {
                                        AnalyzerStatus::Ready => Hsla {
                                            h: 120.0,
                                            s: 1.0,
                                            l: 0.5,
                                            a: 1.0,
                                        },
                                        AnalyzerStatus::Indexing { .. } | AnalyzerStatus::Starting => Hsla {
                                            h: 60.0,
                                            s: 1.0,
                                            l: 0.5,
                                            a: 1.0,
                                        },
                                        AnalyzerStatus::Error(_) => Hsla {
                                            h: 0.0,
                                            s: 1.0,
                                            l: 0.5,
                                            a: 1.0,
                                        },
                                        _ => cx.theme().muted_foreground,
                                    })
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().foreground)
                                    .child(format!("rust-analyzer: {}", self.analyzer_status_text))
                            )
                    )
                    .when(is_running, |this| {
                        this.child(
                            // Stop button
                            Button::new("stop-analyzer")
                                .ghost()
                                .icon(IconName::Close)
                                .tooltip("Stop rust-analyzer")
                                .on_click(cx.listener(|app, _, window, cx| {
                                    app.rust_analyzer.update(cx, |analyzer, cx| {
                                        analyzer.stop(window, cx);
                                    });
                                }))
                        )
                    })
                    .when(!is_running, |this| {
                        this.child(
                            // Start/Restart button
                            Button::new("start-analyzer")
                                .ghost()
                                .icon(IconName::ArrowRight)
                                .tooltip("Start rust-analyzer")
                                .on_click(cx.listener(|app, _, window, cx| {
                                    if let Some(project) = app.project_path.clone() {
                                        app.rust_analyzer.update(cx, |analyzer, cx| {
                                            analyzer.start(project, window, cx);
                                        });
                                    }
                                }))
                        )
                    })
                    .child(
                        // Restart button (always visible)
                        Button::new("restart-analyzer")
                            .ghost()
                            .icon(IconName::Undo)
                            .tooltip("Restart rust-analyzer")
                            .when(!is_running, |btn| btn.opacity(0.5))
                            .on_click(cx.listener(move |app, _, window, cx| {
                                if is_running {
                                    app.rust_analyzer.update(cx, |analyzer, cx| {
                                        analyzer.restart(window, cx);
                                    });
                                }
                            }))
                    )
            )
            .child(
                // Right side - project path
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .children(
                        self.project_path
                            .as_ref()
                            .map(|path| path.display().to_string()),
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
