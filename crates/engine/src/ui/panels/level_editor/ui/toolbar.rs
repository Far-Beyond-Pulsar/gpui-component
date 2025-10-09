use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _}, h_flex, v_flex, ActiveTheme, IconName, Selectable, Sizable, StyledExt,
};

use super::state::{LevelEditorState, TransformTool};
use crate::ui::shared::{Toolbar, ToolbarButton};

/// Toolbar - Transform tools and quick actions
pub struct ToolbarPanel;

impl ToolbarPanel {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, state: &LevelEditorState, cx: &mut App) -> impl IntoElement {
        h_flex()
            .w_full()
            .h_12()
            .px_4()
            .gap_1()
            .items_center()
            .bg(cx.theme().sidebar)
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                // Transform tools
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("tool_select")
                            .icon(IconName::CursorPointer)
                            .tooltip("Select (S)")
                            .selected(matches!(state.current_tool, TransformTool::Select))
                    )
                    .child(
                        Button::new("tool_move")
                            .icon(IconName::Drag)
                            .tooltip("Move (M)")
                            .selected(matches!(state.current_tool, TransformTool::Move))
                    )
                    .child(
                        Button::new("tool_rotate")
                            .icon(IconName::RotateCameraRight)
                            .tooltip("Rotate (R)")
                            .selected(matches!(state.current_tool, TransformTool::Rotate))
                    )
                    .child(
                        Button::new("tool_scale")
                            .icon(IconName::Enlarge)
                            .tooltip("Scale (T)")
                            .selected(matches!(state.current_tool, TransformTool::Scale))
                    )
            )
            .child(
                // Separator
                div()
                    .h_8()
                    .w_px()
                    .bg(cx.theme().border)
                    .mx_2()
            )
            .child(
                // Object creation tools
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("add_mesh")
                            .icon(IconName::Plus)
                            .tooltip("Add Mesh")
                            .xsmall()
                    )
                    .child(
                        Button::new("add_light")
                            .icon(IconName::Sun)
                            .tooltip("Add Light")
                            .xsmall()
                    )
                    .child(
                        Button::new("add_camera")
                            .icon(IconName::Camera)
                            .tooltip("Add Camera")
                            .xsmall()
                    )
            )
            .child(
                // Separator
                div()
                    .h_8()
                    .w_px()
                    .bg(cx.theme().border)
                    .mx_2()
            )
            .child(
                // Scene actions
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("play_scene")
                            .icon(IconName::Play)
                            .tooltip("Play Scene (F5)")
                            .xsmall()
                    )
                    .child(
                        Button::new("stop_scene")
                            .icon(IconName::Square)
                            .tooltip("Stop Scene")
                            .xsmall()
                    )
            )
            .child(
                // Spacer
                div().flex_1()
            )
            .child(
                // Scene file actions
                h_flex()
                    .gap_1()
                    .child({
                        let mut btn = Button::new("save_scene")
                            .icon(IconName::FloppyDisk)
                            .tooltip("Save Scene (Ctrl+S)")
                            .xsmall();

                        if state.has_unsaved_changes {
                            btn = btn.text_color(cx.theme().warning);
                        }

                        btn
                    })
                    .child(
                        Button::new("open_scene")
                            .icon(IconName::FolderOpen)
                            .tooltip("Open Scene (Ctrl+O)")
                            .xsmall()
                    )
                    .child(
                        Button::new("new_scene")
                            .icon(IconName::FolderPlus)
                            .tooltip("New Scene (Ctrl+N)")
                            .xsmall()
                    )
            )
    }
}
