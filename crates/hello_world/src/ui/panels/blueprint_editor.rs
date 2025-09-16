use gpui::*;
use gpui_component::{
    button::Button,
    dock::{Panel, PanelEvent},
    resizable::{h_resizable, resizable_panel, ResizableState},
    h_flex, v_flex,
    ActiveTheme as _, StyledExt, Selectable,
    IconName,
};

use crate::ui::shared::{Toolbar, ToolbarButton, StatusBar};

pub struct BlueprintEditorPanel {
    focus_handle: FocusHandle,
    selected_node: Option<String>,
    zoom_level: f32,
    pan_offset: (f32, f32),
    resizable_state: Entity<ResizableState>,
}

impl BlueprintEditorPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let resizable_state = ResizableState::new(cx);

        Self {
            focus_handle: cx.focus_handle(),
            selected_node: None,
            zoom_level: 1.0,
            pan_offset: (0.0, 0.0),
            resizable_state,
        }
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Toolbar::new()
            .add_button(
                ToolbarButton::new(IconName::Plus, "Add Node")
                    .tooltip("Add Node (A)")
            )
            .add_button(
                ToolbarButton::new(IconName::Copy, "Duplicate")
                    .tooltip("Duplicate Node (Ctrl+D)")
            )
            .add_button(
                ToolbarButton::new(IconName::Delete, "Delete")
                    .tooltip("Delete Node (Del)")
            )
            .add_button(
                ToolbarButton::new(IconName::Plus, "Zoom In")
                    .tooltip("Zoom In (+)")
            )
            .add_button(
                ToolbarButton::new(IconName::Minus, "Zoom Out")
                    .tooltip("Zoom Out (-)")
            )
            .add_button(
                ToolbarButton::new(IconName::CircleCheck, "Fit")
                    .tooltip("Fit to View (F)")
            )
            .render(cx)
    }

    fn render_node_library(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_2()
            .child(
                h_flex()
                    .w_full()
                    .p_2()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Node Library")
                    )
                    .child(
                        Button::new("search")
                            .icon(IconName::Search)
                            .tooltip("Search Nodes")
                    )
            )
            .child(
                div()
                    .flex_1()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    //TODO: Make this scrollable
                    .child(self.render_node_categories(cx))
            )
    }

    fn render_node_categories(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .p_2()
            .gap_3()
            .child(self.render_node_category("Events", &[
                ("On Begin Play", "▶️"),
                ("On Tick", "⏱️"),
                ("On Input", "🎮"),
                ("On Collision", "💥"),
            ], cx))
            .child(self.render_node_category("Logic", &[
                ("Branch", "🔀"),
                ("Sequence", "📝"),
                ("For Loop", "🔄"),
                ("While Loop", "🔁"),
            ], cx))
            .child(self.render_node_category("Math", &[
                ("Add", "➕"),
                ("Multiply", "✖️"),
                ("Vector Math", "📐"),
                ("Random", "🎲"),
            ], cx))
            .child(self.render_node_category("Objects", &[
                ("Spawn Actor", "🏗️"),
                ("Destroy Actor", "💥"),
                ("Get Transform", "📍"),
                ("Set Transform", "🎯"),
            ], cx))
    }

    fn render_node_category(&self, title: &str, nodes: &[(&str, &str)], cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_1()
            .child(
                div()
                    .p_2()
                    .bg(cx.theme().muted.opacity(0.3))
                    .rounded(px(4.0))
                    .text_sm()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .child(title.to_string())
            )
            .child(
                v_flex()
                    .gap_1()
                    .children(
                        nodes.iter().map(|(name, icon)| {
                            h_flex()
                                .items_center()
                                .gap_2()
                                .p_2()
                                .rounded(px(4.0))
                                .hover(|style| style.bg(cx.theme().muted.opacity(0.5)))
                                .child(icon.to_string())
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().foreground)
                                        .child((*name).to_string())
                                )
                                .into_any_element()
                        })
                    )
            )
    }

    fn render_node_graph(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .bg(cx.theme().muted.opacity(0.1))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .overflow_hidden()
            .child(self.render_grid_background(cx))
            .child(self.render_sample_nodes(cx))
            .child(self.render_graph_controls(cx))
    }

    fn render_grid_background(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // Simple grid pattern background
        div()
            .absolute()
            .inset_0()
            .child(
                div()
                    .size_full()
                    .bg(cx.theme().muted.opacity(0.05))
                    // Grid pattern would be implemented with CSS or canvas
            )
    }

    fn render_sample_nodes(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .inset_0()
            .child(
                // Event node
                div()
                    .absolute()
                    .top_16()
                    .left_16()
                    .child(self.render_blueprint_node("Begin Play", "▶️", true, cx))
            )
            .child(
                // Logic node
                div()
                    .absolute()
                    .top_32()
                    .left_80()
                    .child(self.render_blueprint_node("Print String", "📝", false, cx))
            )
            .child(
                // Connection lines would be drawn here
                div()
                    .absolute()
                    .top_20()
                    .left_56()
                    .w_6()
                    .h_px()
                    .bg(cx.theme().primary)
            )
    }

    fn render_blueprint_node(&self, title: &str, icon: &str, is_event: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let node_color = if is_event {
            cx.theme().danger
        } else {
            cx.theme().primary
        };

        v_flex()
            .w_48()
            .bg(cx.theme().background)
            .border_2()
            .border_color(node_color)
            .rounded(px(8.0))
            .shadow_lg()
            .child(
                // Header
                h_flex()
                    .w_full()
                    .p_2()
                    .bg(node_color.opacity(0.2))
                    .items_center()
                    .gap_2()
                    .child(icon.to_string())
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child(title.to_string())
                    )
            )
            .child(
                // Pins
                v_flex()
                    .p_2()
                    .gap_1()
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                // Input pin
                                div()
                                    .size_3()
                                    .bg(cx.theme().muted)
                                    .rounded_full()
                                    .border_1()
                                    .border_color(cx.theme().border)
                            )
                            .child(
                                // Output pin
                                div()
                                    .size_3()
                                    .bg(cx.theme().success)
                                    .rounded_full()
                                    .border_1()
                                    .border_color(cx.theme().border)
                            )
                    )
            )
    }

    fn render_graph_controls(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .absolute()
            .bottom_4()
            .right_4()
            .child(
                h_flex()
                    .gap_2()
                    .p_2()
                    .bg(cx.theme().background.opacity(0.9))
                    .rounded(cx.theme().radius)
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Zoom: {:.0}%", self.zoom_level * 100.0))
                    )
                    .child(
                        Button::new("zoom_fit")
                            .icon(IconName::CircleCheck)
                            .tooltip("Fit to View")
                    )
            )
    }

    fn render_properties(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_2()
            .child(
                h_flex()
                    .w_full()
                    .p_2()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Details")
                    )
            )
            .child(
                div()
                    .flex_1()
                    .p_3()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(
                        if self.selected_node.is_some() {
                            v_flex()
                                .gap_3()
                                .child(
                                    div()
                                        .text_lg()
                                        .font_semibold()
                                        .text_color(cx.theme().foreground)
                                        .child("Print String")
                                )
                                .child(
                                    v_flex()
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_medium()
                                                .text_color(cx.theme().foreground)
                                                .child("Properties")
                                        )
                                        .child(self.render_node_properties(cx))
                                )
                                .into_any_element()
                        } else {
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(cx.theme().muted_foreground)
                                .child("No node selected")
                                .into_any_element()
                        }
                    )
            )
    }

    fn render_node_properties(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child("Message:")
                    )
                    .child(
                        div()
                            .w_full()
                            .px_3()
                            .py_2()
                            .bg(cx.theme().input)
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child("Hello World!")
                    )
            )
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child("Print to Screen:")
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .size_4()
                                    .bg(cx.theme().primary)
                                    .rounded(px(2.0))
                                    .border_1()
                                    .border_color(cx.theme().border)
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().foreground)
                                    .child("Enabled")
                            )
                    )
            )
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        StatusBar::new()
            .add_left_item(format!("Nodes: {}", 2))
            .add_left_item(format!("Connections: {}", 1))
            .add_left_item("Blueprint: PlayerController")
            .add_right_item(format!("Zoom: {:.0}%", self.zoom_level * 100.0))
            .add_right_item("Visual Scripting")
            .render(cx)
    }
}

