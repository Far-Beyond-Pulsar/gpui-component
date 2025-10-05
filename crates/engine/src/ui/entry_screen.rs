use gpui::{prelude::*, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex, StyledExt, Icon, IconName, ActiveTheme as _, TitleBar,
    progress::Progress, input::TextInput,
};
use std::path::PathBuf;
use crate::recent_projects::{RecentProject, RecentProjectsList};
use std::sync::Arc;
use parking_lot::Mutex;
use crate::OpenSettings;

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryScreenView {
    Recent,
    Templates,
    NewProject,
    CloneGit,
}

/// Template definition with Git repository info
#[derive(Clone)]
struct Template {
    name: String,
    description: String,
    icon: IconName,
    repo_url: String,
    category: String,
}

impl Template {
    fn new(name: &str, desc: &str, icon: IconName, repo_url: &str, category: &str) -> Self {
        Self {
            name: name.to_string(),
            description: desc.to_string(),
            icon,
            repo_url: repo_url.to_string(),
            category: category.to_string(),
        }
    }
}

#[derive(Clone)]
struct CloneProgress {
    current: usize,
    total: usize,
    message: String,
    completed: bool,
    error: Option<String>,
}

/// EntryScreen: AAA-quality project manager
pub struct EntryScreen {
    view: EntryScreenView,
    recent_projects: RecentProjectsList,
    templates: Vec<Template>,
    recent_projects_path: PathBuf,
    clone_progress: Option<Arc<Mutex<CloneProgress>>>,
    new_project_name: String,
    new_project_path: Option<PathBuf>,
    git_repo_url: String,
    search_query: String,
    launched: bool,
}

impl EntryScreen {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        let recent_projects_path = directories::ProjectDirs::from("com", "Pulsar", "Pulsar_Engine")
            .map(|proj| proj.data_dir().join("recent_projects.json"))
            .unwrap_or_else(|| PathBuf::from("recent_projects.json"));
        
        let recent_projects = RecentProjectsList::load(&recent_projects_path);
        
        let templates = vec![
            Template::new("Blank Project", "Empty project with minimal structure", IconName::Folder, "https://github.com/pulsar-templates/blank.git", "Basic"),
            Template::new("Core", "Core engine features and systems", IconName::Settings, "https://github.com/pulsar-templates/core.git", "Basic"),
            Template::new("2D Platformer", "Classic side-scrolling platformer", IconName::Gamepad, "https://github.com/pulsar-templates/2d-platformer.git", "2D"),
            Template::new("2D Top-Down", "Top-down 2D game with camera", IconName::Map, "https://github.com/pulsar-templates/2d-topdown.git", "2D"),
            Template::new("3D First Person", "FPS with movement and camera", IconName::Eye, "https://github.com/pulsar-templates/3d-fps.git", "3D"),
            Template::new("3D Platformer", "3D platformer with physics", IconName::Cube, "https://github.com/pulsar-templates/3d-platformer.git", "3D"),
            Template::new("Tower Defense", "Wave-based tower defense", IconName::Shield, "https://github.com/pulsar-templates/tower-defense.git", "Strategy"),
            Template::new("Action RPG", "Action-oriented RPG systems", IconName::Star, "https://github.com/pulsar-templates/action-rpg.git", "RPG"),
            Template::new("Visual Novel", "Narrative-driven visual novel", IconName::BookOpen, "https://github.com/pulsar-templates/visual-novel.git", "Narrative"),
            Template::new("Puzzle", "Puzzle game mechanics", IconName::Box, "https://github.com/pulsar-templates/puzzle.git", "Puzzle"),
            Template::new("Card Game", "Card-based game system", IconName::CreditCard, "https://github.com/pulsar-templates/card-game.git", "Card"),
            Template::new("Racing", "Racing game with physics", IconName::Rocket, "https://github.com/pulsar-templates/racing.git", "Racing"),
        ];
        
