use crate::settings::EngineSettings;
use crate::ui::settings_screen::SettingsScreen;
use gpui::*;

pub struct SettingsWindow {
    settings_screen: Option<Entity<SettingsScreen>>,
}

impl SettingsWindow {}

impl Render for SettingsWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(screen) = &self.settings_screen {
            screen.clone().into_any_element()
        } else {
            div().into_any_element()
        }
    }
}
