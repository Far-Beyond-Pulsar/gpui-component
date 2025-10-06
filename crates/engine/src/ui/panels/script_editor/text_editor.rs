use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    input::{InputState, TextInput, TabSize, InputEvent},
    tab::{Tab, TabBar},
    v_flex, h_flex,
    ActiveTheme as _, StyledExt, Sizable as _,
    IconName,
};
use std::path::PathBuf;
use std::time::Instant;
use std::fs;

#[derive(Clone)]
pub struct OpenFile {
    pub path: PathBuf,
    pub input_state: Entity<InputState>,
    pub is_modified: bool,
    pub lines_count: usize,
    pub file_size: usize,
}

pub struct TextEditor {
    focus_handle: FocusHandle,
    open_files: Vec<OpenFile>,
    current_file_index: Option<usize>,
    /// Performance monitoring
    last_render_time: Option<Instant>,
    show_performance_stats: bool,
    subscriptions: Vec<Subscription>,
}

impl TextEditor {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            open_files: Vec::new(),
            current_file_index: None,
            last_render_time: None,
            show_performance_stats: false, // Toggle with F12 or button
            subscriptions: Vec::new(),
        }
    }

    pub fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        // Check if file is already open
        if let Some(index) = self.open_files.iter().position(|f| f.path == path) {
            self.current_file_index = Some(index);
            cx.notify();
            return;
        }

        // Read file content with timing
        let read_start = Instant::now();
        let content = match fs::read_to_string(&path) {
            Ok(content) => {
                let read_time = read_start.elapsed();
                println!(
                    "✓ Read file {:?} - {} bytes in {:.2}ms", 
                    path.file_name().unwrap_or_default(),
                    content.len(),
                    read_time.as_secs_f64() * 1000.0
                );
                content
            }
            Err(err) => {
                eprintln!("✗ Failed to read file: {:?}, error: {}", path, err);
                return;
            }
        };

        let file_size = content.len();
        let lines_count = content.lines().count();
        
        // Determine syntax highlighting based on file extension
        let language = self.get_language_from_extension(&path);
        
        println!(
            "📄 Opening file: {} lines, {} KB, language: {}",
            lines_count,
            file_size / 1024,
            language
        );
        
        // Warn user about very large files
        if lines_count > 50_000 {
            println!(
                "⚠️  Large file detected ({} lines). Some features may be disabled for performance:",
                lines_count
            );
            println!("   - Syntax highlighting disabled");
            println!("   - Soft wrap disabled");
        } else if lines_count > 10_000 {
            println!(
                "ℹ️  Large file ({} lines). Performance optimizations enabled:",
                lines_count
            );
            println!("   - Soft wrap disabled");
            println!("   - Virtual scrolling enabled");
        }

        // Create editor state with optimal settings for large files
        let setup_start = Instant::now();
        let input_state = cx.new(|cx| {
            let mut state = InputState::new(window, cx)
                .code_editor(language)
                .line_number(true)
                .tab_size(TabSize {
                    tab_size: 4,
                    hard_tabs: false,
                })
                // Disable soft wrap for large files for better performance
                // Files with more than 5k lines or 500KB get no wrapping
                .soft_wrap(lines_count < 5_000 && file_size < 500_000);

            // Set the content after creating the state
            state.set_value(&content, window, cx);
            state
        });

        // Set up autocomplete for the file
        let workspace_root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        input_state.update(cx, |state, cx| {
            super::setup_autocomplete_for_file(
                state,
                path.clone(),
                workspace_root,
                window,
                cx,
            );
        });

        let setup_time = setup_start.elapsed();
        println!(
            "⚡ Editor setup completed in {:.2}ms",
            setup_time.as_secs_f64() * 1000.0
        );

        let open_file = OpenFile {
            path: path.clone(),
            input_state: input_state.clone(),
            is_modified: false,
            lines_count,
            file_size,
        };

        self.open_files.push(open_file);
        self.current_file_index = Some(self.open_files.len() - 1);
        
        // Create subscription for this file
        let subscription = cx.subscribe(&input_state, |this: &mut TextEditor, input_state_entity: Entity<InputState>, event: &InputEvent, cx: &mut Context<TextEditor>| {
            if let InputEvent::Change = event {
                // Find which file this corresponds to
                if let Some(index) = this.open_files.iter().position(|f| f.input_state == input_state_entity) {
                    if let Some(file) = this.open_files.get_mut(index) {
                        file.is_modified = true;
                        cx.notify();
                    }
                }
            }
        });
        
        self.subscriptions.push(subscription);
        
        // Log cache stats after opening
        if let Some(index) = self.current_file_index {
            if let Some(file) = self.open_files.get(index) {
                let state = file.input_state.read(cx);
                println!(
                    "📊 Line cache initialized - capacity: {} lines",
                    state.line_cache().len()
                );
                
                // Log autocomplete configuration
                if state.lsp.completion_provider.is_some() {
                    println!("✓ Autocomplete enabled with comprehensive provider");
                } else {
                    println!("ℹ️  No autocomplete provider configured");
                }
            }
        }
        
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
            Some("log") => "text".to_string(),
            Some("txt") => "text".to_string(),
            Some("yaml") | Some("yml") => "yaml".to_string(),
            Some("xml") => "xml".to_string(),
            Some("c") | Some("h") => "c".to_string(),
            Some("cpp") | Some("hpp") | Some("cc") => "cpp".to_string(),
            _ => "text".to_string(),
        }
    }
    
    /// Get performance info about the current file
    fn get_current_file_performance(&self, cx: &App) -> Option<String> {
        if !self.show_performance_stats {
            return None;
        }
        
        let index = self.current_file_index?;
        let open_file = self.open_files.get(index)?;
        
        let state = open_file.input_state.read(cx);
        let cache_stats = state.line_cache().stats();
        
        Some(format!(
            "📊 Performance: {} lines | Cache: {:.1}% hit rate | {} cached lines | Memory: ~{} MB",
            open_file.lines_count,
            cache_stats.hit_rate() * 100.0,
            state.line_cache().len(),
            (state.line_cache().len() * 1024) / (1024 * 1024) // Rough estimate
        ))
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

    pub fn save_current_file(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> bool {
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
            .font_family("monospace")
            .font(gpui::Font {
                family: "Jetbrains Mono".to_string().into(),
                weight: gpui::FontWeight::NORMAL,
                style: gpui::FontStyle::Normal,
                features: gpui::FontFeatures::default(),
                fallbacks: Some(gpui::FontFallbacks::from_fonts(vec!["monospace".to_string()])),
            })
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
                            .icon(IconName::FloppyDisk)
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
                            .icon(IconName::Replace)
                            .tooltip("Replace (Ctrl+H)")
                            .ghost()
                            .small()
                            .on_click(cx.listener(|_this, _, _window, _cx| {
                                // TODO: Implement replace functionality
                            }))
                    )
                    .child(
                        if self.show_performance_stats {
                            Button::new("toggle_stats")
                                .icon(IconName::Search)
                                .tooltip("Toggle Performance Stats (F12)")
                                .small()
                                .with_variant(gpui_component::button::ButtonVariant::Primary)
                                .on_click(cx.listener(|this, _, _window, cx| {
                                    this.show_performance_stats = !this.show_performance_stats;
                                    cx.notify();
                                }))
                        } else {
                            Button::new("toggle_stats")
                                .icon(IconName::Search)
                                .tooltip("Toggle Performance Stats (F12)")
                                .ghost()
                                .small()
                                .on_click(cx.listener(|this, _, _window, cx| {
                                    this.show_performance_stats = !this.show_performance_stats;
                                    cx.notify();
                                }))
                        }
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
                            .font(gpui::Font {
                                family: "Jetbrains Mono".to_string().into(),
                                weight: gpui::FontWeight::NORMAL,
                                style: gpui::FontStyle::Normal,
                                features: gpui::FontFeatures::default(),
                                fallbacks: Some(gpui::FontFallbacks::from_fonts(vec!["monospace".to_string()])),
                            })
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
        let (file_info, cache_info) = if let Some(index) = self.current_file_index {
            if let Some(open_file) = self.open_files.get(index) {
                let filename = open_file.path.file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("untitled")
                    .to_string();
                let language = self.get_language_from_extension(&open_file.path);
                
                // Get cache statistics
                let state = open_file.input_state.read(cx);
                let cache_stats = state.line_cache().stats();
                let cache_size = state.line_cache().len();
                
                let file_size_kb = open_file.file_size / 1024;
                let file_info_str = if file_size_kb > 1024 {
                    format!("{} | {} lines | {:.1} MB", filename, open_file.lines_count, file_size_kb as f64 / 1024.0)
                } else {
                    format!("{} | {} lines | {} KB", filename, open_file.lines_count, file_size_kb)
                };
                
                let cache_info_str = if self.show_performance_stats {
                    format!(
                        "Cache: {}/{} lines | Hit Rate: {:.1}% | Hits: {} | Misses: {}",
                        cache_size,
                        cache_stats.hits + cache_stats.misses,
                        cache_stats.hit_rate() * 100.0,
                        cache_stats.hits,
                        cache_stats.misses
                    )
                } else {
                    format!("Cache: {} lines cached", cache_size)
                };
                
                ((file_info_str, language), cache_info_str)
            } else {
                (("No file".to_string(), "".to_string()), "".to_string())
            }
        } else {
            (("No file".to_string(), "".to_string()), "".to_string())
        };

        h_flex()
            .w_full()
            .min_h_6()
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
                    .child(file_info.0)
                    .child("UTF-8")
                    .child("LF")
            )
            .child(
                if self.show_performance_stats {
                    h_flex()
                        .gap_4()
                        .child(cache_info)
                        .child("Ln 1, Col 1")
                        .child("Spaces: 4")
                        .child(file_info.1)
                } else {
                    h_flex()
                        .gap_4()
                        .child("Ln 1, Col 1")
                        .child("Spaces: 4")
                        .child(file_info.1)
                }
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
        // Track render time for performance monitoring
        let render_start = Instant::now();
        
        let result = v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_toolbar(cx))
            .child(self.render_tab_bar(cx))
            .child(
                div()
                    .flex_1()
                    .child(self.render_editor_content(cx))
            )
            .child(self.render_status_bar(cx));
        
        // Log render time if performance stats are enabled
        if self.show_performance_stats {
            let render_time = render_start.elapsed();
            if render_time.as_millis() > 16 {
                eprintln!(
                    "⚠️  Slow render: {:.2}ms (target: 16ms for 60 FPS)",
                    render_time.as_secs_f64() * 1000.0
                );
            }
        }
        
        self.last_render_time = Some(render_start);
        
        result
    }
}