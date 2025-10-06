/// Global Rust Analyzer Manager for the Pulsar Engine
/// Manages a single rust-analyzer instance for the entire project

use anyhow::{anyhow, Result};
use gpui::{App, Context, Entity, EventEmitter, Task, Window};
use serde_json::Value;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::io::{BufRead, BufReader, Write};
use std::time::Instant;

#[derive(Clone, Debug, PartialEq)]
pub enum AnalyzerStatus {
    Idle,
    Starting,
    Indexing { progress: f32, message: String },
    Ready,
    Error(String),
    Stopped,
}

#[derive(Clone, Debug)]
pub enum AnalyzerEvent {
    StatusChanged(AnalyzerStatus),
    IndexingProgress { progress: f32, message: String },
    Ready,
    Error(String),
}

pub struct RustAnalyzerManager {
    /// Path to rust-analyzer executable
    analyzer_path: PathBuf,
    /// Current workspace root
    workspace_root: Option<PathBuf>,
    /// LSP process handle
    process: Arc<Mutex<Option<Child>>>,
    /// Current status
    status: AnalyzerStatus,
    /// Whether the manager is initialized
    initialized: bool,
    /// Last indexing update
    last_update: Option<Instant>,
    /// Number of requests sent
    request_id: Arc<Mutex<i64>>,
}

impl EventEmitter<AnalyzerEvent> for RustAnalyzerManager {}

impl RustAnalyzerManager {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let analyzer_path = Self::find_or_use_bundled_analyzer();
        
        println!("🔧 Rust Analyzer Manager initialized");
        println!("   Using: {:?}", analyzer_path);
        
