use gpui::{prelude::*, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex, StyledExt, Icon, IconName, ActiveTheme as _, TitleBar, Placement, ContextModal
};
use std::path::PathBuf;
use crate::recent_projects::{RecentProject, RecentProjectsList};

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryScreenView {
    Recent,
    Templates,
}

/// Template definition with Git repository info
#[derive(Clone)]
struct Template {
    name: String,
    description: String,
    icon: IconName,
    repo_url: String,
    image_url: Option<String>,
}

impl Template {
    fn new(name: &str, desc: &str, icon: IconName, repo_url: &str) -> Self {
        Self {
            name: name.to_string(),
            description: desc.to_string(),
            icon,
            repo_url: repo_url.to_string(),
            image_url: None,
        }
    }
    
    fn with_image(mut self, image_url: &str) -> Self {
        self.image_url = Some(image_url.to_string());
        self
    }
}

/// EntryScreen: Modern entry UI with sidebar navigation for recent projects and templates.
pub struct EntryScreen {
    view: EntryScreenView,
    recent_projects: RecentProjectsList,
    templates: Vec<Template>,
    recent_projects_path: PathBuf,
}

impl EntryScreen {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        // Load recent projects from disk
        let recent_projects_path = directories::ProjectDirs::from("com", "Pulsar", "Pulsar_Engine")
            .map(|proj| proj.data_dir().join("recent_projects.json"))
            .unwrap_or_else(|| PathBuf::from("recent_projects.json"));
        
        let recent_projects = RecentProjectsList::load(&recent_projects_path);
        
        // Define templates with their Git repositories
        let templates = vec![
            Template::new("Blank Project", "A new empty project with basic structure", IconName::Folder, "https://github.com/pulsar-templates/blank.git"),
            Template::new("2D Platformer", "Classic 2D side-scrolling platformer starter", IconName::Gamepad, "https://github.com/pulsar-templates/2d-platformer.git"),
            Template::new("3D First-Person", "First-person 3D game with basic controls", IconName::Cube, "https://github.com/pulsar-templates/3d-fps.git"),
            Template::new("Top-Down RPG", "Top-down RPG with inventory and dialogue", IconName::Map, "https://github.com/pulsar-templates/topdown-rpg.git"),
            Template::new("Visual Novel", "Visual novel with branching narratives", IconName::BookOpen, "https://github.com/pulsar-templates/visual-novel.git"),
            Template::new("Puzzle Game", "Grid-based puzzle game foundation", IconName::Box, "https://github.com/pulsar-templates/puzzle.git"),
            Template::new("Tower Defense", "Tower defense with enemy waves", IconName::Shield, "https://github.com/pulsar-templates/tower-defense.git"),
            Template::new("Card Game", "Card-based game with deck system", IconName::CreditCard, "https://github.com/pulsar-templates/card-game.git"),
            Template::new("Racing Game", "Racing game with physics", IconName::Rocket, "https://github.com/pulsar-templates/racing.git"),
        ];
        
