use gpui::{prelude::*, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex, Icon, IconName, ActiveTheme as _, StyledExt, divider::Divider,
    scroll::ScrollbarAxis,
};
use crate::ui::entry_screen::EntryScreen;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub enum ProjectSettingsTab {
    General,
    GitInfo,
    GitCI,
    Metadata,
    DiskInfo,
    Performance,
    Integrations,
}

#[derive(Clone, Debug)]
pub struct ProjectSettings {
    pub project_path: PathBuf,
    pub project_name: String,
    pub active_tab: ProjectSettingsTab,
    pub git_repo_size: Option<u64>,
    pub disk_size: Option<u64>,
    pub commit_count: Option<usize>,
    pub branch_count: Option<usize>,
    pub remote_url: Option<String>,
    pub last_commit_date: Option<String>,
    pub last_commit_message: Option<String>,
    pub uncommitted_changes: Option<usize>,
    pub workflow_files: Vec<String>,
}

impl ProjectSettings {
    pub fn new(project_path: PathBuf, project_name: String) -> Self {
        Self {
            project_path,
            project_name,
            active_tab: ProjectSettingsTab::General,
            git_repo_size: None,
            disk_size: None,
            commit_count: None,
            branch_count: None,
            remote_url: None,
            last_commit_date: None,
            last_commit_message: None,
            uncommitted_changes: None,
            workflow_files: Vec::new(),
        }
    }

    pub fn load_all_data(&mut self) {
        self.load_disk_info();
        self.load_git_info();
        self.load_git_ci_info();
    }

    fn load_disk_info(&mut self) {
        if let Ok(size) = Self::calculate_directory_size(&self.project_path) {
            self.disk_size = Some(size);
        }
    }

    fn calculate_directory_size(path: &PathBuf) -> Result<u64, std::io::Error> {
        let mut total_size = 0u64;
        
        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                
                if metadata.is_file() {
                    total_size += metadata.len();
                } else if metadata.is_dir() {
                    let dir_path = entry.path();
                    if let Ok(size) = Self::calculate_directory_size(&dir_path) {
                        total_size += size;
                    }
                }
            }
        }
        
        Ok(total_size)
    }

    fn load_git_info(&mut self) {
        let git_dir = self.project_path.join(".git");
        if !git_dir.exists() {
            return;
        }

        // Get git repo size
        if let Ok(size) = Self::calculate_directory_size(&git_dir) {
            self.git_repo_size = Some(size);
        }

        // Get commit count
        if let Ok(output) = std::process::Command::new("git")
            .current_dir(&self.project_path)
            .args(&["rev-list", "--count", "HEAD"])
            .output()
        {
            if let Ok(count_str) = String::from_utf8(output.stdout) {
                if let Ok(count) = count_str.trim().parse::<usize>() {
                    self.commit_count = Some(count);
                }
            }
        }

        // Get branch count
        if let Ok(output) = std::process::Command::new("git")
            .current_dir(&self.project_path)
            .args(&["branch", "-a"])
            .output()
        {
            if let Ok(branches) = String::from_utf8(output.stdout) {
                self.branch_count = Some(branches.lines().count());
            }
        }

        // Get remote URL
        if let Ok(output) = std::process::Command::new("git")
            .current_dir(&self.project_path)
            .args(&["remote", "get-url", "origin"])
            .output()
        {
            if let Ok(url) = String::from_utf8(output.stdout) {
                self.remote_url = Some(url.trim().to_string());
            }
        }

        // Get last commit info
        if let Ok(output) = std::process::Command::new("git")
            .current_dir(&self.project_path)
            .args(&["log", "-1", "--format=%cd|%s", "--date=relative"])
            .output()
        {
            if let Ok(info) = String::from_utf8(output.stdout) {
                let parts: Vec<&str> = info.trim().split('|').collect();
                if parts.len() == 2 {
                    self.last_commit_date = Some(parts[0].to_string());
                    self.last_commit_message = Some(parts[1].to_string());
                }
            }
        }

        // Get uncommitted changes
        if let Ok(output) = std::process::Command::new("git")
            .current_dir(&self.project_path)
            .args(&["status", "--porcelain"])
            .output()
        {
            if let Ok(status) = String::from_utf8(output.stdout) {
                self.uncommitted_changes = Some(status.lines().count());
            }
        }
    }

    fn load_git_ci_info(&mut self) {
        let workflows_dir = self.project_path.join(".github").join("workflows");
        if workflows_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(workflows_dir) {
                self.workflow_files = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| {
                        e.path()
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| ext == "yml" || ext == "yaml")
                            .unwrap_or(false)
                    })
                    .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
                    .collect();
            }
        }
    }
}

pub fn render_project_settings(screen: &EntryScreen, settings: &ProjectSettings, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    let theme = cx.theme();
    
    div()
        .absolute()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(theme.background.opacity(0.95))
        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
            // Close modal when clicking on background
            this.close_project_settings(cx);
        }))
        .child(
            h_flex()
                .w(px(1200.))
                .h(px(800.))
                .bg(theme.background)
                .rounded_xl()
                .border_1()
                .border_color(theme.border)
                .shadow_lg()
                .overflow_hidden()
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                    // Stop propagation for mouse down too
                    cx.stop_propagation();
                })
                .child(render_settings_sidebar(settings, cx))
                .child(render_settings_content(settings, cx))
        )
}

