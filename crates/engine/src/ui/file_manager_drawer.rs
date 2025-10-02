use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariant, ButtonVariants as _},
    h_flex, v_flex, ActiveTheme as _, Icon, IconName, StyledExt,
};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub enum FileType {
    Folder,
    Class,
    Script,
    Other,
}

#[derive(Clone, Debug)]
pub struct FileItem {
    pub path: PathBuf,
    pub name: String,
    pub file_type: FileType,
    pub children: Vec<FileItem>,
    pub expanded: bool,
}

impl FileItem {
    pub fn from_path(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?.to_string();

        let file_type = if path.is_dir() {
            // Check if this is a class folder (contains graph_save.json)
            if path.join("graph_save.json").exists() {
                FileType::Class
            } else {
                FileType::Folder
            }
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            FileType::Script
        } else {
            FileType::Other
        };

        let children = if path.is_dir() {
            std::fs::read_dir(path)
                .ok()?
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = entry.path();
                    // Filter out hidden files and certain directories
                    if path.file_name()?.to_str()?.starts_with('.') {
                        return None;
                    }
                    FileItem::from_path(&path)
                })
                .collect()
        } else {
            Vec::new()
        };

        Some(FileItem {
            path: path.to_path_buf(),
            name,
            file_type,
            children,
            expanded: false,
        })
    }
}

pub struct FileManagerDrawer {
    project_path: Option<PathBuf>,
    root_item: Option<FileItem>,
}

impl FileManagerDrawer {
    pub fn new(project_path: Option<PathBuf>) -> Self {
        let root_item = project_path.as_ref().and_then(|path| FileItem::from_path(path));
        Self {
            project_path,
            root_item,
        }
    }

    pub fn set_project_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.project_path = Some(path.clone());
        self.root_item = FileItem::from_path(&path);
        cx.notify();
    }

    fn toggle_folder(&mut self, item_path: &Path, cx: &mut Context<Self>) {
        if let Some(root) = &mut self.root_item {
            Self::toggle_folder_recursive(root, item_path);
            cx.notify();
        }
    }

    fn toggle_folder_recursive(item: &mut FileItem, target_path: &Path) -> bool {
        if item.path == target_path {
            item.expanded = !item.expanded;
            return true;
        }

        for child in &mut item.children {
            if Self::toggle_folder_recursive(child, target_path) {
                return true;
            }
        }

        false
    }

    fn handle_file_click(&mut self, item: &FileItem, cx: &mut Context<Self>) {
        match &item.file_type {
            FileType::Class => {
                // Emit event to open this class in the BP editor
                cx.emit(FileSelected {
                    path: item.path.clone(),
                    file_type: item.file_type.clone(),
                });
            }
            FileType::Script => {
                // Emit event to open this script in the text editor
                cx.emit(FileSelected {
                    path: item.path.clone(),
                    file_type: item.file_type.clone(),
                });
            }
            _ => {}
        }
    }

    fn render_file_tree_item(&self, item: &FileItem, depth: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let indent = depth * 16;
        let is_folder = matches!(item.file_type, FileType::Folder);
        let is_class = matches!(item.file_type, FileType::Class);

        let icon = match &item.file_type {
            FileType::Folder => if item.expanded { IconName::FolderOpen } else { IconName::FolderClosed },
            FileType::Class => IconName::Frame,
            FileType::Script => IconName::BookOpen,
            FileType::Other => IconName::Dash,
        };

        let item_clone = item.clone();
        let item_clone2 = item.clone();

        v_flex()
            .w_full()
            .child(
                Button::new(SharedString::from(format!("file-item-{}", item.path.display())))
                    .ghost()
                    .w_full()
                    .justify_start()
                    .pl(px(indent as f32))
                    .on_click(cx.listener(move |drawer, _, _, cx| {
                        if is_folder || is_class {
                            drawer.toggle_folder(&item_clone.path, cx);
                        }
                        if is_class {
                            drawer.handle_file_click(&item_clone, cx);
                        }
                    }))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                        Icon::new(icon)
                            .size(px(14.))
                            .text_color(match &item.file_type {
                                FileType::Class => cx.theme().primary,
                                _ => cx.theme().muted_foreground,
                            })
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(match &item.file_type {
                                FileType::Class => cx.theme().foreground,
                                FileType::Folder => cx.theme().foreground,
                                _ => cx.theme().muted_foreground,
                            })
                            .child(item.name.clone())
                    )
            )
            )
            .children(
                if item.expanded {
                    Some(
                        v_flex()
                            .w_full()
                            .children(
                                item.children.iter().map(|child| {
                                    self.render_file_tree_item(child, depth + 1, cx)
                                })
                            )
                    )
                } else {
                    None
                }
            )
    }
}

#[derive(Clone, Debug)]
pub struct FileSelected {
    pub path: PathBuf,
    pub file_type: FileType,
}

impl EventEmitter<FileSelected> for FileManagerDrawer {}

impl Render for FileManagerDrawer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(cx.theme().background)
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                v_flex()
                    .size_full()
                    .child(
                        // Header
                        h_flex()
                            .w_full()
                            .p_2()
                            .border_b_1()
                            .border_color(cx.theme().border)
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("Project Files")
                            )
                            .child(
                                Icon::new(IconName::Folder)
                                    .size(px(14.))
                                    .text_color(cx.theme().muted_foreground)
                            )
                    )
                    .child(
                        // File tree
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .p_1()
                            .child(
                                if let Some(root) = &self.root_item {
                                    v_flex()
                                        .w_full()
                                        .children(
                                            root.children.iter().map(|child| {
                                                self.render_file_tree_item(child, 0, cx)
                                            })
                                        )
                                        .into_any_element()
                                } else {
                                    div()
                                        .p_4()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("No project loaded")
                                        .into_any_element()
                                }
                            )
                    )
            )
    }
}
