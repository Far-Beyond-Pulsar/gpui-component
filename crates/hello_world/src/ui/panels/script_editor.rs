use std::path::PathBuf;
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dock::{Panel, PanelEvent},
    input::{InputState, TextInput},
    resizable::{h_resizable, v_resizable, resizable_panel, ResizableState},
    tab::{Tab, TabBar},
    h_flex, v_flex,
    ActiveTheme as _, StyledExt, Sizable as _,
    IconName,
};
use gpui::prelude::FluentBuilder;

use crate::ui::shared::{Toolbar, ToolbarButton, StatusBar};

pub struct ScriptEditorPanel {
    focus_handle: FocusHandle,
    open_files: Vec<(String, Entity<InputState>)>,
    current_file_index: Option<usize>,
    project_root: Option<PathBuf>,
    horizontal_resizable_state: Entity<ResizableState>,
    vertical_resizable_state: Entity<ResizableState>,
}

impl ScriptEditorPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let horizontal_resizable_state = ResizableState::new(cx);
        let vertical_resizable_state = ResizableState::new(cx);

        // Start with empty files - will be populated when opening real files
        let mut panel = Self {
            focus_handle: cx.focus_handle(),
            open_files: Vec::new(),
            current_file_index: None,
            project_root: None,
            horizontal_resizable_state,
            vertical_resizable_state,
        };

        // Open some sample files for demonstration
        panel.open_sample_files(window, cx);

        panel
    }

    fn open_sample_files(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Sample Rust content for demo purposes
        let main_rs_content = r#"use std::collections::HashMap;

fn main() {
    println!("Hello, Pulsar Engine!");

    let mut game_state = GameState::new();
    game_state.run();
}

struct GameState {
    entities: HashMap<u32, Entity>,
    next_entity_id: u32,
}

impl GameState {
    fn new() -> Self {
        Self {
            entities: HashMap::new(),
            next_entity_id: 0,
        }
    }

    fn run(&mut self) {
        // Game loop implementation
        loop {
            self.update();
            self.render();
        }
    }

    fn update(&mut self) {
        // Update game logic
    }

    fn render(&self) {
        // Render frame
    }
}"#;

        let game_logic_content = r#"use crate::Entity;

pub struct GameLogic {
    pub time: f32,
    pub delta_time: f32,
}

impl GameLogic {
    pub fn new() -> Self {
        Self {
            time: 0.0,
            delta_time: 0.0,
        }
    }

    pub fn update(&mut self, entities: &mut [Entity]) {
        for entity in entities.iter_mut() {
            entity.update(self.delta_time);
        }
    }
}"#;

        // Create input states for the files
        let main_rs_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("rust")
                .line_number(true)
                .soft_wrap(false)
                .default_value(main_rs_content)
        });

        let game_logic_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor("rust")
                .line_number(true)
                .soft_wrap(false)
                .default_value(game_logic_content)
        });

        self.open_files = vec![
            ("main.rs".to_string(), main_rs_state),
            ("game_logic.rs".to_string(), game_logic_state),
        ];
        self.current_file_index = Some(0);
    }

    fn close_file(&mut self, index: usize, _window: &mut Window, cx: &mut Context<Self>) {
        if index < self.open_files.len() {
            self.open_files.remove(index);

            // Adjust current file index
            if let Some(current) = self.current_file_index {
                if current == index {
                    // Closed the current file
                    if self.open_files.is_empty() {
                        self.current_file_index = None;
                    } else if index == self.open_files.len() {
                        // Closed the last file, select the previous one
                        self.current_file_index = Some(index.saturating_sub(1));
                    } else {
                        // Keep the same index (which now points to the next file)
                        self.current_file_index = Some(index);
                    }
                } else if current > index {
                    // Closed a file before the current one
                    self.current_file_index = Some(current - 1);
                }
            }

            cx.notify();
        }
    }

    fn set_active_file(&mut self, index: usize, _window: &mut Window, cx: &mut Context<Self>) {
        if index < self.open_files.len() {
            self.current_file_index = Some(index);
            cx.notify();
        }
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Toolbar::new()
            .add_button(
                ToolbarButton::new(IconName::FolderOpen, "Open")
                    .tooltip("Open Folder (Ctrl+O)")
            )
            .add_button(
                ToolbarButton::new(IconName::Plus, "New")
                    .tooltip("New File (Ctrl+N)")
            )
            .add_button(
                ToolbarButton::new(IconName::Check, "Save")
                    .tooltip("Save File (Ctrl+S)")
            )
            .add_button(
                ToolbarButton::new(IconName::Search, "Find")
                    .tooltip("Find in Files (Ctrl+Shift+F)")
            )
            .add_button(
                ToolbarButton::new(IconName::CircleCheck, "Run")
                    .tooltip("Run Script (F5)")
            )
            .add_button(
                ToolbarButton::new(IconName::CircleX, "Debug")
                    .tooltip("Debug Script (F9)")
            )
            .render(cx)
    }

    fn render_file_explorer(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                            .child("Explorer")
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("new_file")
                                    .icon(IconName::Plus)
                                    .tooltip("New File")
                                    .ghost()
                                    .xsmall()
                            )
                            .child(
                                Button::new("new_folder")
                                    .icon(IconName::Folder)
                                    .tooltip("New Folder")
                                    .ghost()
                                    .xsmall()
                            )
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
                    .child(self.render_file_tree(cx))
            )
    }

    fn render_file_tree(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_1()
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .p_1()
                    .hover(|style| style.bg(cx.theme().muted.opacity(0.5)))
                    .rounded(px(4.0))
                    .child("📁")
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child("src")
                    )
            )
            .child(
                v_flex()
                    .ml_4()
                    .gap_1()
                    .child(self.render_file_item("main.rs", "🦀", false, cx))
                    .child(self.render_file_item("game_logic.rs", "🦀", false, cx))
                    .child(self.render_file_item("player.rs", "🦀", false, cx))
                    .child(self.render_file_item("enemy.rs", "🦀", false, cx))
            )
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .p_1()
                    .hover(|style| style.bg(cx.theme().muted.opacity(0.5)))
                    .rounded(px(4.0))
                    .child("📁")
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child("assets")
                    )
            )
            .child(
                v_flex()
                    .ml_4()
                    .gap_1()
                    .child(self.render_file_item("config.toml", "⚙️", false, cx))
                    .child(self.render_file_item("README.md", "📝", false, cx))
            )
    }

    fn render_file_item(&self, filename: &str, icon: &str, is_open: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let is_current = self.current_file_index
            .and_then(|i| self.open_files.get(i))
            .map(|(name, _)| name == filename)
            .unwrap_or(false);

        h_flex()
            .items_center()
            .gap_2()
            .p_1()
            .rounded(px(4.0))
            .when(is_current, |this| this.bg(cx.theme().primary.opacity(0.2)))
            .when(!is_current, |this| this.hover(|style| style.bg(cx.theme().muted.opacity(0.5))))
            .child(icon.to_string())
            .child(
                div()
                    .text_sm()
                    .text_color(if is_current { cx.theme().primary } else { cx.theme().foreground })
                    .when(is_open, |this| this.font_medium())
                    .child(filename.to_string())
            )
    }

    fn render_editor_area(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                // Use proper TabBar component
                TabBar::new("editor-tabs")
                    .w_full()
                    .selected_index(self.current_file_index.unwrap_or(0))
                    .on_click(cx.listener(|this, ix: &usize, window, cx| {
                        this.set_active_file(*ix, window, cx);
                    }))
                    .children(
                        self.open_files.iter().map(|(filename, _)| {
                            Tab::new(filename.clone())
                        })
                    )
            )
            .child(
                div()
                    .flex_1()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_bl(cx.theme().radius)
                    .rounded_br(cx.theme().radius)
                    .child(self.render_code_editor(cx))
            )
    }

    fn render_code_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Some(index) = self.current_file_index {
            if let Some((_, input_state)) = self.open_files.get(index) {
                div()
                    .size_full()
                    .child(
                        TextInput::new(input_state)
                            .h_full()
                            .w_full()
                    )
                    .into_any_element()
            } else {
                self.render_empty_editor(cx)
            }
        } else {
            self.render_empty_editor(cx)
        }
    }

    fn render_empty_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .child(
                v_flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .text_xl()
                            .font_semibold()
                            .text_color(cx.theme().muted_foreground)
                            .child("No file selected")
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Open a file from the explorer or create a new one")
                    )
            )
            .into_any_element()
    }

    fn render_terminal(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                            .child("Terminal")
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("clear")
                                    .icon(IconName::Delete)
                                    .tooltip("Clear Terminal")
                                    .ghost()
                                    .xsmall()
                            )
                            .child(
                                Button::new("split")
                                    .icon(IconName::Copy)
                                    .tooltip("Split Terminal")
                                    .ghost()
                                    .xsmall()
                            )
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
                    .font_family("monospace")
                    .text_sm()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_color(cx.theme().primary)
                                    .child("$ cargo run")
                            )
                            .child(
                                div()
                                    .text_color(cx.theme().foreground)
                                    .child("   Compiling pulsar-engine v0.1.0")
                            )
                            .child(
                                div()
                                    .text_color(cx.theme().success)
                                    .child("    Finished dev [unoptimized + debuginfo] target(s) in 2.34s")
                            )
                            .child(
                                div()
                                    .text_color(cx.theme().foreground)
                                    .child("     Running `target/debug/pulsar-engine`")
                            )
                            .child(
                                div()
                                    .text_color(cx.theme().foreground)
                                    .child("Hello, Pulsar Engine!")
                            )
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_color(cx.theme().primary)
                                            .child("$")
                                    )
                                    .child(
                                        div()
                                            .w_2()
                                            .h_4()
                                            .bg(cx.theme().foreground)
                                            .opacity(0.7)
                                    )
                            )
                    )
            )
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let current_file = self.current_file_index
            .and_then(|i| self.open_files.get(i))
            .map(|(name, _)| name.as_str())
            .unwrap_or("No file");

        StatusBar::new()
            .add_left_item(current_file.to_string())
            .add_left_item("UTF-8".to_string())
            .add_left_item("LF".to_string())
            .add_right_item("Ln 1, Col 1")
            .add_right_item("Spaces: 4")
            .add_right_item("🦀 Rust")
            .render(cx)
    }
}