fn render_settings_sidebar(settings: &ProjectSettings, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    let theme = cx.theme();
    let active_tab = settings.active_tab.clone();
    
    v_flex()
        .w(px(250.))
        .h_full()
        .bg(theme.sidebar)
        .border_r_1()
        .border_color(theme.border)
        .p_4()
        .gap_2()
        .child(
            h_flex()
                .items_center()
                .gap_2()
                .mb_4()
                .child(
                    Icon::new(IconName::Settings)
                        .size(px(20.))
                        .text_color(theme.primary)
                )
                .child(
                    div()
                        .text_lg()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child("Project Settings")
                )
        )
        .child(Divider::horizontal())
        .child(
            v_flex()
                .gap_1()
                .mt_2()
                .child(render_sidebar_item("General", IconName::Folder, ProjectSettingsTab::General, &active_tab, cx))
                .child(render_sidebar_item("Git Info", IconName::GitHub, ProjectSettingsTab::GitInfo, &active_tab, cx))
                .child(render_sidebar_item("Git CI/CD", IconName::Settings, ProjectSettingsTab::GitCI, &active_tab, cx))
                .child(render_sidebar_item("Metadata", IconName::Folder, ProjectSettingsTab::Metadata, &active_tab, cx))
                .child(render_sidebar_item("Disk Info", IconName::HardDrive, ProjectSettingsTab::DiskInfo, &active_tab, cx))
                .child(render_sidebar_item("Performance", IconName::Activity, ProjectSettingsTab::Performance, &active_tab, cx))
                .child(render_sidebar_item("Integrations", IconName::Link, ProjectSettingsTab::Integrations, &active_tab, cx))
        )
        .child(
            v_flex()
                .flex_1()
                .justify_end()
                .child(
                    Button::new("close-settings")
                        .label("Close")
                        .w_full()
                        .with_variant(gpui_component::button::ButtonVariant::Secondary)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.close_project_settings(cx);
                        }))
                )
        )
}

fn render_sidebar_item(label: &str, icon: IconName, tab: ProjectSettingsTab, active_tab: &ProjectSettingsTab, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    let theme = cx.theme();
    let is_active = *active_tab == tab;
    let label_str = label.to_string();
    
    div()
        .w_full()
        .px_3()
        .py_2()
        .gap_2()
        .flex()
        .items_center()
        .rounded_md()
        .bg(if is_active { theme.primary.opacity(0.1) } else { gpui::transparent_black() })
        .border_1()
        .border_color(if is_active { theme.primary } else { gpui::transparent_black() })
        .hover(|this| {
            if !is_active {
                this.bg(theme.muted.opacity(0.1))
            } else {
                this
            }
        })
        .cursor_pointer()
        .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, _, _, cx| {
            this.change_project_settings_tab(tab.clone(), cx);
        }))
        .child(
            Icon::new(icon)
                .size(px(16.))
                .text_color(if is_active { theme.primary } else { theme.muted_foreground })
        )
        .child(
            div()
                .text_sm()
                .font_weight(if is_active { gpui::FontWeight::SEMIBOLD } else { gpui::FontWeight::NORMAL })
                .text_color(if is_active { theme.primary } else { theme.foreground })
                .child(label_str)
        )
}

fn render_settings_content(settings: &ProjectSettings, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    v_flex()
        .flex_1()
        .h_full()
        .scrollable(ScrollbarAxis::Vertical)
        .p_8()
        .child(
            match settings.active_tab {
                ProjectSettingsTab::General => render_general_tab(settings, cx).into_any_element(),
                ProjectSettingsTab::GitInfo => render_git_info_tab(settings, cx).into_any_element(),
                ProjectSettingsTab::GitCI => render_git_ci_tab(settings, cx).into_any_element(),
                ProjectSettingsTab::Metadata => render_metadata_tab(settings, cx).into_any_element(),
                ProjectSettingsTab::DiskInfo => render_disk_info_tab(settings, cx).into_any_element(),
                ProjectSettingsTab::Performance => render_performance_tab(settings, cx).into_any_element(),
                ProjectSettingsTab::Integrations => render_integrations_tab(settings, cx).into_any_element(),
            }
        )
}

fn render_general_tab(settings: &ProjectSettings, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    let theme = cx.theme();
    
    v_flex()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme.foreground)
                .child("General Settings")
        )
        .child(Divider::horizontal())
        .child(render_info_section("Project Information", vec![
            ("Name", settings.project_name.clone()),
            ("Path", settings.project_path.to_string_lossy().to_string()),
            ("Type", "Pulsar Native Game Project".to_string()),
        ], &theme))
        .child(render_info_section("Project Actions", vec![], &theme))
        .child(
            v_flex()
                .gap_3()
                .child(
                    Button::new("open-in-explorer")
                        .label("Open in File Manager")
                        .icon(IconName::FolderOpen)
                        .with_variant(gpui_component::button::ButtonVariant::Secondary)
                        .on_click({
                            let path = settings.project_path.clone();
                            move |_, _, _| {
                                let _ = open::that(&path);
                            }
                        })
                )
                .child(
                    Button::new("open-in-terminal")
                        .label("Open in Terminal")
                        .icon(IconName::Terminal)
                        .with_variant(gpui_component::button::ButtonVariant::Secondary)
                        .on_click({
                            let path = settings.project_path.clone();
                            move |_, _, _| {
                                #[cfg(windows)]
                                {
                                    let _ = std::process::Command::new("cmd")
                                        .args(&["/c", "start", "cmd", "/k", "cd", path.to_str().unwrap_or("")])
                                        .spawn();
                                }
                                #[cfg(not(windows))]
                                {
                                    let _ = std::process::Command::new("open")
                                        .args(&["-a", "Terminal", path.to_str().unwrap_or("")])
                                        .spawn();
                                }
                            }
                        })
                )
        )
}

