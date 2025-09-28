use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use gpui::{*, prelude::FluentBuilder};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    ActiveTheme as _, StyledExt, Sizable as _,
    IconName, Icon,
};

#[derive(Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_directory: bool,
    pub is_expanded: bool,
    pub depth: usize,
}

pub struct FileExplorer {
    focus_handle: FocusHandle,
    project_root: Option<PathBuf>,
    file_tree: Vec<FileEntry>,
    expanded_folders: HashMap<PathBuf, bool>,
    selected_file: Option<PathBuf>,
    last_opened_file: Option<PathBuf>,
}

impl FileExplorer {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            project_root: None,
            file_tree: Vec::new(),
            expanded_folders: HashMap::new(),
            selected_file: None,
            last_opened_file: None,
        }
    }

    pub fn open_project(&mut self, path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) {
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
        if depth > 10 { return; } // Prevent infinite recursion

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
                    name,
                    path: path.clone(),
                    is_directory: path.is_dir(),
                    is_expanded: self.expanded_folders.get(&path).copied().unwrap_or(false),
                    depth,
                };

                if path.is_dir() {
                    dirs.push(file_entry);
                } else {
                    files.push(file_entry);
                }
            }

            // Sort directories and files alphabetically
            dirs.sort_by(|a, b| a.name.cmp(&b.name));
            files.sort_by(|a, b| a.name.cmp(&b.name));

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

    fn toggle_folder(&mut self, path: &Path, _window: &mut Window, cx: &mut Context<Self>) {
        let is_expanded = self.expanded_folders.get(path).copied().unwrap_or(false);
        println!("Toggling folder {:?} from {} to {}", path, is_expanded, !is_expanded);
        self.expanded_folders.insert(path.to_path_buf(), !is_expanded);
        self.refresh_file_tree(cx);
        println!("File tree now has {} entries", self.file_tree.len());
        cx.notify();
    }

    fn select_file(&mut self, path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) {
        self.selected_file = Some(path);
        cx.notify();
    }

    fn open_file_in_editor(&mut self, path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) {
        println!("Opening file in editor: {:?}", path);
        self.selected_file = Some(path.clone());
        self.last_opened_file = Some(path);
        cx.notify();
    }

    pub fn get_last_opened_file(&mut self) -> Option<PathBuf> {
        self.last_opened_file.take()
    }

    fn create_new_file(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(root) = &self.project_root {
            let new_path = root.join("new_file.rs");

            // Create the file
            if let Ok(_) = fs::write(&new_path, "") {
                self.refresh_file_tree(cx);
                self.selected_file = Some(new_path);
                cx.notify();
            }
        }
    }

    fn create_new_folder(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(root) = &self.project_root {
            let new_path = root.join("new_folder");

            // Create the directory
            if let Ok(_) = fs::create_dir(&new_path) {
                self.refresh_file_tree(cx);
                cx.notify();
            }
        }
    }

    fn get_file_icon(&self, entry: &FileEntry) -> IconName {
        if entry.is_directory {
            if entry.is_expanded {
                IconName::FolderOpen
            } else {
                IconName::Folder
            }
        } else {
            match entry.path.extension().and_then(|ext| ext.to_str()) {
                Some("rs") => IconName::SquareTerminal,
                Some("js") | Some("ts") => IconName::BookOpen,
                Some("py") => IconName::BookOpen,
                Some("toml") | Some("json") => IconName::Settings,
                Some("md") => IconName::BookOpen,
                Some("txt") => IconName::BookOpen,
                Some("html") | Some("css") => IconName::Globe,
                Some("png") | Some("jpg") | Some("jpeg") | Some("gif") => IconName::BookOpen,
                _ => IconName::BookOpen,
            }
        }
    }


    fn get_root_entries(&self) -> Vec<&FileEntry> {
        self.file_tree.iter().filter(|entry| entry.depth == 0).collect()
    }

    fn get_children_of(&self, parent_path: &std::path::Path) -> Vec<&FileEntry> {
        self.file_tree.iter()
            .filter(|entry| {
                entry.path.parent() == Some(parent_path) && entry.depth > 0
            })
            .collect()
    }

    fn render_file_tree_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .p_2()
            .child(self.render_file_tree_items(&self.get_root_entries(), cx))
    }

    fn render_file_tree_items(&self, entries: &[&FileEntry], cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .children(
                entries.iter().map(|entry| self.render_file_item(entry, cx))
            )
    }

    fn render_file_item(&self, entry: &FileEntry, cx: &mut Context<Self>) -> impl IntoElement {
        let is_selected = self.selected_file.as_ref() == Some(&entry.path);
        let path = entry.path.clone();
        let is_directory = entry.is_directory;
        let icon = self.get_file_icon(entry);

        div()
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .rounded_md()
                    .when(is_selected, |style| style.bg(cx.theme().accent))
                    .when(!is_selected, |style| {
                        style.hover(|style| style.bg(cx.theme().accent.opacity(0.1)))
                    })
                    .cursor_pointer()
                    .child(Icon::new(icon).size_4())
                    .child(
                        div()
                            .text_sm()
                            .when(is_selected, |style| style.text_color(cx.theme().accent_foreground))
                            .when(!is_selected, |style| style.text_color(cx.theme().foreground))
                            .child(entry.name.clone())
                    )
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, window, cx| {
                        if is_directory {
                            this.toggle_folder(&path, window, cx);
                        } else {
                            this.select_file(path.clone(), window, cx);
                            this.open_file_in_editor(path.clone(), window, cx);
                        }
                    }))
            )
            .when(is_directory && entry.is_expanded, |container| {
                let children = self.get_children_of(&entry.path);
                if !children.is_empty() {
                    container.child(
                        div()
                            .ml_4()
                            .child(self.render_file_tree_items(&children, cx))
                    )
                } else {
                    container
                }
            })
    }
}

