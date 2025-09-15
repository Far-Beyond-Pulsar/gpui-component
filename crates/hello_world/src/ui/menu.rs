use gpui::*;
use gpui_component::{
    button::Button,
    popup_menu::PopupMenu,
    h_flex,
    ActiveTheme as _, StyledExt,
    IconName,
};

pub struct MenuBar {
    active_menu: Option<String>,
}

impl MenuBar {
    pub fn new() -> Self {
        Self {
            active_menu: None,
        }
    }

    pub fn render(&self, cx: &mut App) -> impl IntoElement {
        h_flex()
            .w_full()
            .h_8()
            .bg(cx.theme().background)
            .border_b_1()
            .border_color(cx.theme().border)
            .items_center()
            .px_2()
            .gap_1()
            .child(self.render_menu_item("File", cx))
            .child(self.render_menu_item("Edit", cx))
            .child(self.render_menu_item("View", cx))
            .child(self.render_menu_item("Build", cx))
            .child(self.render_menu_item("Debug", cx))
            .child(self.render_menu_item("Tools", cx))
            .child(self.render_menu_item("Window", cx))
            .child(self.render_menu_item("Help", cx))
    }

    fn render_menu_item(&self, label: &str, cx: &mut App) -> impl IntoElement {
        let is_active = self.active_menu.as_ref() == Some(&label.to_string());

        Button::new(label)
            .child(label)
            .when(is_active, |this| this.selected())
            .on_click(move |_, _, _| {
                // Menu click handling would go here
            })
    }

    pub fn get_file_menu() -> PopupMenu {
        PopupMenu::new("file_menu")
            .menu("New Project", Box::new(NewProject))
            .menu("Open Project", Box::new(OpenProject))
            .separator()
            .menu("New Scene", Box::new(NewScene))
            .menu("Open Scene", Box::new(OpenScene))
            .menu("Save Scene", Box::new(SaveScene))
            .menu("Save Scene As...", Box::new(SaveSceneAs))
            .separator()
            .menu("Import Asset", Box::new(ImportAsset))
            .menu("Export Selection", Box::new(ExportSelection))
            .separator()
            .menu("Recent Projects", Box::new(RecentProjects))
            .separator()
            .menu("Exit", Box::new(ExitApp))
    }

    pub fn get_edit_menu() -> PopupMenu {
        PopupMenu::new("edit_menu")
            .menu("Undo", Box::new(Undo))
            .menu("Redo", Box::new(Redo))
            .separator()
            .menu("Cut", Box::new(Cut))
            .menu("Copy", Box::new(Copy))
            .menu("Paste", Box::new(Paste))
            .menu("Delete", Box::new(Delete))
            .separator()
            .menu("Select All", Box::new(SelectAll))
            .menu("Deselect All", Box::new(DeselectAll))
            .separator()
            .menu("Preferences", Box::new(ShowPreferences))
    }

    pub fn get_view_menu() -> PopupMenu {
        PopupMenu::new("view_menu")
            .menu("Level Editor", Box::new(ShowLevelEditor))
            .menu("Script Editor", Box::new(ShowScriptEditor))
            .menu("Blueprint Editor", Box::new(ShowBlueprintEditor))
            .menu("Material Editor", Box::new(ShowMaterialEditor))
            .separator()
            .menu("Console", Box::new(ToggleConsole))
            .menu("Output", Box::new(ToggleOutput))
            .menu("Properties", Box::new(ToggleProperties))
            .menu("Scene Hierarchy", Box::new(ToggleHierarchy))
            .separator()
            .menu("Full Screen", Box::new(ToggleFullScreen))
    }
}

// Action definitions
actions!(
    pulsar,
    [
        NewProject,
        OpenProject,
        NewScene,
        OpenScene,
        SaveScene,
        SaveSceneAs,
        ImportAsset,
        ExportSelection,
        RecentProjects,
        ExitApp,
        Undo,
        Redo,
        Cut,
        Copy,
        Paste,
        Delete,
        SelectAll,
        DeselectAll,
        ShowPreferences,
        ShowLevelEditor,
        ShowScriptEditor,
        ShowBlueprintEditor,
        ShowMaterialEditor,
        ToggleConsole,
        ToggleOutput,
        ToggleProperties,
        ToggleHierarchy,
        ToggleFullScreen,
    ]
);