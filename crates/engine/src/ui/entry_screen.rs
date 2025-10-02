use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    tab::{Tab, TabBar},
    h_flex, v_flex, ActiveTheme as _, Icon, IconName, StyledExt,
    input::{TextInput, InputState},
};
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use schemars::JsonSchema;
use std::fs;
use std::io::Write;

#[derive(Clone, Debug)]
pub struct ProjectSelected {
    pub path: PathBuf,
}

#[derive(Clone, Debug, PartialEq)]
enum EntryTab {
    Manage,
    Create,
    Git,
}

#[derive(Clone, Debug)]
struct ProjectCard {
    name: String,
    path: PathBuf,
    description: String,
    image_path: Option<String>,
    last_modified: Option<String>,
}

#[derive(Clone, Debug)]
struct TemplateCard {
    name: String,
    description: String,
    image_path: Option<String>,
    git_url: String,
    tags: Vec<String>,
}

#[derive(Clone, Debug)]
enum CardItem {
    Project(ProjectCard),
    Template(TemplateCard),
    BlankProject,
}

impl CardItem {
    fn name(&self) -> &str {
        match self {
            CardItem::Project(p) => &p.name,
            CardItem::Template(t) => &t.name,
            CardItem::BlankProject => "Blank Project",
        }
    }

    fn description(&self) -> &str {
        match self {
            CardItem::Project(p) => &p.description,
            CardItem::Template(t) => &t.description,
            CardItem::BlankProject => "Start from scratch with an empty Pulsar project",
        }
    }

    fn image_path(&self) -> Option<&str> {
        match self {
            CardItem::Project(p) => p.image_path.as_deref(),
            CardItem::Template(t) => t.image_path.as_deref(),
            CardItem::BlankProject => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct RecentProjectsConfig {
    recent_projects: Vec<String>,
}

pub struct EntryScreen {
    active_tab: EntryTab,
    recent_projects: Vec<PathBuf>,
    selected_card: Option<usize>,
    project_name_input: Entity<InputState>,
    project_path_input: Entity<InputState>,
    pending_path_update: Option<String>,
}

impl EntryScreen {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            active_tab: EntryTab::Manage,
            recent_projects: Self::load_recent_projects(),
            selected_card: None,
            project_name_input: cx.new(|cx| InputState::new(window, cx)),
            project_path_input: cx.new(|cx| InputState::new(window, cx)),
            pending_path_update: None,
        }
    }

    fn get_cards(&self) -> Vec<CardItem> {
        match self.active_tab {
            EntryTab::Manage => {
                self.recent_projects.iter().map(|path| {
                    let name = path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown Project")
                        .to_string();

                    CardItem::Project(ProjectCard {
                        name,
                        path: path.clone(),
                        description: format!("Pulsar project at {}", path.display()),
                        image_path: None,
                        last_modified: None,
                    })
                }).collect()
            }
            EntryTab::Create => {
                vec![
                    CardItem::BlankProject,
                    CardItem::Template(TemplateCard {
                        name: "2D Platformer".to_string(),
                        description: "A complete 2D platformer template with character controller, tilemaps, and basic enemies".to_string(),
                        image_path: None,
                        git_url: "https://github.com/pulsar-engine/template-2d".to_string(),
                        tags: vec!["2D".to_string(), "Platformer".to_string()],
                    }),
                    CardItem::Template(TemplateCard {
                        name: "3D First-Person".to_string(),
                        description: "A 3D first-person template with camera controls, physics, and basic interactions".to_string(),
                        image_path: None,
                        git_url: "https://github.com/pulsar-engine/template-3d".to_string(),
                        tags: vec!["3D".to_string(), "First-Person".to_string()],
                    }),
                ]
            }
            EntryTab::Git => {
                vec![]
            }
        }
    }

    fn get_config_path() -> Option<PathBuf> {
        if cfg!(windows) {
            std::env::var("APPDATA").ok().map(|appdata| {
                PathBuf::from(appdata).join("Pulsar").join("recent_projects.json")
            })
        } else {
            std::env::var("HOME").ok().map(|home| {
                PathBuf::from(home).join(".config").join("pulsar").join("recent_projects.json")
            })
        }
    }

    fn load_recent_projects() -> Vec<PathBuf> {
        if let Some(config_path) = Self::get_config_path() {
            if let Ok(contents) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<RecentProjectsConfig>(&contents) {
                    return config.recent_projects.into_iter()
                        .map(PathBuf::from)
                        .filter(|p| p.exists())
                        .collect();
                }
            }
        }
        Vec::new()
    }

