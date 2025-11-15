use std::path::PathBuf;
use gpui::*;
use ui::{
    button::{Button, ButtonVariants as _}, h_flex, v_flex, scroll::ScrollbarAxis, ActiveTheme, IconName, Sizable, StyledExt,
};

/// Scene Browser - Browse and manage scene files in the project
pub struct SceneBrowser {
    project_path: Option<PathBuf>,
    current_scene: Option<PathBuf>,
    available_scenes: Vec<PathBuf>,
}

impl SceneBrowser {
    pub fn new() -> Self {
        Self {
            project_path: None,
            current_scene: None,
            available_scenes: Vec::new(),
        }
    }

    pub fn set_project_path(&mut self, path: PathBuf) {
        self.project_path = Some(path.clone());
        self.refresh_scenes();
    }

    pub fn set_current_scene(&mut self, scene: Option<PathBuf>) {
        self.current_scene = scene;
    }

    fn refresh_scenes(&mut self) {
        self.available_scenes.clear();

        if let Some(ref project_path) = self.project_path {
            let scenes_dir = project_path.join("scenes");
            if scenes_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&scenes_dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.extension().and_then(|s| s.to_str()) == Some("scene") {
                            self.available_scenes.push(path);
                        }
                    }
                }
            }
        }

        // Sort scenes alphabetically
        self.available_scenes.sort();
    }

    pub fn render(&self, cx: &mut App) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_2()
            .child(
                // Header
                h_flex()
                    .w_full()
                    .p_2()
                    .justify_between()
                    .items_center()
                    .bg(cx.theme().sidebar)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Scenes")
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("new_scene")
                                    .icon(IconName::Plus)
                                    .ghost()
                                    .xsmall()
                                    .tooltip("New Scene")
                            )
                            .child(
                                Button::new("refresh_scenes")
                                    .icon(IconName::Refresh)
                                    .ghost()
                                    .xsmall()
                                    .tooltip("Refresh")
                            )
                    )
            )
            .child(
                // Scene list
                div()
                    .flex_1()
                    .overflow_hidden()
                    .p_2()
                    .child(
                        v_flex()
                            .size_full()
                            .scrollable(ScrollbarAxis::Vertical)
                            .child(if self.available_scenes.is_empty() {
                            v_flex()
                                .size_full()
                                .items_center()
                                .justify_center()
                                .gap_2()
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(cx.theme().muted_foreground)
                                        .child("No scenes found")
                                )
                                .child(
                                    Button::new("create_first_scene")
                                        .child("Create Scene")
                                        .icon(IconName::Plus)
                                )
                                .into_any_element()
                        } else {
                            v_flex()
                                .gap_1()
                                .children(
                                    self.available_scenes.iter().map(|scene_path| {
                                        let is_current = self.current_scene.as_ref() == Some(scene_path);
                                        let scene_name = scene_path
                                            .file_stem()
                                            .and_then(|s| s.to_str())
                                            .unwrap_or("Unnamed").to_string();

                                        let mut base_div = div()
                                            .w_full()
                                            .p_2()
                                            .rounded(cx.theme().radius);

                                        if is_current {
                                            base_div = base_div
                                                .bg(cx.theme().primary.opacity(0.2))
                                                .text_color(cx.theme().primary);
                                        } else {
                                            base_div = base_div
                                                .hover(|style| style.bg(cx.theme().muted.opacity(0.5)))
                                                .text_color(cx.theme().foreground);
                                        }

                                        base_div.child(
                                            h_flex()
                                                .gap_2()
                                                .items_center()
                                                .child(
                                                    div()
                                                        .text_color(cx.theme().accent)
                                                        .child("📄")
                                                )
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .child(scene_name)
                                                )
                                        )
                                    })
                                )
                                .into_any_element()
                        })
                    )
            )
    }
}
