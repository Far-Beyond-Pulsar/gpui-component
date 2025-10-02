use std::path::PathBuf;

/// Event emitted when a project is selected
#[derive(Clone, Debug)]
pub struct ProjectSelected {
    pub path: PathBuf,
}

/// Available tabs in the entry screen
#[derive(Clone, Debug, PartialEq)]
pub enum EntryTab {
    Manage,
    Create,
}

/// Represents a recent project card
#[derive(Clone, Debug)]
pub struct ProjectCard {
    pub name: String,
    pub path: PathBuf,
    pub description: String,
    pub image_path: Option<String>,
    pub last_modified: Option<String>,
}

/// Represents a project template card
#[derive(Clone, Debug)]
pub struct TemplateCard {
    pub name: String,
    pub description: String,
    pub image_path: Option<String>,
    pub git_url: String,
    pub tags: Vec<String>,
}

/// Unified card item type
#[derive(Clone, Debug)]
pub enum CardItem {
    Project(ProjectCard),
    Template(TemplateCard),
    BlankProject,
}

impl CardItem {
    pub fn name(&self) -> &str {
        match self {
            CardItem::Project(p) => &p.name,
            CardItem::Template(t) => &t.name,
            CardItem::BlankProject => "Blank Project",
        }
    }

    pub fn description(&self) -> &str {
        match self {
            CardItem::Project(p) => &p.description,
            CardItem::Template(t) => &t.description,
            CardItem::BlankProject => "Start from scratch with an empty Pulsar project",
        }
    }

    pub fn image_path(&self) -> Option<&str> {
        match self {
            CardItem::Project(p) => p.image_path.as_deref(),
            CardItem::Template(t) => t.image_path.as_deref(),
            CardItem::BlankProject => None,
        }
    }

    pub fn tags(&self) -> Vec<&str> {
        match self {
            CardItem::Template(t) => t.tags.iter().map(|s| s.as_str()).collect(),
            _ => Vec::new(),
        }
    }
}