fn render_git_info_tab(settings: &ProjectSettings, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    let theme = cx.theme();
    
    v_flex()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme.foreground)
                .child("Git Information")
        )
        .child(Divider::horizontal())
        .child(render_info_section("Repository", vec![
            ("Remote URL", settings.remote_url.clone().unwrap_or_else(|| "No remote configured".to_string())),
            ("Total Commits", settings.commit_count.map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string())),
            ("Total Branches", settings.branch_count.map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string())),
            (".git Size", format_size(settings.git_repo_size)),
        ], &theme))
        .child(render_info_section("Latest Commit", vec![
            ("Date", settings.last_commit_date.clone().unwrap_or_else(|| "N/A".to_string())),
            ("Message", settings.last_commit_message.clone().unwrap_or_else(|| "N/A".to_string())),
        ], &theme))
        .child(render_info_section("Working Directory", vec![
            ("Uncommitted Changes", settings.uncommitted_changes.map(|c| {
                if c == 0 {
                    "Clean - No changes".to_string()
                } else {
                    format!("{} file(s) modified", c)
                }
            }).unwrap_or_else(|| "N/A".to_string())),
        ], &theme))
        .child(
            v_flex()
                .gap_3()
                .child(
                    Button::new("refresh-git-info")
                        .label("Refresh Git Info")
                        .icon(IconName::ArrowUp)
                        .with_variant(gpui_component::button::ButtonVariant::Primary)
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.refresh_project_settings(cx);
                        }))
                )
                .child(
                    Button::new("open-git-ui")
                        .label("Open Git GUI")
                        .icon(IconName::GitHub)
                        .with_variant(gpui_component::button::ButtonVariant::Secondary)
                        .on_click({
                            let path = settings.project_path.clone();
                            move |_, _, _| {
                                let _ = std::process::Command::new("git")
                                    .args(&["gui"])
                                    .current_dir(&path)
                                    .spawn();
                            }
                        })
                )
        )
}

fn render_git_ci_tab(settings: &ProjectSettings, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    let theme = cx.theme();
    
    v_flex()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme.foreground)
                .child("Git CI/CD Integration")
        )
        .child(Divider::horizontal())
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child("Continuous Integration and Deployment workflows for your project")
        )
        .child(render_info_section("GitHub Actions", vec![
            ("Workflow Files", settings.workflow_files.len().to_string()),
            ("Status", if settings.workflow_files.is_empty() { "Not configured" } else { "Active" }.to_string()),
        ], &theme))
        .child(
            v_flex()
                .gap_2()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child("Detected Workflows")
                )
                .children(if settings.workflow_files.is_empty() {
                    vec![
                        div()
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child("No workflow files found in .github/workflows/")
                            .into_any_element()
                    ]
                } else {
                    settings.workflow_files.iter().map(|workflow| {
                        h_flex()
                            .gap_2()
                            .items_center()
                            .px_3()
                            .py_2()
                            .border_1()
                            .border_color(theme.border)
                            .rounded_md()
                            .bg(theme.sidebar)
                            .child(
                                Icon::new(IconName::Folder)
                                    .size(px(16.))
                                    .text_color(theme.accent)
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(theme.foreground)
                                    .child(workflow.clone())
                            )
                            .into_any_element()
                    }).collect()
                })
        )
        .child(
            v_flex()
                .gap_3()
                .mt_4()
                .child(
                    Button::new("create-workflow")
                        .label("Create New Workflow")
                        .icon(IconName::Plus)
                        .with_variant(gpui_component::button::ButtonVariant::Primary)
                        .on_click({
                            let path = settings.project_path.clone();
                            move |_, _, _| {
                                let workflows_dir = path.join(".github").join("workflows");
                                let _ = std::fs::create_dir_all(&workflows_dir);
                                let _ = open::that(&workflows_dir);
                            }
                        })
                )
                .child(
                    Button::new("view-actions")
                        .label("View on GitHub")
                        .icon(IconName::GitHub)
                        .with_variant(gpui_component::button::ButtonVariant::Secondary)
                        .on_click({
                            let remote = settings.remote_url.clone();
                            move |_, _, _| {
                                if let Some(url) = &remote {
                                    let actions_url = url
                                        .trim_end_matches(".git")
                                        .to_string() + "/actions";
                                    let _ = open::that(actions_url);
                                }
                            }
                        })
                )
        )
}

