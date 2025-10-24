use gpui::*;
use gpui_component::{ActiveTheme, ContextModal, Root};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use super::rust_analyzer_manager::{RustAnalyzerManager, AnalyzerStatus, AnalyzerEvent};

pub struct LoadingWindow {
    project_path: PathBuf,
    loading_tasks: Vec<LoadingTask>,
    current_task_index: usize,
    progress: f32,
    rust_analyzer: Option<Entity<RustAnalyzerManager>>,
    analyzer_ready: bool,
    initial_tasks_complete: bool,
    _analyzer_subscription: Option<Subscription>,
    /// Current analyzer work message (shown at bottom left)
    analyzer_message: String,
}

#[derive(Clone)]
struct LoadingTask {
    name: String,
    status: TaskStatus,
}

#[derive(Clone, PartialEq)]
enum TaskStatus {
    Pending,
    InProgress,
    Completed,
}

pub struct LoadingComplete {
    pub project_path: PathBuf,
    pub rust_analyzer: Entity<RustAnalyzerManager>,
}

impl EventEmitter<LoadingComplete> for LoadingWindow {}

impl LoadingWindow {
    pub fn new(project_path: PathBuf, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Define all loading tasks
        let loading_tasks = vec![
            LoadingTask {
                name: "Initializing renderer...".to_string(),
                status: TaskStatus::Pending,
            },
            LoadingTask {
                name: "Loading project data...".to_string(),
                status: TaskStatus::Pending,
            },
            LoadingTask {
                name: "Starting Rust Analyzer...".to_string(),
                status: TaskStatus::Pending,
            },
            LoadingTask {
                name: "Analyzing project...".to_string(),
                status: TaskStatus::Pending,
            },
        ];

        let mut loading_window = Self {
            project_path: project_path.clone(),
            loading_tasks,
            current_task_index: 0,
            progress: 0.0,
            rust_analyzer: None,
            analyzer_ready: false,
            initial_tasks_complete: false,
            _analyzer_subscription: None,
            analyzer_message: String::new(),
        };

        // Initialize and start rust analyzer immediately (subscriptions will handle events)
        let analyzer = cx.new(|cx| RustAnalyzerManager::new(window, cx));
        
        // Subscribe to analyzer events BEFORE starting it
        let subscription = cx.subscribe(&analyzer, Self::on_analyzer_event);
        loading_window._analyzer_subscription = Some(subscription);
        
        // Start the analyzer
        analyzer.update(cx, |analyzer, cx| {
            analyzer.start(project_path.clone(), window, cx);
        });
        
        loading_window.rust_analyzer = Some(analyzer.clone());

        // Start initial quick tasks and polling AFTER analyzer is set
        loading_window.start_loading(window, cx);

        loading_window
    }

    fn start_loading(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Start all initialization tasks asynchronously
        self.start_init_tasks(window, cx);
        
        // Start polling analyzer progress in a background thread
        // The analyzer manager has a channel-based system where updates come from a background thread
        // We need to poll update_progress_from_thread() on the main thread to process those updates
        self.start_analyzer_polling(window, cx);
    }
    
    fn start_analyzer_polling(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Poll the analyzer every 100ms to process progress updates from the background thread
        // The rust analyzer manager receives events on a channel from its stdout reader thread
        // We must call update_progress_from_thread() to process those events which will emit
        // AnalyzerEvent events that our subscription (on_analyzer_event) will receive
        cx.spawn_in(window, async move |this, mut cx| {
            loop {
                cx.background_executor().timer(Duration::from_millis(100)).await;
                
                let should_continue = this.update(cx, |this, cx| {
                    // Process analyzer updates - this reads from the channel and emits events
                    if let Some(analyzer) = &this.rust_analyzer {
                        analyzer.update(cx, |analyzer, cx| {
                            analyzer.update_progress_from_thread(cx);
                        });
                    }
                    
                    // Continue polling until analyzer is ready
                    !this.analyzer_ready
                }).ok().unwrap_or(false);
                
                if !should_continue {
                    break;
                }
            }
        }).detach();
    }

    fn start_init_tasks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Task 0: Renderer init (immediate)
        self.loading_tasks[0].status = TaskStatus::InProgress;
        self.progress = 0.0;
        self.analyzer_message = "Initializing renderer...".to_string();
        cx.notify();
        
