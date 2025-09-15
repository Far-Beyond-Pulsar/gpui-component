use gpui::{prelude::FluentBuilder, *};
use gpui_component::{
    button::Button,
    popup_menu::PopupMenuExt as _,
    h_flex,
    Selectable,
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
            .child(self.render_file_menu(cx))
            .child(self.render_edit_menu(cx))
            .child(self.render_view_menu(cx))
            .child(self.render_tools_menu(cx))
            .child(self.render_menu_item("Window", cx))
            .child(self.render_menu_item("Help", cx))
    }

    fn render_menu_item(&self, label: &str, cx: &mut App) -> impl IntoElement {
        let is_active = self.active_menu.as_ref().map(|s| s.as_str()) == Some(label);

        let label_string = label.to_string();
        Button::new(label_string.as_str())
            .label(label_string.as_str())
            .when(is_active, |this| this.selected(true))
            .on_click(move |_, _, _| {
                // Menu click handling would go here
            })
    }

    fn render_file_menu(&self, cx: &mut App) -> impl IntoElement {
        Button::new("file")
            .label("File")
            .popup_menu(move |this, _window, _cx| {
                this.menu("New Project", Box::new(NewProject))
                    .menu("Open Project", Box::new(OpenProject))
                    .separator()
                    .menu("New Scene", Box::new(NewScene))
                    .menu("Save Scene", Box::new(SaveScene))
                    .separator()
                    .menu_with_icon("Import Asset", IconName::ArrowUp, Box::new(ImportAsset))
                    .separator()
                    .menu("Exit", Box::new(ExitApp))
            })
    }

    fn render_edit_menu(&self, cx: &mut App) -> impl IntoElement {
        Button::new("edit")
            .label("Edit")
            .popup_menu(move |this, _window, _cx| {
                this.menu_with_icon("Undo", IconName::ArrowLeft, Box::new(Undo))
                    .menu_with_icon("Redo", IconName::ArrowRight, Box::new(Redo))
                    .separator()
                    .menu_with_icon("Cut", IconName::Close, Box::new(Cut))
                    .menu_with_icon("Copy", IconName::Copy, Box::new(Copy))
                    .menu_with_icon("Paste", IconName::ArrowDown, Box::new(Paste))
                    .separator()
                    .menu("Select All", Box::new(SelectAll))
                    .separator()
                    .menu_with_icon("Preferences", IconName::Settings, Box::new(ShowPreferences))
            })
    }

    fn render_view_menu(&self, cx: &mut App) -> impl IntoElement {
        Button::new("view")
            .label("View")
            .popup_menu(move |this, window, cx| {
                this.submenu("Editors", window, cx, |menu, _, _| {
                        menu.menu("Level Editor", Box::new(ShowLevelEditor))
                            .menu("Script Editor", Box::new(ShowScriptEditor))
                            .menu("Blueprint Editor", Box::new(ShowBlueprintEditor))
                            .menu("Material Editor", Box::new(ShowMaterialEditor))
                    })
                    .separator()
                    .submenu("Panels", window, cx, |menu, _, _| {
                        menu.menu_with_check("Console", true, Box::new(ToggleConsole))
                            .menu_with_check("Properties", true, Box::new(ToggleProperties))
                            .menu_with_check("Scene Hierarchy", true, Box::new(ToggleHierarchy))
                    })
                    .separator()
                    .menu("Full Screen", Box::new(ToggleFullScreen))
            })
    }

    fn render_tools_menu(&self, cx: &mut App) -> impl IntoElement {
        Button::new("tools")
            .label("Tools")
            .popup_menu(move |this, _window, _cx| {
                this.menu_with_icon("Asset Browser", IconName::Folder, Box::new(ShowAssetBrowser))
                    .menu_with_icon("Console", IconName::Inspector, Box::new(ShowConsole))
                    .separator()
                    .menu("Build Project", Box::new(BuildProject))
                    .menu_with_icon("Run Game", IconName::Check, Box::new(RunGame))
                    .separator()
                    .menu("Export Project", Box::new(ExportProject))
            })
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
        ShowAssetBrowser,
        ShowConsole,
        BuildProject,
        RunGame,
        ExportProject,
    ]
);