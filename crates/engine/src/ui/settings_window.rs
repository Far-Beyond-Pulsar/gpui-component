use crate::settings::EngineSettings;
use crate::ui::settings_screen::{SettingsScreen, SettingsScreenProps};
use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex, v_flex, Icon, IconName, ActiveTheme,
};
use std::path::PathBuf;

pub struct SettingsWindow {
    settings_screen: Option<Entity<SettingsScreen>>,
}

impl SettingsWindow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let proj_dirs = directories::ProjectDirs::from("com", "Pulsar", "Pulsar_Engine")
            .expect("Could not determine app data directory");
        let appdata_dir = proj_dirs.data_dir();
        let config_dir = appdata_dir.join("configs");
        let config_file = config_dir.join("engine.toml");

        let settings_screen = cx.new(|cx| SettingsScreen::new(
            SettingsScreenProps {
                config_path: config_file,
            },
            cx
        ));

        Self {
            settings_screen: Some(settings_screen),
        }
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .size_full()
            .bg(theme.background)
            .child(
                // Custom titlebar matching the app style
                h_flex()
                    .h(px(40.))
                    .bg(theme.background)
                    .border_b_1()
                    .border_color(theme.border)
                    .px_4()
                    .items_center()
                    .justify_between()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_3()
                            .child(
                                Icon::new(IconName::Settings)
                                    .size(px(20.))
                                    .text_color(theme.accent)
                            )
                            .child(
                                div()
                                    .text_base()
                                    .text_color(theme.foreground)
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("Settings")
                            )
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                Button::new("minimize-settings")
                                    .ghost()
                                    .compact()
                                    .icon(IconName::Minus)
                                    .on_click(cx.listener(|_this, _, window, cx| {
                                        window.minimize_window();
                                        cx.notify();
                                    }))
                            )
                            .child(
                                Button::new("close-settings")
                                    .ghost()
                                    .compact()
                                    .icon(IconName::Close)
                                    .on_click(cx.listener(|_this, _, window, cx| {
                                        window.remove_window();
                                        cx.notify();
                                    }))
                            )
                    )
            )
            .child(
                if let Some(screen) = &self.settings_screen {
                    screen.clone().into_any_element()
                } else {
                    div().into_any_element()
                }
            )
    }
}