impl Focusable for FileExplorer {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FileExplorer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .child(
                // Header
                div()
                    .w_full()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .w_full()
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
                                                this.create_new_file(window, cx);
                                            }))
                                    )
                                    .child(
                                        Button::new("new_folder")
                                            .icon(IconName::Folder)
                                            .tooltip("New Folder")
                                            .ghost()
                                            .xsmall()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                this.create_new_folder(window, cx);
                                            }))
                                    )
                                    .child(
                                        Button::new("refresh")
                                            .icon(IconName::Asterisk)
                                            .tooltip("Refresh")
                                            .ghost()
                                            .xsmall()
                                            .on_click(cx.listener(|this, _, _window, cx| {
                                                this.refresh_file_tree(cx);
                                            }))
                                    )
                                    .child(
                                        Button::new("open_folder")
                                            .icon(IconName::FolderOpen)
                                            .tooltip("Open Folder")
                                            .ghost()
                                            .xsmall()
                                            .on_click(cx.listener(|this, _, window, cx| {
                                                // Open current working directory as fallback
                                                if let Ok(cwd) = std::env::current_dir() {
                                                    this.open_project(cwd, window, cx);
                                                }
                                            }))
                                    )
                            )
                    )
            )
            .child(
                // Scrollable content area
                div()
                    .flex_1()
                    .when(self.file_tree.is_empty(), |content| {
                        content.child(
                            div()
                                .p_4()
                                .child(
                                    div()
                                        .flex()
                                        .items_center()
                                        .gap_2()
                                        .px_3()
                                        .py_2()
                                        .rounded_md()
                                        .hover(|style| style.bg(cx.theme().accent.opacity(0.1)))
                                        .cursor_pointer()
                                        .child(Icon::new(IconName::FolderOpen).size_4().text_color(cx.theme().muted_foreground))
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("No folder opened")
                                        )
                                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, window, cx| {
                                            if let Ok(cwd) = std::env::current_dir() {
                                                this.open_project(cwd, window, cx);
                                            }
                                        }))
                                )
                        )
                    })
                    .when(!self.file_tree.is_empty(), |content| {
                        content.child(self.render_file_tree_content(cx))
                    })
            )
            .when_some(self.project_root.clone(), |container, root| {
                container.child(
                    // Footer
                    div()
                        .w_full()
                        .px_4()
                        .py_2()
                        .border_t_1()
                        .border_color(cx.theme().border)
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(
                                    root.file_name()
                                        .unwrap_or_default()
                                        .to_string_lossy()
                                        .to_string()
                                )
                        )
                )
            })
    }
}