fn render_metadata_tab(settings: &ProjectSettings, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    let theme = cx.theme();
    let config_path = settings.project_path.join("Pulsar.toml");
    let has_config = config_path.exists();
    
    // Try to read the config file
    let config_content = if has_config {
        std::fs::read_to_string(&config_path).ok()
    } else {
        None
    };
    
    // Parse basic info from config
    let (project_name, project_version, engine_version) = if let Some(ref content) = config_content {
        let name = content.lines()
            .find(|line| line.trim().starts_with("name"))
            .and_then(|line| line.split('=').nth(1))
            .map(|s| s.trim().trim_matches('"').to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        
        let version = content.lines()
            .find(|line| line.trim().starts_with("version"))
            .and_then(|line| line.split('=').nth(1))
            .map(|s| s.trim().trim_matches('"').to_string())
            .unwrap_or_else(|| "0.1.0".to_string());
        
        let engine = content.lines()
            .find(|line| line.trim().starts_with("engine_version"))
            .and_then(|line| line.split('=').nth(1))
            .map(|s| s.trim().trim_matches('"').to_string())
            .unwrap_or_else(|| "Unknown".to_string());
        
        (name, version, engine)
    } else {
        ("Unknown".to_string(), "0.1.0".to_string(), "Unknown".to_string())
    };
    
    v_flex()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme.foreground)
                .child("Project Metadata & Configuration")
        )
        .child(Divider::horizontal())
        .child(
            div()
                .p_4()
                .rounded_lg()
                .bg(if has_config { theme.accent.opacity(0.1) } else { hsla(0.0, 0.8, 0.6, 0.1) })
                .border_1()
                .border_color(if has_config { theme.accent.opacity(0.3) } else { hsla(0.0, 0.8, 0.6, 0.3) })
                .child(
                    h_flex()
                        .gap_3()
                        .items_center()
                        .child(
                            Icon::new(if has_config { IconName::Folder } else { IconName::WarningTriangle })
                                .size(px(24.))
                                .text_color(if has_config { theme.accent } else { hsla(0.0, 0.8, 0.6, 1.0) })
                        )
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(theme.foreground)
                                        .child(if has_config { "Configuration File Found" } else { "Configuration File Missing" })
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child(config_path.to_string_lossy().to_string())
                                )
                        )
                )
        )
        .when(has_config, |this| {
            this.child(render_info_section("Project Information", vec![
                ("Name", project_name.clone()),
                ("Version", project_version),
                ("Engine Version", engine_version),
            ], &theme))
        })
        .child(render_info_section("Project Structure", vec![
            ("Project Root", settings.project_path.to_string_lossy().to_string()),
            ("Assets Folder", settings.project_path.join("assets").to_string_lossy().to_string()),
            ("Scenes Folder", settings.project_path.join("scenes").to_string_lossy().to_string()),
            ("Scripts Folder", settings.project_path.join("scripts").to_string_lossy().to_string()),
        ], &theme))
        .child(
            div()
                .p_4()
                .rounded_lg()
                .bg(theme.accent.opacity(0.1))
                .border_1()
                .border_color(theme.accent.opacity(0.3))
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.accent)
                                .child("📋 Pulsar.toml Fields")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("• [project] - name, version, engine_version")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("• [settings] - default_scene, window settings")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("• [build] - target platforms, optimization")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("• [dependencies] - project dependencies")
                        )
                )
        )
        .child(
            v_flex()
                .gap_3()
                .child(
                    h_flex()
                        .gap_3()
                        .child(
                            Button::new("edit-config")
                                .label("Edit Configuration")
                                .icon(IconName::Folder)
                                .with_variant(gpui_component::button::ButtonVariant::Primary)
                                .on_click({
                                    let path = config_path.clone();
                                    move |_, _, _| {
                                        let _ = open::that(&path);
                                    }
                                })
                        )
                        .child(
                            Button::new("validate-config")
                                .label("Validate Project")
                                .icon(IconName::Activity)
                                .with_variant(gpui_component::button::ButtonVariant::Secondary)
                                .on_click({
                                    let path = settings.project_path.clone();
                                    move |_, _, _| {
                                        // Validate project structure
                                        let required_dirs = ["assets", "scenes", "scripts"];
                                        let mut missing = Vec::new();
                                        
                                        for dir in required_dirs {
                                            let dir_path = path.join(dir);
                                            if !dir_path.exists() {
                                                missing.push(dir);
                                            }
                                        }
                                        
                                        if missing.is_empty() {
                                            println!("✓ Project structure is valid");
                                        } else {
                                            println!("⚠ Missing directories: {}", missing.join(", "));
                                        }
                                    }
                                })
                        )
                        .child(
                            Button::new("create-missing")
                                .label("Create Missing Folders")
                                .icon(IconName::FolderOpen)
                                .with_variant(gpui_component::button::ButtonVariant::Secondary)
                                .on_click({
                                    let path = settings.project_path.clone();
                                    move |_, _, _| {
                                        let dirs = ["assets", "scenes", "scripts", "prefabs"];
                                        for dir in dirs {
                                            let _ = std::fs::create_dir_all(path.join(dir));
                                        }
                                        println!("✓ Created project folders");
                                    }
                                })
                        )
                )
        )
}

