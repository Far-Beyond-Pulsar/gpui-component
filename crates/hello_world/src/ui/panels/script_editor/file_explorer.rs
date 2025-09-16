use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use gpui::{*, prelude::FluentBuilder};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    sidebar::{Sidebar, SidebarHeader, SidebarFooter, SidebarMenu, SidebarMenuItem},
    h_flex,
    ActiveTheme as _, StyledExt, Sizable as _,
    IconName,
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
}

impl FileExplorer {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            project_root: None,
            file_tree: Vec::new(),
            expanded_folders: HashMap::new(),
            selected_file: None,
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
        self.expanded_folders.insert(path.to_path_buf(), !is_expanded);
        self.refresh_file_tree(cx);
        cx.notify();
    }

    fn select_file(&mut self, path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) {
        self.selected_file = Some(path);
        cx.notify();
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

    fn render_file_tree(&self, cx: &mut Context<Self>) -> impl IntoElement {
        SidebarMenu::new().children(
            self.file_tree.iter().map(|entry| {
                let is_selected = self.selected_file.as_ref() == Some(&entry.path);
                let path = entry.path.clone();
                let is_directory = entry.is_directory;
                let icon = self.get_file_icon(entry);

                // Create indentation for nested files/folders
                let indent = "\u{00A0}".repeat(entry.depth * 2);
                let display_name = format!("{}{}", indent, entry.name);

                SidebarMenuItem::new(display_name)
                    .icon(icon)
                    .active(is_selected)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if is_directory {
                            this.toggle_folder(&path, window, cx);
                        } else {
                            this.select_file(path.clone(), window, cx);
                            // TODO: Emit event to open file in editor
                        }
                    }))
            })
        )
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
            .bg(cx.theme().sidebar)
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .w_full()
                    .p_3()
                    .pb_2()
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
                                    .on_click(cx.listener(|_this, _, _window, _cx| {
                                        // TODO: Implement new file creation
                                    }))
                            )
                            .child(
                                Button::new("new_folder")
                                    .icon(IconName::Folder)
                                    .tooltip("New Folder")
                                    .ghost()
                                    .xsmall()
                                    .on_click(cx.listener(|_this, _, _window, _cx| {
                                        // TODO: Implement new folder creation
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
            .child(
                div()
                    .flex_1()
                    .p_2()
                    .when(self.file_tree.is_empty(), |this| {
                        this.child(
                            div()
                                .p_4()
                                .text_center()
                                .text_color(cx.theme().muted_foreground)
                                .text_sm()
                                .child("No folder opened")
                                .child(
                                    div()
                                        .mt_2()
                                        .child(
                                            Button::new("open_folder_empty")
                                                .label("Open Folder")
                                                .small()
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    if let Ok(cwd) = std::env::current_dir() {
                                                        this.open_project(cwd, window, cx);
                                                    }
                                                }))
                                        )
                                )
                        )
                    })
                    .when(!self.file_tree.is_empty(), |this| {
                        this.child(self.render_file_tree(cx))
                    })
            )
            .child(
                div()
                    .w_full()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .p_2()
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .when_some(self.project_root.clone(), |this, root| {
                        this.child(
                            root.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                                .to_string()
                        )
                    })
            )
    }
}