//! Workspace-based rendering for Blueprint Editor
//!
//! This uses the modular workspace system for fully dockable panels

use gpui::*;
use ui::{
    workspace::{Workspace, WorkspacePanel},
    dock::{DockItem, DockPlacement},
    v_flex, h_flex, ActiveTheme, IconName, StyledExt,
};
use super::core::BlueprintEditorPanel;
use super::super::toolbar::ToolbarRenderer;
use super::super::node_graph::NodeGraphRenderer;
use std::sync::Arc;

impl BlueprintEditorPanel {
    /// Initialize the workspace with all dockable panels
    pub(super) fn initialize_workspace(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.workspace.is_some() {
            return; // Already initialized
        }

        let editor_weak = cx.entity().downgrade();
        let workspace = cx.new(|cx| {
            let mut workspace = Workspace::new("blueprint-editor-workspace", window, cx);

            // Create all dockable panels
            let variables_panel = Self::create_variables_panel(editor_weak.clone(), window, cx);
            let macros_panel = Self::create_macros_panel(editor_weak.clone(), window, cx);
            let compiler_panel = Self::create_compiler_panel(editor_weak.clone(), window, cx);
            let find_panel = Self::create_find_panel(editor_weak.clone(), window, cx);
            let properties_panel = Self::create_properties_panel(editor_weak.clone(), window, cx);
            let palette_panel = Self::create_palette_panel(editor_weak.clone(), window, cx);

            // Create center (main graph canvas)
            let center_panel = Self::create_center_panel(editor_weak.clone(), window, cx);
            let center = DockItem::tabs(
                vec![Arc::new(center_panel)],
                None,
                &cx.entity().downgrade(),
                window,
                cx,
            );

            // Create left dock (My Blueprint section)
            let left = DockItem::tabs(
                vec![Arc::new(variables_panel), Arc::new(macros_panel)],
                None,
                &cx.entity().downgrade(),
                window,
                cx,
            );

            // Create bottom dock (Utilities section)
            let bottom = DockItem::tabs(
                vec![Arc::new(compiler_panel), Arc::new(find_panel)],
                None,
                &cx.entity().downgrade(),
                window,
                cx,
            );

            // Create right dock (Inspector section)
            let right = DockItem::tabs(
                vec![Arc::new(properties_panel), Arc::new(palette_panel)],
                None,
                &cx.entity().downgrade(),
                window,
                cx,
            );

            workspace.initialize(center, Some(left), Some(right), Some(bottom), window, cx);
            workspace
        });

        self.workspace = Some(workspace);
    }