fn render_disk_info_tab(settings: &ProjectSettings, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    let theme = cx.theme();
    let project_size = settings.disk_size.unwrap_or(0);
    let git_size = settings.git_repo_size.unwrap_or(0);
    let working_files_size = if project_size > git_size { project_size - git_size } else { 0 };
    
    v_flex()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme.foreground)
                .child("Disk Usage")
        )
        .child(Divider::horizontal())
        .child(render_info_section("Total Size", vec![
            ("Project Size", format_size(Some(project_size))),
            ("Git Repository", format_size(Some(git_size))),
            ("Working Files", format_size(Some(working_files_size))),
        ], &theme))
        .child(
            v_flex()
                .gap_3()
                .p_4()
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .bg(theme.sidebar)
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .mb_3()
                        .child("Size Breakdown")
                )
                .child(render_size_bar("Working Files", working_files_size, project_size, theme.accent, &theme))
                .child(render_size_bar("Git Data", git_size, project_size, theme.primary, &theme))
        )
        .child(
            v_flex()
                .gap_3()
                .child(
                    Button::new("refresh-disk")
                        .label("Refresh Disk Info")
                        .icon(IconName::ArrowUp)
                        .with_variant(gpui_component::button::ButtonVariant::Primary)
                        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, cx| {
                            this.refresh_project_settings(cx);
                        }))
                )
                .child(
                    Button::new("clean-project")
                        .label("Clean Project (Git GC)")
                        .icon(IconName::Trash)
                        .with_variant(gpui_component::button::ButtonVariant::Secondary)
                        .on_click({
                            let path = settings.project_path.clone();
                            move |_, _, _| {
                                let _ = std::process::Command::new("git")
                                    .args(&["gc", "--aggressive", "--prune=now"])
                                    .current_dir(&path)
                                    .spawn();
                            }
                        })
                )
        )
}

fn render_performance_tab(settings: &ProjectSettings, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    let theme = cx.theme();
    let project_size = settings.disk_size.unwrap_or(0);
    let git_size = settings.git_repo_size.unwrap_or(0);
    
    // Calculate repository health score
    let health_score = calculate_repo_health(settings);
    let health_color = if health_score >= 80.0 {
        theme.accent
    } else if health_score >= 50.0 {
        hsla(45.0 / 360.0, 0.8, 0.6, 1.0) // Orange-ish
    } else {
        hsla(0.0, 0.8, 0.6, 1.0) // Red-ish
    };
    
    v_flex()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme.foreground)
                .child("Performance & Optimization")
        )
        .child(Divider::horizontal())
        .child(
            h_flex()
                .gap_6()
                .child(
                    v_flex()
                        .flex_1()
                        .gap_2()
                        .p_4()
                        .border_1()
                        .border_color(theme.border)
                        .rounded_lg()
                        .bg(theme.sidebar)
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Repository Health")
                        )
                        .child(
                            div()
                                .text_3xl()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(health_color)
                                .child(format!("{:.0}%", health_score))
                        )
                        .child(
                            div()
                                .w_full()
                                .h(px(6.))
                                .bg(theme.border)
                                .rounded_full()
                                .child(
                                    div()
                                        .w(relative(health_score / 100.0))
                                        .h_full()
                                        .bg(health_color)
                                        .rounded_full()
                                )
                        )
                )
                .child(
                    v_flex()
                        .flex_1()
                        .gap_2()
                        .p_4()
                        .border_1()
                        .border_color(theme.border)
                        .rounded_lg()
                        .bg(theme.sidebar)
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Total Commits")
                        )
                        .child(
                            div()
                                .text_3xl()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(theme.primary)
                                .child(settings.commit_count.map(|c| c.to_string()).unwrap_or_else(|| "0".to_string()))
                        )
                )
                .child(
                    v_flex()
                        .flex_1()
                        .gap_2()
                        .p_4()
                        .border_1()
                        .border_color(theme.border)
                        .rounded_lg()
                        .bg(theme.sidebar)
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Disk Usage")
                        )
                        .child(
                            div()
                                .text_3xl()
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(theme.accent)
                                .child(format_size(Some(project_size)))
                        )
                )
        )
        .child(render_info_section("Repository Statistics", vec![
            ("Total Commits", settings.commit_count.map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string())),
            ("Total Branches", settings.branch_count.map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string())),
            ("Git Repository Size", format_size(Some(git_size))),
            ("Git Size Ratio", if project_size > 0 {
                format!("{:.1}%", (git_size as f64 / project_size as f64) * 100.0)
            } else {
                "N/A".to_string()
            }),
        ], &theme))
        .child(
            v_flex()
                .gap_2()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .mb_2()
                        .child("Optimization Recommendations")
                )
                .children(generate_optimization_recommendations(settings, &theme))
        )
        .child(
            v_flex()
                .gap_3()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .mb_2()
                        .child("Optimization Actions")
                )
                .child(
                    h_flex()
                        .gap_3()
                        .child(
                            Button::new("run-gc")
                                .label("Run Git GC")
                                .icon(IconName::Activity)
                                .with_variant(gpui_component::button::ButtonVariant::Primary)
                                .on_click({
                                    let path = settings.project_path.clone();
                                    move |_, _, _| {
                                        let _ = std::process::Command::new("git")
                                            .args(&["gc", "--aggressive"])
                                            .current_dir(&path)
                                            .spawn();
                                    }
                                })
                        )
                        .child(
                            Button::new("prune-now")
                                .label("Prune Objects")
                                .icon(IconName::Trash)
                                .with_variant(gpui_component::button::ButtonVariant::Secondary)
                                .on_click({
                                    let path = settings.project_path.clone();
                                    move |_, _, _| {
                                        let _ = std::process::Command::new("git")
                                            .args(&["prune", "--expire=now"])
                                            .current_dir(&path)
                                            .spawn();
                                    }
                                })
                        )
                        .child(
                            Button::new("clean-untracked")
                                .label("Clean Untracked Files")
                                .icon(IconName::Trash)
                                .with_variant(gpui_component::button::ButtonVariant::Secondary)
                                .on_click({
                                    let path = settings.project_path.clone();
                                    move |_, _, _| {
                                        let _ = std::process::Command::new("git")
                                            .args(&["clean", "-fd"])
                                            .current_dir(&path)
                                            .spawn();
                                    }
                                })
                        )
                )
        )
}