        cx.spawn_in(window, async move |this, mut cx| {
            // Minimal delay just to show the task
            cx.background_executor().timer(Duration::from_millis(100)).await;
            let _ = this.update(cx, |this, cx| {
                this.loading_tasks[0].status = TaskStatus::Completed;
                this.progress = 0.25;
                
                // Task 1: Project data
                this.loading_tasks[1].status = TaskStatus::InProgress;
                this.analyzer_message = "Loading project data...".to_string();
                cx.notify();
            });
            
            cx.background_executor().timer(Duration::from_millis(100)).await;
            let _ = this.update(cx, |this, cx| {
                this.loading_tasks[1].status = TaskStatus::Completed;
                this.progress = 0.5;
                
                // Task 2: Starting analyzer
                this.loading_tasks[2].status = TaskStatus::InProgress;
                this.analyzer_message = "Starting Rust Analyzer...".to_string();
                cx.notify();
            });
            
            cx.background_executor().timer(Duration::from_millis(100)).await;
            let _ = this.update(cx, |this, cx| {
                this.loading_tasks[2].status = TaskStatus::Completed;
                this.progress = 0.75;
                this.initial_tasks_complete = true;
                
                // Mark task 3 as in progress - waiting for analyzer
                this.loading_tasks[3].status = TaskStatus::InProgress;
                this.analyzer_message = "Starting analysis...".to_string();
                cx.notify();
                
                // Check if analyzer is already ready (unlikely but possible)
                this.check_completion(cx);
            });
        }).detach();
    }
    
    fn on_analyzer_event(
        &mut self,
        _analyzer: Entity<RustAnalyzerManager>,
        event: &AnalyzerEvent,
        cx: &mut Context<Self>,
    ) {
        println!("🔔 LoadingWindow received analyzer event: {:?}", event);
        
        match event {
            AnalyzerEvent::StatusChanged(status) => {
                println!("   Status changed to: {:?}", status);
                match status {
                    AnalyzerStatus::Indexing { progress, message } => {
                        // Update the indexing task status and detailed message
                        if !self.analyzer_ready {
                            self.loading_tasks[3].status = TaskStatus::InProgress;
                            // Keep task name simple - details go to bottom
                            self.loading_tasks[3].name = "Analyzing project...".to_string();
                            // Store detailed message for bottom display
                            self.analyzer_message = message.clone();
                            // Update overall progress: first 3 tasks (0.75) + analyzer progress (0.25)
                            self.progress = 0.75 + (progress * 0.25);
                            println!("   📊 Indexing progress: {:.1}% - {}", progress * 100.0, message);
                            cx.notify();
                        }
                    }
                    AnalyzerStatus::Ready => {
                        println!("   ✅ Analyzer ready via StatusChanged event");
                        self.mark_analyzer_ready(cx);
                    }
                    AnalyzerStatus::Error(err) => {
                        // Show error but still mark as complete to avoid hanging forever
                        eprintln!("   ❌ Analyzer error: {}", err);
                        if !self.analyzer_ready {
                            self.loading_tasks[3].status = TaskStatus::Completed;
                            self.loading_tasks[3].name = "Analyzing project...".to_string();
                            self.analyzer_message = format!("Error: {}", err);
                            self.progress = 1.0;
                            self.analyzer_ready = true;
                            cx.notify();
                            self.check_completion(cx);
                        }
                    }
                    _ => {}
                }
            }
            AnalyzerEvent::IndexingProgress { progress, message } => {
                println!("   📈 Indexing progress: {:.1}% - {}", progress * 100.0, message);
                // Update the indexing task status and detailed message
                if !self.analyzer_ready {
                    self.loading_tasks[3].status = TaskStatus::InProgress;
                    // Keep task name simple - details go to bottom
                    self.loading_tasks[3].name = "Analyzing project...".to_string();
                    // Store detailed message for bottom display
                    self.analyzer_message = message.clone();
                    // Update overall progress: first 3 tasks (0.75) + analyzer progress (0.25)
                    self.progress = 0.75 + (progress * 0.25);
                    cx.notify();
                }
            }
            AnalyzerEvent::Ready => {
                println!("   ✅ Analyzer ready via Ready event");
                self.mark_analyzer_ready(cx);
            }
            AnalyzerEvent::Error(err) => {
                // Show error but still mark as complete to avoid hanging forever
                eprintln!("   ❌ Analyzer error: {}", err);
                if !self.analyzer_ready {
                    self.loading_tasks[3].status = TaskStatus::Completed;
                    self.loading_tasks[3].name = format!("Analyzer error: {}", err);
                    self.progress = 1.0;
                    self.analyzer_ready = true;
                    cx.notify();
                    self.check_completion(cx);
                }
            }
            AnalyzerEvent::Diagnostics(_) => {
                // Diagnostics indicate the analyzer is working - this is a good sign
                // But don't mark as ready yet, wait for the explicit Ready event
                println!("   📋 Received diagnostics from analyzer (working...)");
            }
        }
    }
    
    fn mark_analyzer_ready(&mut self, cx: &mut Context<Self>) {
        if !self.analyzer_ready {
            println!("✅ Marking analyzer as ready in LoadingWindow");
            self.loading_tasks[3].status = TaskStatus::Completed;
            self.loading_tasks[3].name = "Analyzing project...".to_string();
            self.analyzer_message = "Analysis complete".to_string();
            self.progress = 1.0;
            self.analyzer_ready = true;
            cx.notify();
            
            // Check if we should complete the loading process
            self.check_completion(cx);
        }
    }
    
    fn check_completion(&mut self, cx: &mut Context<Self>) {
        // Only complete when initial tasks are done AND analyzer is ready
        if self.initial_tasks_complete && self.analyzer_ready {
            println!("🎉 Loading complete! Opening main window...");
            let project_path = self.project_path.clone();
            let rust_analyzer = self.rust_analyzer.clone().expect("Rust Analyzer should be initialized");
            cx.emit(LoadingComplete { 
                project_path,
                rust_analyzer,
            });
        } else {
            println!("⏳ Waiting for completion - initial_tasks: {}, analyzer_ready: {}", 
                self.initial_tasks_complete, self.analyzer_ready);
        }
    }
}

