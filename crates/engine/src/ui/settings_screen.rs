use crate::settings::EngineSettings;
use gpui::*;
use gpui_component::label::Label;
use gpui_component::menu::popup_menu::PopupMenuExt;
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex, v_flex, ActiveTheme, Icon, IconName, Theme, ThemeRegistry,
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
    pub fn new(props: SettingsScreenProps, cx: &App) -> Self {
        let settings = EngineSettings::load(&props.config_path);
        let theme_names = ThemeRegistry::global(cx)
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
            .on_action(cx.listener(|screen: &mut SettingsScreen, action: &SelectThemeAction, _window: &mut Window, cx: &mut Context<SettingsScreen>| {
                screen.selected_theme = action.theme_name.clone();
                cx.notify();
            }))
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
                        Label::new("Settings")
                            .text_2xl()
                            .text_color(theme.foreground)
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
                        Label::new("Theme")
                            .text_lg()
                            .text_color(theme.foreground)
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
                                        move |menu, window, cx| {
                                            let mut menu = menu.scrollable().max_h(px(400.));
                                            for name in &theme_names {
                                                let is_selected = *name == selected;
                                                // Use a custom Action type for theme selection

                                                menu = menu.menu_with_check(
                                                    name.clone(),
                                                    is_selected,
                                                    Box::new(SelectThemeAction::new(name.clone())),
                                                );

                                            }
                                            menu
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
/// Custom Action for theme selection in the popup menu
#[derive(Clone, PartialEq, Eq, gpui::Action)]
#[action(namespace = ui, no_json)]
struct SelectThemeAction {
    theme_name: String,
}

impl SelectThemeAction {
    pub fn new(theme_name: String) -> Self {
        Self { theme_name }
    }
}
