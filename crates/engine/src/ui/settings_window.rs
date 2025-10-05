use crate::settings::EngineSettings;
use crate::ui::settings_screen::SettingsScreen;
use gpui::*;

pub struct SettingsWindow {
    settings_screen: Option<Entity<SettingsScreen>>,
}


    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {

        // Pass SettingsScreenProps as required by SettingsScreen::new
        let config_path = EngineSettings::default_path().expect("No config path found");

        let props = crate::ui::settings_screen::SettingsScreenProps {
            config_path: config_path.clone(),
        };
        let settings_screen = cx.new(|_cx| SettingsScreen::new(props));

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