impl Render for LoadingWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        div()
            .id("loading-window")
            .flex()
            .flex_col()
            .size_full()
            .bg(theme.background)
            .child(
                // Main content area (centered)
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .justify_center()
                    .flex_1()
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap_4()
                            .child(
                                // Logo/Title
                                div()
                                    .text_xl()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme.foreground)
                                    .child("Pulsar Engine")
                            )
                            .child(
                                // Task list
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_2()
                                    .mt_4()
                                    .children(
                                        self.loading_tasks.iter().map(|task| {
                                            let color = match task.status {
                                                TaskStatus::Pending => theme.muted_foreground,
                                                TaskStatus::InProgress => theme.accent,
                                                TaskStatus::Completed => theme.success_foreground,
                                            };
                                            let icon = match task.status {
                                                TaskStatus::Pending => "○",
                                                TaskStatus::InProgress => "◐",
                                                TaskStatus::Completed => "●",
                                            };

                                            div()
                                                .flex()
                                                .items_center()
                                                .gap_2()
                                                .child(
                                                    div()
                                                        .text_color(color)
                                                        .child(icon)
                                                )
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .text_color(color)
                                                        .child(task.name.clone())
                                                )
                                        })
                                    )
                            )
                    )
            )
            .child(
                // Bottom section with current work and progress bar
                div()
                    .flex()
                    .flex_col()
                    .w_full()
                    .child(
                        // Current work indicator (bottom left, can truncate)
                        div()
                            .px_4()
                            .pb_2()
                            .w_full()
                            .overflow_hidden()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .whitespace_nowrap()
                                    .overflow_hidden()
                                    .child(
                                        // Show the analyzer message if available, otherwise show current task
                                        if !self.analyzer_message.is_empty() {
                                            self.analyzer_message.clone()
                                        } else {
                                            self.loading_tasks.iter()
                                                .find(|t| t.status == TaskStatus::InProgress)
                                                .map(|t| t.name.clone())
                                                .unwrap_or_else(|| "Initializing...".to_string())
                                        }
                                    )
                            )
                    )
                    .child(
                        // Progress bar at the very bottom edge
                        div()
                            .w_full()
                            .h(px(4.))
                            .bg(theme.border)
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(self.progress))
                                    .bg(theme.accent)
                            )
                    )
            )
    }
}
