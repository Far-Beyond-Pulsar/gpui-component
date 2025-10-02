use gpui::*;
use gpui_component::{
    dock::{DockArea, DockItem, Panel, PanelEvent},
    button::{Button, ButtonVariant, ButtonVariants as _},
    v_flex, h_flex, ActiveTheme as _, Icon, IconName, Placement, StyledExt,
};
use std::sync::Arc;
use std::path::PathBuf;
use serde::Deserialize;
use schemars::JsonSchema;

use super::{
    editors::EditorType,
    menu::AppTitleBar,
    panels::{
        LevelEditorPanel, ScriptEditorPanel, BlueprintEditorPanel,
        MaterialEditorPanel,
    },
    entry_screen::{EntryScreen, ProjectSelected},
    file_manager_drawer::{FileManagerDrawer, FileSelected, FileType},
};

// Action to toggle the file manager drawer
#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = pulsar_app)]
pub struct ToggleFileManager;

pub struct PulsarApp {
    dock_area: Entity<DockArea>,
    project_path: Option<PathBuf>,
    entry_screen: Option<Entity<EntryScreen>>,
    file_manager_drawer: Entity<FileManagerDrawer>,
    drawer_open: bool,
    blueprint_editor: Entity<BlueprintEditorPanel>,
}

impl PulsarApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let dock_area = cx.new(|cx| DockArea::new("main-dock", Some(1), window, cx));

        // Create the initial editor panels using modular components
        let level_editor = cx.new(|cx| LevelEditorPanel::new(window, cx));
        let script_editor = cx.new(|cx| ScriptEditorPanel::new(window, cx));
        let blueprint_editor = cx.new(|cx| BlueprintEditorPanel::new(window, cx));
        let material_editor = cx.new(|cx| MaterialEditorPanel::new(window, cx));

        // Set up the center area with tabs for all editors
        let weak_dock = dock_area.downgrade();
        let center_tabs = DockItem::tabs(
            vec![
                Arc::new(level_editor.clone()),
                Arc::new(script_editor),
                Arc::new(blueprint_editor.clone()),
                Arc::new(material_editor),
            ],
            Some(0),
            &weak_dock,
            window,
            cx,
        );

        dock_area.update(cx, |dock, cx| {
            dock.set_center(center_tabs, window, cx);
        });

        // Create entry screen
        let entry_screen = cx.new(|cx| EntryScreen::new(window, cx));
        cx.subscribe(&entry_screen, Self::on_project_selected).detach();

        // Create file manager drawer
        let file_manager_drawer = cx.new(|cx| FileManagerDrawer::new(None, window, cx));
        cx.subscribe(&file_manager_drawer, Self::on_file_selected).detach();

        Self {
            dock_area,
            project_path: None,
            entry_screen: Some(entry_screen),
            file_manager_drawer,
            drawer_open: false,
            blueprint_editor,
        }
    }

    fn on_project_selected(
        &mut self,
        _selector: Entity<EntryScreen>,
        event: &ProjectSelected,
        cx: &mut Context<Self>,
    ) {
        self.project_path = Some(event.path.clone());
        self.entry_screen = None; // Hide entry screen once project is loaded

        // Update file manager with project path
        self.file_manager_drawer.update(cx, |drawer, cx| {
            drawer.set_project_path(event.path.clone(), cx);
        });

        cx.notify();
    }

    fn on_file_selected(
        &mut self,
        _drawer: Entity<FileManagerDrawer>,
        event: &FileSelected,
        cx: &mut Context<Self>,
    ) {
        match event.file_type {
            FileType::Class => {
                // Load the class's graph_save.json into the blueprint editor
                let graph_save_path = event.path.join("graph_save.json");
                if graph_save_path.exists() {
                    self.blueprint_editor.update(cx, |editor, cx| {
                        if let Err(e) = editor.load_blueprint(graph_save_path.to_str().unwrap(), cx) {
                            eprintln!("Failed to load blueprint: {}", e);
                        }
                    });
                    // Close the drawer after opening a file
                    self.drawer_open = false;
                    cx.notify();
                }
            }
            _ => {}
        }
    }

    fn toggle_drawer(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.drawer_open = !self.drawer_open;
        cx.notify();
    }

    fn on_toggle_file_manager(&mut self, _action: &ToggleFileManager, window: &mut Window, cx: &mut Context<Self>) {
        self.toggle_drawer(window, cx);
    }
}

impl Render for PulsarApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Show entry screen if no project is loaded
        if let Some(screen) = &self.entry_screen {
            return screen.clone().into_any_element();
        }

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .on_action(cx.listener(Self::on_toggle_file_manager))
            .child(
                // Menu bar
                {
                    let title_bar = cx.new(|cx| AppTitleBar::new("Pulsar Engine", window, cx));
                    title_bar.clone()
                }
            )
            .child(
                // Main dock area
                div()
                    .flex_1()
                    .relative()
                    .child(self.dock_area.clone())
            )
            .children(if self.drawer_open {
                Some(
                    div()
                        .w_full()
                        .h(px(300.))
                        .child(self.file_manager_drawer.clone())
                )
            } else {
                None
            })
            .child(
                // Footer with drawer toggle
                h_flex()
                    .w_full()
                    .h(px(32.))
                    .px_2()
                    .items_center()
                    .justify_between()
                    .bg(cx.theme().sidebar)
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        // Left side - drawer toggle button
                        Button::new("toggle-drawer")
                            .ghost()
                            .icon(if self.drawer_open { IconName::ChevronDown } else { IconName::ChevronUp })
                            .label("Project Files")
                            .on_click(cx.listener(|app, _, window, cx| {
                                app.toggle_drawer(window, cx);
                            }))
                    )
                    .child(
                        // Right side - project path
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .children(self.project_path.as_ref().map(|path| {
                                path.display().to_string()
                            }))
                    )
            )
            .into_any_element()
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

    pub fn view(
        editor_type: EditorType,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| Self::new(editor_type, window, cx))
    }
}

impl Panel for EditorPanel {
    fn panel_name(&self) -> &'static str {
        self.editor_type.display_name()
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        div().child(self.editor_type.display_name()).into_any_element()
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
                                    .child(self.editor_type.display_name())
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(self.editor_type.description())
                            )
                    )
            )
            .child(
                // Content
                div()
                    .flex_1()
                    .p_4()
                    .overflow_hidden()
                    .child(self.render_specific_editor(cx))
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
                    .child("Scene Hierarchy")
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
                                    .child("🎮")
                            )
                            .child(
                                div()
                                    .text_lg()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("3D Viewport")
                            )
                    )
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
                    .child("Properties")
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
                    .child("File Explorer")
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
                    .child("Code Editor Area")
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
                    .child("Terminal")
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
                    .child("Node Library")
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
                    .child("Visual Node Graph")
            )
    }

    fn render_placeholder_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .child(
                v_flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .text_lg()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child(format!("{} Editor", self.editor_type.display_name()))
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Coming soon...")
                    )
            )
    }
}