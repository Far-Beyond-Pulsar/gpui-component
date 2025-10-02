use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariant, ButtonVariants as _},
    resizable::{h_resizable, resizable_panel, ResizableState},
    h_flex, v_flex, ActiveTheme as _, Icon, IconName, StyledExt,
};
use std::path::{Path, PathBuf};

use gpui::Axis;

#[derive(Clone, Debug, PartialEq)]
pub enum FileType {
    Folder,
    Class,      // A folder containing graph_save.json
    Script,
    Other,
}

#[derive(Clone, Debug)]
pub struct FileItem {
    pub path: PathBuf,
    pub name: String,
    pub file_type: FileType,
}

impl FileItem {
    pub fn is_class_folder(path: &Path) -> bool {
        path.is_dir() && path.join("graph_save.json").exists()
    }

    pub fn from_path(path: &Path) -> Option<Self> {
        let name = path.file_name()?.to_str()?.to_string();

        let file_type = if path.is_dir() {
            if Self::is_class_folder(path) {
                FileType::Class
            } else {
                FileType::Folder
            }
        } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
            FileType::Script
        } else {
            FileType::Other
        };

        Some(FileItem {
            path: path.to_path_buf(),
            name,
            file_type,
        })
    }
}

#[derive(Clone, Debug)]
pub struct FolderNode {
    pub path: PathBuf,
    pub name: String,
    pub children: Vec<FolderNode>,
    pub expanded: bool,
    pub is_class: bool,
}

impl FolderNode {
    pub fn from_path(path: &Path) -> Option<Self> {
        if !path.is_dir() {
            return None;
        }

        let name = path.file_name()?.to_str()?.to_string();
        let is_class = FileItem::is_class_folder(path);

        // Class folders should NOT appear in the tree at all
        if is_class {
            return None;
        }

        // Only recurse into non-class folders
        let children = std::fs::read_dir(path)
            .ok()?
            .filter_map(|entry| {
                let entry = entry.ok()?;
                let entry_path = entry.path();

                // Skip hidden files and non-directories
                if !entry_path.is_dir() {
                    return None;
                }
                if entry_path.file_name()?.to_str()?.starts_with('.') {
                    return None;
                }

                FolderNode::from_path(&entry_path)
            })
            .collect();

        Some(FolderNode {
            path: path.to_path_buf(),
            name,
            children,
            expanded: false,
            is_class,
        })
    }

    fn toggle_expanded(&mut self, target_path: &Path) -> bool {
        if self.path == target_path {
            self.expanded = !self.expanded;
            return true;
        }

        for child in &mut self.children {
            if child.toggle_expanded(target_path) {
                return true;
            }
        }

        false
    }
}

pub struct FileManagerDrawer {
    project_path: Option<PathBuf>,
    folder_tree: Option<FolderNode>,
    selected_folder: Option<PathBuf>,
    resizable_state: Entity<ResizableState>,
}

impl FileManagerDrawer {
    pub fn new(project_path: Option<PathBuf>, cx: &mut Context<Self>) -> Self {
        let resizable_state = ResizableState::new(cx);
        Self {
            folder_tree: project_path.as_ref().and_then(|p| FolderNode::from_path(p)),
            project_path: project_path.clone(),
            selected_folder: project_path,
            resizable_state,
        }
    }