    fn save_recent_project(&mut self, path: PathBuf) {
        // Add to recent projects if not already there
        if !self.recent_projects.contains(&path) {
            self.recent_projects.insert(0, path);
            // Keep only last 10 projects
            self.recent_projects.truncate(10);

            // Save to config file
            if let Some(config_path) = Self::get_config_path() {
                // Ensure parent directory exists
                if let Some(parent) = config_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }

                let config = RecentProjectsConfig {
                    recent_projects: self.recent_projects.iter()
                        .map(|p| p.display().to_string())
                        .collect(),
                };

                if let Ok(json) = serde_json::to_string_pretty(&config) {
                    let _ = fs::write(&config_path, json);
                }
            }
        }
    }

    fn open_folder_dialog(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let file_dialog = rfd::AsyncFileDialog::new()
            .set_title("Select Pulsar Project Folder")
            .set_directory(std::env::current_dir().unwrap_or_default());

        cx.spawn(async move |this, mut cx| {
            if let Some(folder) = file_dialog.pick_folder().await {
                let path = folder.path().to_path_buf();

                // Validate that Pulsar.toml exists
                let toml_path = path.join("Pulsar.toml");
                if !toml_path.exists() {
                    eprintln!("Invalid project: Pulsar.toml not found in selected folder");
                    return;
                }

                cx.update(|cx| {
                    this.update(cx, |screen, cx| {
                        screen.save_recent_project(path.clone());
                        cx.emit(ProjectSelected { path });
                    });
                }).ok();
            }
        }).detach();
    }

    fn open_project(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.save_recent_project(path.clone());
        cx.emit(ProjectSelected { path });
    }

    fn create_blank_project(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let file_dialog = rfd::AsyncFileDialog::new()
            .set_title("Create New Pulsar Project")
            .set_directory(std::env::current_dir().unwrap_or_default());

        cx.spawn(async move |this, mut cx| {
            if let Some(folder) = file_dialog.pick_folder().await {
                let path = folder.path().to_path_buf();

                // Create project structure
                if let Err(e) = Self::init_blank_project(&path) {
                    eprintln!("Failed to create project: {}", e);
                    return;
                }

                cx.update(|cx| {
                    this.update(cx, |screen, cx| {
                        screen.save_recent_project(path.clone());
                        cx.emit(ProjectSelected { path });
                    });
                }).ok();
            }
        }).detach();
    }

    fn init_blank_project(path: &PathBuf) -> Result<(), String> {
        // Create Pulsar.toml
        let toml_content = r#"[package]
name = "new_project"
version = "0.1.0"

[pulsar]
engine_version = "0.1.0"
"#;
        std::fs::write(path.join("Pulsar.toml"), toml_content)
            .map_err(|e| format!("Failed to create Pulsar.toml: {}", e))?;

        // Create src directory
        std::fs::create_dir_all(path.join("src"))
            .map_err(|e| format!("Failed to create src directory: {}", e))?;

        // Create assets directory
        std::fs::create_dir_all(path.join("assets"))
            .map_err(|e| format!("Failed to create assets directory: {}", e))?;

        Ok(())
    }

    fn clone_template(&mut self, template_url: &str, _window: &mut Window, cx: &mut Context<Self>) {
        let template_url = template_url.to_string();
        let file_dialog = rfd::AsyncFileDialog::new()
            .set_title("Choose Location for Template")
            .set_directory(std::env::current_dir().unwrap_or_default());

        cx.spawn(async move |this, mut cx| {
            if let Some(folder) = file_dialog.pick_folder().await {
                let parent_path = folder.path().to_path_buf();

                // Extract repository name from URL for the target directory
                let repo_name = template_url
                    .trim_end_matches('/')
                    .split('/')
                    .last()
                    .unwrap_or("template")
                    .trim_end_matches(".git");

                let path = parent_path.join(repo_name);

                // Clone the git repository
                match git2::Repository::clone(&template_url, &path) {
                    Ok(_) => {
                        // Validate that Pulsar.toml exists
                        let toml_path = path.join("Pulsar.toml");
                        if !toml_path.exists() {
                            eprintln!("Warning: Cloned template does not contain Pulsar.toml");
                        }

                        cx.update(|cx| {
                            this.update(cx, |screen, cx| {
                                screen.save_recent_project(path.clone());
                                cx.emit(ProjectSelected { path });
                            });
                        }).ok();
                    }
                    Err(e) => {
                        eprintln!("Failed to clone template: {}", e);
                    }
                }
            }
        }).detach();
    }

    fn clone_repository(&mut self, repo_url: &str, _window: &mut Window, cx: &mut Context<Self>) {
        let repo_url = repo_url.to_string();
        let file_dialog = rfd::AsyncFileDialog::new()
            .set_title("Choose Location for Repository")
            .set_directory(std::env::current_dir().unwrap_or_default());

        cx.spawn(async move |this, mut cx| {
            if let Some(folder) = file_dialog.pick_folder().await {
                let parent_path = folder.path().to_path_buf();

                // Extract repository name from URL for the target directory
                let repo_name = repo_url
                    .trim_end_matches('/')
                    .split('/')
                    .last()
                    .unwrap_or("repository")
                    .trim_end_matches(".git");

                let path = parent_path.join(repo_name);

                // Clone the git repository
                match git2::Repository::clone(&repo_url, &path) {
                    Ok(_) => {
                        // Validate that Pulsar.toml exists
                        let toml_path = path.join("Pulsar.toml");
                        if !toml_path.exists() {
                            eprintln!("Warning: Cloned repository does not contain Pulsar.toml");
                        }

                        cx.update(|cx| {
                            this.update(cx, |screen, cx| {
                                screen.save_recent_project(path.clone());
                                cx.emit(ProjectSelected { path });
                            });
                        }).ok();
                    }
                    Err(e) => {
                        eprintln!("Failed to clone repository: {}", e);
                    }
                }
            }
        }).detach();
    }

    fn render_card(&self, card: &CardItem, index: usize, cx: &mut Context<Self>) -> impl IntoElement {
        let is_selected = self.selected_card == Some(index);
        let card_clone = card.clone();

        div()
            .id(SharedString::from(format!("card-{}", index)))
            .w(px(280.))
            .h(px(200.))
            .bg(cx.theme().input)
            .border_1()
            .border_color(if is_selected {
                cx.theme().primary
            } else {
                cx.theme().border
            })
            .rounded(px(12.))
            .overflow_hidden()
            .cursor_pointer()
            .hover(|style| {
                style.border_color(cx.theme().primary.opacity(0.5))
            })
            .on_click(cx.listener(move |screen, _, window, cx| {
                screen.selected_card = Some(index);
                // Initialize project name and path when selecting
                if let CardItem::Project(ref p) = card_clone {
                    let name = p.name.clone();
                    let path = p.path.display().to_string();
                    screen.project_name_input.update(cx, |input, cx| {
                        input.set_value(&name, window, cx);
                    });
                    screen.project_path_input.update(cx, |input, cx| {
                        input.set_value(&path, window, cx);
                    });
                } else if let CardItem::BlankProject = card_clone {
                    let path = std::env::current_dir()
                        .unwrap_or_default()
                        .display()
                        .to_string();
                    screen.project_name_input.update(cx, |input, cx| {
                        input.set_value("New Project", window, cx);
                    });
                    screen.project_path_input.update(cx, |input, cx| {
                        input.set_value(&path, window, cx);
                    });
                }
                cx.notify();
            }))
            .child(
                v_flex()
                    .size_full()
                    .child(
                        // Image placeholder
                        div()
                            .w_full()
                            .h(px(120.))
                            .bg(cx.theme().muted)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::new(IconName::Folder)
                                    .size_8()
                                    .text_color(cx.theme().muted_foreground)
                            )
                    )
                    .child(
                        // Card content
                        v_flex()
                            .p_3()
                            .gap_1()
                            .child(
                                div()
                                    .text_base()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child(card.name().to_string())
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .overflow_hidden()
                                    .child(card.description().to_string())
                            )
                    )
            )
    }

    fn render_sidebar(&mut self, cards: &[CardItem], cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let selected_index = self.selected_card?;
        let card = cards.get(selected_index)?.clone();

        Some(
            v_flex()
                .w(px(350.))
                .h_full()
                .bg(cx.theme().sidebar)
                .border_l_1()
                .border_color(cx.theme().border)
                .p_6()
                .gap_6()
                .child(
                    // Close button
                    div()
                        .w_full()
                        .flex()
                        .justify_end()
                        .child(
                            Button::new("close-sidebar")
                                .ghost()
                                .icon(IconName::Close)
                                .on_click(cx.listener(|screen, _, _, cx| {
                                    screen.selected_card = None;
                                    cx.notify();
                                }))
                        )
                )
                .child(
                    // Expanded image
                    div()
                        .w_full()
                        .h(px(200.))
                        .bg(cx.theme().muted)
                        .rounded(px(8.))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            Icon::new(IconName::Folder)
                                .size_16()
                                .text_color(cx.theme().muted_foreground)
                        )
                )
                .child(
                    // Title and description
                    v_flex()
                        .gap_2()
                        .child(
                            div()
                                .text_xl()
                                .font_bold()
                                .text_color(cx.theme().foreground)
                                .child(card.name().to_string())
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().muted_foreground)
                                .child(card.description().to_string())
                        )
                )
                .child(
                    // Settings
                    v_flex()
                        .gap_4()
                        .child(
                            v_flex()
                                .gap_2()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_semibold()
                                        .text_color(cx.theme().foreground)
                                        .child("Project Name")
                                )
                                .child(
                                    TextInput::new(&self.project_name_input)
                                )
                        )
                        .child(
                            v_flex()
                                .gap_2()
                                .child(
                                    div()
                                        .text_sm()
                                        .font_semibold()
                                        .text_color(cx.theme().foreground)
                                        .child("Project Path")
                                )
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .child(
                                            TextInput::new(&self.project_path_input)
                                        )
                                        .child(
                                            Button::new("browse-path")
                                                .ghost()
                                                .icon(IconName::Folder)
                                                .on_click(cx.listener(|screen, _, window, cx| {
                                                    screen.browse_project_path(window, cx);
                                                }))
                                        )
                                )
                        )
                )
                .child(
                    // Action button
                    {
                        let card_for_action = card.clone();
                        Button::new("create-open-project")
                            .primary()
                            .w_full()
                            .on_click(cx.listener(move |screen, _, window, cx| {
                                screen.handle_card_action(&card_for_action, window, cx);
                            }))
                            .child(match &card {
                                CardItem::Project(_) => "Open Project",
                                CardItem::Template(_) => "Create from Template",
                                CardItem::BlankProject => "Create Project",
                            })
                    }
                )
        )
    }

    fn browse_project_path(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let file_dialog = rfd::AsyncFileDialog::new()
            .set_title("Select Project Location")
            .set_directory(std::env::current_dir().unwrap_or_default());

        cx.spawn(async move |this, mut cx| {
            if let Some(folder) = file_dialog.pick_folder().await {
                let path = folder.path().to_path_buf();
                let path_str = path.display().to_string();
                cx.update(|cx| {
                    this.update(cx, |screen, cx| {
                        screen.pending_path_update = Some(path_str);
                        cx.notify();
                    });
                }).ok();
            }
        }).detach();
    }

    fn handle_card_action(&mut self, card: &CardItem, window: &mut Window, cx: &mut Context<Self>) {
        match card {
            CardItem::Project(p) => {
                self.open_project(p.path.clone(), cx);
            }
            CardItem::Template(t) => {
                let project_name = self.project_name_input.read(cx).text().to_string();
                let project_path = self.project_path_input.read(cx).text().to_string();
                let target_path = PathBuf::from(project_path).join(project_name);
                self.clone_template_to_path(&t.git_url, target_path, window, cx);
            }
            CardItem::BlankProject => {
                let project_name = self.project_name_input.read(cx).text().to_string();
                let project_path = self.project_path_input.read(cx).text().to_string();
                let target_path = PathBuf::from(project_path).join(project_name);
                self.create_blank_project_at_path(target_path, window, cx);
            }
        }
    }

    fn clone_template_to_path(&mut self, template_url: &str, path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) {
        let template_url = template_url.to_string();

        cx.spawn(async move |this, mut cx| {
            match git2::Repository::clone(&template_url, &path) {
                Ok(_) => {
                    cx.update(|cx| {
                        this.update(cx, |screen, cx| {
                            screen.save_recent_project(path.clone());
                            cx.emit(ProjectSelected { path });
                        });
                    }).ok();
                }
                Err(e) => {
                    eprintln!("Failed to clone template: {}", e);
                }
            }
        }).detach();
    }

    fn create_blank_project_at_path(&mut self, path: PathBuf, _window: &mut Window, cx: &mut Context<Self>) {
        cx.spawn(async move |this, mut cx| {
            if let Err(e) = Self::init_blank_project(&path) {
                eprintln!("Failed to create project: {}", e);
                return;
            }

            cx.update(|cx| {
                this.update(cx, |screen, cx| {
                    screen.save_recent_project(path.clone());
                    cx.emit(ProjectSelected { path });
                });
            }).ok();
        }).detach();
    }
}

