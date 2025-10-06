use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use gpui::{*, prelude::FluentBuilder};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex,
    scroll::Scrollbar,
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
    /// Flattened list of visible entries (after applying expansion states)
    visible_entries: Vec<usize>, // Indices into file_tree
    expanded_folders: HashMap<PathBuf, bool>,
    selected_file: Option<PathBuf>,
    last_opened_file: Option<PathBuf>,
    /// Scroll state for virtualization
    scroll_handle: ScrollHandle,
    scroll_state: gpui_component::scroll::ScrollbarState,
    /// Item height in pixels (fixed for all items)
    item_height: Pixels,
}

impl FileExplorer {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            project_root: None,
            file_tree: Vec::new(),
            visible_entries: Vec::new(),
            expanded_folders: HashMap::new(),
            selected_file: None,
            last_opened_file: None,
            scroll_handle: ScrollHandle::new(),
            scroll_state: gpui_component::scroll::ScrollbarState::default(),
            item_height: px(28.0), // Fixed height for each item
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
        self.visible_entries.clear();
        
        if let Some(root) = self.project_root.clone() {
            self.scan_directory(&root, 0);
            self.rebuild_visible_entries();
        }
        
        // Clamp scroll after refresh (file tree may have changed)
        let current_offset = self.scroll_handle.offset();
        self.set_scroll_offset_clamped(current_offset);
    }
    
    /// Rebuild the flat list of visible entries based on expansion states
    fn rebuild_visible_entries(&mut self) {
        self.visible_entries.clear();
        
        for (idx, entry) in self.file_tree.iter().enumerate() {
            // Check if this entry should be visible based on parent expansion
            if self.is_entry_visible(idx) {
                self.visible_entries.push(idx);
            }
        }
    }
    
    /// Check if an entry is visible (all parents are expanded)
    fn is_entry_visible(&self, entry_idx: usize) -> bool {
        let entry = &self.file_tree[entry_idx];
        
        // Root level is always visible
        if entry.depth == 0 {
            return true;
        }
        
        // Check if parent is expanded
        if let Some(parent_path) = entry.path.parent() {
            // Find parent in file tree
            for (idx, potential_parent) in self.file_tree.iter().enumerate() {
                if idx >= entry_idx {
                    break; // Parent must come before child
                }
                
                if potential_parent.path == parent_path {
                    return potential_parent.is_expanded && self.is_entry_visible(idx);
                }
            }
        }
        
        false
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
        
        // Update the file tree entry
        for entry in &mut self.file_tree {
            if entry.path == path {
                entry.is_expanded = !is_expanded;
                break;
            }
        }
        
        // Rebuild the file tree to reflect expansion changes
        self.refresh_file_tree(cx);
        
        // Clamp scroll offset after tree changes (number of visible entries changed)
        let current_offset = self.scroll_handle.offset();
        self.set_scroll_offset_clamped(current_offset);
        
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
                Some("rs") => IconName::Code,
                Some("js") | Some("ts") => IconName::Code,
                Some("py") => IconName::Code,
                Some("toml") | Some("json") => IconName::Settings,
                Some("md") => IconName::Notes,
                Some("txt") => IconName::Page,
                Some("html") => IconName::HTML5,
                Some("css") => IconName::CSS3,
                Some("png") | Some("jpg") | Some("jpeg") | Some("gif") => IconName::MediaImage,
                _ => IconName::Page,
            }
        }
    }
    
    /// Calculate which entries are visible in the viewport (virtualization)
    fn calculate_visible_range(&self, scroll_offset: Pixels) -> (usize, usize, Pixels) {
        // Assume a reasonable viewport height (will be more accurate with measured bounds)
        let viewport_height = px(600.0); // Conservative estimate
        
        // Convert to float for division
        let scroll_f = -scroll_offset;
        let item_height_f = self.item_height;
        
        let start_index = ((scroll_f / item_height_f).floor().max(0.0)) as usize;
        let visible_count = ((viewport_height / item_height_f).ceil() as usize) + 4; // +4 for buffer
        let end_index = (start_index + visible_count).min(self.visible_entries.len());
        
        (start_index, end_index, viewport_height)
    }

    fn render_file_tree_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let scroll_offset = self.scroll_handle.offset();
        let (start_idx, end_idx, _viewport_height) = self.calculate_visible_range(scroll_offset.y);
        
        // Total height of all items
        let total_height = self.item_height * self.visible_entries.len() as f32;
        
        // Offset for virtualization
        let offset_y = self.item_height * start_idx as f32;
        
        div()
            .relative()
            .size_full()
            .overflow_hidden()
            .child(
                // Visible items container with absolute positioning for virtualization
                div()
                    .absolute()
                    .top(offset_y + scroll_offset.y)
                    .left(scroll_offset.x)
                    .w_full()
                    .flex()
                    .flex_col()
                    .children(
                        (start_idx..end_idx)
                            .filter_map(|visible_idx| {
                                self.visible_entries.get(visible_idx)
                                    .and_then(|&tree_idx| self.file_tree.get(tree_idx))
                                    .map(|entry| self.render_file_item(entry, cx))
                            })
                    )
            )
            .child(
                // Scrollbar overlay
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .bottom_0()
                    .w(px(12.0))
                    .child(
                        Scrollbar::vertical(&self.scroll_state, &self.scroll_handle)
                            .scroll_size(gpui::Size {
                                width: px(250.0), // Approximate width
                                height: total_height,
                            })
                    )
            )
    }

    fn render_file_item(&self, entry: &FileEntry, cx: &mut Context<Self>) -> impl IntoElement {
        let is_selected = self.selected_file.as_ref() == Some(&entry.path);
        let path = entry.path.clone();
        let is_directory = entry.is_directory;
        let icon = self.get_file_icon(entry);
        let indent = px(entry.depth as f32 * 16.0); // 16px per depth level

        div()
            .flex()
            .items_center()
            .gap_2()
            .h(self.item_height)
            .pl(indent + px(12.0))
            .pr_3()
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
    }
    
    fn on_scroll_wheel(&mut self, event: &ScrollWheelEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let delta = event.delta.pixel_delta(px(20.0));
        let current_offset = self.scroll_handle.offset();
        let new_offset = current_offset + delta;
        
        self.set_scroll_offset_clamped(new_offset);
        cx.notify();
    }
    
    /// Set scroll offset with bounds checking to prevent out-of-bounds scrolling
    fn set_scroll_offset_clamped(&mut self, offset: gpui::Point<Pixels>) {
        // Calculate bounds for scrolling
        let total_height = self.item_height * self.visible_entries.len() as f32;
        let viewport_height = px(600.0); // Same as in calculate_visible_range
        
        // Clamp scroll offset to valid range
        // Y: Can scroll from 0 (top) to -(total_height - viewport_height) (bottom)
        let max_scroll_y = px(0.0);
        let min_scroll_y = -(total_height - viewport_height).max(px(0.0));
        
        // X: No horizontal scrolling needed, keep at 0
        let clamped_offset = gpui::point(
            px(0.0), // No horizontal scroll
            offset.y.max(min_scroll_y).min(max_scroll_y)
        );
        
        self.scroll_handle.set_offset(clamped_offset);
    }
}

impl Focusable for FileExplorer {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FileExplorer {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let file_tree_empty = self.file_tree.is_empty();
        
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
                // Scrollable content area with virtualization
                div()
                    .id("file-explorer-content")
                    .flex_1()
                    .overflow_hidden()
                    .on_scroll_wheel(cx.listener(Self::on_scroll_wheel))
                    .when(file_tree_empty, |content| {
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
                    .when(!file_tree_empty, |content| {
                        // Virtualized scrollable list
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