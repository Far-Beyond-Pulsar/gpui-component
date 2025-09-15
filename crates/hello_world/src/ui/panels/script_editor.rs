use gpui::*;
use gpui_component::{
    button::Button,
    dock::{Panel, PanelEvent},
    h_flex, v_flex,
    ActiveTheme as _, StyledExt, Selectable,
    IconName,
};
use gpui::prelude::FluentBuilder;

use crate::ui::shared::{Toolbar, ToolbarButton, StatusBar};

pub struct ScriptEditorPanel {
    focus_handle: FocusHandle,
    open_files: Vec<String>,
    current_file: Option<String>,
    file_tree_visible: bool,
    terminal_visible: bool,
}

impl ScriptEditorPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            open_files: vec![
                "main.rs".to_string(),
                "game_logic.rs".to_string(),
                "player.rs".to_string(),
            ],
            current_file: Some("main.rs".to_string()),
            file_tree_visible: true,
            terminal_visible: true,
        }
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Toolbar::new()
            .add_button(
                ToolbarButton::new(IconName::Folder, "New")
                    .tooltip("New File (Ctrl+N)")
            )
            .add_button(
                ToolbarButton::new(IconName::FolderOpen, "Open")
                    .tooltip("Open File (Ctrl+O)")
            )
            .add_button(
                ToolbarButton::new(IconName::Folder, "Save")
                    .tooltip("Save File (Ctrl+S)")
            )
            .add_button(
                ToolbarButton::new(IconName::Search, "Find")
                    .tooltip("Find in Files (Ctrl+Shift+F)")
            )
            .add_button(
                ToolbarButton::new(IconName::Check, "Run")
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
                                    .icon(IconName::Folder)
                                    .tooltip("New File")
                            )
                            .child(
                                Button::new("new_folder")
                                    .icon(IconName::Folder)
                                    .tooltip("New Folder")
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
                    .child(self.render_file_item("main.rs", "🦀", cx))
                    .child(self.render_file_item("game_logic.rs", "🦀", cx))
                    .child(self.render_file_item("player.rs", "🦀", cx))
                    .child(self.render_file_item("enemy.rs", "🦀", cx))
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
                    .child(self.render_file_item("config.toml", "⚙️", cx))
                    .child(self.render_file_item("README.md", "📝", cx))
            )
    }

    fn render_file_item(&self, filename: &str, icon: &str, cx: &mut Context<Self>) -> impl IntoElement {
        let is_open = self.open_files.contains(&filename.to_string());
        let is_current = self.current_file.as_ref() == Some(&filename.to_string());

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
            .child(self.render_editor_tabs(cx))
            .child(
                div()
                    .flex_1()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .p_4()
                    .child(self.render_code_editor(cx))
            )
    }

    fn render_editor_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_0()
            .children(
                self.open_files.iter().map(|file| {
                    let is_current = self.current_file.as_ref() == Some(file);

                    h_flex()
                        .items_center()
                        .gap_2()
                        .px_3()
                        .py_2()
                        .bg(if is_current { cx.theme().background } else { cx.theme().muted.opacity(0.3) })
                        .border_t_1()
                        .border_l_1()
                        .border_r_1()
                        .border_color(cx.theme().border)
                        .when(is_current, |this| this.border_b_1().border_color(cx.theme().background))
                        .rounded_tl(cx.theme().radius)
                        .rounded_tr(cx.theme().radius)
                        .hover(|style| style.bg(cx.theme().background))
                        .child(
                            div()
                                .text_sm()
                                .text_color(if is_current { cx.theme().foreground } else { cx.theme().muted_foreground })
                                .child(file.clone())
                        )
                        .child({
                            let close_label = format!("close_{}", file);
                            Button::new(close_label)
                                .icon(IconName::Close)
                                .tooltip("Close")
                        })
                        .into_any_element()
                })
            )
    }

    fn render_code_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(current_file) = &self.current_file {
            let content = match current_file.as_str() {
                "main.rs" => {
                    r#"use std::collections::HashMap;

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
}"#
                }
                "game_logic.rs" => {
                    r#"use crate::Entity;

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
}"#
                }
                _ => "// New file\n\n"
            };

            div()
                .size_full()
                .font_family("monospace")
                .text_sm()
                .text_color(cx.theme().foreground)
                .child(
                    v_flex()
                        .gap_0()
                        .children(
                            content.lines().enumerate().map(|(i, line)| {
                                h_flex()
                                    .items_start()
                                    .gap_3()
                                    .child(
                                        div()
                                            .w_8()
                                            .text_right()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!("{}", i + 1))
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .child(line)
                                    )
                                    .into_any_element()
                            })
                        )
                )
        } else {
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(cx.theme().muted_foreground)
                .child("No file selected")
        }
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
                            )
                            .child(
                                Button::new("split")
                                    .icon(IconName::Copy)
                                    .tooltip("Split Terminal")
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
        StatusBar::new()
            .add_left_item(format!("Rust"))
            .add_left_item(format!("UTF-8"))
            .add_left_item(format!("LF"))
            .add_right_item("Ln 23, Col 16")
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
                h_flex()
                    .flex_1()
                    .gap_1()
                    .child(
                        // Left panel - File Explorer
                        div()
                            .w_64()
                            .h_full()
                            .bg(cx.theme().sidebar)
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .p_2()
                            .child(self.render_file_explorer(cx))
                    )
                    .child(
                        // Center - Editor
                        div()
                            .flex_1()
                            .h_full()
                            .p_2()
                            .child(self.render_editor_area(cx))
                    )
                    .child(
                        // Right panel - Terminal
                        div()
                            .w_80()
                            .h_full()
                            .bg(cx.theme().sidebar)
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .p_2()
                            .child(self.render_terminal(cx))
                    )
            )
            .child(self.render_status_bar(cx))
    }
}