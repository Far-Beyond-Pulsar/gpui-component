/// Main DAW Panel Component
/// Top-level container that assembles all UI components

use super::state::*;
use super::super::{audio_service::AudioService, audio_types::*, project::DawProject};
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{v_flex, h_flex, StyledExt, ActiveTheme};
use std::path::PathBuf;
use std::sync::Arc;

pub struct DawPanel {
    focus_handle: FocusHandle,
    pub(super) state: DawUiState,
}

impl DawPanel {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            state: DawUiState::new(),
        }
    }

    pub fn load_project(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        match self.state.load_project(path) {
            Ok(_) => {
                eprintln!("✅ DAW: Project loaded successfully");
                cx.notify();
            }
            Err(e) => {
                eprintln!("❌ DAW: Failed to load project: {}", e);
            }
        }
    }

    pub fn save_project(&self, _path: PathBuf) -> anyhow::Result<()> {
        self.state.save_project()
    }

    pub fn new_project(&mut self, name: String, cx: &mut Context<Self>) {
        if let Some(ref project_dir) = self.state.project_dir {
            self.state.new_project(name, project_dir.clone());
            cx.notify();
        }
    }

    pub fn set_audio_service(&mut self, service: Arc<AudioService>) {
        self.state.audio_service = Some(service);
    }
}

impl Focusable for DawPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DawPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context("DawPanel")
            .track_focus(&self.focus_handle)
            .size_full()
            .bg(cx.theme().background)
            .overflow_hidden()
            .child(self.render_content(cx))
    }
}

impl DawPanel {
    fn render_content(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_0()
            // Toolbar
            .child(self.render_toolbar(cx))
            // Transport controls
            .child(self.render_transport(cx))
            // Main content area
            .child(
                h_flex()
                    .flex_1()
                    .overflow_hidden()
                    .gap_0()
                    // Left sidebar (browser)
                    .when(self.state.show_browser, |this| {
                        this.child(self.render_browser(cx))
                    })
                    // Center content (timeline/mixer)
                    .child(self.render_main_area(cx))
                    // Right sidebar (inspector)
                    .when(self.state.show_inspector, |this| {
                        this.child(self.render_inspector(cx))
                    })
            )
    }

    // Placeholder implementations - to be filled in panel by panel
    fn render_toolbar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h(px(40.0))
            .bg(cx.theme().muted.opacity(0.3))
            .border_b_1()
            .border_color(cx.theme().border)
            .child("Toolbar Placeholder")
    }

    fn render_transport(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .h(px(60.0))
            .bg(cx.theme().muted.opacity(0.2))
            .border_b_1()
            .border_color(cx.theme().border)
            .child("Transport Placeholder")
    }

    fn render_browser(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(250.0))
            .h_full()
            .bg(cx.theme().muted.opacity(0.1))
            .border_r_1()
            .border_color(cx.theme().border)
            .child("Browser Placeholder")
    }

    fn render_main_area(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex_1()
            .h_full()
            .bg(cx.theme().background)
            .child(match self.state.view_mode {
                ViewMode::Arrange => self.render_timeline(cx).into_any_element(),
                ViewMode::Mix => self.render_mixer(cx).into_any_element(),
                ViewMode::Edit => self.render_clip_editor(cx).into_any_element(),
            })
    }

    fn render_inspector(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w(px(300.0))
            .h_full()
            .bg(cx.theme().muted.opacity(0.1))
            .border_l_1()
            .border_color(cx.theme().border)
            .child("Inspector Placeholder")
    }

    fn render_timeline(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child("Timeline Placeholder")
    }

    fn render_mixer(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child("Mixer Placeholder")
    }

    fn render_clip_editor(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child("Clip Editor Placeholder")
    }
}
