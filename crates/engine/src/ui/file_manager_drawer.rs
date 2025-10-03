use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariant, ButtonVariants as _},
    resizable::{h_resizable, resizable_panel, ResizableState},
    context_menu::ContextMenuExt,
    popup_menu::PopupMenu,
    input::{TextInput, InputState},
    h_flex, v_flex, ActiveTheme as _, Icon, IconName, StyledExt,
};
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;

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
    renaming_item: Option<PathBuf>,
    rename_input_state: Entity<InputState>,
}

impl FileManagerDrawer {
    pub fn new(project_path: Option<PathBuf>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let resizable_state = ResizableState::new(cx);
        let rename_input_state = cx.new(|cx| InputState::new(window, cx));

        // Subscribe to input events to handle Enter key for committing rename
        cx.subscribe(&rename_input_state, |drawer, _input, event: &gpui_component::input::InputEvent, cx| {
            match event {
                gpui_component::input::InputEvent::PressEnter { .. } => {
                    if drawer.renaming_item.is_some() {
                        cx.dispatch_action(&CommitRename);
                    }
                }
                _ => {}
            }
        }).detach();

        Self {
            folder_tree: project_path.as_ref().and_then(|p| FolderNode::from_path(p)),
            project_path: project_path.clone(),
            selected_folder: project_path,
            resizable_state,
            renaming_item: None,
            rename_input_state,
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
            now.format("%Y-%m-%d %H:%M:%S"), version
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

    fn on_delete_item(&mut self, action: &DeleteItem, _window: &mut Window, cx: &mut Context<Self>) {
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
        if let Some(file_name) = item_path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()) {
            self.renaming_item = Some(item_path.to_path_buf());
            self.rename_input_state.update(cx, |state, cx| {
                state.set_value(&file_name, window, cx);
            });
            cx.notify();
        }
    }

    fn start_rename(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()).map(|s| s.to_string()) {
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
            let new_name = self.rename_input_state.read(cx).text().to_string().trim().to_string();
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

    fn on_commit_rename(&mut self, _action: &CommitRename, window: &mut Window, cx: &mut Context<Self>) {
        self.commit_rename(window, cx);
    }

    fn on_cancel_rename(&mut self, _action: &CancelRename, _window: &mut Window, cx: &mut Context<Self>) {
        self.cancel_rename(cx);
    }

    fn render_folder_tree_node(&self, node: &FolderNode, depth: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let indent = depth * 16;
        let icon = if node.is_class {
            IconName::Component
        } else if node.expanded {
            IconName::FolderOpen
        } else {
            IconName::FolderClosed
        };

        let path_clone = node.path.clone();
        let path_clone2 = node.path.clone();
        let path_for_menu = node.path.clone();
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
                div()
                    .id(SharedString::from(format!("tree-item-{}", node.path.display())))
                    .w_full()
                    .context_menu(move |menu, _window, _cx| {
                        menu.menu("New Folder", Box::new(NewFolder {
                            folder_path: path_for_menu.to_string_lossy().to_string()
                        }))
                        .menu("New Class", Box::new(NewClass {
                            folder_path: path_for_menu.to_string_lossy().to_string()
                        }))
                        .separator()
                        .menu("Rename", Box::new(RenameItem {
                            item_path: path_for_menu.to_string_lossy().to_string()
                        }))
                        .menu("Delete", Box::new(DeleteItem {
                            item_path: path_for_menu.to_string_lossy().to_string()
                        }))
                    })
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
                                        if self.renaming_item.as_ref() == Some(&node.path) {
                                            div()
                                                .text_sm()
                                                .child(
                                                    TextInput::new(&self.rename_input_state)
                                                        .appearance(false)
                                                        .w_full()
                                                )
                                                .into_any_element()
                                        } else {
                                            div()
                                                .text_sm()
                                                .child(node.name.clone())
                                                .into_any_element()
                                        }
                                    )
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
            FileType::Class => IconName::Component,
            FileType::Script => IconName::Code,
            FileType::Other => IconName::Page,
        };

        let item_clone = item.clone();
        let item_path = item.path.clone();
        let item_type = item.file_type.clone();

        // Grid item with icon on top, text below
        div()
            .id(SharedString::from(format!("content-item-{}", item.path.display())))
            .w(px(100.))
            .p_2()
            .context_menu(move |menu, _window, _cx| {
                let path_str = item_path.to_string_lossy().to_string();
                match item_type {
                    FileType::Folder => {
                        menu.menu("New Folder", Box::new(NewFolder {
                            folder_path: path_str.clone()
                        }))
                        .menu("New Class", Box::new(NewClass {
                            folder_path: path_str.clone()
                        }))
                        .separator()
                        .menu("Rename", Box::new(RenameItem {
                            item_path: path_str.clone()
                        }))
                        .menu("Delete", Box::new(DeleteItem {
                            item_path: path_str.clone()
                        }))
                    }
                    FileType::Class => {
                        menu.separator()
                        .menu("Rename", Box::new(RenameItem {
                            item_path: path_str.clone()
                        }))
                        .menu("Delete", Box::new(DeleteItem {
                            item_path: path_str.clone()
                        }))
                    }
                    _ => {
                        menu.menu("Rename", Box::new(RenameItem {
                            item_path: path_str.clone()
                        }))
                        .menu("Delete", Box::new(DeleteItem {
                            item_path: path_str.clone()
                        }))
                    }
                }
            })
            .child(
                div()
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
                                if self.renaming_item.as_ref() == Some(&item.path) {
                                    div()
                                        .text_xs()
                                        .text_center()
                                        .w_full()
                                        .child(
                                            TextInput::new(&self.rename_input_state)
                                                .appearance(false)
                                                .w_full()
                                        )
                                        .into_any_element()
                                } else {
                                    div()
                                        .text_xs()
                                        .text_center()
                                        .w_full()
                                        .overflow_hidden()
                                        .child(item.name.clone())
                                        .into_any_element()
                                }
                            )
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
            .on_action(cx.listener(Self::on_new_folder))
            .on_action(cx.listener(Self::on_new_class))
            .on_action(cx.listener(Self::on_new_file))
            .on_action(cx.listener(Self::on_delete_item))
            .on_action(cx.listener(Self::on_rename_item))
            .on_action(cx.listener(Self::on_commit_rename))
            .on_action(cx.listener(Self::on_cancel_rename))
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
                                            .child({
                                                let selected_folder_for_menu = self.selected_folder.clone();
                                                div()
                                                    .id("content-area-wrapper")
                                                    .size_full()
                                                    .context_menu(move |menu, _window, _cx| {
                                                        if let Some(folder) = &selected_folder_for_menu {
                                                            let folder_str = folder.to_string_lossy().to_string();
                                                            menu.menu("New Folder", Box::new(NewFolder {
                                                                folder_path: folder_str.clone()
                                                            }))
                                                            .menu("New Class", Box::new(NewClass {
                                                                folder_path: folder_str.clone()
                                                            }))
                                                            .menu("New File", Box::new(NewFile {
                                                                folder_path: folder_str.clone()
                                                            }))
                                                        } else {
                                                            menu
                                                        }
                                                    })
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
                                            })
                                    )
                            )
                    )
            )
    }
}
