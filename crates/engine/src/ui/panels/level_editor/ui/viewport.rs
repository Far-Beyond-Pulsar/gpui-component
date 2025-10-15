use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _}, h_flex, v_flex, ActiveTheme, IconName, Selectable, Sizable, StyledExt,
};
use gpui_component::viewport_final::Viewport;

use super::state::{CameraMode, LevelEditorState};
use super::actions::*;
use crate::ui::shared::ViewportControls;
use std::sync::{Arc, Mutex};

/// Viewport Panel - 3D rendering viewport with camera controls
pub struct ViewportPanel {
    viewport: Entity<Viewport>,
    viewport_controls: ViewportControls,
    render_enabled: Arc<std::sync::atomic::AtomicBool>,
}

impl ViewportPanel {
    pub fn new(viewport: Entity<Viewport>, render_enabled: Arc<std::sync::atomic::AtomicBool>) -> Self {
        Self {
            viewport,
            viewport_controls: ViewportControls::new(),
            render_enabled,
        }
    }

    pub fn render<V: 'static>(
        &self,
        state: &LevelEditorState,
        render_engine: &Arc<Mutex<crate::ui::rainbow_engine_final::RainbowRenderEngine>>,
        current_pattern: crate::ui::rainbow_engine_final::RainbowPattern,
        cx: &mut Context<V>,
    ) -> impl IntoElement
    where
        V: EventEmitter<gpui_component::dock::PanelEvent> + Render,
    {
        let mut viewport_div = div()
            .size_full()
            .flex() // Enable flexbox
            .flex_col() // Column direction
            .relative()
            .bg(cx.theme().muted.opacity(0.2))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(
                // Main viewport - should grow to fill space
                div()
                    .size_full()
                    .flex_1() // Grow to fill available space
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

        if state.show_performance_overlay {
            viewport_div = viewport_div.child(
                // Performance overlay (bottom-right)
                div()
                    .absolute()
                    .bottom_4()
                    .right_4()
                    .child(self.render_performance_overlay(render_engine, current_pattern, cx))
            );
        }

        viewport_div
    }

    fn render_camera_mode_selector<V: 'static>(camera_mode: CameraMode, cx: &mut Context<V>) -> impl IntoElement
    where
        V: EventEmitter<gpui_component::dock::PanelEvent> + Render,
    {
        h_flex()
            .gap_1()
            .p_1()
            .bg(cx.theme().background.opacity(0.9))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(
                Button::new("camera_perspective")
                    .child("Persp")
                    .xsmall()
                    .selected(matches!(camera_mode, CameraMode::Perspective))
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.dispatch_action(&PerspectiveView);
                    }))
            )
            .child(
                Button::new("camera_orthographic")
                    .child("Ortho")
                    .xsmall()
                    .selected(matches!(camera_mode, CameraMode::Orthographic))
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.dispatch_action(&OrthographicView);
                    }))
            )
            .child(
                Button::new("camera_top")
                    .child("Top")
                    .xsmall()
                    .selected(matches!(camera_mode, CameraMode::Top))
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.dispatch_action(&TopView);
                    }))
            )
            .child(
                Button::new("camera_front")
                    .child("Front")
                    .xsmall()
                    .selected(matches!(camera_mode, CameraMode::Front))
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.dispatch_action(&FrontView);
                    }))
            )
            .child(
                Button::new("camera_side")
                    .child("Side")
                    .xsmall()
                    .selected(matches!(camera_mode, CameraMode::Side))
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.dispatch_action(&SideView);
                    }))
            )
    }

    fn render_viewport_options<V: 'static>(state: &LevelEditorState, cx: &mut Context<V>) -> impl IntoElement
    where
        V: EventEmitter<gpui_component::dock::PanelEvent> + Render,
    {
        h_flex()
            .gap_1()
            .p_1()
            .bg(cx.theme().background.opacity(0.9))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(
                Button::new("toggle_grid")
                    .child("Grid")
                    .xsmall()
                    .selected(state.show_grid)
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.dispatch_action(&ToggleGrid);
                    }))
            )
            .child(
                Button::new("toggle_wireframe")
                    .child("Wireframe")
                    .xsmall()
                    .selected(state.show_wireframe)
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.dispatch_action(&ToggleWireframe);
                    }))
            )
            .child(
                Button::new("toggle_lighting")
                    .child("Lighting")
                    .xsmall()
                    .selected(state.show_lighting)
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.dispatch_action(&ToggleLighting);
                    }))
            )
            .child(
                Button::new("toggle_performance")
                    .child("Stats")
                    .xsmall()
                    .selected(state.show_performance_overlay)
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.dispatch_action(&TogglePerformanceOverlay);
                    }))
            )
    }

    fn render_performance_overlay<V: 'static>(
        &self,
        render_engine: &Arc<Mutex<crate::ui::rainbow_engine_final::RainbowRenderEngine>>,
        current_pattern: crate::ui::rainbow_engine_final::RainbowPattern,
        cx: &mut Context<V>,
    ) -> impl IntoElement
    where
        V: EventEmitter<gpui_component::dock::PanelEvent> + Render,
    {
        let (engine_fps, frame_count, pattern_name) = if let Ok(engine) = render_engine.lock() {
            let fps = engine.get_fps();
            let frames = engine.get_frame_count();
            let pattern = format!("{:?}", current_pattern);
            (fps, frames, pattern)
        } else {
            (0.0, 0, "Unknown".to_string())
        };

        h_flex()
            .gap_2()
            .p_1()
            .items_center()
            .bg(cx.theme().background.opacity(0.9))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .text_xs()
                    .font_semibold()
                    .text_color(if engine_fps > 200.0 {
                        cx.theme().success
                    } else if engine_fps > 120.0 {
                        cx.theme().warning
                    } else {
                        cx.theme().muted_foreground
                    })
                    .child(format!("{:.0} FPS", engine_fps))
            )
            .child({
                let enabled = self.render_enabled.clone();
                Button::new("toggle_render")
                    .child(if self.render_enabled.load(std::sync::atomic::Ordering::Relaxed) {
                        "⏸"
                    } else {
                        "▶"
                    })
                    .xsmall()
                    .on_click(move |_event, _window, _cx| {
                        let current = enabled.load(std::sync::atomic::Ordering::Relaxed);
                        enabled.store(!current, std::sync::atomic::Ordering::Relaxed);
                    })
            })
    }
}
