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
                name: "Waiting for Rust Analyzer...".to_string(),
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
        };

        // Start initial quick tasks
        loading_window.start_loading(window, cx);
        
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

        loading_window
    }

    fn start_loading(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Start all initialization tasks asynchronously
        self.start_init_tasks(window, cx);
    }

    fn start_init_tasks(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Task 0: Renderer init (300ms)
        self.loading_tasks[0].status = TaskStatus::InProgress;
        cx.notify();
        
        cx.spawn_in(window, async move |this, mut cx| {
            cx.background_executor().timer(Duration::from_millis(300)).await;
            let _ = this.update(cx, |this, cx| {
                this.loading_tasks[0].status = TaskStatus::Completed;
                this.progress = 0.25;
                
                // Task 1: Project data
                this.loading_tasks[1].status = TaskStatus::InProgress;
                cx.notify();
            });
            
            cx.background_executor().timer(Duration::from_millis(400)).await;
            let _ = this.update(cx, |this, cx| {
                this.loading_tasks[1].status = TaskStatus::Completed;
                this.progress = 0.5;
                
                // Task 2: Starting analyzer
                this.loading_tasks[2].status = TaskStatus::InProgress;
                cx.notify();
            });
            
            cx.background_executor().timer(Duration::from_millis(500)).await;
            let _ = this.update(cx, |this, cx| {
                this.loading_tasks[2].status = TaskStatus::Completed;
                this.progress = 0.75;
                this.initial_tasks_complete = true;
                
                // Mark task 3 as in progress
                this.loading_tasks[3].status = TaskStatus::InProgress;
                cx.notify();
            });
        }).detach();
    }
    
    fn on_analyzer_event(
        &mut self,
        _analyzer: Entity<RustAnalyzerManager>,
        event: &AnalyzerEvent,
        cx: &mut Context<Self>,
    ) {
        match event {
            AnalyzerEvent::StatusChanged(status) => {
                match status {
                    AnalyzerStatus::Indexing { progress, message } => {
                        // Update the indexing task with the current crate name
                        if self.loading_tasks[3].status != TaskStatus::Completed {
                            self.loading_tasks[3].status = TaskStatus::InProgress;
                            self.loading_tasks[3].name = if message.is_empty() {
                                "Analyzing project...".to_string()
                            } else {
                                format!("Analyzing: {}", message)
                            };
                            // Update overall progress: first 3 tasks + analyzer progress
                            self.progress = (3.0 + progress) / 4.0;
                            cx.notify();
                        }
                    }
                    AnalyzerStatus::Ready => {
                        self.mark_analyzer_ready(cx);
                    }
                    AnalyzerStatus::Error(err) => {
                        // Show error but still mark as complete to avoid hanging forever
                        eprintln!("Analyzer error: {}", err);
                        if !self.analyzer_ready {
                            self.loading_tasks[3].status = TaskStatus::Completed;
                            self.loading_tasks[3].name = format!("Analyzer error: {}", err);
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
                // Update the indexing task with the current crate name
                if self.loading_tasks[3].status != TaskStatus::Completed {
                    self.loading_tasks[3].status = TaskStatus::InProgress;
                    self.loading_tasks[3].name = if message.is_empty() {
                        "Analyzing project...".to_string()
                    } else {
                        format!("Analyzing: {}", message)
                    };
                    // Update overall progress: first 3 tasks + analyzer progress
                    self.progress = (3.0 + progress) / 4.0;
                    cx.notify();
                }
            }
            AnalyzerEvent::Ready => {
                self.mark_analyzer_ready(cx);
            }
            AnalyzerEvent::Error(err) => {
                // Show error but still mark as complete to avoid hanging forever
                eprintln!("Analyzer error: {}", err);
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
                // Ignore diagnostics during loading
            }
        }
    }
    
    fn mark_analyzer_ready(&mut self, cx: &mut Context<Self>) {
        if !self.analyzer_ready {
            self.loading_tasks[3].status = TaskStatus::Completed;
            self.loading_tasks[3].name = "Rust Analyzer ready".to_string();
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
            let project_path = self.project_path.clone();
            let rust_analyzer = self.rust_analyzer.clone().expect("Rust Analyzer should be initialized");
            cx.emit(LoadingComplete { 
                project_path,
                rust_analyzer,
            });
        }
    }
}

impl Render for LoadingWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Poll rust-analyzer for progress updates
        if let Some(analyzer) = &self.rust_analyzer {
            analyzer.update(cx, |analyzer, cx| {
                analyzer.update_progress_from_thread(cx);
            });
        }
        
        let theme = cx.theme();

        div()
            .id("loading-window")
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .bg(theme.background)
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
                        // Loading progress bar
                        div()
                            .w(px(400.))
                            .h(px(4.))
                            .rounded_md()
                            .bg(theme.border)
                            .child(
                                div()
                                    .h_full()
                                    .w(relative(self.progress))
                                    .rounded_md()
                                    .bg(theme.accent)
                            )
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
                    .child(
                        // Loading percentage
                        div()
                            .mt_4()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(format!("{}%", (self.progress * 100.0) as u32))
                    )
            )
    }
}