    /// Create Variables panel
    fn create_variables_panel(
        editor_weak: WeakEntity<Self>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<WorkspacePanel> {
        cx.new(|cx| {
            WorkspacePanel::new("blueprint-variables", "Variables", move |_window, panel_cx| {
                if let Some(editor) = editor_weak.upgrade() {
                    v_flex()
                        .size_full()
                        .bg(panel_cx.theme().sidebar)
                        .child(
                            editor.read(panel_cx)
                                .render_variables_list(panel_cx)
                        )
                        .into_any_element()
                } else {
                    div().child("Editor not available").into_any_element()
                }
            }, cx)
            .closable(true)
        })
    }

    /// Create Macros panel
    fn create_macros_panel(
        editor_weak: WeakEntity<Self>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<WorkspacePanel> {
        cx.new(|cx| {
            WorkspacePanel::new("blueprint-macros", "Macros", move |_window, panel_cx| {
                if let Some(editor) = editor_weak.upgrade() {
                    v_flex()
                        .size_full()
                        .bg(panel_cx.theme().sidebar)
                        .child(
                            editor.read(panel_cx).render_macros_list(panel_cx)
                        )
                        .into_any_element()
                } else {
                    div().child("Editor not available").into_any_element()
                }
            }, cx)
            .closable(true)
        })
    }

    /// Create Compiler panel
    fn create_compiler_panel(
        editor_weak: WeakEntity<Self>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<WorkspacePanel> {
        cx.new(|cx| {
            WorkspacePanel::new("blueprint-compiler", "Compiler", move |_window, panel_cx| {
                if let Some(editor) = editor_weak.upgrade() {
                    editor.read(panel_cx).render_compiler_results(panel_cx).into_any_element()
                } else {
                    div().child("Editor not available").into_any_element()
                }
            }, cx)
            .closable(true)
        })
    }

    /// Create Find panel
    fn create_find_panel(
        editor_weak: WeakEntity<Self>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<WorkspacePanel> {
        cx.new(|cx| {
            WorkspacePanel::new("blueprint-find", "Find", move |_window, panel_cx| {
                if let Some(editor) = editor_weak.upgrade() {
                    editor.read(panel_cx).render_find_panel(panel_cx).into_any_element()
                } else {
                    div().child("Editor not available").into_any_element()
                }
            }, cx)
            .closable(true)
        })
    }

    /// Create Properties panel
    fn create_properties_panel(
        editor_weak: WeakEntity<Self>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<WorkspacePanel> {
        cx.new(|cx| {
            WorkspacePanel::new("blueprint-properties", "Details", move |_window, panel_cx| {
                if let Some(editor) = editor_weak.upgrade() {
                    v_flex()
                        .size_full()
                        .bg(panel_cx.theme().sidebar)
                        .child(
                            editor.read(panel_cx).render_properties_panel(panel_cx)
                        )
                        .into_any_element()
                } else {
                    div().child("Editor not available").into_any_element()
                }
            }, cx)
            .closable(true)
        })
    }

    /// Create Palette panel
    fn create_palette_panel(
        editor_weak: WeakEntity<Self>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<WorkspacePanel> {
        cx.new(|cx| {
            WorkspacePanel::new("blueprint-palette", "Palette", move |_window, panel_cx| {
                if let Some(editor) = editor_weak.upgrade() {
                    v_flex()
                        .size_full()
                        .bg(panel_cx.theme().sidebar)
                        .child(
                            editor.read(panel_cx).render_node_library(panel_cx)
                        )
                        .into_any_element()
                } else {
                    div().child("Editor not available").into_any_element()
                }
            }, cx)
            .closable(true)
        })
    }

    /// Create Center panel (main graph canvas with tabs)
    fn create_center_panel(
        editor_weak: WeakEntity<Self>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<WorkspacePanel> {
        cx.new(|cx| {
            WorkspacePanel::new("blueprint-graph", "Event Graph", move |window, panel_cx| {
                if let Some(editor) = editor_weak.upgrade() {
                    v_flex()
                        .size_full()
                        .child(
                            // Tab bar for graph navigation
                            editor.read(panel_cx).render_tab_bar(panel_cx)
                        )
                        .child(
                            // Main node graph
                            div()
                                .flex_1()
                                .min_h_0()
                                .child(NodeGraphRenderer::render(&editor.read(panel_cx), panel_cx))
                        )
                        .into_any_element()
                } else {
                    div().child("Editor not available").into_any_element()
                }
            }, cx)
            .closable(false) // Main graph shouldn't be closable
        })
    }

    // Helper methods to render panel content

    pub(super) fn render_variables_list(&self, cx: &App) -> impl IntoElement {
        super::super::variables::VariablesRenderer::render(self, cx)
    }

    pub(super) fn render_macros_list(&self, cx: &App) -> impl IntoElement {
        super::super::macros::MacrosRenderer::render(self, cx)
    }

    pub(super) fn render_properties_panel(&self, cx: &App) -> impl IntoElement {
        super::super::properties::PropertiesRenderer::render(self, cx)
    }

    pub(super) fn render_node_library(&self, cx: &App) -> impl IntoElement {
        super::super::node_library::NodeLibraryRenderer::render(self, cx)
    }
}
