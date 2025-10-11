/// Complete embedded DAW engine for Pulsar game engine
/// 
/// This module provides a production-ready Digital Audio Workstation (DAW) engine
/// with real-time multi-track mixing, sample-accurate automation, GPU-accelerated DSP,
/// and a complete GPUI-based user interface.

mod asset_manager;
mod audio_graph;
mod audio_service;
mod audio_types;
mod ecs_integration;
mod gpu_dsp;
mod project;
mod real_time_audio;
mod ui;

pub use audio_service::AudioService;
pub use audio_types::*;
pub use ecs_integration::{AudioEvent, EcsAudioBridge, TrackState};
pub use project::{DawProject, ExportFormat, create_demo_project};
pub use ui::DawPanel;

use gpui::*;
use gpui_component::dock::{Panel, PanelEvent};
use std::path::PathBuf;
use std::sync::Arc;

/// Main DAW Editor Panel that integrates with the engine's panel system
pub struct DawEditorPanel {
    focus_handle: FocusHandle,
    daw_panel: Entity<DawPanel>,
    project_path: Option<PathBuf>,
    audio_service: Option<Arc<AudioService>>,
}

impl DawEditorPanel {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let daw_panel = cx.new(|cx| DawPanel::new(window, cx));
        
        Self {
            focus_handle: cx.focus_handle(),
            daw_panel,
            project_path: None,
            audio_service: None,
        }
    }

    pub fn new_with_project(
        project_path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let daw_panel = cx.new(|cx| {
            let mut panel = DawPanel::new(window, cx);
            panel.load_project(project_path.clone(), cx);
            panel
        });

        let mut panel = Self {
            focus_handle: cx.focus_handle(),
            daw_panel,
            project_path: Some(project_path),
            audio_service: None,
        };

        panel.initialize_audio_service(window, cx);

        panel
    }

    fn initialize_audio_service(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let daw_panel = self.daw_panel.clone();

        cx.spawn(async move |this, mut cx| {
            match AudioService::new().await {
                Ok(service) => {
                    let service = Arc::new(service);

                    cx.update(|cx| {
                        // Set audio service on DawEditorPanel
                        this.update(cx, |this, cx| {
                            this.audio_service = Some(service.clone());
                            cx.notify();
                        }).ok();

                        // Set audio service on DawPanel
                        daw_panel.update(cx, |panel, cx| {
                            panel.set_audio_service(service, cx);
                        }).ok();
                    }).ok();
                }
                Err(e) => {
                    eprintln!("❌ Failed to initialize audio service: {}", e);
                }
            }
        })
        .detach();
    }

    pub fn load_project(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        self.project_path = Some(path.clone());
        
        self.daw_panel.update(cx, |panel, cx| {
            panel.load_project(path, cx);
        });

        if self.audio_service.is_none() {
            self.initialize_audio_service(window, cx);
        }
    }

    pub fn save_project(&self, cx: &mut Context<Self>) -> anyhow::Result<()> {
        if let Some(path) = &self.project_path {
            self.daw_panel.update(cx, |panel, _cx| {
                panel.save_project(path.clone())
            })?;
        }
        Ok(())
    }

    pub fn new_project(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
        self.daw_panel.update(cx, |panel, cx| {
            panel.new_project(name, cx);
        });

        if self.audio_service.is_none() {
            self.initialize_audio_service(window, cx);
        }
    }

    pub fn get_audio_service(&self) -> Option<Arc<AudioService>> {
        self.audio_service.clone()
    }
}

impl Panel for DawEditorPanel {
    fn panel_name(&self) -> &'static str {
        "DAW Editor"
    }

    fn title(&self, _window: &Window, _cx: &App) -> AnyElement {
        div()
            .child(
                if let Some(path) = &self.project_path {
                    format!(
                        "DAW - {}",
                        path.file_stem()
                            .and_then(|s| s.to_str())
                            .unwrap_or("Untitled")
                    )
                } else {
                    "DAW Editor".to_string()
                }
            )
            .into_any_element()
    }

    fn dump(&self, _cx: &App) -> gpui_component::dock::PanelState {
        let info = self.project_path.as_ref().map(|p| {
            serde_json::json!({
                "project_path": p.to_string_lossy().to_string()
            })
        }).unwrap_or(serde_json::Value::Null);

        gpui_component::dock::PanelState {
            panel_name: self.panel_name().to_string(),
            info: gpui_component::dock::PanelInfo::Panel(info),
            ..Default::default()
        }
    }
}

impl Focusable for DawEditorPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<PanelEvent> for DawEditorPanel {}

impl Render for DawEditorPanel {
    fn render(&mut self, _: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.daw_panel.clone()
    }
}