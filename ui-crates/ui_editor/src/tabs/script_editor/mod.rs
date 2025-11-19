mod file_explorer;
pub mod text_editor;
mod autocomplete_integration;
mod workspace_panels;

pub use file_explorer::FileExplorer;
pub use text_editor::{TextEditor, TextEditorEvent};
pub use autocomplete_integration::*;
pub use workspace_panels::*;

use std::path::PathBuf;
use gpui::*;
use ui::{
    dock::{Panel, PanelEvent, DockItem, DockPlacement, DockChannel},
    workspace::Workspace,
    resizable::{h_resizable, resizable_panel, ResizableState},
    h_flex,
    ActiveTheme,
};

use engine_backend::services::rust_analyzer_manager::RustAnalyzerManager;

actions!(script_editor, [SaveCurrentFile, CloseCurrentFile]);

pub struct ScriptEditor {
    focus_handle: FocusHandle,
    file_explorer: Entity<FileExplorer>,
    text_editor: Entity<TextEditor>,
    horizontal_resizable_state: Entity<ResizableState>,
    /// Global rust analyzer for LSP support
    rust_analyzer: Option<Entity<RustAnalyzerManager>>,
    /// Workspace for draggable file tabs
    workspace: Option<Entity<Workspace>>,
}

impl ScriptEditor {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.bind_keys([
            KeyBinding::new("ctrl-s", SaveCurrentFile, Some("ScriptEditor")),
            KeyBinding::new("ctrl-w", CloseCurrentFile, Some("ScriptEditor")),
        ]);

        let horizontal_resizable_state = ResizableState::new(cx);

        let file_explorer = cx.new(|cx| FileExplorer::new(window, cx));
        let text_editor = cx.new(|cx| TextEditor::new(window, cx));

        // Forward text editor events
        cx.subscribe(&text_editor, |this: &mut Self, _editor, event: &TextEditorEvent, cx| {
            cx.emit(event.clone());
        }).detach();