fn calculate_repo_health(settings: &ProjectSettings) -> f32 {
    let mut score: f32 = 100.0;
    
    // Penalize for large git size ratio
    if let (Some(git_size), Some(project_size)) = (settings.git_repo_size, settings.disk_size) {
        if project_size > 0 {
            let ratio = (git_size as f64 / project_size as f64) * 100.0;
            if ratio > 50.0 {
                score -= 20.0; // Large git size
            } else if ratio > 30.0 {
                score -= 10.0;
            }
        }
    }
    
    // Penalize for uncommitted changes
    if let Some(changes) = settings.uncommitted_changes {
        if changes > 50 {
            score -= 20.0; // Too many uncommitted changes
        } else if changes > 20 {
            score -= 10.0;
        }
    }
    
    // Bonus for having CI/CD
    if !settings.workflow_files.is_empty() {
        score += 10.0;
    }
    
    // Bonus for recent activity
    if settings.last_commit_date.is_some() {
        score += 5.0;
    }
    
    score.max(0.0_f32).min(100.0)
}

fn generate_optimization_recommendations(settings: &ProjectSettings, theme: &gpui_component::theme::Theme) -> Vec<gpui::AnyElement> {
    let mut recommendations = Vec::new();
    
    // Check git size ratio
    if let (Some(git_size), Some(project_size)) = (settings.git_repo_size, settings.disk_size) {
        if project_size > 0 {
            let ratio = (git_size as f64 / project_size as f64) * 100.0;
            if ratio > 30.0 {
                recommendations.push(
                    render_recommendation_card(
                        "Large Git Repository",
                        &format!("Your .git folder is {:.1}% of total project size. Consider running 'git gc' to compress the repository.", ratio),
                        "high",
                        theme,
                    )
                );
            }
        }
    }
    
    // Check uncommitted changes
    if let Some(changes) = settings.uncommitted_changes {
        if changes > 20 {
            recommendations.push(
                render_recommendation_card(
                    "Many Uncommitted Changes",
                    &format!("You have {} uncommitted file(s). Consider committing or stashing your changes.", changes),
                    "medium",
                    theme,
                )
            );
        }
    }
    
    // Check for CI/CD
    if settings.workflow_files.is_empty() && settings.remote_url.is_some() {
        recommendations.push(
            render_recommendation_card(
                "No CI/CD Configuration",
                "Consider adding GitHub Actions workflows to automate builds and tests.",
                "low",
                theme,
            )
        );
    }
    
    // General recommendations
    recommendations.push(
        render_recommendation_card(
            "Use .gitignore",
            "Ensure build artifacts and dependencies are excluded from version control.",
            "info",
            theme,
        )
    );
    
    recommendations.push(
        render_recommendation_card(
            "Consider Git LFS",
            "For large binary assets (textures, models), use Git Large File Storage to reduce repository size.",
            "info",
            theme,
        )
    );
    
    if recommendations.is_empty() {
        recommendations.push(
            render_recommendation_card(
                "Repository Optimized",
                "Your repository is in good shape! No major optimizations needed.",
                "success",
                theme,
            )
        );
    }
    
    recommendations
}

fn render_recommendation_card(title: &str, desc: &str, severity: &str, theme: &gpui_component::theme::Theme) -> gpui::AnyElement {
    let (bg_color, border_color, icon_color) = match severity {
        "high" => (
            hsla(0.0, 0.8, 0.6, 0.1),
            hsla(0.0, 0.8, 0.6, 0.3),
            hsla(0.0, 0.8, 0.6, 1.0),
        ),
        "medium" => (
            hsla(45.0 / 360.0, 0.8, 0.6, 0.1),
            hsla(45.0 / 360.0, 0.8, 0.6, 0.3),
            hsla(45.0 / 360.0, 0.8, 0.6, 1.0),
        ),
        "low" | "info" => (
            theme.accent.opacity(0.1),
            theme.accent.opacity(0.3),
            theme.accent,
        ),
        "success" => (
            theme.primary.opacity(0.1),
            theme.primary.opacity(0.3),
            theme.primary,
        ),
        _ => (
            theme.muted.opacity(0.1),
            theme.muted.opacity(0.3),
            theme.muted_foreground,
        ),
    };
    
    h_flex()
        .gap_3()
        .p_3()
        .rounded_lg()
        .bg(bg_color)
        .border_1()
        .border_color(border_color)
        .child(
            Icon::new(IconName::Activity)
                .size(px(20.))
                .text_color(icon_color)
        )
        .child(
            v_flex()
                .flex_1()
                .gap_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child(title.to_string())
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(theme.muted_foreground)
                        .child(desc.to_string())
                )
        )
        .into_any_element()
}

