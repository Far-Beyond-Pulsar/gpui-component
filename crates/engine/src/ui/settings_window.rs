use crate::settings::EngineSettings;
use crate::ui::settings_screen::SettingsScreen;
use gpui::*;

pub struct SettingsWindow {
    settings_screen: Option<Entity<SettingsScreen>>,
}

impl SettingsWindow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // You may want to pass the config path here if needed
        let config_path = EngineSettings::default_path().expect("No config path found");
        let settings_screen = cx.new(|_cx| SettingsScreen::new(config_path.clone()));

        Self {
            settings_screen: Some(settings_screen),
        }
    }

    pub fn new_placeholder(_cx: &mut Context<Self>) -> Self {
        Self {
            settings_screen: None,
        }
    }
}

impl Render for SettingsWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(screen) = &self.settings_screen {
            screen.clone().into_any_element()
        } else {
            div().into_any_element()
        }
    }
}
