use gpui::*;
use gpui_component::{ActiveTheme, ContextModal, Root};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use smol::Timer;
use super::rust_analyzer_manager::RustAnalyzerManager;

pub struct LoadingWindow {
    project_path: PathBuf,
    loading_tasks: Vec<LoadingTask>,
    current_task_index: usize,
    progress: f32,
    rust_analyzer: Option<Entity<RustAnalyzerManager>>,
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
                name: "Preparing workspace...".to_string(),
                status: TaskStatus::Pending,
            },
        ];

        let mut loading_window = Self {
            project_path: project_path.clone(),
            loading_tasks,
            current_task_index: 0,
            progress: 0.0,
            rust_analyzer: None,
        };

        // Start the loading process
        loading_window.start_loading(window, cx);

        loading_window
    }

    fn start_loading(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Process tasks sequentially with real initialization
        self.process_next_task(window, cx);
    }

    fn process_next_task(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.current_task_index >= self.loading_tasks.len() {
            // All tasks complete - emit event with initialized resources
            let project_path = self.project_path.clone();
            let rust_analyzer = self.rust_analyzer.clone().expect("Rust Analyzer should be initialized");
            cx.emit(LoadingComplete { 
                project_path,
                rust_analyzer,
            });
            return;
        }

        // Mark current task as in progress
        self.loading_tasks[self.current_task_index].status = TaskStatus::InProgress;
        cx.notify();

        let task_index = self.current_task_index;
        
        // Execute actual initialization tasks
        match task_index {
            0 => {
                // Task 0: Renderer init (simulated as it's already initialized)
                cx.spawn(|this, mut cx: AsyncApp| async move {
                    Timer::after(Duration::from_millis(300)).await;
                    let _ = cx.update(|cx| {
                        this.update(cx, |this, cx| {
                            this.complete_task(task_index, cx);
                        })
                    });
                }).detach();
            }
            1 => {
                // Task 1: Load project data (simulated)
                cx.spawn(|this, mut cx: AsyncApp| async move {
                    Timer::after(Duration::from_millis(400)).await;
                    let _ = cx.update(|cx| {
                        this.update(cx, |this, cx| {
                            this.complete_task(task_index, cx);
                        })
                    });
                }).detach();
            }
            2 => {
                // Task 2: Initialize Rust Analyzer - REAL initialization
                let project_path = self.project_path.clone();
                let analyzer = cx.new(|cx| RustAnalyzerManager::new(window, cx));
                
                // Start the analyzer
                analyzer.update(cx, |analyzer, cx| {
                    analyzer.start(project_path.clone(), window, cx);
                });
                
                self.rust_analyzer = Some(analyzer.clone());
                
                // Wait a bit for analyzer to start (it will continue indexing in background)
                cx.spawn(|this, mut cx: AsyncApp| async move {
                    Timer::after(Duration::from_millis(800)).await;
                    let _ = cx.update(|cx| {
                        this.update(cx, |this, cx| {
                            this.complete_task(task_index, cx);
                        })
                    });
                }).detach();
            }
            3 => {
                // Task 3: Prepare workspace (simulated)
                cx.spawn(|this, mut cx: AsyncApp| async move {
                    Timer::after(Duration::from_millis(300)).await;
                    let _ = cx.update(|cx| {
                        this.update(cx, |this, cx| {
                            this.complete_task(task_index, cx);
                        })
                    });
                }).detach();
            }
            _ => {
                // Unknown task, just complete it
                self.complete_task(task_index, cx);
            }
        }
    }
    
    fn complete_task(&mut self, task_index: usize, cx: &mut Context<Self>) {
        self.loading_tasks[task_index].status = TaskStatus::Completed;
        self.current_task_index += 1;
        self.progress = (self.current_task_index as f32) / (self.loading_tasks.len() as f32);
        cx.notify();
        
        // Trigger next task - we'll handle this through the window update pattern
        let project_path = self.project_path.clone();
        cx.defer(move |this, cx| {
            cx.update_window(cx.entity(&this).unwrap(), |_, window, cx| {
                this.update(cx, |this, cx| {
                    this.process_next_task(window, cx);
                })
            }).ok();
        });
    }
}

impl Render for LoadingWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