impl Panel for ScriptEditorPanel {
    fn panel_name(&self) -> &'static str {
        "Script Editor"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        div().child("Script Editor").into_any_element()
    }

    fn dump(&self, _cx: &App) -> gpui_component::dock::PanelState {
        gpui_component::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}

impl Focusable for ScriptEditorPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for ScriptEditorPanel {}

impl Render for ScriptEditorPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_toolbar(cx))
            .child(
                div()
                    .flex_1()
                    .child(
                        h_resizable("script-editor-horizontal", self.horizontal_resizable_state.clone())
                            .child(
                                resizable_panel()
                                    .size(px(250.))
                                    .size_range(px(180.)..px(400.))
                                    .child(
                                        div()
                                            .size_full()
                                            .bg(cx.theme().sidebar)
                                            .border_1()
                                            .border_color(cx.theme().border)
                                            .rounded(cx.theme().radius)
                                            .p_2()
                                            .child(self.render_file_explorer(cx))
                                    )
                            )
                            .child(
                                resizable_panel()
                                    .child(
                                        div()
                                            .size_full()
                                            .p_2()
                                            .child(
                                                v_resizable("script-editor-vertical", self.vertical_resizable_state.clone())
                                                    .child(
                                                        resizable_panel()
                                                            .child(
                                                                div()
                                                                    .size_full()
                                                                    .child(self.render_editor_area(cx))
                                                            )
                                                    )
                                                    .child(
                                                        resizable_panel()
                                                            .size(px(200.))
                                                            .size_range(px(120.)..px(400.))
                                                            .child(
                                                                div()
                                                                    .size_full()
                                                                    .bg(cx.theme().sidebar)
                                                                    .border_1()
                                                                    .border_color(cx.theme().border)
                                                                    .rounded(cx.theme().radius)
                                                                    .p_2()
                                                                    .child(self.render_terminal(cx))
                                                            )
                                                    )
                                            )
                                    )
                            )
                    )
            )
            .child(self.render_status_bar(cx))
    }
}