        Self {
            view: EntryScreenView::Recent,
            recent_projects,
            templates,
            recent_projects_path,
            clone_progress: None,
            new_project_name: String::new(),
            new_project_path: None,
            git_repo_url: String::new(),
            search_query: String::new(),
            launched: false,
        }
    }
    
    fn calculate_columns(&self, width: Pixels) -> usize {
        // Account for left and right padding (12px each) on the card list container
        let available_width = width - px(24.); // 12px left + 12px right padding
        // Minimum card width (320px) + gap (6px) = 326px per column
        // Calculate how many columns fit in the available width
        let card_width_with_gap = px(326.); // 320px card + 6px gap
        let columns = (available_width / card_width_with_gap).floor() as usize;
        // Ensure at least 1 column, maximum reasonable number
        columns.max(1).min(6)
    }
    
    fn open_folder_dialog(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let file_dialog = rfd::AsyncFileDialog::new()
            .set_title("Select Pulsar Project Folder")
            .set_directory(std::env::current_dir().unwrap_or_default());
        
        let recent_projects_path = self.recent_projects_path.clone();
        
        cx.spawn(async move |this, mut cx| {
            if let Some(folder) = file_dialog.pick_folder().await {
                let path = folder.path().to_path_buf();
                let toml_path = path.join("Pulsar.toml");
                
                if !toml_path.exists() {
                    eprintln!("Invalid project: Pulsar.toml not found");
                    return;
                }
                
                let project_name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown")
                    .to_string();
                
                let is_git = path.join(".git").exists();
                
                let recent_project = RecentProject {
                    name: project_name,
                    path: path.to_string_lossy().to_string(),
                    last_opened: Some(chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()),
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
    
    fn clone_git_repo(&mut self, repo_url: String, target_name: String, _window: &mut Window, cx: &mut Context<Self>) {
        let progress = Arc::new(Mutex::new(CloneProgress {
            current: 0,
            total: 100,
            message: "Initializing...".to_string(),
            completed: false,
            error: None,
        }));
        
        self.clone_progress = Some(progress.clone());
        let recent_projects_path = self.recent_projects_path.clone();
        
        cx.spawn(async move |this, mut cx| {
            let file_dialog = rfd::AsyncFileDialog::new()
                .set_title(format!("Choose location for {}", target_name))
                .set_directory(std::env::current_dir().unwrap_or_default());
            
            if let Some(folder) = file_dialog.pick_folder().await {
                let parent_path = folder.path().to_path_buf();
                let project_name = target_name.replace(" ", "_").to_lowercase();
                let target_path = parent_path.join(&project_name);
                let target_path_str = target_path.to_string_lossy().to_string();
                
                {
                    let mut prog = progress.lock();
                    prog.message = "Cloning repository...".to_string();
                    prog.current = 10;
                }
                
                cx.update(|cx| {
                    this.update(cx, |_, cx| cx.notify()).ok();
                }).ok();
                
                let repo_url_clone = repo_url.clone();
                let progress_clone = progress.clone();
                let target_path_clone = target_path.clone();
                
                let repo_result = std::thread::spawn(move || {
                    let mut callbacks = git2::RemoteCallbacks::new();
                    let progress_inner = progress_clone.clone();
                    
                    callbacks.transfer_progress(move |stats| {
                        let mut prog = progress_inner.lock();
                        prog.current = stats.received_objects();
                        prog.total = stats.total_objects();
                        prog.message = format!(
                            "Receiving objects: {}/{} ({:.1}%)",
                            stats.received_objects(),
                            stats.total_objects(),
                            (stats.received_objects() as f32 / stats.total_objects() as f32) * 100.0
                        );
                        true
                    });
                    
                    let mut fetch_options = git2::FetchOptions::new();
                    fetch_options.remote_callbacks(callbacks);
                    
                    let mut builder = git2::build::RepoBuilder::new();
                    builder.fetch_options(fetch_options);
                    
                    builder.clone(&repo_url_clone, &target_path_clone)
                }).join();
                
                match repo_result {
                    Ok(Ok(_repo)) => {
                        {
                            let mut prog = progress.lock();
                            prog.completed = true;
                            prog.current = prog.total;
                            prog.message = "Clone completed!".to_string();
                        }
                        
                        let recent_project = RecentProject {
                            name: project_name.clone(),
                            path: target_path_str,
                            last_opened: Some(chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()),
                            is_git: true,
                        };
                        
                        cx.update(|cx| {
                            this.update(cx, |screen, cx| {
                                screen.recent_projects.add_or_update(recent_project);
                                screen.recent_projects.save(&recent_projects_path);
                                screen.clone_progress = None;
                                cx.emit(crate::ui::project_selector::ProjectSelected { path: target_path });
                            }).ok();
                        }).ok();
                    }
                    Ok(Err(e)) => {
                        let mut prog = progress.lock();
                        prog.error = Some(format!("Clone failed: {}", e));
                        prog.message = "Error occurred".to_string();
                    }
                    Err(_) => {
                        let mut prog = progress.lock();
                        prog.error = Some("Thread panic during clone".to_string());
                    }
                }
                
                cx.update(|cx| {
                    this.update(cx, |_, cx| cx.notify()).ok();
                }).ok();
            } else {
                cx.update(|cx| {
                    this.update(cx, |screen, cx| {
                        screen.clone_progress = None;
                        cx.notify();
                    }).ok();
                }).ok();
            }
        }).detach();
    }
    
    fn clone_template(&mut self, template: &Template, window: &mut Window, cx: &mut Context<Self>) {
        self.clone_git_repo(template.repo_url.clone(), template.name.clone(), window, cx);
    }
    
    fn launch_project(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.launched {
            return;
        }
        self.launched = true;
        
        eprintln!("DEBUG: launch_project called with path: {:?}", path);
        
        let project_name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("Unknown")
            .to_string();
        
        let is_git = path.join(".git").exists();
        
        let recent_project = RecentProject {
            name: project_name,
            path: path.to_string_lossy().to_string(),
            last_opened: Some(chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()),
            is_git,
        };
        
        self.recent_projects.add_or_update(recent_project);
        self.recent_projects.save(&self.recent_projects_path);
        
        eprintln!("DEBUG: Emitting ProjectSelected event");
        cx.emit(crate::ui::project_selector::ProjectSelected { path });
        eprintln!("DEBUG: ProjectSelected event emitted");
    }
    
    fn remove_recent_project(&mut self, path: String, cx: &mut Context<Self>) {
        self.recent_projects.remove(&path);
        self.recent_projects.save(&self.recent_projects_path);
        cx.notify();
    }
    
    fn browse_project_location(&mut self, cx: &mut Context<Self>) {
        let file_dialog = rfd::AsyncFileDialog::new()
            .set_title("Choose Project Location")
            .set_directory(std::env::current_dir().unwrap_or_default());
        
        cx.spawn(async move |this, mut cx| {
            if let Some(folder) = file_dialog.pick_folder().await {
                cx.update(|cx| {
                    this.update(cx, |screen, cx| {
                        screen.new_project_path = Some(folder.path().to_path_buf());
                        cx.notify();
                    }).ok();
                }).ok();
            }
        }).detach();
    }
    
    fn create_new_project(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        if self.new_project_name.is_empty() {
            return;
        }
        
        let name = self.new_project_name.clone();
        let base_path = self.new_project_path.clone()
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        
        let project_path = base_path.join(&name);
        let recent_projects_path = self.recent_projects_path.clone();
        
        cx.spawn(async move |this, mut cx| {
            if let Err(e) = std::fs::create_dir_all(&project_path) {
                eprintln!("Failed to create project directory: {}", e);
                return;
            }
            
            let toml_content = format!(
                r#"[project]
name = "{}"
version = "0.1.0"
engine_version = "0.1.23"

[settings]
default_scene = "scenes/main.scene"
"#,
                name
            );
            
            if let Err(e) = std::fs::write(project_path.join("Pulsar.toml"), toml_content) {
                eprintln!("Failed to create Pulsar.toml: {}", e);
                return;
            }
            
            let dirs = ["assets", "scenes", "scripts", "prefabs"];
            for dir in dirs {
                let _ = std::fs::create_dir_all(project_path.join(dir));
            }
            
            let _ = std::process::Command::new("git")
                .args(["init"])
                .current_dir(&project_path)
                .output();
            
            let recent_project = RecentProject {
                name: name.clone(),
                path: project_path.to_string_lossy().to_string(),
                last_opened: Some(chrono::Local::now().format("%Y-%m-%d %H:%M").to_string()),
                is_git: project_path.join(".git").exists(),
            };
            
            cx.update(|cx| {
                this.update(cx, |screen, cx| {
                    screen.recent_projects.add_or_update(recent_project);
                    screen.recent_projects.save(&recent_projects_path);
                    screen.new_project_name.clear();
                    screen.view = EntryScreenView::Recent;
                    cx.emit(crate::ui::project_selector::ProjectSelected { path: project_path });
                }).ok();
            }).ok();
        }).detach();
    }
}

impl EventEmitter<crate::ui::project_selector::ProjectSelected> for EntryScreen {}

impl Render for EntryScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let bounds = window.viewport_size();
        let cols = self.calculate_columns(bounds.width);
        let view = self.view;
        
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(TitleBar::new())
            .child(
                h_flex()
                    .size_full()
                    .child(self.render_sidebar(cx))
                    .child(
                        v_flex()
                            .flex_1()
                            .h_full()
                            .bg(cx.theme().background)
                            .child(
                                match view {
                                    EntryScreenView::Recent => self.render_recent_projects(cols, cx).into_any_element(),
                                    EntryScreenView::Templates => self.render_templates(cols, cx).into_any_element(),
                                    EntryScreenView::NewProject => self.render_new_project(cx).into_any_element(),
                                    EntryScreenView::CloneGit => self.render_clone_git(cx).into_any_element(),
                                }
                            )
                    )
            )
    }
}

impl EntryScreen {
    fn render_sidebar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        
        v_flex()
            .w(px(72.))
            .h_full()
            .bg(theme.sidebar)
            .border_r_1()
            .border_color(theme.border)
            .gap_2()
            .items_center()
            .pt_8()
            .pb_4()
            .child(
                Button::new("recent-projects")
                    .icon(IconName::FolderClosed)
                    .label("")
                    .tooltip("Recent Projects")
                    .with_variant(if self.view == EntryScreenView::Recent {
                        gpui_component::button::ButtonVariant::Primary
                    } else {
                        gpui_component::button::ButtonVariant::Ghost
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.view = EntryScreenView::Recent;
                        cx.notify();
                    }))
            )
            .child(
                Button::new("templates")
                    .icon(IconName::Star)
                    .label("")
                    .tooltip("Project Templates")
                    .with_variant(if self.view == EntryScreenView::Templates {
                        gpui_component::button::ButtonVariant::Primary
                    } else {
                        gpui_component::button::ButtonVariant::Ghost
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.view = EntryScreenView::Templates;
                        cx.notify();
                    }))
            )
            .child(
                Button::new("new-project")
                    .icon(IconName::Plus)
                    .label("")
                    .tooltip("Create New Project")
                    .with_variant(if self.view == EntryScreenView::NewProject {
                        gpui_component::button::ButtonVariant::Primary
                    } else {
                        gpui_component::button::ButtonVariant::Ghost
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.view = EntryScreenView::NewProject;
                        cx.notify();
                    }))
            )
            .child(
                Button::new("clone-git")
                    .icon(IconName::GitHub)
                    .label("")
                    .tooltip("Clone from Git")
                    .with_variant(if self.view == EntryScreenView::CloneGit {
                        gpui_component::button::ButtonVariant::Primary
                    } else {
                        gpui_component::button::ButtonVariant::Ghost
                    })
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.view = EntryScreenView::CloneGit;
                        cx.notify();
                    }))
            )
            .child(div().flex_1())
            .child(
                Button::new("open-existing")
                    .icon(IconName::FolderOpen)
                    .label("")
                    .tooltip("Open Existing Project")
                    .with_variant(gpui_component::button::ButtonVariant::Ghost)
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.open_folder_dialog(window, cx);
                    }))
            )
            .child(
                Button::new("settings")
                    .icon(IconName::Settings)
                    .label("")
                    .tooltip("Settings")
                    .with_variant(gpui_component::button::ButtonVariant::Ghost)
                    .on_click(cx.listener(|_, _, window, cx| {
                        window.dispatch_action(Box::new(OpenSettings), cx);
                    }))
            )
    }
    
    fn render_recent_projects(&mut self, cols: usize, cx: &mut Context<Self>) -> impl IntoElement {
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
                        h_flex()
                            .gap_2()
                            .child(
                                Button::new("refresh-btn")
                                    .label("Refresh")
                                    .icon(IconName::ArrowUp)
                                    .with_variant(gpui_component::button::ButtonVariant::Secondary)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        let path = this.recent_projects_path.clone();
                                        this.recent_projects = RecentProjectsList::load(&path);
                                        cx.notify();
                                    }))
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
                                .child("Create a new project or open an existing one to get started")
                        )
                        .into_any_element()
                } else {
                    self.render_project_grid(cols, cx).into_any_element()
                }
            })
    }
    
    fn render_project_grid(&mut self, cols: usize, cx: &mut Context<Self>) -> impl IntoElement {
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
                .flex_1()
                .min_w(px(280.))
                .h(px(240.))
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .bg(theme.sidebar)
                .shadow_lg()
                .overflow_hidden()
                .hover(|style| style.border_color(theme.primary).shadow_xl())
                .child(
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
                                .overflow_hidden()
                                .child(proj_name)
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
                    v_flex()
                        .flex_1()
                        .p_4()
                        .gap_2()
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("Path")
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.foreground)
                                .overflow_hidden()
                                .child(proj_path.clone())
                        )
                        .when(proj_last_opened.is_some(), |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .mt_2()
                                    .child(format!("Last opened: {}", proj_last_opened.as_ref().unwrap()))
                            )
                        })
                )
                .child(
                    h_flex()
                        .p_4()
                        .gap_2()
                        .border_t_1()
                        .border_color(theme.border)
                        .child(
                            Button::new(SharedString::from(format!("open-{}", proj_path)))
                                .label("Launch")
                                .icon(IconName::Play)
                                .with_variant(gpui_component::button::ButtonVariant::Primary)
                                .on_click(cx.listener({
                                    let path = PathBuf::from(proj_path.clone());
                                    move |this, _, _, cx| {
                                        this.launch_project(path.clone(), cx);
                                    }
                                }))
                        )
                        .child(
                            Button::new(SharedString::from(format!("location-{}", proj_path)))
                                .icon(IconName::FolderOpen)
                                .tooltip("Open in file manager")
                                .with_variant(gpui_component::button::ButtonVariant::Ghost)
                                .on_click({
                                    let path = proj_path.clone();
                                    move |_, _, _| {
                                        let _ = open::that(&path);
                                    }
                                })
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
            
            if count >= cols {
                container = container.child(row);
                row = h_flex().gap_6();
                count = 0;
            }
        }
        
        if count > 0 {
            container = container.child(row);
        }
        
        container
    }
    
    fn render_templates(&mut self, cols: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let templates = self.templates.clone();
        let has_progress = self.clone_progress.is_some();
        
        v_flex()
            .size_full()
            .scrollable(gpui_component::scroll::ScrollbarAxis::Vertical)
            .p_12()
            .gap_6()
            .child(
                div()
                    .text_2xl()
                    .font_bold()
                    .text_color(theme.foreground)
                    .child("Project Templates")
            )
            .child(gpui_component::divider::Divider::horizontal())
            .child(
                div()
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .mb_4()
                    .child("Choose a template to start your project. Templates are cloned from Git with full progress tracking.")
            )
            .children(if has_progress {
                Some(
                    v_flex()
                        .gap_4()
                        .p_6()
                        .border_1()
                        .border_color(theme.primary)
                        .rounded_lg()
                        .bg(theme.sidebar)
                        .child(
                            div()
                                .font_semibold()
                                .text_color(theme.foreground)
                                .child("Cloning Repository...")
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Please wait...")
                        )
                        .child(Progress::new().value(50.0))
                )
            } else {
                None
            })
            .child(self.render_template_grid(templates, cols, cx))
    }
    
    fn render_template_grid(&mut self, templates: Vec<Template>, cols: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        
        let mut container = v_flex().gap_6();
        let mut row = h_flex().gap_6();
        let mut count = 0;
        
        for template in templates.iter() {
            let template_clone = template.clone();
            let template_icon = template.icon.clone();
            let template_name = template.name.clone();
            let template_desc = template.description.clone();
            let template_category = template.category.clone();
            
            let card = v_flex()
                .flex_1()
                .min_w(px(280.))
                .h(px(300.))
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .bg(theme.sidebar)
                .shadow_lg()
                .overflow_hidden()
                .hover(|style| style.border_color(theme.primary).shadow_xl())
                .child(
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
                    v_flex()
                        .p_4()
                        .gap_2()
                        .child(
                            h_flex()
                                .justify_between()
                                .items_center()
                                .child(
                                    div()
                                        .font_semibold()
                                        .text_color(theme.foreground)
                                        .child(template_name.clone())
                                )
                                .child(
                                    div()
                                        .px_2()
                                        .py_1()
                                        .rounded_md()
                                        .bg(theme.muted)
                                        .text_xs()
                                        .text_color(theme.foreground)
                                        .child(template_category)
                                )
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(template_desc)
                        )
                )
                .child(div().flex_1())
                .child(
                    h_flex()
                        .p_4()
                        .border_t_1()
                        .border_color(theme.border)
                        .child(
                            Button::new(SharedString::from(format!("create-{}", template_name)))
                                .label("Use Template")
                                .icon(IconName::Plus)
                                .with_variant(gpui_component::button::ButtonVariant::Primary)
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.clone_template(&template_clone, window, cx);
                                }))
                        )
                );
            
            row = row.child(card);
            count += 1;
            
            if count >= cols {
                container = container.child(row);
                row = h_flex().gap_6();
                count = 0;
            }
        }
        
        if count > 0 {
            container = container.child(row);
        }
        
        container
    }
    
    fn render_new_project(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let project_name_owned = self.new_project_name.clone();
        let project_name_empty = project_name_owned.is_empty();
        let project_name_display: String = if project_name_empty {
            "Enter project name...".to_string()
        } else {
            project_name_owned.clone()
        };
        let project_path_display = self.new_project_path.as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or("Click Browse to select location...")
            .to_string();

        // Clone/copy data needed in UI so no reference to self escapes
        let new_project_name = self.new_project_name.clone();
        let project_name_display_owned = project_name_display.clone();
        let project_path_display_owned = project_path_display.clone();

        v_flex()
            .size_full()
            .p_12()
            .gap_6()
            .child(
                div()
                    .text_2xl()
                    .font_bold()
                    .text_color(theme.foreground)
                    .child("Create New Project")
            )
            .child(gpui_component::divider::Divider::horizontal())
            .child(
                v_flex()
                    .max_w(px(600.))
                    .gap_6()
                    .p_6()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .bg(theme.sidebar)
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .font_semibold()
                                    .text_color(theme.foreground)
                                    .child("Project Name")
                            )
                            .child(
                                div()
                                    .px_3()
                                    .py_2()
                                    .border_1()
                                    .border_color(theme.border)
                                    .rounded_md()
                                    .bg(theme.background)
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .child(project_name_display_owned)
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("Note: Use the text input component when available")
                            )
                    )
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .font_semibold()
                                    .text_color(theme.foreground)
                                    .child("Project Location")
                            )
                            .child(
                                h_flex()
                                    .gap_2()
                                    .child(
                                        div()
                                            .flex_1()
                                            .px_3()
                                            .py_2()
                                            .border_1()
                                            .border_color(theme.border)
                                            .rounded_md()
                                            .bg(theme.background)
                                            .text_sm()
                                            .text_color(theme.muted_foreground)
                                            .child(project_path_display_owned)
                                    )
                                    .child(
                                        Button::new("browse-location")
                                            .label("Browse...")
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.browse_project_location(cx);
                                            }))
                                    )
                            )
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("A new folder will be created with your project name in the selected location.")
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .justify_end()
                            .mt_4()
                            .child(
                                Button::new("cancel-new-project")
                                    .label("Cancel")
                                    .with_variant(gpui_component::button::ButtonVariant::Secondary)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.view = EntryScreenView::Recent;
                                        this.new_project_name.clear();
                                        this.new_project_path = None;
                                        cx.notify();
                                    }))
                            )
                            .child(
                                Button::new("create-new-project")
                                    .label("Create Project")
                                    .icon(IconName::Plus)
                                    .with_variant(gpui_component::button::ButtonVariant::Primary)
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        if !new_project_name.is_empty() {
                                            this.create_new_project(window, cx);
                                        }
                                    }))
                            )
                    )
            )
    }
    
    fn render_clone_git(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let git_url_display = if self.git_repo_url.is_empty() {
            "Enter Git repository URL..."
        } else {
            self.git_repo_url.as_str()
        };
        let progress_message = self.clone_progress.as_ref()
            .map(|p| {
                let prog = p.lock();
                (prog.message.clone(), prog.current, prog.total, prog.error.clone())
            });
        
        v_flex()
            .size_full()
            .p_12()
            .gap_6()
            .child(
                div()
                    .text_2xl()
                    .font_bold()
                    .text_color(theme.foreground)
                    .child("Clone from Git Repository")
            )
            .child(gpui_component::divider::Divider::horizontal())
            .child(
                v_flex()
                    .max_w(px(600.))
                    .gap_6()
                    .p_6()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .bg(theme.sidebar)
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                div()
                                    .font_semibold()
                                    .text_color(theme.foreground)
                                    .child("Repository URL")
                            )
                            .child(
                                {
                                    let git_url_display_owned = git_url_display.to_string();
                                    div()
                                        .px_3()
                                        .py_2()
                                        .border_1()
                                        .border_color(theme.border)
                                        .rounded_md()
                                        .bg(theme.background)
                                        .text_sm()
                                        .text_color(theme.foreground)
                                        .child(git_url_display_owned)
                                }
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(theme.muted_foreground)
                                    .child("Enter the Git repository URL (HTTPS or SSH)")
                            )
                    )
                    .children(if let Some((message, current, total, error)) = progress_message {
                        Some(
                            v_flex()
                                .gap_3()
                                .p_4()
                                .border_1()
                                .border_color(theme.primary)
                                .rounded_md()
                                .bg(theme.background)
                                .child(
                                    div()
                                        .font_semibold()
                                        .text_color(theme.foreground)
                                        .child("Cloning Repository...")
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child(message)
                                )
                                .child(
                                    Progress::new()
                                        .value(if total > 0 {
                                            (current as f32 / total as f32) * 100.0
                                        } else {
                                            0.0
                                        })
                                )
                                .children(error.map(|e| {
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child(e)
                                }))
                        )
                    } else {
                        None
                    })
                    .child(
                        h_flex()
                            .gap_2()
                            .justify_end()
                            .mt_4()
                            .child(
                                Button::new("cancel-clone")
                                    .label("Cancel")
                                    .with_variant(gpui_component::button::ButtonVariant::Secondary)
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.view = EntryScreenView::Recent;
                                        this.git_repo_url.clear();
                                        cx.notify();
                                    }))
                            )
                            .child(
                                Button::new("clone-repo")
                                    .label("Clone Repository")
                                    .icon(IconName::GitHub)
                                    .with_variant(gpui_component::button::ButtonVariant::Primary)
                                    .on_click(cx.listener(|this, _, window, cx| {
                                        if !this.git_repo_url.is_empty() && this.clone_progress.is_none() {
                                            let url = this.git_repo_url.clone();
                                            let name = url.split('/').last()
                                                .unwrap_or("repository")
                                                .replace(".git", "");
                                            this.clone_git_repo(url, name, window, cx);
                                        }
                                    }))
                            )
                    )
            )
    }
}
