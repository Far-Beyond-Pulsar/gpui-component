use crate::settings::EngineSettings;
use gpui::*;
use gpui_component::label::Label;
use gpui_component::menu::popup_menu::PopupMenuExt;
use gpui_component::{
    button::{Button, ButtonVariants},
    h_flex, v_flex, ActiveTheme, Icon, IconName, Theme, ThemeRegistry, StyledExt,
    scroll::ScrollbarAxis,
    switch::Switch,
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
    /// Currently selected theme (may be unsaved)
    selected_theme: String,
}

impl SettingsScreen {
    pub fn new(props: SettingsScreenProps, _cx: &App) -> Self {
        let settings = EngineSettings::load(&props.config_path);
        let selected_theme = settings.active_theme.clone();
        Self {
            settings,
            config_path: props.config_path,
            selected_theme,
        }
    }
}

impl Render for SettingsScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme_names: Vec<String> = ThemeRegistry::global(cx)
            .sorted_themes()
            .iter()
            .map(|t| t.name.to_string())
            .collect();

        // Update selected_theme if it's not in the current list
        if !theme_names.contains(&self.selected_theme) {
            self.selected_theme = cx.theme().theme_name().to_string();
        }

        // Create sections first (these borrow cx mutably)
        let appearance_section = self.render_section(
            "Appearance",
            IconName::Palette,
            vec![
                self.render_theme_setting(&theme_names, cx),
            ],
            cx
        );

        let editor_section = self.render_section(
            "Editor",
            IconName::Code,
            vec![
                self.render_font_size_setting(cx),
                self.render_line_numbers_setting(cx),
                self.render_word_wrap_setting(cx),
            ],
            cx
        );

        let project_section = self.render_section(
            "Project",
            IconName::Folder,
            vec![
                self.render_default_project_path_setting(cx),
                self.render_auto_save_setting(cx),
                self.render_backup_setting(cx),
            ],
            cx
        );

        let advanced_section = self.render_section(
            "Advanced",
            IconName::Settings,
            vec![
                self.render_performance_setting(cx),
                self.render_debugging_setting(cx),
                self.render_extensions_setting(cx),
            ],
            cx
        );

        // Now get theme after mutable borrows are done
        let theme = cx.theme();

        v_flex()
            .size_full()
            .bg(theme.background)
            .on_action(cx.listener(|screen, action: &SelectThemeAction, w: &mut gpui::Window, cx| {
                screen.selected_theme = action.theme_name.to_string();
                if let Some(theme_config) = ThemeRegistry::global(cx).themes().get(&action.theme_name).cloned() {
                    Theme::global_mut(cx).apply_config(&theme_config);
                    cx.refresh_windows();
                }
                cx.notify();
            }))
            .child(
                // Header section (fixed)
                v_flex()
                    .p_6()
                    .gap_2()
                    .child(
                        Label::new("Settings")
                            .text_3xl()
                            .text_color(theme.foreground)
                            .font_weight(gpui::FontWeight::BOLD)
                    )
                    .child(
                        Label::new("Customize your Pulsar Engine experience")
                            .text_base()
                            .text_color(theme.muted_foreground)
                    )
            )
            .child(
                // Scrollable content area
                div()
                    .flex_1()
                    .scrollable(ScrollbarAxis::Vertical)
                    .child(
                        v_flex()
                            .p_6()
                            .pt_0()
                            .gap_6()
                            .child(appearance_section)
                            .child(editor_section)
                            .child(project_section)
                            .child(advanced_section)
                    )
            )
            .child(
                // Footer (fixed)
                v_flex()
                    .gap_4()
                    .p_6()
                    .pt_0()
                    .border_t_1()
                    .border_color(theme.border)
                    .child(
                        h_flex()
                            .justify_between()
                            .items_center()
                            .child(
                                Label::new("Config file location:")
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                            )
                            .child(
                                Button::new("open-config-folder")
                                    .ghost()
                                    .label("Open Folder")
                                    .icon(IconName::Folder)
                                    .on_click(cx.listener(|_this, _, _window, cx| {
                                        // TODO: Open config folder in file explorer
                                        cx.notify();
                                    }))
                            )
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(self.config_path.to_string_lossy().to_string())
                    )
            )
    }
}

