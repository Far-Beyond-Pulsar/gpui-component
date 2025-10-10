//! File Manager Window - Standalone file browser and manager
//! Similar to the drawer but in a separate window

use gpui::*;
use gpui_component::{
    v_flex, ActiveTheme as _, TitleBar,
};

use super::file_manager_drawer::{FileManagerDrawer, FileSelected};

pub struct FileManagerWindow {
    file_manager: Entity<FileManagerDrawer>,
}

impl FileManagerWindow {
    pub fn new(
        file_manager: Entity<FileManagerDrawer>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        // Subscribe to file selected events and forward them
        cx.subscribe_in(&file_manager, window, Self::on_file_selected).detach();

        Self { file_manager }
    }

    pub fn file_manager(&self) -> &Entity<FileManagerDrawer> {
        &self.file_manager
    }

    fn on_file_selected(
        &mut self,
        _drawer: &Entity<FileManagerDrawer>,
        event: &FileSelected,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        // Forward the event so other windows can handle it
        cx.emit(event.clone());
    }
}

impl EventEmitter<FileSelected> for FileManagerWindow {}

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
