use gpui::*;
use ui::{
    dock::{Panel, PanelEvent},
    resizable::ResizableState, v_flex,
    ActiveTheme as _, StyledExt,
};

pub struct MaterialEditorPanel {
    focus_handle: FocusHandle,
    resizable_state: Entity<ResizableState>,
}

impl MaterialEditorPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let resizable_state = ResizableState::new(cx);

        Self {
            focus_handle: cx.focus_handle(),
            resizable_state,
        }
    }
}

impl Panel for MaterialEditorPanel {
    fn panel_name(&self) -> &'static str {
        "Material Editor"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        div().child("Material Editor").into_any_element()
    }

    fn dump(&self, _cx: &App) -> ui::dock::PanelState {
        ui::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            ..Default::default()
        }
    }
}

impl Focusable for MaterialEditorPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for MaterialEditorPanel {}

impl Render for MaterialEditorPanel {
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
                            .child("🎨")
                    )
                    .child(
                        div()
                            .text_lg()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Material Editor")
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Create and edit materials")
                    )
            )
    }
}