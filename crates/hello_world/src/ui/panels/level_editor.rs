use gpui::*;
use gpui_component::{
    button::Button, dock::{Panel, PanelEvent}, h_flex, resizable::{h_resizable, resizable_panel, ResizableState}, v_flex, viewport::Viewport, ActiveTheme as _, IconName, Selectable, StyledExt
};
use crate::renderer::ShaderRenderer;

use crate::ui::shared::{Toolbar, ToolbarButton, ViewportControls, StatusBar};

pub struct LevelEditorPanel {
    focus_handle: FocusHandle,
    selected_object: Option<String>,
    viewport_controls: ViewportControls,
    show_wireframe: bool,
    show_lighting: bool,
    camera_mode: CameraMode,
    resizable_state: Entity<ResizableState>,
    viewport: Entity<Viewport<crate::renderer::ShaderRenderer>>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CameraMode {
    Perspective,
    Orthographic,
    Top,
    Front,
    Side,
}

impl LevelEditorPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let resizable_state = ResizableState::new(cx);
        let viewport = cx.new(|cx| {
            Viewport::new(
                crate::renderer::ShaderRenderer::new(),
                800,
                600,
                cx,
            )
        });

        Self {
            focus_handle: cx.focus_handle(),
            selected_object: None,
            viewport_controls: ViewportControls::new(),
            show_wireframe: false,
            show_lighting: true,
            camera_mode: CameraMode::Perspective,
            resizable_state,
            viewport,
        }
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        Toolbar::new()
            .add_button(
                ToolbarButton::new(IconName::Asterisk, "Select")
                    .tooltip("Select Tool (S)")
                    .active(true)
            )
            .add_button(
                ToolbarButton::new(IconName::Asterisk, "Move")
                    .tooltip("Move Tool (M)")
            )
            .add_button(
                ToolbarButton::new(IconName::Asterisk, "Rotate")
                    .tooltip("Rotate Tool (R)")
            )
            .add_button(
                ToolbarButton::new(IconName::Maximize, "Scale")
                    .tooltip("Scale Tool (T)")
            )
            .render(cx)
    }

    fn render_scene_hierarchy(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                            .child("Scene Hierarchy")
                    )
                    .child(
                        Button::new("add_object")
                            .icon(IconName::Plus)
                            .tooltip("Add Object")
                    )
            )
            .child(
                div()
                    .flex_1()
                    .p_2()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .p_2()
                                    .rounded(cx.theme().radius)
                                    .bg(cx.theme().primary.opacity(0.1))
                                    .text_color(cx.theme().primary)
                                    .child("🎮 Main Camera")
                            )
                            .child(
                                div()
                                    .p_2()
                                    .rounded(cx.theme().radius)
                                    .hover(|style| style.bg(cx.theme().muted.opacity(0.5)))
                                    .child("☀️ Directional Light")
                            )
                            .child(
                                div()
                                    .p_2()
                                    .rounded(cx.theme().radius)
                                    .hover(|style| style.bg(cx.theme().muted.opacity(0.5)))
                                    .child("📦 Cube")
                            )
                            .child(
                                div()
                                    .p_2()
                                    .rounded(cx.theme().radius)
                                    .hover(|style| style.bg(cx.theme().muted.opacity(0.5)))
                                    .child("🟢 Sphere")
                            )
                    )
            )
    }

    fn render_viewport(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .relative()
            .bg(cx.theme().muted.opacity(0.2))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(div().size_full().child(self.viewport.clone()))
            .child(
                // Viewport controls overlay
                div()
                    .absolute()
                    .top_4()
                    .right_4()
                    .child(self.viewport_controls.render(cx))
            )
            .child(
                // Camera mode selector
                div()
                    .absolute()
                    .bottom_4()
                    .left_4()
                    .child(
                        h_flex()
                            .gap_2()
                            .p_2()
                            .bg(cx.theme().background.opacity(0.9))
                            .rounded(cx.theme().radius)
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                Button::new("perspective")
                                    .child("Perspective")
                                    .selected(matches!(self.camera_mode, CameraMode::Perspective))
                            )
                            .child(
                                Button::new("orthographic")
                                    .child("Orthographic")
                                    .selected(matches!(self.camera_mode, CameraMode::Orthographic))
                            )
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
                            .child("Properties")
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
                        if self.selected_object.is_some() {
                            v_flex()
                                .gap_3()
                                .child(
                                    div()
                                        .text_lg()
                                        .font_semibold()
                                        .text_color(cx.theme().foreground)
                                        .child("Cube")
                                )
                                .child(
                                    v_flex()
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_medium()
                                                .text_color(cx.theme().foreground)
                                                .child("Transform")
                                        )
                                        .child(self.render_transform_section(cx))
                                )
                                .child(
                                    v_flex()
                                        .gap_2()
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_medium()
                                                .text_color(cx.theme().foreground)
                                                .child("Material")
                                        )
                                        .child(self.render_material_section(cx))
                                )
                                .into_any_element()
                        } else {
                            div()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_color(cx.theme().muted_foreground)
                                .child("No object selected")
                                .into_any_element()
                        }
                    )
            )
    }

    fn render_transform_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(self.render_vector3_field("Position", (0.0, 0.0, 0.0), cx))
            .child(self.render_vector3_field("Rotation", (0.0, 0.0, 0.0), cx))
            .child(self.render_vector3_field("Scale", (1.0, 1.0, 1.0), cx))
    }

    fn render_material_section(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child("Material:")
                    )
                    .child(
                        Button::new("select_material")
                            .child("Default Material")
                    )
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child("Color:")
                    )
                    .child(
                        div()
                            .size_8()
                            .bg(cx.theme().primary)
                            .rounded(px(4.0))
                            .border_1()
                            .border_color(cx.theme().border)
                    )
            )
    }

    fn render_vector3_field(&self, label: &str, values: (f32, f32, f32), cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_2()
            .items_center()
            .child(
                div()
                    .w_16()
                    .text_sm()
                    .text_color(cx.theme().foreground)
                    .child(format!("{}:", label))
            )
            .child(
                h_flex()
                    .gap_1()
                    .child(self.render_float_input("X", values.0, cx))
                    .child(self.render_float_input("Y", values.1, cx))
                    .child(self.render_float_input("Z", values.2, cx))
            )
    }

    fn render_float_input(&self, axis: &str, value: f32, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_1()
            .items_center()
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(axis.to_string())
            )
            .child(
                div()
                    .w_16()
                    .px_2()
                    .py_1()
                    .bg(cx.theme().input)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(px(4.0))
                    .text_xs()
                    .text_color(cx.theme().foreground)
                    .child(format!("{:.2}", value))
            )
    }

    fn render_status_bar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        StatusBar::new()
            .add_left_item(format!("Objects: {}", 4))
            .add_left_item(format!("Vertices: {}", 24))
            .add_left_item(format!("Triangles: {}", 12))
            .add_right_item("FPS: 60")
            .add_right_item("Perspective")
            .add_right_item("Grid: On")
            .render(cx)
    }
}

impl Panel for LevelEditorPanel {
    fn panel_name(&self) -> &'static str {
        "Level Editor"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        div().child("Level Editor").into_any_element()
    }

    fn dump(&self, _cx: &App) -> gpui_component::dock::PanelState {
        gpui_component::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}

impl Focusable for LevelEditorPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for LevelEditorPanel {}

impl Render for LevelEditorPanel {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(self.render_toolbar(cx))
            .child(
                div()
                    .flex_1()
                    .child(
                        h_resizable("level-editor-panels", self.resizable_state.clone())
                            .child(
                                resizable_panel()
                                    .size(px(280.))
                                    .size_range(px(200.)..px(400.))
                                    .child(
                                        div()
                                            .size_full()
                                            .bg(cx.theme().sidebar)
                                            .border_1()
                                            .border_color(cx.theme().border)
                                            .rounded(cx.theme().radius)
                                            .p_2()
                                            .child(self.render_scene_hierarchy(cx))
                                    )
                            )
                            .child(
                                resizable_panel()
                                    .child(
                                        div()
                                            .size_full()
                                            .p_2()
                                            .child(self.render_viewport(window, cx))
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