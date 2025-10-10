use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::{Button, ButtonVariant, ButtonVariants as _},
    context_menu::ContextMenuExt,
    h_flex,
    input::{InputState, TextInput},
    popup_menu::PopupMenu,
    resizable::{h_resizable, resizable_panel, v_resizable, ResizableState},
    v_flex, ActiveTheme as _, Icon, IconName, StyledExt,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use gpui::Axis;

// Actions for file management
#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = file_manager)]
pub struct NewClass {
    pub folder_path: String,
}

#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = file_manager)]
pub struct NewFolder {
    pub folder_path: String,
}

#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = file_manager)]
pub struct NewFile {
    pub folder_path: String,
}

#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = file_manager)]
pub struct DeleteItem {
    pub item_path: String,
}

#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = file_manager)]
pub struct RenameItem {
    pub item_path: String,
}

#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = file_manager, no_json)]
pub struct CommitRename;

#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = file_manager, no_json)]
pub struct CancelRename;

#[derive(Action, Clone, Debug, PartialEq, Eq, Deserialize, JsonSchema)]
#[action(namespace = file_manager, no_json)]
pub struct PopoutFileManager;

#[derive(Clone, Debug, PartialEq)]
pub enum FileType {
    Folder,
    Class, // A folder containing graph_save.json
    Script,
    DawProject, // .pdaw files
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
        } else {
            match path.extension().and_then(|s| s.to_str()) {
                Some("rs") => FileType::Script,
                Some("pdaw") => FileType::DawProject,
                _ => FileType::Other,
            }
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
    height_resizable_state: Entity<ResizableState>,
    renaming_item: Option<PathBuf>,
    rename_input_state: Entity<InputState>,
}

impl FileManagerDrawer {
    pub fn new(project_path: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let resizable_state = ResizableState::new(cx);
        let height_resizable_state = ResizableState::new(cx);
        let rename_input_state = cx.new(|cx| InputState::new(window, cx));

        // Subscribe to input events to handle Enter key for committing rename
        cx.subscribe(
            &rename_input_state,
            |drawer, _input, event: &gpui_component::input::InputEvent, cx| match event {
                gpui_component::input::InputEvent::PressEnter { .. } => {
                    if drawer.renaming_item.is_some() {
                        cx.dispatch_action(&CommitRename);
                    }
                }
                _ => {}
            },
        )
        .detach();

        Self {
            folder_tree: project_path.as_ref().and_then(|p| FolderNode::from_path(p)),
            project_path: project_path.clone(),
            selected_folder: project_path,
            resizable_state,
            height_resizable_state,
            renaming_item: None,
            rename_input_state,
        }
    }