impl SettingsScreen {
    fn render_section(
        &self,
        title: &str,
        icon: IconName,
        settings: Vec<AnyElement>,
        cx: &mut Context<Self>
    ) -> AnyElement {
        let theme = cx.theme();

        v_flex()
            .gap_4()
            .p_6()
            .bg(theme.background)
            .border_1()
            .border_color(theme.border)
            .rounded(px(8.))
            .child(
                h_flex()
                    .items_center()
                    .gap_3()
                    .child(
                        Icon::new(icon)
                            .size(px(24.))
                            .text_color(theme.accent)
                    )
                    .child(
                        Label::new(title.to_string())
                            .text_xl()
                            .text_color(theme.foreground)
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                    )
            )
            .children(settings)
            .into_any_element()
    }

    fn render_theme_setting(&self, theme_names: &[String], cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();

        v_flex()
            .gap_3()
            .child(
                Label::new("Theme")
                    .text_base()
                    .text_color(theme.foreground)
                    .font_weight(gpui::FontWeight::MEDIUM)
            )
            .child(
                Label::new("Choose your preferred visual theme")
                    .text_sm()
                    .text_color(theme.muted_foreground)
            )
            .child(
                h_flex()
                    .gap_4()
                    .items_center()
                    .child(
                        Button::new("theme-dropdown")
                            .label(&self.selected_theme)
                            .icon(IconName::Palette)
                            .popup_menu({
                                let theme_names = theme_names.to_vec();
                                let selected = self.selected_theme.clone();
                                move |menu, _w: &mut gpui::Window, _cx| {
                                    let mut menu = menu.scrollable().max_h(px(300.));
                                    for name in &theme_names {
                                        let is_selected = name == &selected;
                                        menu = menu.menu_with_check(
                                            name.clone(),
                                            is_selected,
                                            Box::new(SelectThemeAction::new(SharedString::from(name.clone()))),
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
                            .on_click(cx.listener(|screen, _, _window: &mut gpui::Window, cx| {
                                screen.settings.active_theme = screen.selected_theme.clone();
                                screen.settings.save(&screen.config_path);
                                cx.notify();
                            }))
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .max_w(px(300.))
                            .child("Theme changes are applied instantly, but only saved when you click Save.")
                    )
            )
            .into_any_element()
    }

    fn render_font_size_setting(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();

        v_flex()
            .gap_3()
            .pt_4()
            .border_t_1()
            .border_color(theme.border)
            .child(
                Label::new("Font Size")
                    .text_base()
                    .text_color(theme.foreground)
                    .font_weight(gpui::FontWeight::MEDIUM)
            )
            .child(
                Label::new("Set the font size for the editor")
                    .text_sm()
                    .text_color(theme.muted_foreground)
            )
            .child(
                h_flex()
                    .gap_4()
                    .items_center()
                    .child(
                        Label::new(format!("{:.1}", self.settings.editor.font_size))
                            .text_sm()
                            .text_color(theme.foreground)
                            .bg(theme.background)
                            .border_1()
                            .border_color(theme.border)
                            .rounded(px(4.))
                            .p_2()
                    )
                    .child(
                        Button::new("save-font-size")
                            .primary()
                            .label("Save")
                            .on_click(cx.listener(|screen, _, _window, cx| {
                                screen.settings.save(&screen.config_path);
                                cx.notify();
                            }))
                    )
            )
            .into_any_element()
    }

    fn render_line_numbers_setting(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();

        v_flex()
            .gap_3()
            .pt_4()
            .border_t_1()
            .border_color(theme.border)
            .child(
                Label::new("Line Numbers")
                    .text_base()
                    .text_color(theme.foreground)
                    .font_weight(gpui::FontWeight::MEDIUM)
            )
            .child(
                Label::new("Show or hide line numbers in the editor")
                    .text_sm()
                    .text_color(theme.muted_foreground)
            )
            .child(
                h_flex()
                    .gap_4()
                    .items_center()
                    .child(
                        Switch::new("line-numbers-switch")
                            .checked(self.settings.editor.show_line_numbers)
                            .on_click(cx.listener(|screen, _, _window, cx| {
                                screen.settings.editor.show_line_numbers = !screen.settings.editor.show_line_numbers;
                                cx.notify();
                            }))
                    )
                    .child(
                        Label::new(if self.settings.editor.show_line_numbers { "Enabled" } else { "Disabled" })
                            .text_sm()
                            .text_color(theme.foreground)
                    )
                    .child(
                        Button::new("save-line-numbers")
                            .primary()
                            .label("Save")
                            .on_click(cx.listener(|screen, _, _window, cx| {
                                screen.settings.save(&screen.config_path);
                                cx.notify();
                            }))
                    )
            )
            .into_any_element()
    }

    fn render_word_wrap_setting(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();

        v_flex()
            .gap_3()
            .pt_4()
            .border_t_1()
            .border_color(theme.border)
            .child(
                Label::new("Word Wrap")
                    .text_base()
                    .text_color(theme.foreground)
                    .font_weight(gpui::FontWeight::MEDIUM)
            )
            .child(
                Label::new("Enable or disable automatic word wrapping")
                    .text_sm()
                    .text_color(theme.muted_foreground)
            )
            .child(
                h_flex()
                    .gap_4()
                    .items_center()
                    .child(
                        Switch::new("word-wrap-switch")
                            .checked(self.settings.editor.word_wrap)
                            .on_click(cx.listener(|screen, _, _window, cx| {
                                screen.settings.editor.word_wrap = !screen.settings.editor.word_wrap;
                                cx.notify();
                            }))
                    )
                    .child(
                        Label::new(if self.settings.editor.word_wrap { "Enabled" } else { "Disabled" })
                            .text_sm()
                            .text_color(theme.foreground)
                    )
                    .child(
                        Button::new("save-word-wrap")
                            .primary()
                            .label("Save")
                            .on_click(cx.listener(|screen, _, _window, cx| {
                                screen.settings.save(&screen.config_path);
                                cx.notify();
                            }))
                    )
            )
            .into_any_element()
    }

    fn render_default_project_path_setting(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();

        v_flex()
            .gap_3()
            .pt_4()
            .border_t_1()
            .border_color(theme.border)
            .child(
                Label::new("Default Project Path")
                    .text_base()
                    .text_color(theme.foreground)
                    .font_weight(gpui::FontWeight::MEDIUM)
            )
            .child(
                Label::new("Set the default directory for new projects")
                    .text_sm()
                    .text_color(theme.muted_foreground)
            )
            .child(
                h_flex()
                    .gap_4()
                    .items_center()
                    .child(
                        Label::new(self.settings.project.default_project_path.as_deref().unwrap_or("Not set").to_string())
                            .text_sm()
                            .text_color(theme.foreground)
                            .bg(theme.background)
                            .border_1()
                            .border_color(theme.border)
                            .rounded(px(4.))
                            .p_2()
                    )
                    .child(
                        Button::new("browse-project-path")
                            .ghost()
                            .label("Browse")
                            .icon(IconName::Folder)
                            .on_click(cx.listener(|_this, _, _window, cx| {
                                // TODO: Implement folder picker
                                cx.notify();
                            }))
                    )
                    .child(
                        Button::new("save-project-path")
                            .primary()
                            .label("Save")
                            .on_click(cx.listener(|screen, _, _window, cx| {
                                screen.settings.save(&screen.config_path);
                                cx.notify();
                            }))
                    )
            )
            .into_any_element()
    }

    fn render_auto_save_setting(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();

        v_flex()
            .gap_3()
            .pt_4()
            .border_t_1()
            .border_color(theme.border)
            .child(
                Label::new("Auto Save")
                    .text_base()
                    .text_color(theme.foreground)
                    .font_weight(gpui::FontWeight::MEDIUM)
            )
            .child(
                Label::new("Automatically save project changes")
                    .text_sm()
                    .text_color(theme.muted_foreground)
            )
            .child(
                h_flex()
                    .gap_4()
                    .items_center()
                    .child(
                        Switch::new("auto-save-switch")
                            .checked(self.settings.project.auto_save_interval > 0)
                            .on_click(cx.listener(|screen, _, _window, cx| {
                                if screen.settings.project.auto_save_interval > 0 {
                                    screen.settings.project.auto_save_interval = 0;
                                } else {
                                    screen.settings.project.auto_save_interval = 30; // Default 30 seconds
                                }
                                cx.notify();
                            }))
                    )
                    .child(
                        Label::new(if self.settings.project.auto_save_interval > 0 { "Enabled" } else { "Disabled" })
                            .text_sm()
                            .text_color(theme.foreground)
                    )
                    .child(
                        Label::new(format!("Interval: {} seconds", self.settings.project.auto_save_interval))
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .bg(theme.background)
                            .border_1()
                            .border_color(theme.border)
                            .rounded(px(4.))
                            .p_2()
                    )
                    .child(
                        Label::new("seconds")
                            .text_sm()
                            .text_color(theme.muted_foreground)
                    )
                    .child(
                        Button::new("save-auto-save")
                            .primary()
                            .label("Save")
                            .on_click(cx.listener(|screen, _, _window, cx| {
                                screen.settings.save(&screen.config_path);
                                cx.notify();
                            }))
                    )
            )
            .into_any_element()
    }

    fn render_backup_setting(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();

        v_flex()
            .gap_3()
            .pt_4()
            .border_t_1()
            .border_color(theme.border)
            .child(
                Label::new("Backup Settings")
                    .text_base()
                    .text_color(theme.foreground)
                    .font_weight(gpui::FontWeight::MEDIUM)
            )
            .child(
                Label::new("Configure automatic project backups")
                    .text_sm()
                    .text_color(theme.muted_foreground)
            )
            .child(
                h_flex()
                    .gap_4()
                    .items_center()
                    .child(
                        Switch::new("backup-enabled-switch")
                            .checked(self.settings.project.enable_backups)
                            .on_click(cx.listener(|screen, _, _window, cx| {
                                screen.settings.project.enable_backups = !screen.settings.project.enable_backups;
                                cx.notify();
                            }))
                    )
                    .child(
                        Label::new(if self.settings.project.enable_backups { "Enabled" } else { "Disabled" })
                            .text_sm()
                            .text_color(theme.foreground)
                    )
                    .child(
                        Label::new("Backups are automatically created when saving projects")
                            .text_sm()
                            .text_color(theme.muted_foreground)
                    )
                    .child(
                        Button::new("save-backup")
                            .primary()
                            .label("Save")
                            .on_click(cx.listener(|screen, _, _window, cx| {
                                screen.settings.save(&screen.config_path);
                                cx.notify();
                            }))
                    )
            )
            .into_any_element()
    }

    fn render_performance_setting(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();
        
        let fps_options = vec![30u32, 60, 120, 144, 240, 0];
        let current_fps = self.settings.advanced.max_viewport_fps;

        v_flex()
            .gap_3()
            .pt_4()
            .border_t_1()
            .border_color(theme.border)
            .child(
                Label::new("Performance Settings")
                    .text_base()
                    .text_color(theme.foreground)
                    .font_weight(gpui::FontWeight::MEDIUM)
            )
            .child(
                Label::new("Configure performance-related options")
                    .text_sm()
                    .text_color(theme.muted_foreground)
            )
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        Label::new(format!("Performance Level: {}", self.settings.advanced.performance_level))
                            .text_sm()
                            .text_color(theme.foreground)
                            .bg(theme.background)
                            .border_1()
                            .border_color(theme.border)
                            .rounded(px(4.))
                            .p_2()
                    )
                    .child(
                        Label::new("Higher levels may improve performance but use more resources")
                            .text_sm()
                            .text_color(theme.muted_foreground)
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                Label::new("Viewport Max FPS (Frame Pacing)")
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .font_weight(gpui::FontWeight::MEDIUM)
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .children(fps_options.iter().map(|&fps| {
                                        let label = if fps == 0 { "Unlimited".to_string() } else { format!("{} FPS", fps) };
                                        let is_selected = current_fps == fps;
                                        
                                        let mut btn = Button::new(SharedString::from(format!("fps-{}", fps)))
                                            .label(label);
                                        
                                        if is_selected {
                                            btn = btn.primary();
                                        } else {
                                            btn = btn.ghost();
                                        }
                                        
                                        btn.on_click(cx.listener(move |screen, _, _window, cx| {
                                            screen.settings.advanced.max_viewport_fps = fps;
                                            screen.settings.save(&screen.config_path);
                                            cx.notify();
                                        }))
                                    }))
                            )
                            .child(
                                Label::new("Controls viewport refresh rate for consistent frame pacing")
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                            )
                    )
                    .child(
                        Button::new("save-performance")
                            .primary()
                            .label("Save All Settings")
                            .on_click(cx.listener(|screen, _, _window, cx| {
                                screen.settings.save(&screen.config_path);
                                cx.notify();
                            }))
                    )
            )
            .into_any_element()
    }

    fn render_debugging_setting(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();

        v_flex()
            .gap_3()
            .pt_4()
            .border_t_1()
            .border_color(theme.border)
            .child(
                Label::new("Debugging Options")
                    .text_base()
                    .text_color(theme.foreground)
                    .font_weight(gpui::FontWeight::MEDIUM)
            )
            .child(
                Label::new("Configure debugging and development features")
                    .text_sm()
                    .text_color(theme.muted_foreground)
            )
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        h_flex()
                            .gap_4()
                            .items_center()
                            .child(
                                Switch::new("debug-logging-switch")
                                    .checked(self.settings.advanced.debug_logging)
                                    .on_click(cx.listener(|screen, _, _window, cx| {
                                        screen.settings.advanced.debug_logging = !screen.settings.advanced.debug_logging;
                                        cx.notify();
                                    }))
                            )
                            .child(
                                Label::new("Debug Logging")
                                    .text_sm()
                                    .text_color(theme.foreground)
                            )
                    )
                    .child(
                        h_flex()
                            .gap_4()
                            .items_center()
                            .child(
                                Switch::new("experimental-features-switch")
                                    .checked(self.settings.advanced.experimental_features)
                                    .on_click(cx.listener(|screen, _, _window, cx| {
                                        screen.settings.advanced.experimental_features = !screen.settings.advanced.experimental_features;
                                        cx.notify();
                                    }))
                            )
                            .child(
                                Label::new("Experimental Features")
                                    .text_sm()
                                    .text_color(theme.foreground)
                            )
                    )
                    .child(
                        Button::new("save-debugging")
                            .primary()
                            .label("Save")
                            .on_click(cx.listener(|screen, _, _window, cx| {
                                screen.settings.save(&screen.config_path);
                                cx.notify();
                            }))
                    )
            )
            .into_any_element()
    }

