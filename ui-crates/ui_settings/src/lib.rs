//! Settings UI
//!
//! Application and project settings

pub mod settings;
pub mod window;

// Re-export main types
pub use window::SettingsWindow;
pub use settings::{SettingsScreen, SettingsScreenProps};

use gpui::*;
use ui::Root;
use engine_state::EngineState;

/// Create a settings window component as a composable piece
pub fn create_settings_component(
    window: &mut Window,
    cx: &mut App,
    _engine_state: &EngineState,
) -> Entity<Root> {
    let settings_view = cx.new(|cx| SettingsWindow::new(window, cx));
    cx.new(|cx| Root::new(settings_view.clone().into(), window, cx))
}
