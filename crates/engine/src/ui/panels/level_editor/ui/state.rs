use std::path::PathBuf;
use std::collections::HashSet;
use serde::{Deserialize, Serialize};

/// Shared state for the Level Editor
#[derive(Clone, Debug)]
pub struct LevelEditorState {
    /// Currently selected object in the hierarchy
    pub selected_object: Option<String>,
    /// Currently open scene file
    pub current_scene: Option<PathBuf>,
    /// Whether the scene has unsaved changes
    pub has_unsaved_changes: bool,
    /// Current transform tool (Select, Move, Rotate, Scale)
    pub current_tool: TransformTool,
    /// Viewport camera mode
    pub camera_mode: CameraMode,
    /// Viewport rendering options
    pub show_wireframe: bool,
    pub show_lighting: bool,
    pub show_grid: bool,
    pub show_performance_overlay: bool,
    pub show_viewport_controls: bool,
    pub show_camera_mode_selector: bool,
    pub show_viewport_options: bool,
    /// Scene objects (simplified for now)
    pub scene_objects: Vec<SceneObject>,
    /// Expanded state for hierarchy items
    pub expanded_objects: HashSet<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformTool {
    Select,
    Move,
    Rotate,
    Scale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CameraMode {
    Perspective,
    Orthographic,
    Top,
    Front,
    Side,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SceneObject {
    pub id: String,
    pub name: String,
    pub object_type: ObjectType,
    pub transform: Transform,
    pub visible: bool,
    pub children: Vec<SceneObject>,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ObjectType {
    Camera,
    Light,
    Mesh,
    Empty,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Transform {
    pub position: [f32; 3],
    pub rotation: [f32; 3],
    pub scale: [f32; 3],
}

impl Default for LevelEditorState {
    fn default() -> Self {
        Self {
            selected_object: None,
            current_scene: None,
            has_unsaved_changes: false,
            current_tool: TransformTool::Select,
            camera_mode: CameraMode::Perspective,
            show_wireframe: false,
            show_lighting: true,
            show_grid: true,
            show_performance_overlay: true,
            show_viewport_controls: true,
            show_camera_mode_selector: true,
            show_viewport_options: true,
            scene_objects: Self::create_default_scene(),
            expanded_objects: HashSet::new(),
        }
    }
}

impl LevelEditorState {
    pub fn new() -> Self {
        Self::default()
    }

    fn create_default_scene() -> Vec<SceneObject> {
        vec![
            SceneObject {
                id: "main_camera".to_string(),
                name: "Main Camera".to_string(),
                object_type: ObjectType::Camera,
                transform: Transform {
                    position: [0.0, 5.0, 10.0],
                    rotation: [0.0, 0.0, 0.0],
                    scale: [1.0, 1.0, 1.0],
                },
                visible: true,
                children: vec![],
            },
            SceneObject {
                id: "directional_light".to_string(),
                name: "Directional Light".to_string(),
                object_type: ObjectType::Light,
                transform: Transform {
                    position: [5.0, 10.0, 5.0],
                    rotation: [-45.0, 45.0, 0.0],
                    scale: [1.0, 1.0, 1.0],
                },
                visible: true,
                children: vec![],
            },
            SceneObject {
                id: "cube_1".to_string(),
                name: "Cube".to_string(),
                object_type: ObjectType::Mesh,
                transform: Transform {
                    position: [0.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0],
                    scale: [1.0, 1.0, 1.0],
                },
                visible: true,
                children: vec![],
            },
            SceneObject {
                id: "sphere_1".to_string(),
                name: "Sphere".to_string(),
                object_type: ObjectType::Mesh,
                transform: Transform {
                    position: [3.0, 0.0, 0.0],
                    rotation: [0.0, 0.0, 0.0],
                    scale: [1.0, 1.0, 1.0],
                },
                visible: true,
                children: vec![],
            },
        ]
    }

    pub fn select_object(&mut self, object_id: Option<String>) {
        self.selected_object = object_id;
    }

    pub fn get_selected_object(&self) -> Option<&SceneObject> {
        let id = self.selected_object.as_ref()?;
        self.find_object_by_id(id, &self.scene_objects)
    }

    pub fn get_selected_object_mut(&mut self) -> Option<&mut SceneObject> {
        let id = self.selected_object.as_ref()?.clone();
        Self::find_object_by_id_mut_static(&id, &mut self.scene_objects)
    }

    fn find_object_by_id<'a>(&self, id: &str, objects: &'a [SceneObject]) -> Option<&'a SceneObject> {
        for obj in objects {
            if obj.id == id {
                return Some(obj);
            }
            if let Some(child) = self.find_object_by_id(id, &obj.children) {
                return Some(child);
            }
        }
        None
    }

    fn find_object_by_id_mut_static<'a>(id: &str, objects: &'a mut [SceneObject]) -> Option<&'a mut SceneObject> {
        for obj in objects {
            if obj.id == id {
                return Some(obj);
            }
            if let Some(child) = Self::find_object_by_id_mut_static(id, &mut obj.children) {
                return Some(child);
            }
        }
        None
    }

    pub fn add_object(&mut self, object: SceneObject) {
        self.scene_objects.push(object);
        self.has_unsaved_changes = true;
    }

    pub fn remove_selected_object(&mut self) {
        if let Some(id) = &self.selected_object {
            let id = id.clone();
            Self::remove_object_by_id_static(&id, &mut self.scene_objects);
            self.selected_object = None;
            self.has_unsaved_changes = true;
        }
    }

    fn remove_object_by_id_static(id: &str, objects: &mut Vec<SceneObject>) -> bool {
        if let Some(pos) = objects.iter().position(|obj| obj.id == id) {
            objects.remove(pos);
            return true;
        }
        for obj in objects {
            if Self::remove_object_by_id_static(id, &mut obj.children) {
                return true;
            }
        }
        false
    }

    pub fn set_tool(&mut self, tool: TransformTool) {
        self.current_tool = tool;
    }

    pub fn set_camera_mode(&mut self, mode: CameraMode) {
        self.camera_mode = mode;
    }

    pub fn toggle_object_expanded(&mut self, object_id: &str) {
        if self.expanded_objects.contains(object_id) {
            self.expanded_objects.remove(object_id);
        } else {
            self.expanded_objects.insert(object_id.to_string());
        }
    }

    pub fn is_object_expanded(&self, object_id: &str) -> bool {
        self.expanded_objects.contains(object_id)
    }

    pub fn expand_all(&mut self, objects: &[SceneObject]) {
        for obj in objects {
            if !obj.children.is_empty() {
                self.expanded_objects.insert(obj.id.clone());
                self.expand_all(&obj.children);
            }
        }
    }

    pub fn collapse_all(&mut self) {
        self.expanded_objects.clear();
    }

    pub fn duplicate_selected_object(&mut self) {
        if let Some(id) = &self.selected_object.clone() {
            if let Some(obj) = self.find_object_by_id(id, &self.scene_objects) {
                let mut new_obj = obj.clone();
                new_obj.id = format!("{}_copy", obj.id);
                new_obj.name = format!("{} Copy", obj.name);
                self.scene_objects.push(new_obj);
                self.has_unsaved_changes = true;
            }
        }
    }

    pub fn toggle_grid(&mut self) {
        self.show_grid = !self.show_grid;
    }

    pub fn toggle_wireframe(&mut self) {
        self.show_wireframe = !self.show_wireframe;
    }

    pub fn toggle_lighting(&mut self) {
        self.show_lighting = !self.show_lighting;
    }

    pub fn toggle_performance_overlay(&mut self) {
        self.show_performance_overlay = !self.show_performance_overlay;
    }

    pub fn toggle_viewport_controls(&mut self) {
        self.show_viewport_controls = !self.show_viewport_controls;
    }

    pub fn toggle_camera_mode_selector(&mut self) {
        self.show_camera_mode_selector = !self.show_camera_mode_selector;
    }

    pub fn toggle_viewport_options(&mut self) {
        self.show_viewport_options = !self.show_viewport_options;
    }
}

impl Transform {
    pub fn default() -> Self {
        Self {
            position: [0.0, 0.0, 0.0],
            rotation: [0.0, 0.0, 0.0],
            scale: [1.0, 1.0, 1.0],
        }
    }
}
