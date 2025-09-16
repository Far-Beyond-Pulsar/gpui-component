use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use gpui::{*, InteractiveElement, Styled, Render};
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

#[derive(Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub is_expanded: bool,
}

pub struct ScriptEditorPanel {
    focus_handle: FocusHandle,
    open_files: Vec<(PathBuf, Entity<InputState>)>,
    current_file_index: Option<usize>,
    project_root: Option<PathBuf>,
    file_tree: Vec<FileEntry>,
    expanded_folders: HashMap<PathBuf, bool>,
    horizontal_resizable_state: Entity<ResizableState>,
    vertical_resizable_state: Entity<ResizableState>,
}

impl ScriptEditorPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let horizontal_resizable_state = ResizableState::new(cx);
        let vertical_resizable_state = ResizableState::new(cx);

        Self {
            focus_handle: cx.focus_handle(),
            open_files: Vec::new(),
            current_file_index: None,
            project_root: None,
            file_tree: Vec::new(),
            expanded_folders: HashMap::new(),
            horizontal_resizable_state,
            vertical_resizable_state,
        }
    }

    pub fn open_project_folder(&mut self, path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) {
        if path.is_dir() {
            self.project_root = Some(path.clone());
            self.refresh_file_tree(cx);
            cx.notify();
        }
    }

    fn refresh_file_tree(&mut self, _cx: &mut Context<Self>) {
        self.file_tree.clear();
        if let Some(root) = self.project_root.clone() {
            self.scan_directory(&root, 0);
        }
    }

    fn scan_directory(&mut self, dir: &Path, depth: usize) {
        if depth > 5 { return; } // Limit recursion depth

        if let Ok(entries) = fs::read_dir(dir) {
            let mut dirs = Vec::new();
            let mut files = Vec::new();

            for entry in entries.flatten() {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files and common ignore patterns
                if name.starts_with('.') || name == "target" || name == "node_modules" {
                    continue;
                }

                let file_entry = FileEntry {
                    name: format!("{}{}", "  ".repeat(depth), name),
                    path: path.clone(),
                    is_directory: path.is_dir(),
                    is_expanded: self.expanded_folders.get(&path).copied().unwrap_or(false),
                };

                if path.is_dir() {
                    dirs.push(file_entry);
                } else {
                    files.push(file_entry);
                }
            }

            // Add directories first, then files
            for dir_entry in dirs {
                let is_expanded = dir_entry.is_expanded;
                let path = dir_entry.path.clone();
                self.file_tree.push(dir_entry);

                if is_expanded {
                    self.scan_directory(&path, depth + 1);
                }
            }

            for file_entry in files {
                self.file_tree.push(file_entry);
            }
        }
    }

    pub fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        // Check if file is already open
        if let Some(index) = self.open_files.iter().position(|(p, _)| p == &path) {
            self.current_file_index = Some(index);
            cx.notify();
            return;
        }

        // Read file content
        let content = match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(_) => String::new(),
        };

        // Determine syntax highlighting based on file extension
        let language = match path.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => "rust",
            Some("js") | Some("ts") => "javascript",
            Some("py") => "python",
            Some("toml") => "toml",
            Some("json") => "json",
            Some("md") => "markdown",
            _ => "text",
        };

        // Create editor state for the file
        let input_state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(language)
                .line_number(true)
                .soft_wrap(false)
                .default_value(&content)
        });

        self.open_files.push((path, input_state));
        self.current_file_index = Some(self.open_files.len() - 1);
        cx.notify();
    }

    pub fn create_new_file(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(root) = &self.project_root {
            let new_path = root.join(&name);

            // Create the file
            if let Ok(_) = fs::write(&new_path, "") {
                // Open the new file
                self.open_file(new_path, window, cx);
                self.refresh_file_tree(cx);
            }
        }
    }

    pub fn create_new_directory(&mut self, name: String, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(root) = &self.project_root {
            let new_path = root.join(&name);

            // Create the directory
            if let Ok(_) = fs::create_dir(&new_path) {
                self.refresh_file_tree(cx);
            }
        }
    }

    pub fn save_current_file(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> bool {
        if let Some(index) = self.current_file_index {
            if let Some((_path, _input_state)) = self.open_files.get(index) {
                // Get content from input state (this would need to be implemented in the InputState API)
                // For now, we'll simulate saving
                cx.notify();
                return true;
            }
        }
        false
    }

    fn toggle_folder(&mut self, path: &Path, _window: &mut Window, cx: &mut Context<Self>) {
        let is_expanded = self.expanded_folders.get(path).copied().unwrap_or(false);
        self.expanded_folders.insert(path.to_path_buf(), !is_expanded);
        self.refresh_file_tree(cx);
        cx.notify();
    }

    pub fn close_file(&mut self, index: usize, _window: &mut Window, cx: &mut Context<Self>) {
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
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.create_new_file("new_file.rs".to_string(), window, cx);
                                    }))
                            )
                            .child(
                                Button::new("new_folder")
                                    .icon(IconName::Folder)
                                    .tooltip("New Folder")
                                    .ghost()
                                    .xsmall()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.create_new_directory("new_folder".to_string(), window, cx);
                                    }))
                            )
                            .child(
                                Button::new("open_folder")
                                    .icon(IconName::FolderOpen)
                                    .tooltip("Open Folder")
                                    .ghost()
                                    .xsmall()
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        // In a real implementation, this would open a file dialog
                                        // For now, let's open the current working directory
                                        if let Ok(cwd) = std::env::current_dir() {
                                            this.open_project_folder(cwd, window, cx);
                                        }
                                    }))
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
            .children(
                self.file_tree.iter().map(|entry| {
                    self.render_file_tree_item(entry, cx).into_any_element()
                })
            )
            .when(self.file_tree.is_empty(), |this| {
                this.child(
                    div()
                        .p_4()
                        .text_center()
                        .text_color(cx.theme().muted_foreground)
                        .child("Open a folder to see files")
                )
            })
    }

    fn render_file_tree_item(&self, entry: &FileEntry, cx: &mut Context<Self>) -> impl IntoElement {
        let is_current = self.current_file_index
            .and_then(|i| self.open_files.get(i))
            .map(|(path, _)| path == &entry.path)
            .unwrap_or(false);

        let is_open = self.open_files.iter().any(|(path, _)| path == &entry.path);

        let icon = if entry.is_directory {
            if entry.is_expanded { "📁" } else { "📁" }
        } else {
            match entry.path.extension().and_then(|ext| ext.to_str()) {
                Some("rs") => "🦀",
                Some("js") | Some("ts") => "📜",
                Some("py") => "🐍",
                Some("toml") => "⚙️",
                Some("json") => "📋",
                Some("md") => "📝",
                _ => "📄",
            }
        };

        let path = entry.path.clone();
        let is_directory = entry.is_directory;
        let entry_name = entry.name.clone();

        Button::new(SharedString::from(format!("file-{}", path.to_string_lossy())))
            .ghost()
            .w_full()
            .justify_start()
            .p_1()
            .rounded(px(4.0))
            .when(is_current, |this| this.bg(cx.theme().primary.opacity(0.2)))
            .when(!is_current, |this| this.hover(|style| style.bg(cx.theme().muted.opacity(0.5))))
            .on_click(cx.listener(move |this, _, window, cx| {
                if is_directory {
                    this.toggle_folder(&path, window, cx);
                } else {
                    this.open_file(path.clone(), window, cx);
                }
            }))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(icon.to_string())
                    .child(
                        div()
                            .text_sm()
                            .text_color(if is_current { cx.theme().primary } else { cx.theme().foreground })
                            .when(is_open, |this| this.font_medium())
                            .child(entry_name)
                    )
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
                        self.open_files.iter().map(|(path, _)| {
                            let filename = path.file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or("untitled")
                                .to_string();
                            Tab::new(filename)
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
            .map(|(path, _)| {
                path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("No file")
            })
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