        Self {
            focus_handle: cx.focus_handle(),
            file_explorer,
            text_editor,
            horizontal_resizable_state,
            rust_analyzer: None,
            workspace: None,
        }
    }
    
    /// Initialize workspace with text editor as center panel
    fn initialize_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspace.is_some() {
            return;
        }
        
        let workspace = cx.new(|cx| {
            // Use channel 2 for script editor to isolate from main app dock (channel 0) and BP editor (channel 1)
            Workspace::new_with_channel(
                "script-editor-workspace",
                ui::dock::DockChannel(2),
                window,
                cx
            )
        });
        
        // Initialize workspace with text editor in center
        workspace.update(cx, |workspace, cx| {
            // Create a wrapper panel for the text editor
            let text_editor_panel = cx.new(|cx| {
                TextEditorPanel::new(self.text_editor.clone(), cx)
            });
            
            // Create file explorer panel
            let file_explorer_panel = cx.new(|cx| {
                FileExplorerPanel::new(self.file_explorer.clone(), cx)
            });
            
            // Initialize with file explorer on left, text editor in center
            workspace.initialize(
                DockItem::Panel { view: std::sync::Arc::new(text_editor_panel) },
                Some(DockItem::Panel { view: std::sync::Arc::new(file_explorer_panel) }),
                None,
                None,
                window,
                cx,
            );
        });
        
        self.workspace = Some(workspace);
    }

    /// Set the global rust analyzer manager
    pub fn set_rust_analyzer(&mut self, analyzer: Entity<RustAnalyzerManager>, cx: &mut Context<Self>) {
        println!("🔧 ScriptEditor::set_rust_analyzer called");
        self.rust_analyzer = Some(analyzer.clone());
        
        // Pass it to the text editor
        self.text_editor.update(cx, |editor, cx| {
            println!("🔧 Passing rust-analyzer to TextEditor");
            editor.set_rust_analyzer(analyzer.clone(), cx);
        });
        
        // Subscribe to text editor events to forward to rust-analyzer
        let analyzer_for_sub = analyzer.clone();
        cx.subscribe(&self.text_editor, move |this: &mut Self, _editor, event: &TextEditorEvent, cx| {
            println!("📨 ScriptEditor received TextEditorEvent: {:?}", std::mem::discriminant(event));
            if let Some(ref analyzer) = this.rust_analyzer {
                match event {
                    TextEditorEvent::FileOpened { path, content } => {
                        println!("📂 ScriptEditor handling FileOpened: {:?}", path);
                        // Notify rust-analyzer that a file was opened
                        analyzer.update(cx, |analyzer, _cx| {
                            let language_id = if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                                "rust"
                            } else {
                                "text"
                            };
                            
                            println!("🚀 Calling did_open_file for {:?} (language: {})", path.file_name(), language_id);
                            if let Err(e) = analyzer.did_open_file(path, content, language_id) {
                                eprintln!("⚠️  Failed to notify rust-analyzer of file open: {}", e);
                            } else {
                                println!("✓ Notified rust-analyzer: file opened {:?}", path.file_name());
                            }
                        });
                    }
                    TextEditorEvent::FileSaved { path, content } => {
                        println!("💾 ScriptEditor handling FileSaved: {:?}", path);
                        // Notify rust-analyzer that a file was saved
                        analyzer.update(cx, |analyzer, _cx| {
                            if let Err(e) = analyzer.did_save_file(path, content) {
                                eprintln!("⚠️  Failed to notify rust-analyzer of file save: {}", e);
                            } else {
                                println!("✓ Notified rust-analyzer: file saved {:?}", path.file_name());
                            }
                        });
                    }
                    TextEditorEvent::FileClosed { path } => {
                        println!("❌ ScriptEditor handling FileClosed: {:?}", path);
                        // Notify rust-analyzer that a file was closed
                        analyzer.update(cx, |analyzer, _cx| {
                            if let Err(e) = analyzer.did_close_file(path) {
                                eprintln!("⚠️  Failed to notify rust-analyzer of file close: {}", e);
                            } else {
                                println!("✓ Notified rust-analyzer: file closed {:?}", path.file_name());
                            }
                        });
                    }
                    _ => {}
                }
            } else {
                println!("⚠️  ScriptEditor: rust_analyzer is None!");
            }
        }).detach();
        
        println!("✓ ScriptEditor rust-analyzer setup complete");
    }
    
    /// Set the project path and load it in the file explorer
    pub fn set_project_path(&mut self, project_path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        println!("📁 ScriptEditor::set_project_path called with: {:?}", project_path);
        self.file_explorer.update(cx, |explorer, cx| {
            explorer.open_project(project_path, window, cx);
        });
    }

    pub fn open_file(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.text_editor.update(cx, |editor, cx| {
            editor.open_file(path, window, cx);
        });
    }

    pub fn get_file_explorer(&self) -> &Entity<FileExplorer> {
        &self.file_explorer
    }

    pub fn get_text_editor(&self) -> &Entity<TextEditor> {
        &self.text_editor
    }

    /// Navigate to a specific line and column in the currently open file
    pub fn go_to_line(&mut self, line: usize, column: usize, window: &mut Window, cx: &mut Context<Self>) {
        self.text_editor.update(cx, |editor, cx| {
            editor.go_to_line(line, column, window, cx);
        });
    }

    fn save_current_file(&mut self, _action: &SaveCurrentFile, window: &mut Window, cx: &mut Context<Self>) {
        self.text_editor.update(cx, |editor, cx| {
            editor.save_current_file(window, cx);
        });
    }
    
    fn close_current_file(&mut self, _action: &CloseCurrentFile, window: &mut Window, cx: &mut Context<Self>) {
        self.text_editor.update(cx, |editor, cx| {
            editor.close_current_file(window, cx);
        });
    }
}

impl Panel for ScriptEditor {
    fn panel_name(&self) -> &'static str {
        "Script Editor"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        div().child("Script Editor").into_any_element()
    }

    fn dump(&self, _cx: &App) -> ui::dock::PanelState {
        ui::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}

impl Focusable for ScriptEditor {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for ScriptEditor {}
impl EventEmitter<crate::tabs::script_editor::text_editor::TextEditorEvent> for ScriptEditor {}

impl Render for ScriptEditor {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Initialize workspace on first render
        self.initialize_workspace(window, cx);
        
        // Check for file opening requests from the file explorer
        if let Some(path) = self.file_explorer.update(cx, |explorer, _| explorer.get_last_opened_file()) {
            self.open_file(path, window, cx);
        }
        
        div()
            .size_full()
            .bg(cx.theme().background)
            .key_context("ScriptEditor")
            .on_action(cx.listener(Self::save_current_file))
            .on_action(cx.listener(Self::close_current_file))
            .child(
                if let Some(ref workspace) = self.workspace {
                    workspace.clone().into_any_element()
                } else {
                    div().child("Loading workspace...").into_any_element()
                }
            )
    }
}