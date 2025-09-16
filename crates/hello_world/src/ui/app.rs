use gpui::*;
use gpui_component::{
    dock::{DockArea, DockItem, Panel, PanelEvent},
    v_flex, h_flex, ActiveTheme as _, StyledExt,
};
use std::sync::Arc;

use super::{
    editors::EditorType,
    menu::AppTitleBar,
    panels::{
        LevelEditorPanel, ScriptEditorPanel, BlueprintEditorPanel,
        MaterialEditorPanel,
    },
};

pub struct PulsarApp {
    dock_area: Entity<DockArea>,
}

impl PulsarApp {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let dock_area = cx.new(|cx| DockArea::new("main-dock", Some(1), window, cx));

        // Create the initial editor panels using modular components
        let level_editor = cx.new(|cx| LevelEditorPanel::new(window, cx));
        let script_editor = cx.new(|cx| ScriptEditorPanel::new(window, cx));
        let blueprint_editor = cx.new(|cx| BlueprintEditorPanel::new(window, cx));
        let material_editor = cx.new(|cx| MaterialEditorPanel::new(window, cx));

        // Set up the center area with tabs for all editors
        let weak_dock = dock_area.downgrade();
        let center_tabs = DockItem::tabs(
            vec![
                Arc::new(level_editor),
                Arc::new(script_editor),
                Arc::new(blueprint_editor),
                Arc::new(material_editor),
            ],
            Some(0),
            &weak_dock,
            window,
            cx,
        );

        dock_area.update(cx, |dock, cx| {
            dock.set_center(center_tabs, window, cx);
        });

        Self { dock_area }
    }
}

impl Render for PulsarApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                // Menu bar
                {
                    let title_bar = cx.new(|cx| AppTitleBar::new("Pulsar Engine", window, cx));
                    title_bar.clone()
                }
            )
            .child(
                // Main dock area
                div()
                    .flex_1()
                    .child(self.dock_area.clone())
            )
    }
}

pub struct EditorPanel {
    editor_type: EditorType,
    focus_handle: FocusHandle,
}

impl EditorPanel {
    pub fn new(editor_type: EditorType, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            editor_type,
            focus_handle: cx.focus_handle(),
        }
    }

    pub fn view(
        editor_type: EditorType,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        cx.new(|cx| Self::new(editor_type, window, cx))
    }
}

impl Panel for EditorPanel {
    fn panel_name(&self) -> &'static str {
        self.editor_type.display_name()
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        div().child(self.editor_type.display_name()).into_any_element()
    }

    fn dump(&self, _cx: &App) -> gpui_component::dock::PanelState {
        gpui_component::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}

impl Focusable for EditorPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for EditorPanel {}

impl Render for EditorPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_editor_content(cx)
    }
}

impl EditorPanel {
    fn render_editor_content(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                // Header
                h_flex()
                    .w_full()
                    .p_4()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .justify_between()
                    .items_center()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_lg()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child(self.editor_type.display_name())
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(self.editor_type.description())
                            )
                    )
            )
            .child(
                // Content
                div()
                    .flex_1()
                    .p_4()
                    .overflow_hidden()
                    .child(self.render_specific_editor(cx))
            )
    }

    fn render_specific_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        match self.editor_type {
            EditorType::Level => self.render_level_editor(cx).into_any_element(),
            EditorType::Script => self.render_script_editor(cx).into_any_element(),
            EditorType::Blueprint => self.render_blueprint_editor(cx).into_any_element(),
            _ => self.render_placeholder_editor(cx).into_any_element(),
        }
    }

    fn render_level_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .size_full()
            .gap_4()
            .child(
                // Left panel - Scene Hierarchy
                div()
                    .w_64()
                    .h_full()
                    .bg(cx.theme().sidebar)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .p_3()
                    .child("Scene Hierarchy")
            )
            .child(
                // Center - 3D Viewport
                div()
                    .flex_1()
                    .h_full()
                    .bg(cx.theme().muted.opacity(0.2))
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        v_flex()
                            .items_center()
                            .gap_3()
                            .child(
                                div()
                                    .size_16()
                                    .bg(cx.theme().primary.opacity(0.2))
                                    .rounded_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .child("🎮")
                            )
                            .child(
                                div()
                                    .text_lg()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child("3D Viewport")
                            )
                    )
            )
            .child(
                // Right panel - Properties
                div()
                    .w_64()
                    .h_full()
                    .bg(cx.theme().sidebar)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .p_3()
                    .child("Properties")
            )
    }

    fn render_script_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .size_full()
            .gap_4()
            .child(
                // Left panel - File Explorer
                div()
                    .w_64()
                    .h_full()
                    .bg(cx.theme().sidebar)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .p_3()
                    .child("File Explorer")
            )
            .child(
                // Center - Code Editor
                div()
                    .flex_1()
                    .h_full()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .p_4()
                    .child("Code Editor Area")
            )
            .child(
                // Right panel - Output/Terminal
                div()
                    .w_64()
                    .h_full()
                    .bg(cx.theme().sidebar)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .p_3()
                    .child("Terminal")
            )
    }

    fn render_blueprint_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .size_full()
            .gap_4()
            .child(
                div()
                    .w_64()
                    .h_full()
                    .bg(cx.theme().sidebar)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .p_3()
                    .child("Node Library")
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .bg(cx.theme().muted.opacity(0.2))
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .flex()
                    .items_center()
                    .justify_center()
                    .child("Visual Node Graph")
            )
    }

    fn render_placeholder_editor(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .flex()
            .items_center()
            .justify_center()
            .child(
                v_flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .text_lg()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child(format!("{} Editor", self.editor_type.display_name()))
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Coming soon...")
                    )
            )
    }
}