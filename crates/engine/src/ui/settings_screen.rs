use crate::settings::EngineSettings;
use gpui::*;
use gpui_component::{
    button::Button, h_flex, popup_menu::PopupMenuExt, v_flex, ActiveTheme, Icon, IconName, Theme,
    ThemeRegistry,
};
use std::path::PathBuf;

/// Props for the settings screen
pub struct SettingsScreenProps {
    /// Path to the config file (engine.toml)
    pub config_path: PathBuf,
}

/// The settings screen entity
pub struct SettingsScreen {
    /// Current settings loaded from disk
    settings: EngineSettings,
    /// Path to config file
    config_path: PathBuf,
    /// List of available theme names
    theme_names: Vec<String>,
    /// Currently selected theme (may be unsaved)
    selected_theme: String,
}

impl SettingsScreen {
    pub fn new(props: SettingsScreenProps) -> Self {
        let settings = EngineSettings::load(&props.config_path);
        let theme_names = ThemeRegistry::global()
            .sorted_themes()
            .iter()
            .map(|theme| theme.name.to_string())
            .collect::<Vec<_>>();
        let selected_theme = settings.active_theme.clone();
        Self {
            settings,
            config_path: props.config_path,
            theme_names,
            selected_theme,
        }
    }
}

impl Render for SettingsScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .size_full()
            .bg(theme.background)
            .p_8()
            .gap_8()
            .child(
                h_flex()
                    .items_center()
                    .gap_4()
                    .child(
                        Icon::new(IconName::Settings)
                            .size(px(28.))
                            .text_color(theme.accent)
                    )
                    .child(
                        div()
                            .text_2xl()
                            .font_bold()
                            .text_color(theme.foreground)
                            .child("Settings")
                    )
                    .child(
                        Button::new("close-settings")
                            .ghost()
                            .icon(IconName::Close)
                            .on_click(cx.listener(|_screen, _, window, cx| {
                                window.remove_window();
                                cx.notify();
                            }))
                    )
            )
            .child(
                v_flex()
                    .gap_6()
                    .child(
                        div()
                            .text_lg()
                            .font_semibold()
                            .text_color(theme.foreground)
                            .child("Theme")
                    )
                    .child(
                        h_flex()
                            .gap_4()
                            .items_center()
                            .child(
                                div()
                                    .text_base()
                                    .text_color(theme.foreground)
                                    .child("Select a theme:")
                            )
                            .child(
                                Button::new("theme-dropdown")
                                    .label(&self.selected_theme)
                                    .icon(IconName::Palette)
                                    .popup_menu({
                                        let theme_names = self.theme_names.clone();
                                        let selected = self.selected_theme.clone();
                                        move |menu, _, cx| {
                                            let mut menu = menu.scrollable().max_h(px(400.));
                                            for name in &theme_names {
                                                let is_selected = *name == selected;
                                                menu = menu.menu_with_check(
                                                    name.clone(),
                                                    is_selected,
                                                    Box::new(name.clone()),
                                                );
                                            }
                                            menu.on_menu(cx.listener(|screen, name: &String, _, cx| {
                                                screen.selected_theme = name.clone();
                                                // Apply theme immediately
                                                if let Some(theme) = ThemeRegistry::global(cx)
                                                    .themes()
                                                    .get(name)
                                                    .cloned()
                                                {
                                                    Theme::global_mut(cx).apply_config(&theme);
                                                }
                                                cx.notify();
                                            }))
                                        }
                                    })
                            )
                            .child(
                                Button::new("save-theme")
                                    .primary()
                                    .label("Save")
                                    .on_click(cx.listener(|screen, _, _, cx| {
                                        screen.settings.active_theme = screen.selected_theme.clone();
                                        screen.settings.save(&screen.config_path);
                                        cx.notify();
                                    }))
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .ml_4()
                                    .child("Theme changes are applied instantly, but only saved when you click Save.")
                            )
                    )
            )
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child("More settings coming soon...")
            )
    }
}
