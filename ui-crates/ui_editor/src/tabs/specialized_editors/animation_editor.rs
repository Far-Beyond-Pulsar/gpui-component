use gpui::*;
use ui::{
    dock::{Panel, PanelEvent},
    h_flex, v_flex,
    ActiveTheme as _, StyledExt,
};

pub struct AnimationEditorPanel {
    focus_handle: FocusHandle,
}

impl AnimationEditorPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }
}

impl Panel for AnimationEditorPanel {
    fn panel_name(&self) -> &'static str {
        "Animation Editor"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        div().child("Animation Editor").into_any_element()
    }

    fn dump(&self, _cx: &App) -> ui::dock::PanelState {
        ui::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}

impl Focusable for AnimationEditorPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for AnimationEditorPanel {}

impl Render for AnimationEditorPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(cx.theme().background)
            .child(
                v_flex()
                    .items_center()
                    .gap_4()
                    .child(
                        div()
                            .size_16()
                            .bg(cx.theme().primary.opacity(0.2))
                            .rounded_full()
                            .flex()
                            .items_center()
                            .justify_center()
                            .child("🎬")
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Animation Editor")
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Animate objects and characters")
                    )
            )
    }
}