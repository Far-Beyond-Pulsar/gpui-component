use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use gpui::{*, prelude::FluentBuilder};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    sidebar::{Sidebar, SidebarHeader, SidebarFooter, SidebarGroup, SidebarMenu, SidebarMenuItem},
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

    fn render_file_tree(&self, cx: &mut Context<Self>) -> SidebarMenu {
        // Simple flat list approach - this ensures folder contents actually show up
        SidebarMenu::new().children(
            self.file_tree.iter().map(|entry| {
                let is_selected = self.selected_file.as_ref() == Some(&entry.path);
                let path = entry.path.clone();
                let is_directory = entry.is_directory;
                let icon = self.get_file_icon(entry);

                // Create simple visual indentation using spaces
                let indent = "  ".repeat(entry.depth); // 2 spaces per level
                let display_name = format!("{}{}", indent, entry.name);

                SidebarMenuItem::new(display_name)
                    .icon(icon)
                    .active(is_selected)
                    .on_click(cx.listener(move |this, _, window, cx| {
                        if is_directory {
                            this.toggle_folder(&path, window, cx);
                        } else {
                            this.select_file(path.clone(), window, cx);
                            this.open_file_in_editor(path.clone(), window, cx);
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
        Sidebar::new(gpui_component::Side::Left)
            .header(
                SidebarHeader::new()
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
            .when(self.file_tree.is_empty(), |sidebar| {
                sidebar.child(
                    SidebarMenu::new()
                        .child(
                            SidebarMenuItem::new("No folder opened")
                                .icon(IconName::FolderOpen)
                                .on_click(cx.listener(|this, _, window, cx| {
                                    if let Ok(cwd) = std::env::current_dir() {
                                        this.open_project(cwd, window, cx);
                                    }
                                }))
                        )
                )
            })
            .when(!self.file_tree.is_empty(), |sidebar| {
                sidebar.child(self.render_file_tree(cx))
            })
            .footer(
                SidebarFooter::new()
                    .when_some(self.project_root.clone(), |footer, root| {
                        footer.child(
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
                    })
            )
    }
}