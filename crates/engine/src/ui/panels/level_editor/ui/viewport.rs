use gpui::*;
use gpui_component::{
    button::Button, h_flex, v_flex, ActiveTheme, IconName, Selectable, Sizable, StyledExt,
};
use gpui_component::viewport_final::Viewport;

use super::state::{CameraMode, LevelEditorState};
use crate::ui::shared::ViewportControls;
use std::sync::{Arc, Mutex};

/// Viewport Panel - 3D rendering viewport with camera controls
pub struct ViewportPanel {
    viewport: Entity<Viewport>,
    viewport_controls: ViewportControls,
    show_performance_overlay: bool,
}

impl ViewportPanel {
    pub fn new(viewport: Entity<Viewport>) -> Self {
        Self {
            viewport,
            viewport_controls: ViewportControls::new(),
            show_performance_overlay: true,
        }
    }

    pub fn render(
        &self,
        state: &LevelEditorState,
        render_engine: &Arc<Mutex<crate::ui::rainbow_engine_final::RainbowRenderEngine>>,
        render_enabled: &Arc<std::sync::atomic::AtomicBool>,
        current_pattern: crate::ui::rainbow_engine_final::RainbowPattern,
        cx: &mut App,
    ) -> impl IntoElement {
        let mut viewport_div = div()
            .size_full()
            .relative()
            .bg(cx.theme().muted.opacity(0.2))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(
                // Main viewport
                div()
                    .size_full()
                    .child(self.viewport.clone())
            )
            .child(
                // Viewport controls overlay (top-right)
                div()
                    .absolute()
                    .top_4()
                    .right_4()
                    .child(self.viewport_controls.render(cx))
            )
            .child(
                // Camera mode selector (bottom-left)
                div()
                    .absolute()
                    .bottom_4()
                    .left_4()
                    .child(Self::render_camera_mode_selector(state.camera_mode, cx))
            )
            .child(
                // Grid and rendering options (top-left)
                div()
                    .absolute()
                    .top_4()
                    .left_4()
                    .child(Self::render_viewport_options(state, cx))
            );

        if self.show_performance_overlay {
            viewport_div = viewport_div.child(
                // Performance overlay (bottom-right)
                div()
                    .absolute()
                    .bottom_4()
                    .right_4()
                    .child(Self::render_performance_overlay(render_engine, render_enabled, current_pattern, cx))
            );
        }

        viewport_div
    }

    fn render_camera_mode_selector(camera_mode: CameraMode, cx: &App) -> impl IntoElement {
        h_flex()
            .gap_1()
            .p_2()
            .bg(cx.theme().background.opacity(0.95))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .shadow_lg()
            .child(
                Button::new("camera_perspective")
                    .child("Perspective")
                    .xsmall()
                    .selected(matches!(camera_mode, CameraMode::Perspective))
            )
            .child(
                Button::new("camera_orthographic")
                    .child("Ortho")
                    .xsmall()
                    .selected(matches!(camera_mode, CameraMode::Orthographic))
            )
            .child(
                Button::new("camera_top")
                    .child("Top")
                    .xsmall()
                    .selected(matches!(camera_mode, CameraMode::Top))
            )
            .child(
                Button::new("camera_front")
                    .child("Front")
                    .xsmall()
                    .selected(matches!(camera_mode, CameraMode::Front))
            )
            .child(
                Button::new("camera_side")
                    .child("Side")
                    .xsmall()
                    .selected(matches!(camera_mode, CameraMode::Side))
            )
    }

    fn render_viewport_options(state: &LevelEditorState, cx: &App) -> impl IntoElement {
        v_flex()
            .gap_2()
            .p_2()
            .bg(cx.theme().background.opacity(0.95))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .shadow_lg()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().foreground)
                            .child("View Options")
                    )
            )
            .child(
                h_flex()
                    .gap_1()
                    .child(
                        Button::new("toggle_grid")
                            .child("Grid")
                            .xsmall()
                            .selected(state.show_grid)
                    )
                    .child(
                        Button::new("toggle_wireframe")
                            .child("Wireframe")
                            .xsmall()
                            .selected(state.show_wireframe)
                    )
                    .child(
                        Button::new("toggle_lighting")
                            .child("Lighting")
                            .xsmall()
                            .selected(state.show_lighting)
                    )
            )
    }

    fn render_performance_overlay(
        render_engine: &Arc<Mutex<crate::ui::rainbow_engine_final::RainbowRenderEngine>>,
        render_enabled: &Arc<std::sync::atomic::AtomicBool>,
        current_pattern: crate::ui::rainbow_engine_final::RainbowPattern,
        cx: &App,
    ) -> impl IntoElement {
        let (engine_fps, frame_count, pattern_name) = if let Ok(engine) = render_engine.lock() {
            let fps = engine.get_fps();
            let frames = engine.get_frame_count();
            let pattern = format!("{:?}", current_pattern);
            (fps, frames, pattern)
        } else {
            (0.0, 0, "Unknown".to_string())
        };

        v_flex()
            .gap_1()
            .p_2()
            .bg(cx.theme().background.opacity(0.95))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .shadow_lg()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(if engine_fps > 200.0 {
                                cx.theme().success
                            } else if engine_fps > 120.0 {
                                cx.theme().warning
                            } else {
                                cx.theme().accent
                            })
                            .child(format!("🌈 {:.1} FPS", engine_fps))
                    )
                    .child(
                        Button::new("toggle_render")
                            .child(if render_enabled.load(std::sync::atomic::Ordering::Relaxed) {
                                "⏸"
                            } else {
                                "▶"
                            })
                            .xsmall()
                    )
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Frames: {}", frame_count))
                    )
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(format!("Pattern: {}", pattern_name))
            )
    }
}