impl EventEmitter<ProjectSelected> for EntryScreen {}

impl Render for EntryScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Apply pending path update if any
        if let Some(path) = self.pending_path_update.take() {
            self.project_path_input.update(cx, |input, cx| {
                input.set_value(&path, window, cx);
            });
        }

        let cards = self.get_cards();

        div()
            .size_full()
            .bg(cx.theme().background)
            .child(
                h_flex()
                    .size_full()
                    .child(
                        // Left sidebar with tabs
                        v_flex()
                            .w(px(180.))
                            .h_full()
                            .bg(cx.theme().sidebar)
                            .border_r_1()
                            .border_color(cx.theme().border)
                            .child(
                                v_flex()
                                    .gap_1()
                                    .p_4()
                                    .child(
                                        // Logo/Title
                                        div()
                                            .mb_6()
                                            .child(
                                                div()
                                                    .text_lg()
                                                    .font_bold()
                                                    .text_color(cx.theme().foreground)
                                                    .child("Pulsar")
                                            )
                                    )
                                    .child(
                                        if self.active_tab == EntryTab::Manage {
                                            Button::new("tab-manage").primary().w_full()
                                        } else {
                                            Button::new("tab-manage").ghost().w_full()
                                        }
                                        .on_click(cx.listener(|screen, _, _, cx| {
                                            screen.active_tab = EntryTab::Manage;
                                            screen.selected_card = None;
                                            cx.notify();
                                        }))
                                        .child("Recent Projects")
                                    )
                                    .child(
                                        if self.active_tab == EntryTab::Create {
                                            Button::new("tab-create").primary().w_full()
                                        } else {
                                            Button::new("tab-create").ghost().w_full()
                                        }
                                        .on_click(cx.listener(|screen, _, _, cx| {
                                            screen.active_tab = EntryTab::Create;
                                            screen.selected_card = None;
                                            cx.notify();
                                        }))
                                        .child("Create New")
                                    )
                            )
                    )
                    .child(
                        // Main content area with card grid
                        v_flex()
                            .id("entry-screen-content")
                            .flex_1()
                            .overflow_y_scroll()
                            .child(
                                v_flex()
                                    .p_8()
                                    .gap_6()
                                    .child(
                                        // Header
                                        v_flex()
                                            .gap_2()
                                            .child(
                                                div()
                                                    .text_2xl()
                                                    .font_bold()
                                                    .text_color(cx.theme().foreground)
                                                    .child(match self.active_tab {
                                                        EntryTab::Manage => "Recent Projects",
                                                        EntryTab::Create => "Create New Project",
                                                        EntryTab::Git => "Clone from Git",
                                                    })
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(match self.active_tab {
                                                        EntryTab::Manage => "Open a recent project or select a folder",
                                                        EntryTab::Create => "Choose a template or start from scratch",
                                                        EntryTab::Git => "Clone a repository from Git",
                                                    })
                                            )
                                    )
                                    .child(
                                        // Card grid
                                        div()
                                            .flex()
                                            .flex_wrap()
                                            .gap_4()
                                            .children(
                                                if cards.is_empty() {
                                                    vec![
                                                        div()
                                                            .p_8()
                                                            .child(
                                                                v_flex()
                                                                    .gap_4()
                                                                    .items_center()
                                                                    .child(
                                                                        Icon::new(IconName::Inbox)
                                                                            .size_12()
                                                                            .text_color(cx.theme().muted_foreground)
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .text_base()
                                                                            .text_color(cx.theme().muted_foreground)
                                                                            .child("No recent projects")
                                                                    )
                                                                    .child(
                                                                        Button::new("open-folder-empty")
                                                                            .primary()
                                                                            .on_click(cx.listener(|screen, _, window, cx| {
                                                                                screen.open_folder_dialog(window, cx);
                                                                            }))
                                                                            .child("Open Folder")
                                                                    )
                                                            )
                                                            .into_any_element()
                                                    ]
                                                } else {
                                                    cards.iter().enumerate().map(|(index, card)| {
                                                        self.render_card(card, index, cx).into_any_element()
                                                    }).collect()
                                                }
                                            )
                                    )
                                    .children(if self.active_tab == EntryTab::Manage && !cards.is_empty() {
                                        Some(
                                            Button::new("open-folder")
                                                .ghost()
                                                .icon(IconName::FolderOpen)
                                                .label("Open from Folder")
                                                .on_click(cx.listener(|screen, _, window, cx| {
                                                    screen.open_folder_dialog(window, cx);
                                                }))
                                        )
                                    } else {
                                        None
                                    })
                            )
                    )
                    .children(self.render_sidebar(&cards, cx))
            )
    }
}
