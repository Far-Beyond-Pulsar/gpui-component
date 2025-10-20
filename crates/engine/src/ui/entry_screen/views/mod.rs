mod sidebar;
mod recent_projects;
mod templates;
mod new_project;
mod clone_git;
mod upstream_prompt;

pub use sidebar::render_sidebar;
pub use recent_projects::render_recent_projects;
pub use templates::render_templates;
pub use new_project::render_new_project;
pub use clone_git::render_clone_git;
pub use upstream_prompt::render_upstream_prompt;
