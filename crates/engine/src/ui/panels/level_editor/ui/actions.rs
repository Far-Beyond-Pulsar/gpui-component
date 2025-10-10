use gpui::actions;
use serde::Deserialize;
use schemars::JsonSchema;

// Define actions for the level editor
actions!(
    level_editor,
    [
        // Transform tools
        SelectTool,
        MoveTool,
        RotateTool,
        ScaleTool,

        // Scene operations
        NewScene,
        OpenScene,
        SaveScene,
        SaveSceneAs,

        // Object operations
        AddObject,
        DeleteObject,
        DuplicateObject,
        RenameObject,

        // Hierarchy operations
        ExpandAll,
        CollapseAll,

        // View operations
        ToggleGrid,
        ToggleWireframe,
        ToggleLighting,
        TogglePerformanceOverlay,

        // Camera modes
        PerspectiveView,
        OrthographicView,
        TopView,
        FrontView,
        SideView,

        // Edit operations
        Undo,
        Redo,
        Copy,
        Paste,
        Cut,
    ]
);

#[derive(gpui::Action, Clone, PartialEq, Deserialize, JsonSchema)]
#[action(namespace = level_editor)]
pub struct AddObjectOfType {
    pub object_type: String,
}

#[derive(gpui::Action, Clone, PartialEq, Deserialize, JsonSchema)]
#[action(namespace = level_editor)]
pub struct SelectObject {
    pub object_id: String,
}

#[derive(gpui::Action, Clone, PartialEq, Deserialize, JsonSchema)]
#[action(namespace = level_editor)]
pub struct ToggleObjectExpanded {
    pub object_id: String,
}

#[derive(gpui::Action, Clone, PartialEq, Deserialize, JsonSchema)]
#[action(namespace = level_editor)]
pub struct UpdateObjectProperty {
    pub object_id: String,
    pub property: String,
    pub value: String,
}