fn render_integrations_tab(settings: &ProjectSettings, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    let theme = cx.theme();
    let remote_url = settings.remote_url.clone();
    let project_path = settings.project_path.clone();
    
    v_flex()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme.foreground)
                .child("Editor & Tool Integrations")
        )
        .child(Divider::horizontal())
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child("Connect your project with external tools and code editors")
        )
        .child(
            v_flex()
                .gap_3()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .mb_2()
                        .child("Code Editors")
                )
                .child(render_editor_integration_card(
                    "Visual Studio Code",
                    "Open project in VS Code",
                    IconName::Code,
                    "code",
                    &project_path,
                    &theme,
                ))
                .child(render_editor_integration_card(
                    "Visual Studio",
                    "Open project in Visual Studio",
                    IconName::Code,
                    "devenv",
                    &project_path,
                    &theme,
                ))
                .child(render_editor_integration_card(
                    "Sublime Text",
                    "Open project in Sublime Text",
                    IconName::Code,
                    "subl",
                    &project_path,
                    &theme,
                ))
                .child(render_editor_integration_card(
                    "Vim / Neovim",
                    "Open project in terminal editor",
                    IconName::Terminal,
                    "vim",
                    &project_path,
                    &theme,
                ))
        )
        .child(
            v_flex()
                .gap_3()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .mb_2()
                        .child("Version Control")
                )
                .child(render_tool_integration_card(
                    "GitHub Desktop",
                    "Open in GitHub Desktop application",
                    IconName::GitHub,
                    settings.remote_url.is_some(),
                    {
                        let url = remote_url.clone();
                        let path = project_path.clone();
                        move |_, _, _| {
                            if let Some(remote) = &url {
                                // GitHub Desktop protocol
                                let repo_url = remote.trim_end_matches(".git");
                                let _ = open::that(format!("x-github-client://openRepo/{}?branch=main", repo_url));
                            } else {
                                let _ = open::that(&path);
                            }
                        }
                    },
                    &theme,
                ))
                .child(render_tool_integration_card(
                    "GitKraken",
                    "Open repository in GitKraken",
                    IconName::GitHub,
                    true,
                    {
                        let path = project_path.clone();
                        move |_, _, _| {
                            let _ = std::process::Command::new("gitkraken")
                                .args(&["-p", path.to_str().unwrap_or("")])
                                .spawn();
                        }
                    },
                    &theme,
                ))
                .child(render_tool_integration_card(
                    "SourceTree",
                    "Open repository in SourceTree",
                    IconName::GitHub,
                    true,
                    {
                        let path = project_path.clone();
                        move |_, _, _| {
                            let _ = open::that(format!("sourcetree://cloneRepo?type=local&url={}", path.to_str().unwrap_or("")));
                        }
                    },
                    &theme,
                ))
                .child(render_tool_integration_card(
                    "Git GUI",
                    "Launch built-in Git graphical interface",
                    IconName::GitHub,
                    true,
                    {
                        let path = project_path.clone();
                        move |_, _, _| {
                            let _ = std::process::Command::new("git")
                                .args(&["gui"])
                                .current_dir(&path)
                                .spawn();
                        }
                    },
                    &theme,
                ))
        )
        .child(
            v_flex()
                .gap_3()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .mb_2()
                        .child("System Tools")
                )
                .child(render_tool_integration_card(
                    "File Manager",
                    "Open project folder in system file manager",
                    IconName::FolderOpen,
                    true,
                    {
                        let path = project_path.clone();
                        move |_, _, _| {
                            let _ = open::that(&path);
                        }
                    },
                    &theme,
                ))
                .child(render_tool_integration_card(
                    "Terminal",
                    "Open project in system terminal",
                    IconName::Terminal,
                    true,
                    {
                        let path = project_path.clone();
                        move |_, _, _| {
                            #[cfg(windows)]
                            {
                                let _ = std::process::Command::new("cmd")
                                    .args(&["/c", "start", "cmd", "/k", "cd", path.to_str().unwrap_or("")])
                                    .spawn();
                            }
                            #[cfg(target_os = "macos")]
                            {
                                let _ = std::process::Command::new("open")
                                    .args(&["-a", "Terminal", path.to_str().unwrap_or("")])
                                    .spawn();
                            }
                            #[cfg(target_os = "linux")]
                            {
                                let _ = std::process::Command::new("gnome-terminal")
                                    .args(&["--working-directory", path.to_str().unwrap_or("")])
                                    .spawn();
                            }
                        }
                    },
                    &theme,
                ))
        )
        .child(
            div()
                .p_4()
                .rounded_lg()
                .bg(theme.accent.opacity(0.1))
                .border_1()
                .border_color(theme.accent.opacity(0.3))
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(theme.accent)
                                .child("💡 Integration Tips")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("• Install tools to enable integrations")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("• Some integrations require tool-specific setup")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("• Click any card to launch the tool with your project")
                        )
                )
        )
}

