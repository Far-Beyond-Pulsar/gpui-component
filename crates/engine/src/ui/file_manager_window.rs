//! File Manager Window - Standalone file browser and manager
//! Similar to the drawer but in a separate window

use gpui::*;
use gpui_component::{
    v_flex, ActiveTheme as _, TitleBar,
};

use super::file_manager_drawer::{FileManagerDrawer, FileSelected};
use super::app::PulsarApp;

pub struct FileManagerWindow {
    file_manager: Entity<FileManagerDrawer>,
    parent_app: Entity<PulsarApp>,
}

impl FileManagerWindow {
    pub fn new(
        file_manager: Entity<FileManagerDrawer>,
        parent_app: Entity<PulsarApp>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // Subscribe to file selected events and forward them to parent window
        cx.subscribe_in(&file_manager, window, Self::on_file_selected).detach();

        Self { 
            file_manager,
            parent_app,
        }
    }

    pub fn file_manager(&self) -> &Entity<FileManagerDrawer> {
        &self.file_manager
    }

    fn on_file_selected(
        &mut self,
        _drawer: &Entity<FileManagerDrawer>,
        event: &FileSelected,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Forward the event to the parent app
        let event_clone = event.clone();
        let parent = self.parent_app.clone();
        
        // Update parent in its own window
        parent.update(cx, |app, cx| {
            app.handle_file_selected_from_external_window(&event_clone, window, cx);
        });
    }
}

impl Render for FileManagerWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        v_flex()
            .size_full()
            .bg(theme.background)
            .child(TitleBar::new().child("File Manager"))
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(self.file_manager.clone())
            )
    }
}
