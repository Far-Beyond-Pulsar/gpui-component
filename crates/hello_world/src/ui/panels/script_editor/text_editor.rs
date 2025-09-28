use std::fs;
use std::path::PathBuf;
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    input::{InputState, TextInput, TabSize},
    tab::{Tab, TabBar},
    v_flex, h_flex,
    ActiveTheme as _, StyledExt, Sizable as _,
    IconName,
};

#[derive(Clone)]
pub struct OpenFile {
    pub path: PathBuf,
    pub input_state: Entity<InputState>,
    pub is_modified: bool,
}

pub struct TextEditor {
    focus_handle: FocusHandle,
    open_files: Vec<OpenFile>,
    current_file_index: Option<usize>,
}

impl TextEditor {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            open_files: Vec::new(),
            current_file_index: None,
        }
    }

    pub fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        // Check if file is already open
        if let Some(index) = self.open_files.iter().position(|f| f.path == path) {
            self.current_file_index = Some(index);
            cx.notify();
            return;
        }

        // Read file content
        let content = match fs::read_to_string(&path) {
            Ok(content) => {
                println!("Successfully read file {:?} with {} characters", path, content.len());
                content
            }
            Err(err) => {
                eprintln!("Failed to read file: {:?}, error: {}", path, err);
                return;
            }
        };

        // Determine syntax highlighting based on file extension
        let language = self.get_language_from_extension(&path);

        // Create editor state for the file
        let input_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .code_editor(language)
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                .soft_wrap(false);

            // Set the content after creating the state
            state.set_value(&content, window, cx);
            state
        });

        let open_file = OpenFile {
            path,
            input_state,
            is_modified: false,
        };

        self.open_files.push(open_file);
        self.current_file_index = Some(self.open_files.len() - 1);
        cx.notify();
    }

    fn get_language_from_extension(&self, path: &PathBuf) -> String {
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("rs") => "rust".to_string(),
            Some("js") => "javascript".to_string(),
            Some("ts") => "typescript".to_string(),
            Some("py") => "python".to_string(),
            Some("toml") => "toml".to_string(),
            Some("json") => "json".to_string(),
            Some("md") => "markdown".to_string(),
            Some("html") => "html".to_string(),
            Some("css") => "css".to_string(),
            Some("go") => "go".to_string(),
            Some("rb") => "ruby".to_string(),
            Some("sql") => "sql".to_string(),
            _ => "text".to_string(),
        }
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

    fn save_current_file(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> bool {
        if let Some(index) = self.current_file_index {
            if let Some(open_file) = self.open_files.get_mut(index) {
                // Get content from input state
                let content = open_file.input_state.read(cx).value();

                // Write to file
                if let Ok(_) = fs::write(&open_file.path, content.as_str()) {
                    open_file.is_modified = false;
                    cx.notify();
                    return true;
                }
            }
        }
        false
    }

    fn render_tab_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        if self.open_files.is_empty() {
            return div().into_any_element();
        }

        TabBar::new("editor-tabs")
            .w_full()
            .bg(cx.theme().secondary)
            .border_b_1()
            .border_color(cx.theme().border)
            .selected_index(self.current_file_index.unwrap_or(0))
            .on_click(cx.listener(|this, ix: &usize, window, cx| {
                this.set_active_file(*ix, window, cx);
            }))
            .children(
                self.open_files.iter().enumerate().map(|(index, open_file)| {
                    let filename = open_file.path.file_name()
                        .and_then(|name| name.to_str())
                        .unwrap_or("untitled")
                        .to_string();

                    let display_name = if open_file.is_modified {
                        format!("● {}", filename)
                    } else {
                        filename
                    };

                    Tab::new(display_name)
                        .child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_sm()
                                        .child(if open_file.is_modified {
                                            format!("● {}", open_file.path.file_name()
                                                .and_then(|name| name.to_str())
                                                .unwrap_or("untitled"))
                                        } else {
                                            open_file.path.file_name()
                                                .and_then(|name| name.to_str())
                                                .unwrap_or("untitled")
                                                .to_string()
                                        })
                                )
                                .child(
                                    Button::new(("close", index))
                                        .icon(IconName::Close)
                                        .ghost()
                                        .xsmall()
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.close_file(index, window, cx);
                                        }))
                                )
                        )
                })
            )
            .into_any_element()
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .w_full()
            .p_2()
            .bg(cx.theme().secondary)
            .border_b_1()
            .border_color(cx.theme().border)
            .justify_between()
            .items_center()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("new_file")
                            .icon(IconName::Plus)
                            .tooltip("New File (Ctrl+N)")
                            .ghost()
                            .small()
                            .on_click(cx.listener(|_this, _, _window, _cx| {
                                // TODO: Implement new file creation
                            }))
                    )
                    .child(
                        Button::new("save")
                            .icon(IconName::Asterisk)
                            .tooltip("Save (Ctrl+S)")
                            .ghost()
                            .small()
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.save_current_file(window, cx);
                            }))
                    )
                    .child(
                        Button::new("find")
                            .icon(IconName::Search)
                            .tooltip("Find (Ctrl+F)")
                            .ghost()
                            .small()
                            .on_click(cx.listener(|_this, _, _window, _cx| {
                                // TODO: Implement find functionality
                            }))
                    )
                    .child(
                        Button::new("replace")
                            .icon(IconName::Search)
                            .tooltip("Replace (Ctrl+H)")
                            .ghost()
                            .small()
                            .on_click(cx.listener(|_this, _, _window, _cx| {
                                // TODO: Implement replace functionality
                            }))
                    )
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("run")
                            .icon(IconName::ArrowRight)
                            .tooltip("Run Script (F5)")
                            .ghost()
                            .small()
                            .on_click(cx.listener(|_this, _, _window, _cx| {
                                // TODO: Implement run functionality
                            }))
                    )
                    .child(
                        Button::new("debug")
                            .icon(IconName::Search)
                            .tooltip("Debug Script (F9)")
                            .ghost()
                            .small()
                            .on_click(cx.listener(|_this, _, _window, _cx| {
                                // TODO: Implement debug functionality
                            }))
                    )
            )
    }

    fn render_editor_content(&self, cx: &mut Context<Self>) -> AnyElement {
        if let Some(index) = self.current_file_index {
            if let Some(open_file) = self.open_files.get(index) {
                div()
                    .size_full()
                    .overflow_hidden()
                    .child(
                        TextInput::new(&open_file.input_state)
                            .h_full()
                            .w_full()
                            .font_family("monospace")
                            .text_size(px(14.0))
                            .border_0()
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
            .bg(cx.theme().background)
            .child(
                v_flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .text_2xl()
                            .font_semibold()
                            .text_color(cx.theme().muted_foreground)
                            .child("Welcome to Script Editor")
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .text_center()
                            .child("Open a file from the explorer to start editing")
                    )
                    .child(
                        h_flex()
                            .gap_3()
                            .mt_4()
                            .child(
                                Button::new("new_file_welcome")
                                    .label("New File")
                                    .icon(IconName::Plus)
                                    .on_click(cx.listener(|_this, _, _window, _cx| {
                                        // TODO: Implement new file creation
                                    }))
                            )
                            .child(
                                Button::new("open_folder_welcome")
                                    .label("Open Folder")
                                    .icon(IconName::FolderOpen)
                                    .with_variant(gpui_component::button::ButtonVariant::Primary)
                                    .on_click(cx.listener(|_this, _, _window, _cx| {
                                        // TODO: Implement folder opening
                                    }))
                            )
                    )
            )
            .into_any_element()
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let current_file_info = if let Some(index) = self.current_file_index {
            if let Some(open_file) = self.open_files.get(index) {
                let filename = open_file.path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("untitled");
                let language = self.get_language_from_extension(&open_file.path);
                (filename.to_string(), language)
            } else {
                ("No file".to_string(), "".to_string())
            }
        } else {
            ("No file".to_string(), "".to_string())
        };

        h_flex()
            .w_full()
            .h_6()
            .px_4()
            .py_1()
            .bg(cx.theme().accent)
            .border_t_1()
            .border_color(cx.theme().border)
            .justify_between()
            .items_center()
            .text_xs()
            .text_color(cx.theme().accent_foreground)
            .child(
                h_flex()
                    .gap_4()
                    .child(current_file_info.0)
                    .child("UTF-8")
                    .child("LF")
            )
            .child(
                h_flex()
                    .gap_4()
                    .child("Ln 1, Col 1")
                    .child("Spaces: 4")
                    .child(current_file_info.1)
            )
    }
}

impl Focusable for TextEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for TextEditor {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_toolbar(cx))
            .child(self.render_tab_bar(cx))
            .child(
                div()
                    .flex_1()
                    .child(self.render_editor_content(cx))
            )
            .child(self.render_status_bar(cx))
    }
}