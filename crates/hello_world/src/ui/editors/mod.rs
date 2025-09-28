#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EditorType {
    Level,
    Script,
    Blueprint,
    Material,
    Animation,
    Particle,
    Sound,
    Terrain,
    NavMesh,
    Physics,
    Diagram,
    Prefab,
    Skeleton,
    Behavior,
    Foliage,
    UI,
}

impl EditorType {
    pub fn display_name(&self) -> &'static str {
        match self {
            EditorType::Level => "Level Editor",
            EditorType::Script => "Script Editor",
            EditorType::Blueprint => "Blueprint Editor",
            EditorType::Material => "Material Editor",
            EditorType::Animation => "Animation Editor",
            EditorType::Particle => "Particle System",
            EditorType::Sound => "Sound Editor",
            EditorType::Terrain => "Terrain Editor",
            EditorType::NavMesh => "Navigation Mesh",
            EditorType::Physics => "Physics Debug",
            EditorType::Diagram => "Diagram Editor",
            EditorType::Prefab => "Prefab Editor",
            EditorType::Skeleton => "Skeleton Editor",
            EditorType::Behavior => "Behavior Tree Editor",
            EditorType::Foliage => "Foliage Editor",
            EditorType::UI => "UI Editor",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            EditorType::Level => "Design and build game levels",
            EditorType::Script => "Write and edit game scripts",
            EditorType::Blueprint => "Visual scripting system",
            EditorType::Material => "Create and edit materials",
            EditorType::Animation => "Animate objects and characters",
            EditorType::Particle => "Design particle effects",
            EditorType::Sound => "Manage audio and sound effects",
            EditorType::Terrain => "Create and sculpt terrain",
            EditorType::NavMesh => "Configure AI navigation",
            EditorType::Physics => "Debug physics simulation",
            EditorType::Diagram => "Create flowcharts and diagrams",
            EditorType::Prefab => "Manage reusable game objects",
            EditorType::Skeleton => "Edit character skeletons",
            EditorType::Behavior => "Design AI behavior trees",
            EditorType::Foliage => "Paint vegetation and foliage",
            EditorType::UI => "Design user interfaces",
        }
    }
}