    pub fn set_project_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.project_path = Some(path.clone());
        self.folder_tree = FolderNode::from_path(&path);
        self.selected_folder = Some(path);
        self.resizable_state = ResizableState::new(cx);
        cx.notify();
    }

    fn toggle_folder(&mut self, path: &Path, cx: &mut Context<Self>) {
        if let Some(tree) = &mut self.folder_tree {
            tree.toggle_expanded(path);
            cx.notify();
        }
    }

    fn select_folder(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.selected_folder = Some(path);
        cx.notify();
    }

    fn get_folder_contents(&self) -> Vec<FileItem> {
        let folder = match &self.selected_folder {
            Some(f) => f,
            None => return Vec::new(),
        };

        std::fs::read_dir(folder)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|entry| {
                        let entry = entry.ok()?;
                        let path = entry.path();

                        // Skip hidden files
                        if path.file_name()?.to_str()?.starts_with('.') {
                            return None;
                        }

                        FileItem::from_path(&path)
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn handle_item_click(&mut self, item: &FileItem, cx: &mut Context<Self>) {
        match &item.file_type {
            FileType::Class => {
                // Emit event to open this class in BP editor
                cx.emit(FileSelected {
                    path: item.path.clone(),
                    file_type: item.file_type.clone(),
                });
            }
            FileType::Folder => {
                // Select this folder to show its contents
                self.select_folder(item.path.clone(), cx);
            }
            _ => {}
        }
    }

    fn render_folder_tree_node(&self, node: &FolderNode, depth: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let indent = depth * 16;
        let icon = if node.is_class {
            IconName::Frame
        } else if node.expanded {
            IconName::FolderOpen
        } else {
            IconName::FolderClosed
        };

        let path_clone = node.path.clone();
        let path_clone2 = node.path.clone();
        let is_selected = self.selected_folder.as_ref() == Some(&node.path);
        let is_class = node.is_class;

        let button = if is_selected {
            Button::new(SharedString::from(format!("tree-{}", node.path.display())))
                .primary()
                .w_full()
                .justify_start()
                .pl(px(indent as f32))
        } else {
            Button::new(SharedString::from(format!("tree-{}", node.path.display())))
                .ghost()
                .w_full()
                .justify_start()
                .pl(px(indent as f32))
        };

        v_flex()
            .w_full()
            .child(
                button
                    .on_click(cx.listener(move |drawer, _, _, cx| {
                        if !is_class {
                            drawer.toggle_folder(&path_clone, cx);
                        }
                        drawer.select_folder(path_clone2.clone(), cx);
                    }))
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                Icon::new(icon)
                                    .size(px(14.))
                                    .text_color(if node.is_class {
                                        cx.theme().primary
                                    } else {
                                        cx.theme().muted_foreground
                                    })
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .child(node.name.clone())
                            )
                    )
            )
            .children(if node.expanded && !node.is_class {
                Some(
                    v_flex()
                        .w_full()
                        .children(
                            node.children.iter().map(|child| {
                                self.render_folder_tree_node(child, depth + 1, cx)
                            })
                        )
                )
            } else {
                None
            })
    }

    fn render_content_item(&self, item: &FileItem, cx: &mut Context<Self>) -> impl IntoElement {
        let icon = match &item.file_type {
            FileType::Folder => IconName::Folder,
            FileType::Class => IconName::Frame,
            FileType::Script => IconName::BookOpen,
            FileType::Other => IconName::Dash,
        };

        let item_clone = item.clone();

        // Grid item with icon on top, text below
        div()
            .w(px(100.))
            .p_2()
            .cursor_pointer()
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |drawer, _, _, cx| {
                drawer.handle_item_click(&item_clone, cx);
            }))
            .child(
                v_flex()
                    .gap_2()
                    .items_center()
                    .hover(|this| this.bg(cx.theme().muted.opacity(0.5)))
                    .rounded(cx.theme().radius)
                    .p_2()
                    .child(
                        Icon::new(icon)
                            .size(px(48.))
                            .text_color(match &item.file_type {
                                FileType::Class => cx.theme().primary,
                                FileType::Folder => cx.theme().accent,
                                _ => cx.theme().muted_foreground,
                            })
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_center()
                            .w_full()
                            .overflow_hidden()
                            .child(item.name.clone())
                    )
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let contents = self.get_folder_contents();

        div()
            .size_full()
            .bg(cx.theme().background)
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                h_resizable("file-manager-split", self.resizable_state.clone())
                    .child(
                        resizable_panel()
                            .size(px(250.))
                            .size_range(px(150.)..px(400.))
                            .child(
                                v_flex()
                                    .size_full()
                                    .child(
                                        // Tree header
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
                                                    .child("Folders")
                                            )
                                    )
                                    .child(
                                        // Tree content with scrolling
                                        div()
                                            .flex_1()
                                            .overflow_hidden()
                                            .p_1()
                                            .child(
                                                div()
                                                    .size_full()
                                                    .scrollable(Axis::Vertical)
                                                    .child(
                                                        if let Some(tree) = &self.folder_tree {
                                                            v_flex()
                                                                .w_full()
                                                                .children(
                                                                    tree.children.iter().map(|child| {
                                                                        self.render_folder_tree_node(child, 0, cx)
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
                            )
                    )
                    .child(
                        resizable_panel()
                            .child(
                                v_flex()
                                    .size_full()
                                    .child(
                                        // Content header
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
                                                    .children(self.selected_folder.as_ref().and_then(|p| {
                                                        p.file_name().and_then(|n| n.to_str()).map(|s| s.to_string())
                                                    }))
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(format!("{} items", contents.len()))
                                            )
                                    )
                                    .child(
                                        // Content grid with scrolling
                                        div()
                                            .flex_1()
                                            .overflow_hidden()
                                            .p_3()
                                            .child(
                                                div()
                                                    .size_full()
                                                    .scrollable(Axis::Vertical)
                                                    .child(
                                                        h_flex()
                                                            .w_full()
                                                            .flex_wrap()
                                                            .gap_2()
                                                            .children(
                                                                contents.iter().map(|item| {
                                                                    self.render_content_item(item, cx)
                                                                })
                                                            )
                                                    )
                                            )
                                    )
                            )
                    )
            )
    }
}