impl Panel for BlueprintEditorPanel {
    fn panel_name(&self) -> &'static str {
        "Blueprint Editor"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        div().child("Blueprint Editor").into_any_element()
    }

    fn dump(&self, _cx: &App) -> gpui_component::dock::PanelState {
        gpui_component::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}

impl Focusable for BlueprintEditorPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for BlueprintEditorPanel {}

impl Render for BlueprintEditorPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_toolbar(cx))
            .child(
                div()
                    .flex_1()
                    .child(
                        h_resizable("blueprint-editor-panels", self.resizable_state.clone())
                            .child(
                                resizable_panel()
                                    .size(px(260.))
                                    .size_range(px(200.)..px(400.))
                                    .child(
                                        div()
                                            .size_full()
                                            .bg(cx.theme().sidebar)
                                            .border_1()
                                            .border_color(cx.theme().border)
                                            .rounded(cx.theme().radius)
                                            .p_2()
                                            .child(self.render_node_library(cx))
                                    )
                            )
                            .child(
                                resizable_panel()
                                    .child(
                                        div()
                                            .size_full()
                                            .p_2()
                                            .child(self.render_node_graph(cx))
                                    )
                            )
                            .child(
                                resizable_panel()
                                    .size(px(320.))
                                    .size_range(px(250.)..px(500.))
                                    .child(
                                        div()
                                            .size_full()
                                            .bg(cx.theme().sidebar)
                                            .border_1()
                                            .border_color(cx.theme().border)
                                            .rounded(cx.theme().radius)
                                            .p_2()
                                            .child(self.render_properties(cx))
                                    )
                            )
                    )
            )
            .child(self.render_status_bar(cx))
    }
}