    pub fn set_project_path(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.project_path = Some(path.clone());
        self.folder_tree = FolderNode::from_path(&path);
        self.selected_folder = Some(path);
        self.resizable_state = ResizableState::new(cx);
        self.height_resizable_state = ResizableState::new(cx);
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

    /// Handle clicking on an item in the file manager drawer
    fn handle_item_click(&mut self, item: &FileItem, cx: &mut Context<Self>) {
        eprintln!("DEBUG: handle_item_click called for: {:?}, type: {:?}", item.path, item.file_type);
        
        match &item.file_type {
            FileType::Class | FileType::Script | FileType::DawProject => {
                eprintln!("DEBUG: Emitting FileSelected event");
                // Emit event to open this class in BP editor, script in script editor, or DAW project
                cx.emit(FileSelected {
                    path: item.path.clone(),
                    file_type: item.file_type.clone(),
                });
            }
            FileType::Folder => {
                eprintln!("DEBUG: Selecting folder");
                // Select this folder to show its contents
                self.select_folder(item.path.clone(), cx);
            }
            _ => {
                eprintln!("DEBUG: Unknown file type, ignoring");
            }
        }
    }

    fn on_new_folder(&mut self, action: &NewFolder, window: &mut Window, cx: &mut Context<Self>) {
        let base_path = std::path::Path::new(&action.folder_path);
        let new_folder_name = "NewFolder";
        let new_path = base_path.join(new_folder_name);

        if let Err(e) = std::fs::create_dir_all(&new_path) {
            eprintln!("Failed to create folder: {}", e);
            return;
        }

        // Refresh the folder tree
        if let Some(project_path) = &self.project_path {
            self.folder_tree = FolderNode::from_path(project_path);
        }

        // Start rename mode for the new folder
        self.start_rename(new_path, window, cx);
    }

    fn on_new_class(&mut self, action: &NewClass, window: &mut Window, cx: &mut Context<Self>) {
        let base_path = std::path::Path::new(&action.folder_path);
        let new_class_name = "NewClass";
        let new_path = base_path.join(new_class_name);

        // Create class folder and events subfolder
        if let Err(e) = std::fs::create_dir_all(&new_path.join("events")) {
            eprintln!("Failed to create class folder: {}", e);
            return;
        }

        // Create empty graph_save.json with header
        let now = chrono::Local::now();
        let version = crate::ENGINE_VERSION;
        let header = format!(
            "// Auto Generated by the Pulsar Blueprint Editor\n\
             // DO NOT EDIT MANUALLY - YOUR CHANGES WILL BE OVERWRITTEN\n\
             // Generated on {} - Engine version {}\n\
             //\n\
             // This file contains the visual blueprint graph for this class.\n\
             // EDITING THE JSON STRUCTURE COULD BREAK THE EDITOR - BE CAREFUL\n\n",
            now.format("%Y-%m-%d %H:%M:%S"),
            version
        );

        let empty_graph = crate::graph::GraphDescription {
            nodes: std::collections::HashMap::new(),
            connections: Vec::new(),
            metadata: crate::graph::GraphMetadata {
                name: new_class_name.to_string(),
                description: String::from("New class created from file manager"),
                version: crate::ENGINE_VERSION.to_string(),
                created_at: now.to_rfc3339(),
                modified_at: now.to_rfc3339(),
            },
            comments: Vec::new(),
        };

        let json = serde_json::to_string_pretty(&empty_graph).unwrap_or_default();
        let content = format!("{}{}", header, json);

        if let Err(e) = std::fs::write(new_path.join("graph_save.json"), content) {
            eprintln!("Failed to create graph_save.json: {}", e);
            return;
        }

        // Refresh the folder tree
        if let Some(project_path) = &self.project_path {
            self.folder_tree = FolderNode::from_path(project_path);
        }

        // Start rename mode for the new class
        self.start_rename(new_path, window, cx);
    }

    fn on_new_file(&mut self, action: &NewFile, window: &mut Window, cx: &mut Context<Self>) {
        let base_path = std::path::Path::new(&action.folder_path);
        let new_file_name = "new_file.rs";
        let new_path = base_path.join(new_file_name);

        // Create empty Rust file with basic header
        let content = "// New Rust file\n\npub fn main() {\n    // Your code here\n}\n";

        if let Err(e) = std::fs::write(&new_path, content) {
            eprintln!("Failed to create file: {}", e);
            return;
        }

        // Start rename mode for the new file
        self.start_rename(new_path, window, cx);
    }

    fn on_delete_item(
        &mut self,
        action: &DeleteItem,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let item_path = std::path::Path::new(&action.item_path);

        // TODO: Add confirmation dialog
        if item_path.is_dir() {
            if let Err(e) = std::fs::remove_dir_all(item_path) {
                eprintln!("Failed to delete folder: {}", e);
                return;
            }
        } else {
            if let Err(e) = std::fs::remove_file(item_path) {
                eprintln!("Failed to delete file: {}", e);
                return;
            }
        }

        // Refresh the folder tree
        if let Some(project_path) = &self.project_path {
            self.folder_tree = FolderNode::from_path(project_path);
        }
        cx.notify();
    }

    fn on_rename_item(&mut self, action: &RenameItem, window: &mut Window, cx: &mut Context<Self>) {
        let item_path = std::path::Path::new(&action.item_path);
        if let Some(file_name) = item_path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
        {
            self.renaming_item = Some(item_path.to_path_buf());
            self.rename_input_state.update(cx, |state, cx| {
                state.set_value(&file_name, window, cx);
            });
            cx.notify();
        }
    }

    fn start_rename(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(file_name) = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.to_string())
        {
            self.renaming_item = Some(path);
            self.rename_input_state.update(cx, |state, cx| {
                state.set_value(&file_name, window, cx);
            });
            cx.notify();
        }
    }