        Self {
            view: EntryScreenView::Recent,
            recent_projects,
            templates,
            recent_projects_path,
        }
    }
    
    fn open_folder_dialog(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let file_dialog = rfd::AsyncFileDialog::new()
            .set_title("Select Pulsar Project Folder")
            .set_directory(std::env::current_dir().unwrap_or_default());
        
        let recent_projects_path = self.recent_projects_path.clone();
        
        cx.spawn(async move |this, mut cx| {
            if let Some(folder) = file_dialog.pick_folder().await {
                let path = folder.path().to_path_buf();
                
                // Validate that Pulsar.toml exists
                let toml_path = path.join("Pulsar.toml");
                if !toml_path.exists() {
                    eprintln!("Invalid project: Pulsar.toml not found in selected folder");
                    return;
                }
                
                // Add to recent projects
                let project_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string();
                
                let is_git = path.join(".git").exists();
                
                let recent_project = RecentProject {
                    name: project_name,
                    path: path.to_string_lossy().to_string(),
                    last_opened: Some(chrono::Local::now().format("%Y-%m-%d").to_string()),
                    is_git,
                };
                
                cx.update(|cx| {
                    this.update(cx, |screen, cx| {
                        screen.recent_projects.add_or_update(recent_project);
                        screen.recent_projects.save(&recent_projects_path);
                        cx.emit(crate::ui::project_selector::ProjectSelected { path });
                    }).ok();
                }).ok();
            }
        }).detach();
    }
    
    fn clone_template(&self, template: &Template, _window: &mut Window, cx: &mut Context<Self>) {
        let repo_url = template.repo_url.clone();
        let template_name = template.name.clone();
        let recent_projects_path = self.recent_projects_path.clone();
        
        // Ask user where to clone the template
        let file_dialog = rfd::AsyncFileDialog::new()
            .set_title(format!("Choose location for {}", template_name))
            .set_directory(std::env::current_dir().unwrap_or_default());
        
        cx.spawn(async move |this, mut cx| {
            if let Some(folder) = file_dialog.pick_folder().await {
                let parent_path = folder.path().to_path_buf();
                let project_name = template_name.replace(" ", "_").to_lowercase();
                let target_path = parent_path.join(&project_name);
                
                // Clone the repository
                eprintln!("Cloning template from {} to {:?}", repo_url, target_path);
                
                // Use git2 or std::process::Command to clone
                let clone_result = std::process::Command::new("git")
                    .args(["clone", &repo_url, target_path.to_str().unwrap()])
                    .output();
                
                match clone_result {
                    Ok(output) if output.status.success() => {
                        eprintln!("Successfully cloned template");
                        
                        // Add to recent projects
                        let recent_project = RecentProject {
                            name: project_name.clone(),
                            path: target_path.to_string_lossy().to_string(),
                            last_opened: Some(chrono::Local::now().format("%Y-%m-%d").to_string()),
                            is_git: true,
                        };
                        
                        cx.update(|cx| {
                            this.update(cx, |screen, cx| {
                                screen.recent_projects.add_or_update(recent_project);
                                screen.recent_projects.save(&recent_projects_path);
                                cx.emit(crate::ui::project_selector::ProjectSelected { path: target_path });
                            }).ok();
                        }).ok();
                    }
                    Ok(output) => {
                        eprintln!("Failed to clone template: {}", String::from_utf8_lossy(&output.stderr));
                    }
                    Err(e) => {
                        eprintln!("Failed to execute git: {}", e);
                    }
                }
            }
        }).detach();
    }
    
    fn open_project(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        // Update last opened time
        let project_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        
        let is_git = path.join(".git").exists();
        
        let recent_project = RecentProject {
            name: project_name,
            path: path.to_string_lossy().to_string(),
            last_opened: Some(chrono::Local::now().format("%Y-%m-%d").to_string()),
            is_git,
        };
        
        self.recent_projects.add_or_update(recent_project);
        self.recent_projects.save(&self.recent_projects_path);
        
        cx.emit(crate::ui::project_selector::ProjectSelected { path });
    }
    
    fn remove_recent_project(&mut self, path: String, cx: &mut Context<Self>) {
        self.recent_projects.remove(&path);
        self.recent_projects.save(&self.recent_projects_path);
        cx.notify();
    }
}

impl EventEmitter<crate::ui::project_selector::ProjectSelected> for EntryScreen {}

