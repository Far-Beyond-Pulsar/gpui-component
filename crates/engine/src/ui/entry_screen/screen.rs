use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
    input::{InputState, TextInput},
    ActiveTheme as _, Icon, IconName, StyledExt,
};
use std::path::PathBuf;

use super::{card, models::*, sidebar, storage};

/// Main entry screen component for project selection and creation
pub struct EntryScreen {
    active_tab: EntryTab,
    recent_projects: Vec<PathBuf>,
    selected_card: Option<usize>,
    project_name_input: Entity<InputState>,
    project_path_input: Entity<InputState>,
    pending_path_update: Option<String>,
    search_query: String,
}

impl EntryScreen {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            active_tab: EntryTab::Manage,
            recent_projects: storage::load_recent_projects(),
            selected_card: None,
            project_name_input: cx.new(|cx| InputState::new(window, cx)),
            project_path_input: cx.new(|cx| InputState::new(window, cx)),
            pending_path_update: None,
            search_query: String::new(),
        }
    }

    fn get_cards(&self) -> Vec<CardItem> {
        match self.active_tab {
            EntryTab::Manage => self
                .recent_projects
                .iter()
                .map(|path| {
                    let name = path
                        .file_name()
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
                })
                .collect(),
            EntryTab::Create => vec![
                CardItem::BlankProject,
                CardItem::Template(TemplateCard {
                    name: "2D Platformer".to_string(),
                    description:
                        "A complete 2D platformer template with character controller, tilemaps, and basic enemies"
                            .to_string(),
                    image_path: None,
                    git_url: "https://github.com/pulsar-engine/template-2d".to_string(),
                    tags: vec!["2D".to_string(), "Platformer".to_string(), "Starter".to_string()],
                }),
                CardItem::Template(TemplateCard {
                    name: "3D First-Person".to_string(),
                    description:
                        "A 3D first-person template with camera controls, physics, and basic interactions"
                            .to_string(),
                    image_path: None,
                    git_url: "https://github.com/pulsar-engine/template-3d".to_string(),
                    tags: vec!["3D".to_string(), "First-Person".to_string(), "Advanced".to_string()],
                }),
                CardItem::Template(TemplateCard {
                    name: "Top-Down RPG".to_string(),
                    description:
                        "A top-down RPG template with inventory, dialogue system, and quest management"
                            .to_string(),
                    image_path: None,
                    git_url: "https://github.com/pulsar-engine/template-rpg".to_string(),
                    tags: vec!["2D".to_string(), "RPG".to_string(), "Advanced".to_string()],
                }),
            ],
        }
    }

    fn save_recent_project(&mut self, path: PathBuf) {
        if !self.recent_projects.contains(&path) {
            self.recent_projects.insert(0, path);
            self.recent_projects.truncate(10);
            let _ = storage::save_recent_projects(&self.recent_projects);
        }
    }

    fn open_folder_dialog(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let file_dialog = rfd::AsyncFileDialog::new()
            .set_title("Select Pulsar Project Folder")
            .set_directory(std::env::current_dir().unwrap_or_default());

        cx.spawn(async move |this, mut cx| {
            if let Some(folder) = file_dialog.pick_folder().await {
                let path = folder.path().to_path_buf();
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
                })
                .ok();
            }
        })
        .detach();
    }

    fn open_project(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.save_recent_project(path.clone());
        cx.emit(ProjectSelected { path });
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
                })
                .ok();
            }
        })
        .detach();
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

    fn clone_template_to_path(
        &mut self,
        template_url: &str,
        path: PathBuf,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let template_url = template_url.to_string();

        cx.spawn(async move |this, mut cx| {
            match git2::Repository::clone(&template_url, &path) {
                Ok(_) => {
                    cx.update(|cx| {
                        this.update(cx, |screen, cx| {
                            screen.save_recent_project(path.clone());
                            cx.emit(ProjectSelected { path });
                        });
                    })
                    .ok();
                }
                Err(e) => {
                    eprintln!("Failed to clone template: {}", e);
                }
            }
        })
        .detach();
    }

    fn create_blank_project_at_path(
        &mut self,
        path: PathBuf,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.spawn(async move |this, mut cx| {
            if let Err(e) = storage::init_blank_project(&path) {
                eprintln!("Failed to create project: {}", e);
                return;
            }

            cx.update(|cx| {
                this.update(cx, |screen, cx| {
                    screen.save_recent_project(path.clone());
                    cx.emit(ProjectSelected { path });
                });
            })
            .ok();
        })
        .detach();
    }

    fn render_header(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .child(
                div()
                    .text_3xl()
                    .font_bold()
                    .text_color(cx.theme().foreground)
                    .child(match self.active_tab {
                        EntryTab::Manage => "Recent Projects",
                        EntryTab::Create => "Create New Project",
                    })
            )
            .child(
                div()
                    .text_base()
                    .text_color(cx.theme().muted_foreground)
                    .child(match self.active_tab {
                        EntryTab::Manage => "Open a recent project or select a folder",
                        EntryTab::Create => "Choose a template or start from scratch",
                    })
            )
    }

    fn render_nav_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(240.))
            .h_full()
            .bg(cx.theme().sidebar)
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                v_flex()
                    .gap_6()
                    .p_6()
                    .child(self.render_logo(cx))
                    .child(self.render_nav_buttons(cx))
            )
    }

    fn render_logo(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(
                        div()
                            .w(px(40.))
                            .h(px(40.))
                            .rounded(px(10.))
                            .bg(cx.theme().primary)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                Icon::new(IconName::CircleCheck)
                                    .size(px(24.))
                                    .text_color(cx.theme().primary_foreground)
                            )
                    )
                    .child(
                        div()
                            .text_xl()
                            .font_bold()
                            .text_color(cx.theme().foreground)
                            .child("Pulsar")
                    )
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child("Game Engine")
            )
    }

    fn render_nav_buttons(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .child(self.render_nav_button(
                "Recent Projects",
                IconName::FolderClosed,
                EntryTab::Manage,
                cx,
            ))
            .child(self.render_nav_button(
                "Create New",
                IconName::Plus,
                EntryTab::Create,
                cx,
            ))
            .child(
                div()
                    .h(px(1.))
                    .w_full()
                    .my_2()
                    .bg(cx.theme().border)
            )
            .child(
                Button::new("open-folder-nav")
                    .ghost()
                    .w_full()
                    .justify_start()
                    .icon(IconName::FolderOpen)
                    .label("Open Folder")
                    .on_click(cx.listener(|screen, _, window, cx| {
                        screen.open_folder_dialog(window, cx);
                    }))
            )
    }

    fn render_nav_button(
        &mut self,
        label: impl Into<SharedString>,
        icon: IconName,
        tab: EntryTab,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let is_active = self.active_tab == tab;

        let button_id = match tab {
            EntryTab::Manage => "tab-manage",
            EntryTab::Create => "tab-create",
        };

        if is_active {
            Button::new(button_id)
                .primary()
                .w_full()
                .justify_start()
                .icon(icon)
                .label(label)
        } else {
            Button::new(button_id)
                .ghost()
                .w_full()
                .justify_start()
                .icon(icon)
                .label(label)
                .on_click(cx.listener(move |screen, _, _, cx| {
                    screen.active_tab = tab.clone();
                    screen.selected_card = None;
                    cx.notify();
                }))
        }
    }

    fn render_card_grid(&self, cards: &[CardItem], cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_wrap()
            .gap_6()
            .children(cards.iter().enumerate().map(|(index, card_item)| {
                let is_selected = self.selected_card == Some(index);
                let card_clone = card_item.clone();

                div()
                    .cursor_pointer()
                    .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |screen, _, window, cx| {
                        screen.selected_card = Some(index);
                        screen.update_inputs_for_card(&card_clone, window, cx);
                        cx.notify();
                    }))
                    .child(card::render_card(card_item, index, is_selected, cx))
            }))
    }

    fn update_inputs_for_card(&mut self, card: &CardItem, window: &mut Window, cx: &mut Context<Self>) {
        match card {
            CardItem::Project(p) => {
                let name = p.name.clone();
                let path = p.path.display().to_string();
                self.project_name_input.update(cx, |input, cx| {
                    input.set_value(&name, window, cx);
                });
                self.project_path_input.update(cx, |input, cx| {
                    input.set_value(&path, window, cx);
                });
            }
            CardItem::BlankProject | CardItem::Template(_) => {
                let path = std::env::current_dir()
                    .unwrap_or_default()
                    .display()
                    .to_string();
                self.project_name_input.update(cx, |input, cx| {
                    input.set_value("New Project", window, cx);
                });
                self.project_path_input.update(cx, |input, cx| {
                    input.set_value(&path, window, cx);
                });
            }
        }
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
                    .child(self.render_nav_sidebar(cx))
                    .child(
                        v_flex()
                            .id("entry-screen-content")
                            .flex_1()
                            .overflow_hidden()
                            .child(
                                v_flex()
                                    .p(px(48.))
                                    .gap(px(32.))
                                    .child(self.render_header(cx))
                                    .child(if cards.is_empty() {
                                        div().child(card::render_empty_state(cx))
                                    } else {
                                        div().child(self.render_card_grid(&cards, cx))
                                    })
                            )
                    )
                    .children(
                        self.selected_card
                            .and_then(|index| cards.get(index).cloned())
                            .map(|card| {
                                let card_for_action = card.clone();
                                sidebar::render_sidebar(
                                    &card,
                                    &self.project_name_input,
                                    &self.project_path_input,
                                    |screen, _, _, cx| {
                                        screen.selected_card = None;
                                        cx.notify();
                                    },
                                    |screen, _, window, cx| {
                                        screen.browse_project_path(window, cx);
                                    },
                                    move |screen, _, window, cx| {
                                        screen.handle_card_action(&card_for_action, window, cx);
                                    },
                                    cx,
                                )
                            })
                    )
            )
    }
}
