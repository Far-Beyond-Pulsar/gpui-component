//! Workspace panels for Script Editor

use gpui::*;
use ui::{ActiveTheme, StyledExt, dock::{Panel, PanelEvent}, v_flex};
use std::path::PathBuf;
use super::text_editor::TextEditor;
use super::file_explorer::FileExplorer;

/// Text Editor Panel - wraps the text editor with all open files
/// Note: Individual file tabs are managed internally by TextEditor
/// Future enhancement: Split each file into its own draggable panel
pub struct TextEditorPanel {
    text_editor: Entity<TextEditor>,
    focus_handle: FocusHandle,
}

impl TextEditorPanel {
    pub fn new(text_editor: Entity<TextEditor>, cx: &mut Context<Self>) -> Self {
        Self {
            text_editor,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl EventEmitter<PanelEvent> for TextEditorPanel {}

impl Render for TextEditorPanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(self.text_editor.clone())
    }
}

impl Focusable for TextEditorPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for TextEditorPanel {
    fn panel_name(&self) -> &'static str {
        "text_editor"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        "Editor".into_any_element()
    }
}

/// File Explorer Panel
pub struct FileExplorerPanel {
    file_explorer: Entity<FileExplorer>,
    focus_handle: FocusHandle,
}

impl FileExplorerPanel {
    pub fn new(file_explorer: Entity<FileExplorer>, cx: &mut Context<Self>) -> Self {
        Self {
            file_explorer,
            focus_handle: cx.focus_handle(),
        }
    }
}

impl EventEmitter<PanelEvent> for FileExplorerPanel {}

impl Render for FileExplorerPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().sidebar)
            .child(self.file_explorer.clone())
    }
}

impl Focusable for FileExplorerPanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Panel for FileExplorerPanel {
    fn panel_name(&self) -> &'static str {
        "file_explorer"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        "Files".into_any_element()
    }
}