impl Render for EntryScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_recent_active = self.view == EntryScreenView::Recent;
        let is_templates_active = self.view == EntryScreenView::Templates;
        
        v_flex()
            .size_full()
            .bg(theme.background)
            // Title bar at the top
            .child(TitleBar::new())
            // Main content area
            .child(
                h_flex()
                    .size_full()
                    .child(
                        // Sidebar with icons and tooltips
                        v_flex()
                            .w(px(72.))
                            .h_full()
                            .bg(theme.sidebar)
                            .border_r_1()
                            .border_color(theme.border)
                            .gap_2()
                            .items_center()
                            .pt_8()
                            .child(
                                Button::new("recent-projects")
                                    .icon(IconName::FolderClosed)
                                    .label("")
                                    .tooltip("Recent Projects")
                                    .with_variant(if is_recent_active {
                                        gpui_component::button::ButtonVariant::Primary
                                    } else {
                                        gpui_component::button::ButtonVariant::Ghost
                                    })
                                    .on_click(cx.listener(|this: &mut Self, _, _, cx| {
                                        this.view = EntryScreenView::Recent;
                                        cx.notify();
                                    }))
                            )
                            .child(
                                Button::new("templates")
                                    .icon(IconName::Star)
                                    .label("")
                                    .tooltip("Templates")
                                    .with_variant(if is_templates_active {
                                        gpui_component::button::ButtonVariant::Primary
                                    } else {
                                        gpui_component::button::ButtonVariant::Ghost
                                    })
                                    .on_click(cx.listener(|this: &mut Self, _, _, cx| {
                                        this.view = EntryScreenView::Templates;
                                        cx.notify();
                                    }))
                            )
                            .child(
                                // Spacer
                                div().flex_1()
                            )
                            .child(
                                Button::new("new-project")
                                    .icon(IconName::Plus)
                                    .label("")
                                    .tooltip("Open Project Folder")
                                    .with_variant(gpui_component::button::ButtonVariant::Ghost)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        this.open_folder_dialog(window, cx);
                                    }))
                            )
                    )
                    .child(
                        // Main area
                        v_flex()
                            .flex_1()
                            .h_full()
                            .bg(theme.background)
                            .child(
                                match self.view {
                                    EntryScreenView::Recent => self.render_recent_projects(cx).into_any_element(),
                                    EntryScreenView::Templates => self.render_templates(cx).into_any_element(),
                                }
                            )
                    )
            )
    }
}