    fn commit_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(old_path) = &self.renaming_item {
            // Get text as string
            let new_name = self
                .rename_input_state
                .read(cx)
                .text()
                .to_string()
                .trim()
                .to_string();
            if new_name.is_empty() {
                self.cancel_rename(cx);
                return;
            }

            if let Some(parent) = old_path.parent() {
                let new_path = parent.join(new_name);

                if old_path != &new_path {
                    if let Err(e) = std::fs::rename(old_path, &new_path) {
                        eprintln!("Failed to rename: {}", e);
                    } else {
                        // Update selected folder if we renamed it
                        if self.selected_folder.as_ref() == Some(old_path) {
                            self.selected_folder = Some(new_path);
                        }

                        // Refresh the folder tree
                        if let Some(project_path) = &self.project_path {
                            self.folder_tree = FolderNode::from_path(project_path);
                        }
                    }
                }
            }

            self.cancel_rename(cx);
        }
    }

    fn cancel_rename(&mut self, cx: &mut Context<Self>) {
        self.renaming_item = None;
        cx.notify();
    }

    fn on_commit_rename(
        &mut self,
        _action: &CommitRename,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.commit_rename(window, cx);
    }

    fn on_cancel_rename(
        &mut self,
        _action: &CancelRename,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.cancel_rename(cx);
    }

    fn on_popout_file_manager(
        &mut self,
        _action: &PopoutFileManager,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Create a new file manager drawer for the window
        let new_drawer = cx.new(|cx| {
            FileManagerDrawer::new(self.project_path.clone(), window, cx)
        });

        // Open the file manager window
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(Bounds {
                    origin: point(px(100.), px(100.)),
                    size: size(px(1000.), px(700.)),
                })),
                titlebar: Some(TitlebarOptions {
                    title: Some("File Manager".into()),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                let file_manager_window = cx.new(|cx| {
                    crate::ui::file_manager_window::FileManagerWindow::new(new_drawer, window, cx)
                });
                file_manager_window
            },
        );
    }

    fn render_folder_tree_node(
        &self,
        node: &FolderNode,
        depth: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let indent = (depth + 1) * 16;
        let icon = if node.is_class {
            IconName::Component
        } else if node.expanded {
            IconName::ChevronDown
        } else {
            IconName::ChevronRight
        };

        let path_clone = node.path.clone();
        let path_clone2 = node.path.clone();
        let path_for_menu = node.path.clone();
        let is_selected = self.selected_folder.as_ref() == Some(&node.path);
        let is_class = node.is_class;

        // STUDIO-QUALITY TREE NODE
        let node_button = div()
            .w_full()
            .h(px(32.))
            .rounded(px(6.))
            .pl(px(indent as f32 + 8.))
            .pr_2()
            .flex()
            .items_center()
            .gap_2()
            .cursor_pointer()
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(move |drawer, _, _, cx| {
                    if !is_class {
                        drawer.toggle_folder(&path_clone, cx);
                    }
                    drawer.select_folder(path_clone2.clone(), cx);
                }),
            )
            .when(is_selected, |this| {
                this.bg(cx.theme().primary.opacity(0.15))
                    .border_1()
                    .border_color(cx.theme().primary.opacity(0.3))
            })
            .when(!is_selected, |this| {
                this.hover(|this| {
                    this.bg(cx.theme().muted.opacity(0.5))
                })
            })
            .child(
                Icon::new(icon)
                    .size(px(16.))
                    .text_color(
                        if is_selected {
                            cx.theme().primary
                        } else if node.is_class {
                            cx.theme().accent
                        } else {
                            cx.theme().muted_foreground
                        }
                    )
            )
            .child(
                if self.renaming_item.as_ref() == Some(&node.path) {
                    div()
                        .flex_1()
                        .child(
                            TextInput::new(&self.rename_input_state)
                                .appearance(false)
                                .w_full()
                        )
                        .into_any_element()
                } else {
                    div()
                        .flex_1()
                        .text_sm()
                        .when(is_selected, |this| {
                            this.font_semibold()
                                .text_color(cx.theme().foreground)
                        })
                        .when(!is_selected, |this| {
                            this.text_color(cx.theme().foreground.opacity(0.8))
                        })
                        .child(node.name.clone())
                        .into_any_element()
                }
            )
            .when(is_class, |this| {
                this.child(
                    // Class badge
                    div()
                        .px_1p5()
                        .py_0p5()
                        .rounded(px(4.))
                        .bg(cx.theme().accent.opacity(0.2))
                        .border_1()
                        .border_color(cx.theme().accent.opacity(0.3))
                        .child(
                            div()
                                .text_xs()
                                .font_semibold()
                                .text_color(cx.theme().accent)
                                .child("BP")
                        )
                )
            });

        v_flex()
            .w_full()
            .gap_0p5()
            .child(
                div()
                    .id(SharedString::from(format!(
                        "tree-item-{}",
                        node.path.display()
                    )))
                    .w_full()
                    .context_menu(move |menu, _window, _cx| {
                        menu.menu(
                            "New Folder",
                            Box::new(NewFolder {
                                folder_path: path_for_menu.to_string_lossy().to_string(),
                            }),
                        )
                        .menu(
                            "New Class",
                            Box::new(NewClass {
                                folder_path: path_for_menu.to_string_lossy().to_string(),
                            }),
                        )
                        .separator()
                        .menu(
                            "Rename",
                            Box::new(RenameItem {
                                item_path: path_for_menu.to_string_lossy().to_string(),
                            }),
                        )
                        .menu(
                            "Delete",
                            Box::new(DeleteItem {
                                item_path: path_for_menu.to_string_lossy().to_string(),
                            }),
                        )
                    })
                    .child(node_button)
            )
            .children(if node.expanded && !node.is_class {
                Some(
                    v_flex()
                        .w_full()
                        .gap_0p5()
                        .children(
                            node.children
                                .iter()
                                .map(|child| self.render_folder_tree_node(child, depth + 1, cx)),
                        ),
                )
            } else {
                None
            })
    }

    fn render_content_item(&self, item: &FileItem, cx: &mut Context<Self>) -> impl IntoElement {
        let icon = match &item.file_type {
            FileType::Folder => IconName::Folder,
            FileType::Class => IconName::Component,
            FileType::Script => IconName::Code,
            FileType::DawProject => IconName::MusicNote,
            FileType::Other => IconName::Page,
        };

        let item_clone = item.clone();
        let item_path = item.path.clone();
        let item_type = item.file_type.clone();

        // STUDIO-QUALITY GRID ITEM CARD
        div()
            .id(SharedString::from(format!(
                "content-item-{}",
                item.path.display()
            )))
            .w(px(110.))
            .h(px(120.))
            .context_menu(move |menu, _window, _cx| {
                let path_str = item_path.to_string_lossy().to_string();
                match item_type {
                    FileType::Folder => menu
                        .menu(
                            "New Folder",
                            Box::new(NewFolder {
                                folder_path: path_str.clone(),
                            }),
                        )
                        .menu(
                            "New Class",
                            Box::new(NewClass {
                                folder_path: path_str.clone(),
                            }),
                        )
                        .separator()
                        .menu(
                            "Rename",
                            Box::new(RenameItem {
                                item_path: path_str.clone(),
                            }),
                        )
                        .menu(
                            "Delete",
                            Box::new(DeleteItem {
                                item_path: path_str.clone(),
                            }),
                        ),
                    FileType::Class => menu
                        .separator()
                        .menu(
                            "Rename",
                            Box::new(RenameItem {
                                item_path: path_str.clone(),
                            }),
                        )
                        .menu(
                            "Delete",
                            Box::new(DeleteItem {
                                item_path: path_str.clone(),
                            }),
                        ),
                    _ => menu
                        .menu(
                            "Rename",
                            Box::new(RenameItem {
                                item_path: path_str.clone(),
                            }),
                        )
                        .menu(
                            "Delete",
                            Box::new(DeleteItem {
                                item_path: path_str.clone(),
                            }),
                        ),
                }
            })
            .child(
                div()
                    .w_full()
                    .h_full()
                    .p_2()
                    .rounded(px(8.))
                    .border_1()
                    .border_color(cx.theme().border.opacity(0.5))
                    .bg(cx.theme().background)
                    .cursor_pointer()
                    .hover(|this| {
                        this.bg(cx.theme().muted.opacity(0.3))
                            .border_color(cx.theme().primary.opacity(0.5))
                            .shadow_md()
                    })
                    .on_mouse_down(
                        gpui::MouseButton::Left,
                        cx.listener(move |drawer, _, _, cx| {
                            drawer.handle_item_click(&item_clone, cx);
                        }),
                    )
                    .child(
                        v_flex()
                            .w_full()
                            .h_full()
                            .gap_2()
                            .items_center()
                            .justify_center()
                            .child(
                                // Icon container with background
                                div()
                                    .size(px(56.))
                                    .rounded(px(8.))
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(match &item.file_type {
                                        FileType::Class => cx.theme().accent.opacity(0.15),
                                        FileType::Folder => cx.theme().primary.opacity(0.1),
                                        FileType::Script => cx.theme().info.opacity(0.15),
                                        FileType::DawProject => cx.theme().success.opacity(0.15),
                                        _ => cx.theme().muted.opacity(0.1),
                                    })
                                    .border_1()
                                    .border_color(match &item.file_type {
                                        FileType::Class => cx.theme().accent.opacity(0.3),
                                        FileType::Folder => cx.theme().primary.opacity(0.2),
                                        FileType::Script => cx.theme().info.opacity(0.3),
                                        FileType::DawProject => cx.theme().success.opacity(0.3),
                                        _ => cx.theme().border.opacity(0.3),
                                    })
                                    .child(
                                        Icon::new(icon)
                                            .size(px(32.))
                                            .text_color(match &item.file_type {
                                                FileType::Class => cx.theme().accent,
                                                FileType::Folder => cx.theme().primary,
                                                FileType::Script => cx.theme().info,
                                                FileType::DawProject => cx.theme().success,
                                                _ => cx.theme().muted_foreground,
                                            })
                                    )
                            )
                            .child(
                                if self.renaming_item.as_ref() == Some(&item.path) {
                                    div()
                                        .w_full()
                                        .child(
                                            TextInput::new(&self.rename_input_state)
                                                .appearance(false)
                                                .w_full()
                                        )
                                        .into_any_element()
                                } else {
                                    div()
                                        .w_full()
                                        .text_xs()
                                        .text_center()
                                        .font_medium()
                                        .text_color(cx.theme().foreground)
                                        .overflow_hidden()
                                        .line_clamp(2)
                                        .child(item.name.clone())
                                        .into_any_element()
                                }
                            )
                            .when(matches!(item.file_type, FileType::Class), |this| {
                                this.child(
                                    // Blueprint badge
                                    div()
                                        .px_2()
                                        .py_0p5()
                                        .rounded(px(4.))
                                        .bg(cx.theme().accent.opacity(0.2))
                                        .border_1()
                                        .border_color(cx.theme().accent.opacity(0.4))
                                        .child(
                                            div()
                                                .text_xs()
                                                .font_semibold()
                                                .text_color(cx.theme().accent)
                                                .child("Blueprint")
                                        )
                                )
                            })
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
            .border_t_2()
            .border_color(cx.theme().border)
            .on_action(cx.listener(Self::on_new_folder))
            .on_action(cx.listener(Self::on_new_class))
            .on_action(cx.listener(Self::on_new_file))
            .on_action(cx.listener(Self::on_delete_item))
            .on_action(cx.listener(Self::on_rename_item))
            .on_action(cx.listener(Self::on_commit_rename))
            .on_action(cx.listener(Self::on_cancel_rename))
            .on_action(cx.listener(Self::on_popout_file_manager))
            .child(
                // Vertical resizable for drawer height
                v_resizable("file-manager-height", self.height_resizable_state.clone())
                    .child(
                        resizable_panel()
                            .size(px(400.))
                            .size_range(px(200.)..px(800.))
                            .child(
                                h_resizable("file-manager-split", self.resizable_state.clone())
                                    .child(
                        resizable_panel()
                            .size(px(280.))
                            .size_range(px(200.)..px(450.))
                            .child(
                                v_flex()
                                    .size_full()
                                    .bg(cx.theme().secondary.opacity(0.3))
                                    .child(
                                        // PROFESSIONAL TREE HEADER
                                        v_flex()
                                            .w_full()
                                            .gap_2()
                                            .px_3()
                                            .py_3()
                                            .bg(cx.theme().secondary)
                                            .border_b_2()
                                            .border_color(cx.theme().border)
                                            .child(
                                                // Header with icon and title
                                                h_flex()
                                                    .w_full()
                                                    .items_center()
                                                    .gap_2()
                                                    .child(
                                                        // Folder icon with glow
                                                        div()
                                                            .flex_shrink_0()
                                                            .size(px(32.))
                                                            .rounded(px(6.))
                                                            .bg(cx.theme().accent.opacity(0.15))
                                                            .border_1()
                                                            .border_color(cx.theme().accent.opacity(0.3))
                                                            .shadow_sm()
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .child(
                                                                Icon::new(IconName::Folder)
                                                                    .size(px(18.))
                                                                    .text_color(cx.theme().accent)
                                                            )
                                                    )
                                                    .child(
                                                        v_flex()
                                                            .flex_1()
                                                            .gap_0p5()
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .font_semibold()
                                                                    .text_color(cx.theme().foreground)
                                                                    .child("Project Explorer")
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(cx.theme().muted_foreground)
                                                                    .child("Folder Structure")
                                                            )
                                                    )
                                                    .child(
                                                        // Popout button
                                                        Button::new("popout-file-manager")
                                                            .ghost()
                                                            .compact()
                                                            .icon(IconName::ExternalLink)
                                                            .tooltip("Open in New Window")
                                                            .on_click(cx.listener(|_, _, _, cx| {
                                                                cx.dispatch_action(&PopoutFileManager);
                                                            }))
                                                    )
                                            )
                                            .child(
                                                // Search box (visual only for now)
                                                div()
                                                    .w_full()
                                                    .h(px(32.))
                                                    .px_3()
                                                    .py_1p5()
                                                    .rounded(px(6.))
                                                    .bg(cx.theme().background)
                                                    .border_1()
                                                    .border_color(cx.theme().border)
                                                    .flex()
                                                    .items_center()
                                                    .gap_2()
                                                    .child(
                                                        Icon::new(IconName::Search)
                                                            .size(px(14.))
                                                            .text_color(cx.theme().muted_foreground)
                                                    )
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .text_color(cx.theme().muted_foreground.opacity(0.6))
                                                            .child("Search folders...")
                                                    )
                                            )
                                    )
                                    .child(
                                        // TREE CONTENT with professional styling
                                        div()
                                            .flex_1()
                                            .overflow_hidden()
                                            .p_2()
                                            .child(
                                                div()
                                                    .size_full()
                                                    .scrollable(Axis::Vertical)
                                                    .child(
                                                        if let Some(tree) = &self.folder_tree {
                                                            v_flex()
                                                                .w_full()
                                                                .gap_0p5()
                                                                .children(tree.children.iter().map(
                                                                    |child| {
                                                                        self.render_folder_tree_node(
                                                                            child, 0, cx,
                                                                        )
                                                                    },
                                                                ))
                                                                .into_any_element()
                                                        } else {
                                                            // Beautiful empty state
                                                            v_flex()
                                                                .w_full()
                                                                .p_8()
                                                                .gap_3()
                                                                .items_center()
                                                                .justify_center()
                                                                .child(
                                                                    Icon::new(IconName::Folder)
                                                                        .size(px(64.))
                                                                        .text_color(cx.theme().muted_foreground.opacity(0.3))
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_sm()
                                                                        .font_semibold()
                                                                        .text_color(cx.theme().muted_foreground)
                                                                        .child("No Project Loaded")
                                                                )
                                                                .child(
                                                                    div()
                                                                        .text_xs()
                                                                        .text_center()
                                                                        .text_color(cx.theme().muted_foreground.opacity(0.7))
                                                                        .child("Open a project to explore files")
                                                                )
                                                                .into_any_element()
                                                        },
                                                    ),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        resizable_panel().child(
                            v_flex()
                                .size_full()
                                .bg(cx.theme().background)
                                .child(
                                    // PROFESSIONAL CONTENT HEADER
                                    v_flex()
                                        .w_full()
                                        .gap_2()
                                        .px_3()
                                        .py_3()
                                        .bg(cx.theme().secondary)
                                        .border_b_2()
                                        .border_color(cx.theme().border)
                                        .child(
                                            // Breadcrumb-style header
                                            h_flex()
                                                .w_full()
                                                .items_center()
                                                .gap_2()
                                                .child(
                                                    // File icon with glow
                                                    div()
                                                        .flex_shrink_0()
                                                        .size(px(32.))
                                                        .rounded(px(6.))
                                                        .bg(cx.theme().primary.opacity(0.15))
                                                        .border_1()
                                                        .border_color(cx.theme().primary.opacity(0.3))
                                                        .shadow_sm()
                                                        .flex()
                                                        .items_center()
                                                        .justify_center()
                                                        .child(
                                                            Icon::new(IconName::Page)
                                                                .size(px(18.))
                                                                .text_color(cx.theme().primary)
                                                        )
                                                )
                                                .child(
                                                    v_flex()
                                                        .flex_1()
                                                        .gap_0p5()
                                                        .child(
                                                            h_flex()
                                                                .items_center()
                                                                .gap_1p5()
                                                                .child(
                                                                    div()
                                                                        .text_sm()
                                                                        .font_semibold()
                                                                        .text_color(cx.theme().foreground)
                                                                        .children(self.selected_folder.as_ref().and_then(
                                                                            |p| {
                                                                                p.file_name()
                                                                                    .and_then(|n| n.to_str())
                                                                                    .map(|s| s.to_string())
                                                                            },
                                                                        ))
                                                                        .when(self.selected_folder.is_none(), |this| {
                                                                            this.child("No Folder Selected")
                                                                        })
                                                                )
                                                        )
                                                        .child(
                                                            div()
                                                                .text_xs()
                                                                .text_color(cx.theme().muted_foreground)
                                                                .child(format!("{} items", contents.len()))
                                                        )
                                                )
                                                .child(
                                                    // View mode toggle (future enhancement)
                                                    h_flex()
                                                        .gap_1()
                                                        .child(
                                                            Button::new("view-grid")
                                                                .ghost()
                                                                .compact()
                                                                .primary()
                                                                .icon(IconName::LayoutDashboard)
                                                                .tooltip("Grid View")
                                                        )
                                                        .child(
                                                            Button::new("view-list")
                                                                .ghost()
                                                                .compact()
                                                                .icon(IconName::List)
                                                                .tooltip("List View")
                                                        )
                                                )
                                        )
                                )
                                .child(
                                    // PROFESSIONAL CONTENT GRID
                                    div().flex_1().overflow_hidden().p_3().child({
                                        let selected_folder_for_menu = self.selected_folder.clone();
                                        div()
                                            .id("content-area-wrapper")
                                            .size_full()
                                            .context_menu(move |menu, _window, _cx| {
                                                if let Some(folder) = &selected_folder_for_menu {
                                                    let folder_str =
                                                        folder.to_string_lossy().to_string();
                                                    menu.menu(
                                                        "New Folder",
                                                        Box::new(NewFolder {
                                                            folder_path: folder_str.clone(),
                                                        }),
                                                    )
                                                    .menu(
                                                        "New Class",
                                                        Box::new(NewClass {
                                                            folder_path: folder_str.clone(),
                                                        }),
                                                    )
                                                    .menu(
                                                        "New File",
                                                        Box::new(NewFile {
                                                            folder_path: folder_str.clone(),
                                                        }),
                                                    )
                                                } else {
                                                    menu
                                                }
                                            })
                                            .child(
                                                div().size_full().scrollable(Axis::Vertical).child(
                                                    h_flex().w_full().flex_wrap().gap_2().children(
                                                        contents.iter().map(|item| {
                                                            self.render_content_item(item, cx)
                                                        }),
                                                    ),
                                                ),
                                            )
                                    })
                                )
                        ),
                    )
                )
            )
    )
    }
}