fn render_editor_integration_card(
    name: &str,
    desc: &str,
    icon: IconName,
    command: &str,
    project_path: &std::path::PathBuf,
    theme: &gpui_component::theme::Theme,
) -> impl IntoElement {
    let cmd = command.to_string();
    let path = project_path.clone();
    
    h_flex()
        .p_4()
        .gap_3()
        .border_1()
        .border_color(theme.border)
        .rounded_lg()
        .bg(theme.sidebar)
        .hover(|this| this.bg(theme.muted.opacity(0.1)).border_color(theme.primary))
        .cursor_pointer()
        .on_mouse_down(gpui::MouseButton::Left, move |_, _, _| {
            let _ = std::process::Command::new(&cmd)
                .arg(path.to_str().unwrap_or(""))
                .spawn();
        })
        .child(
            Icon::new(icon)
                .size(px(24.))
                .text_color(theme.primary)
        )
        .child(
            v_flex()
                .flex_1()
                .gap_1()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child(name.to_string())
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(desc.to_string())
                )
        )
        .child(
            Icon::new(IconName::ArrowUp)
                .size(px(16.))
                .text_color(theme.muted_foreground)
        )
}

fn render_tool_integration_card<F>(
    name: &str,
    desc: &str,
    icon: IconName,
    available: bool,
    on_click: F,
    theme: &gpui_component::theme::Theme,
) -> impl IntoElement
where
    F: Fn(&gpui::MouseDownEvent, &mut Window, &mut App) + 'static,
{
    h_flex()
        .p_4()
        .gap_3()
        .border_1()
        .border_color(theme.border)
        .rounded_lg()
        .bg(theme.sidebar)
        .hover(|this| {
            if available {
                this.bg(theme.muted.opacity(0.1)).border_color(theme.primary)
            } else {
                this
            }
        })
        .when(available, |this| this.cursor_pointer().on_mouse_down(gpui::MouseButton::Left, on_click))
        .child(
            Icon::new(icon)
                .size(px(24.))
                .text_color(if available { theme.primary } else { theme.muted_foreground })
        )
        .child(
            v_flex()
                .flex_1()
                .gap_1()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(if available { theme.foreground } else { theme.muted_foreground })
                        .child(name.to_string())
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(desc.to_string())
                )
        )
        .child(
            div()
                .px_2()
                .py_1()
                .rounded_md()
                .bg(if available { theme.accent.opacity(0.1) } else { theme.muted_foreground.opacity(0.1) })
                .text_xs()
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(if available { theme.accent } else { theme.muted_foreground })
                .child(if available { "Ready" } else { "N/A" })
        )
}

fn render_info_section(title: &str, items: Vec<(&str, String)>, theme: &gpui_component::theme::Theme) -> impl IntoElement {
    v_flex()
        .gap_2()
        .child(
            div()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.foreground)
                .child(title.to_string())
        )
        .child(
            v_flex()
                .gap_2()
                .p_4()
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .bg(theme.sidebar)
                .children(items.into_iter().map(|(key, value)| {
                    h_flex()
                        .justify_between()
                        .gap_4()
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child(key.to_string())
                        )
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::MEDIUM)
                                .text_color(theme.foreground)
                                .child(value)
                        )
                }))
        )
}

fn render_size_bar(label: &str, size: u64, total: u64, color: Hsla, theme: &gpui_component::theme::Theme) -> impl IntoElement {
    let percentage = if total > 0 {
        ((size as f64 / total as f64) * 100.0) as f32
    } else {
        0.0
    };
    
    v_flex()
        .gap_1()
        .child(
            h_flex()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.foreground)
                        .child(label.to_string())
                )
                .child(
                    div()
                        .text_sm()
                        .text_color(theme.muted_foreground)
                        .child(format!("{} ({:.1}%)", format_size(Some(size)), percentage))
                )
        )
        .child(
            div()
                .w_full()
                .h(px(8.))
                .bg(theme.border)
                .rounded_full()
                .child(
                    div()
                        .w(relative(percentage / 100.0))
                        .h_full()
                        .bg(color)
                        .rounded_full()
                )
        )
}

fn format_size(size: Option<u64>) -> String {
    match size {
        Some(bytes) => {
            const KB: u64 = 1024;
            const MB: u64 = KB * 1024;
            const GB: u64 = MB * 1024;
            
            if bytes >= GB {
                format!("{:.2} GB", bytes as f64 / GB as f64)
            } else if bytes >= MB {
                format!("{:.2} MB", bytes as f64 / MB as f64)
            } else if bytes >= KB {
                format!("{:.2} KB", bytes as f64 / KB as f64)
            } else {
                format!("{} bytes", bytes)
            }
        }
        None => "N/A".to_string(),
    }
}
