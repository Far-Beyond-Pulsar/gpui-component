use gpui::*;
use super::entry_screen::{EntryScreen, ProjectSelected};

pub struct EntryWindow {
    entry_screen: Option<Entity<EntryScreen>>,
}

impl EntryWindow {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let entry_screen = cx.new(|cx| EntryScreen::new(window, cx));

        // Forward ProjectSelected events from EntryScreen to EntryWindow
        cx.subscribe(&entry_screen, |_this, _screen, event: &ProjectSelected, cx| {
            cx.emit(event.clone());
        }).detach();

        Self {
            entry_screen: Some(entry_screen),
        }
    }

    pub fn new_placeholder(cx: &mut Context<Self>) -> Self {
        Self {
            entry_screen: None,
        }
    }
}

impl EventEmitter<ProjectSelected> for EntryWindow {}

impl Render for EntryWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if let Some(screen) = &self.entry_screen {
            screen.clone().into_any_element()
        } else {
            div().into_any_element()
        }
    }
}
