use gpui::*;
use gpui::prelude::FluentBuilder;
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
        rainbow_engine: &Arc<Mutex<crate::ui::rainbow_engine_final::RainbowRenderEngine>>,
        wgpu_3d_engine: &Arc<Mutex<crate::ui::wgpu_3d_renderer::Wgpu3DRenderer>>,
        use_3d_renderer: &Arc<std::sync::atomic::AtomicBool>,
        current_pattern: crate::ui::rainbow_engine_final::RainbowPattern,
        cx: &mut Context<V>,
    ) -> impl IntoElement
    where
        V: EventEmitter<gpui_component::dock::PanelEvent> + Render,
    {
        let mut viewport_div = div()
            .flex() // Enable flexbox
            .flex_col() // Column direction
            .flex_1() // Allow growing to fill parent
            .size_full() // Take full size
            .relative()
            .bg(cx.theme().muted.opacity(0.2))
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(
                // Main viewport - should grow to fill space
                div()
                    .flex() // Enable flex
                    .flex_1() // Grow to fill available space
                    .size_full() // Take full size
                    .child(self.viewport.clone())
            )
            .when(state.show_viewport_controls, |viewport_div| {
                viewport_div.child(
                    // Viewport controls overlay (top-right)
                    div()
                        .absolute()
                        .top_4()
                        .right_4()
                        .w(px(200.0)) // Hardcoded width to prevent inheritance issues
                        .child(Self::render_viewport_controls_overlay(cx))
                )
            })
            .when(state.show_camera_mode_selector, |viewport_div| {
                viewport_div.child(
                    // Camera mode selector (bottom-left)
                    div()
                        .absolute()
                        .bottom_4()
                        .left_4()
                        .w(px(320.0)) // Hardcoded width to prevent inheritance issues
                        .child(Self::render_camera_mode_selector(state.camera_mode, cx))
                )
            })
            .when(state.show_viewport_options, |viewport_div| {
                viewport_div.child(
                    // Grid and rendering options (top-left)
                    div()
                        .absolute()
                        .top_4()
                        .left_4()
                        .w(px(400.0)) // Hardcoded width to prevent inheritance issues
                        .child(Self::render_viewport_options(state, cx))
                )
            });

        if state.show_performance_overlay {
            viewport_div = viewport_div.child(
                // Performance overlay (bottom-right)
                div()
                    .absolute()
                    .bottom_4()
                    .right_4()
                    .w(px(280.0)) // Hardcoded width to prevent inheritance issues
                    .child(self.render_performance_overlay(rainbow_engine, wgpu_3d_engine, use_3d_renderer, current_pattern, cx))
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
            .w_full()
            .bg(cx.theme().background.opacity(0.9))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .justify_between()
            .items_center()
            .child(
                h_flex()
                    .gap_1()
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
            )
            .child(
                Button::new("close_camera_mode")
                    .icon(IconName::X)
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.dispatch_action(&ToggleCameraModeSelector);
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
            .w_full()
            .bg(cx.theme().background.opacity(0.9))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .justify_between()
            .items_center()
            .child(
                h_flex()
                    .gap_1()
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
                    .child(
                        Button::new("toggle_3d")
                            .child("3D")
                            .xsmall()
                            .on_click(cx.listener(|_, _, _, cx| {
                                cx.dispatch_action(&Toggle3DRenderer);
                            }))
                    )
            )
            .child(
                Button::new("close_viewport_options")
                    .icon(IconName::X)
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.dispatch_action(&ToggleViewportOptions);
                    }))
            )
    }

    fn render_viewport_controls_overlay<V: 'static>(cx: &mut Context<V>) -> impl IntoElement
    where
        V: EventEmitter<gpui_component::dock::PanelEvent> + Render,
    {
        h_flex()
            .gap_2()
            .p_2()
            .w_full()
            .bg(cx.theme().background.opacity(0.9))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
            .justify_between()
            .items_center()
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("grid")
                            .icon(IconName::LayoutDashboard)
                            .tooltip("Toggle Grid")
                            .xsmall()
                    )
                    .child(
                        Button::new("axes")
                            .icon(IconName::Axes)
                            .tooltip("Toggle Axes")
                            .xsmall()
                    )
                    .child(
                        Button::new("perspective")
                            .icon(IconName::Cube)
                            .tooltip("Toggle Perspective")
                            .xsmall()
                    )
            )
            .child(
                Button::new("close_viewport_controls")
                    .icon(IconName::X)
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.dispatch_action(&ToggleViewportControls);
                    }))
            )
    }

    fn render_performance_overlay<V: 'static>(
        &self,
        rainbow_engine: &Arc<Mutex<crate::ui::rainbow_engine_final::RainbowRenderEngine>>,
        wgpu_3d_engine: &Arc<Mutex<crate::ui::wgpu_3d_renderer::Wgpu3DRenderer>>,
        use_3d_renderer: &Arc<std::sync::atomic::AtomicBool>,
        current_pattern: crate::ui::rainbow_engine_final::RainbowPattern,
        cx: &mut Context<V>,
    ) -> impl IntoElement
    where
        V: EventEmitter<gpui_component::dock::PanelEvent> + Render,
    {
        let use_3d = use_3d_renderer.load(std::sync::atomic::Ordering::Relaxed);
        
        let (engine_fps, _frame_count, _pattern_name) = if use_3d {
            if let Ok(engine) = wgpu_3d_engine.lock() {
                let fps = engine.get_fps();
                let frames = engine.get_frame_count();
                (fps, frames, "3D Scene".to_string())
            } else {
                (0.0, 0, "Unknown".to_string())
            }
        } else {
            if let Ok(engine) = rainbow_engine.lock() {
                let fps = engine.get_fps();
                let frames = engine.get_frame_count();
                let pattern = format!("{:?}", current_pattern);
                (fps, frames, pattern)
            } else {
                (0.0, 0, "Unknown".to_string())
            }
        };

        h_flex()
            .gap_2()
            .p_1()
            .w_full()
            .items_center()
            .justify_between()
            .bg(cx.theme().background.opacity(0.9))
            .rounded(cx.theme().radius)
            .border_1()
            .border_color(cx.theme().border)
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
            )
            .child(
                Button::new("close_performance")
                    .icon(IconName::X)
                    .ghost()
                    .xsmall()
                    .on_click(cx.listener(|_, _, _, cx| {
                        cx.dispatch_action(&TogglePerformanceOverlay);
                    }))
            )
    }
}