    fn render_extensions_setting(&self, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();

        v_flex()
            .gap_3()
            .pt_4()
            .border_t_1()
            .border_color(theme.border)
            .child(
                Label::new("Extensions")
                    .text_base()
                    .text_color(theme.foreground)
                    .font_weight(gpui::FontWeight::MEDIUM)
            )
            .child(
                Label::new("Manage installed extensions and plugins")
                    .text_sm()
                    .text_color(theme.muted_foreground)
            )
            .child(
                v_flex()
                    .gap_3()
                    .child(
                        Label::new("Extension management features are coming soon")
                            .text_sm()
                            .text_color(theme.muted_foreground)
                    )
            )
            .into_any_element()
    }

    fn render_placeholder_setting(&self, title: &str, description: &str, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme();

        v_flex()
            .gap_2()
            .pt_4()
            .border_t_1()
            .border_color(theme.border)
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                Label::new(title.to_string())
                                    .text_base()
                                    .text_color(theme.foreground)
                                    .font_weight(gpui::FontWeight::MEDIUM)
                            )
                            .child(
                                Label::new(description.to_string())
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                            )
                    )
                    .child(
                        Button::new("configure-placeholder")
                            .ghost()
                            .label("Configure")
                            .on_click(cx.listener(move |_this, _, _window, cx| {
                                // TODO: Implement configuration for this setting
                                cx.notify();
                            }))
                    )
            )
            .into_any_element()
    }
}

#[derive(Clone, PartialEq, Eq, gpui::Action)]
#[action(namespace = ui, no_json)]
struct SelectThemeAction {
    theme_name: SharedString,
}

impl SelectThemeAction {
    pub fn new(theme_name: SharedString) -> Self {
        Self { theme_name }
    }
}
