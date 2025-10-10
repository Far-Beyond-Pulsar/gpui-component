//! File Manager Window - Standalone file browser and manager
//! Similar to the drawer but in a separate window

use gpui::*;
use gpui_component::{
    v_flex, ActiveTheme as _, TitleBar,
};

use super::file_manager_drawer::FileManagerDrawer;

pub struct FileManagerWindow {
    file_manager: Entity<FileManagerDrawer>,
}

impl FileManagerWindow {
    pub fn new(
        file_manager: Entity<FileManagerDrawer>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self { file_manager }
    }

    pub fn file_manager(&self) -> &Entity<FileManagerDrawer> {
        &self.file_manager
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