        Self {
            analyzer_path,
            workspace_root: None,
            process: Arc::new(Mutex::new(None)),
            status: AnalyzerStatus::Idle,
            initialized: false,
            last_update: None,
            request_id: Arc::new(Mutex::new(0)),
        }
    }

    /// Find rust-analyzer in PATH or use bundled version
    fn find_or_use_bundled_analyzer() -> PathBuf {
        // Try to find in PATH first
        let candidates = vec![
            "rust-analyzer",
            "rust-analyzer.exe",
        ];

        for candidate in &candidates {
            if let Ok(output) = Command::new(candidate).arg("--version").output() {
                if output.status.success() {
                    println!("✓ Found system rust-analyzer");
                    return PathBuf::from(candidate);
                }
            }
        }

        // Check cargo bin directory
        if let Ok(home) = std::env::var("CARGO_HOME") {
            let cargo_bin = PathBuf::from(home).join("bin").join("rust-analyzer");
            if cargo_bin.exists() {
                println!("✓ Found rust-analyzer in cargo bin");
                return cargo_bin;
            }
        }

        if let Ok(home) = std::env::var("HOME") {
            let cargo_bin = PathBuf::from(home).join(".cargo").join("bin").join("rust-analyzer");
            if cargo_bin.exists() {
                println!("✓ Found rust-analyzer in ~/.cargo/bin");
                return cargo_bin;
            }
        }

        // TODO: Use bundled rust-analyzer binary
        // For now, just use the system command and hope it's in PATH
        println!("⚠️  rust-analyzer not found, will try 'rust-analyzer' command");
        PathBuf::from("rust-analyzer")
    }

    /// Start rust-analyzer for the given workspace
    pub fn start(&mut self, workspace_root: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        println!("🚀 Starting rust-analyzer for: {:?}", workspace_root);
        
        self.workspace_root = Some(workspace_root.clone());
        self.status = AnalyzerStatus::Starting;
        cx.emit(AnalyzerEvent::StatusChanged(AnalyzerStatus::Starting));
        cx.notify();

        // Stop existing process if any
        self.stop_internal();

        // Start new process
        match self.spawn_process(&workspace_root) {
            Ok(child) => {
                let mut process_lock = self.process.lock().unwrap();
                *process_lock = Some(child);
                drop(process_lock);

                // Initialize the LSP session
                self.initialize_lsp(workspace_root, window, cx);
            }
            Err(e) => {
                let error_msg = format!("Failed to start rust-analyzer: {}", e);
                eprintln!("❌ {}", error_msg);
                self.status = AnalyzerStatus::Error(error_msg.clone());
                cx.emit(AnalyzerEvent::Error(error_msg));
                cx.notify();
            }
        }
    }

    fn spawn_process(&self, _workspace_root: &PathBuf) -> Result<Child> {
        let child = Command::new(&self.analyzer_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        Ok(child)
    }

    fn initialize_lsp(&mut self, workspace_root: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let uri = format!("file://{}", workspace_root.display().to_string().replace("\\", "/"));

        // Simplified initialization - just send a basic init request
        let process = self.process.clone();
        let request_id = self.request_id.clone();
        
        cx.spawn_in(window, async move |manager, cx| {
            if let Ok(mut process_lock) = process.lock() {
                if let Some(child) = process_lock.as_mut() {
                    let mut req_id = request_id.lock().unwrap();
                    *req_id += 1;
                    let id = *req_id;
                    drop(req_id);

                    // Create a simple initialization request
                    let init_request = serde_json::json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "method": "initialize",
                        "params": {
                            "processId": std::process::id(),
                            "rootUri": uri,
                            "capabilities": {},
                        },
                    });

                    if let Some(stdin) = child.stdin.as_mut() {
                        let content = serde_json::to_string(&init_request).unwrap();
                        let message = format!("Content-Length: {}\r\n\r\n{}", content.len(), content);
                        
                        if let Err(e) = stdin.write_all(message.as_bytes()) {
                            eprintln!("❌ Failed to write initialize request: {}", e);
                        } else if let Err(e) = stdin.flush() {
                            eprintln!("❌ Failed to flush: {}", e);
                        } else {
                            println!("✓ Sent initialize request to rust-analyzer");
                            
                            // Update status to indexing
                            manager.update_in(cx, |manager, window, cx| {
                                manager.status = AnalyzerStatus::Indexing {
                                    progress: 0.0,
                                    message: "Starting indexing...".to_string(),
                                };
                                cx.emit(AnalyzerEvent::StatusChanged(manager.status.clone()));
                                cx.notify();
                            }).ok();
                        }
                    }
                }
            }
        }).detach();
    }

    /// Stop rust-analyzer
    pub fn stop(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        println!("🛑 Stopping rust-analyzer");
        self.stop_internal();
        self.status = AnalyzerStatus::Stopped;
        cx.emit(AnalyzerEvent::StatusChanged(AnalyzerStatus::Stopped));
        cx.notify();
    }

    fn stop_internal(&mut self) {
        let mut process_lock = self.process.lock().unwrap();
        if let Some(mut child) = process_lock.take() {
            let _ = child.kill();
            let _ = child.wait();
            println!("✓ rust-analyzer process terminated");
        }
        self.initialized = false;
    }

    /// Restart rust-analyzer
    pub fn restart(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        println!("🔄 Restarting rust-analyzer");
        if let Some(workspace) = self.workspace_root.clone() {
            self.stop(window, cx);
            self.start(workspace, window, cx);
        }
    }

    /// Get current status
    pub fn status(&self) -> &AnalyzerStatus {
        &self.status
    }

    /// Check if analyzer is running
    pub fn is_running(&self) -> bool {
        matches!(
            self.status,
            AnalyzerStatus::Starting
                | AnalyzerStatus::Indexing { .. }
                | AnalyzerStatus::Ready
        )
    }

    /// Simulate progress updates (in real implementation, parse LSP notifications)
    pub fn simulate_indexing_progress(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.status, AnalyzerStatus::Indexing { .. }) {
            return;
        }

        // Simulate progress
        let (progress, message) = match &self.status {
            AnalyzerStatus::Indexing { progress, .. } => {
                let new_progress = (progress + 0.1).min(1.0);
                let new_message = if new_progress < 0.3 {
                    "Parsing crates...".to_string()
                } else if new_progress < 0.6 {
                    "Building type information...".to_string()
                } else if new_progress < 0.9 {
                    "Indexing symbols...".to_string()
                } else if new_progress < 1.0 {
                    "Finalizing...".to_string()
                } else {
                    "Ready".to_string()
                };
                (new_progress, new_message)
            }
            _ => return,
        };

        if progress >= 1.0 {
            self.status = AnalyzerStatus::Ready;
            cx.emit(AnalyzerEvent::Ready);
            println!("✅ rust-analyzer ready");
        } else {
            self.status = AnalyzerStatus::Indexing {
                progress,
                message: message.clone(),
            };
            cx.emit(AnalyzerEvent::IndexingProgress {
                progress,
                message,
            });
        }

        cx.notify();
    }
}

impl Drop for RustAnalyzerManager {
    fn drop(&mut self) {
        self.stop_internal();
    }
}