impl EntryScreen {
    fn render_recent_projects(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        
        v_flex()
            .size_full()
            .scrollable(gpui_component::scroll::ScrollbarAxis::Vertical)
            .p_12()
            .gap_6()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_2xl()
                            .font_bold()
                            .text_color(theme.foreground)
                            .child("Recent Projects")
                    )
                    .child(
                        Button::new("open-folder-btn")
                            .label("Open Folder")
                            .icon(IconName::FolderOpen)
                            .with_variant(gpui_component::button::ButtonVariant::Primary)
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.open_folder_dialog(window, cx);
                            }))
                    )
            )
            .child(gpui_component::divider::Divider::horizontal())
            .child({
                if self.recent_projects.projects.is_empty() {
                    v_flex()
                        .flex_1()
                        .items_center()
                        .justify_center()
                        .gap_4()
                        .child(
                            Icon::new(IconName::FolderOpen)
                                .size(px(64.))
                                .text_color(theme.muted_foreground)
                        )
                        .child(
                            div()
                                .text_lg()
                                .text_color(theme.muted_foreground)
                                .child("No recent projects")
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Open a project or create one from a template")
                        )
                        .into_any_element()
                } else {
                    self.render_project_cards(cx).into_any_element()
                }
            })
    }
    
    fn render_project_cards(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let projects = self.recent_projects.projects.clone();
        
        let mut container = v_flex().gap_6();
        let mut row = h_flex().gap_6();
        let mut count = 0;
        
        for project in projects.iter() {
            let proj_name = project.name.clone();
            let proj_path = project.path.clone();
            let proj_last_opened = project.last_opened.clone();
            let is_git = project.is_git;
            
            let card = v_flex()
                .w(px(320.))
                .h(px(240.))
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .bg(theme.sidebar)
                .shadow_lg()
                .overflow_hidden()
                .cursor_pointer()
                .hover(|style| style.border_color(theme.primary))
                .child(
                    // Card header with icon and title
                    h_flex()
                        .p_4()
                        .gap_3()
                        .items_center()
                        .border_b_1()
                        .border_color(theme.border)
                        .child(
                            Icon::new(if is_git { IconName::GitBranch } else { IconName::Folder })
                                .size(px(24.))
                                .text_color(if is_git { theme.primary } else { theme.foreground })
                        )
                        .child(
                            div()
                                .flex_1()
                                .font_semibold()
                                .text_color(theme.foreground)
                                .child(proj_name.clone())
                        )
                        .when(is_git, |this| {
                            this.child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(theme.primary)
                                    .text_xs()
                                    .text_color(theme.background)
                                    .child("Git")
                            )
                        })
                )
                .child(
                    // Card content
                    v_flex()
                        .flex_1()
                        .p_4()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(format!("Path: {}", proj_path))
                        )
                        .when(proj_last_opened.is_some(), |this| {
                            this.child(
                                div()
                                    .text_sm()
                                    .text_color(theme.muted_foreground)
                                    .child(format!("Last opened: {}", proj_last_opened.as_ref().unwrap()))
                            )
                        })
                )
                .child(
                    // Card actions
                    h_flex()
                        .p_4()
                        .gap_2()
                        .border_t_1()
                        .border_color(theme.border)
                        .child(
                            Button::new(SharedString::from(format!("open-{}", proj_path)))
                                .label("Open")
                                .icon(IconName::Play)
                                .with_variant(gpui_component::button::ButtonVariant::Primary)
                                .on_click(cx.listener({
                                    let path = PathBuf::from(proj_path.clone());
                                    move |this, _, _, cx| {
                                        this.open_project(path.clone(), cx);
                                    }
                                }))
                        )
                        .child(
                            Button::new(SharedString::from(format!("remove-{}", proj_path)))
                                .icon(IconName::Trash)
                                .tooltip("Remove from recent")
                                .with_variant(gpui_component::button::ButtonVariant::Ghost)
                                .on_click(cx.listener({
                                    let path = proj_path.clone();
                                    move |this, _, _, cx| {
                                        this.remove_recent_project(path.clone(), cx);
                                    }
                                }))
                        )
                );
            
            row = row.child(card);
            count += 1;
            
            if count == 3 {
                container = container.child(row);
                row = h_flex().gap_6();
                count = 0;
            }
        }
        
        // Add remaining items if any
        if count > 0 {
            container = container.child(row);
        }
        
        container
    }
    
    fn render_templates(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let templates = self.templates.clone();
        
        v_flex()
            .size_full()
            .scrollable(gpui_component::scroll::ScrollbarAxis::Vertical)
            .p_12()
            .gap_6()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_2xl()
                            .font_bold()
                            .text_color(theme.foreground)
                            .child("Project Templates")
                    )
            )
            .child(gpui_component::divider::Divider::horizontal())
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .mb_4()
                    .child("Choose a template to start your project. Each template will be cloned from its Git repository.")
            )
            .child({
                let mut container = v_flex().gap_6();
                let mut row = h_flex().gap_6();
                let mut count = 0;
                
                for template in templates.iter() {
                    let template_clone = template.clone();
                    let template_icon = template.icon.clone();
                    let template_name = template.name.clone();
                    let template_desc = template.description.clone();
                    
                    let card = v_flex()
                        .w(px(320.))
                        .h(px(280.))
                        .border_1()
                        .border_color(theme.border)
                        .rounded_lg()
                        .bg(theme.sidebar)
                        .shadow_lg()
                        .overflow_hidden()
                        .cursor_pointer()
                        .hover(|style| style.border_color(theme.primary))
                        .child(
                            // Optional image placeholder
                            div()
                                .h(px(120.))
                                .w_full()
                                .bg(theme.background)
                                .border_b_1()
                                .border_color(theme.border)
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(
                                    Icon::new(template_icon)
                                        .size(px(48.))
                                        .text_color(theme.primary)
                                )
                        )
                        .child(
                            // Card header
                            h_flex()
                                .p_4()
                                .gap_3()
                                .items_center()
                                .child(
                                    div()
                                        .flex_1()
                                        .font_semibold()
                                        .text_color(theme.foreground)
                                        .child(template_name.clone())
                                )
                        )
                        .child(
                            // Card description
                            div()
                                .flex_1()
                                .px_4()
                                .pb_4()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(template_desc)
                        )
                        .child(
                            // Card action
                            h_flex()
                                .p_4()
                                .border_t_1()
                                .border_color(theme.border)
                                .child(
                                    Button::new(SharedString::from(format!("create-{}", template_name)))
                                        .label("Create Project")
                                        .icon(IconName::Plus)
                                        .with_variant(gpui_component::button::ButtonVariant::Primary)
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.clone_template(&template_clone, window, cx);
                                        }))
                                )
                        );
                    
                    row = row.child(card);
                    count += 1;
                    
                    if count == 3 {
                        container = container.child(row);
                        row = h_flex().gap_6();
                        count = 0;
                    }
                }
                
                // Add remaining items if any
                if count > 0 {
                    container = container.child(row);
                }
                
                container
            })
    }
}
