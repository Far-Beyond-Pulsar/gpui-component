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
                .w(px(1100.))
                .h(px(700.))
                .bg(theme.background)
                .rounded_xl()
                .border_1()
                .border_color(theme.border)
                .shadow_lg()
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| {
                    // Stop propagation to prevent closing modal when clicking inside content
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
    
    v_flex()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme.foreground)
                .child("Project Metadata")
        )
        .child(Divider::horizontal())
        .child(render_info_section("Configuration", vec![
            ("Pulsar.toml", if has_config { "Present" } else { "Missing" }.to_string()),
            ("Config Path", config_path.to_string_lossy().to_string()),
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
                                .child("Project Metadata Fields")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("• Project name, version, and description")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("• Author information and license")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("• Engine version and dependencies")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("• Build settings and target platforms")
                        )
                )
        )
        .child(
            v_flex()
                .gap_3()
                .child(
                    Button::new("edit-config")
                        .label("Edit Pulsar.toml")
                        .icon(IconName::Folder)
                        .with_variant(gpui_component::button::ButtonVariant::Primary)
                        .on_click({
                            let path = config_path.clone();
                            move |_, _, _| {
                                let _ = open::that(&path);
                            }
                        })
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
        .child(render_info_section("Project Stats", vec![
            ("Total Commits", settings.commit_count.map(|c| c.to_string()).unwrap_or_else(|| "N/A".to_string())),
            ("Repository Age", "Calculate from first commit".to_string()),
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
                                .child("Optimization Recommendations")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("• Run 'git gc' to compress repository data")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("• Use .gitignore to exclude build artifacts")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.muted_foreground)
                                .child("• Consider Git LFS for large binary assets")
                        )
                )
        )
        .child(
            v_flex()
                .gap_3()
                .child(
                    Button::new("run-gc")
                        .label("Optimize Repository")
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
        )
}

fn render_integrations_tab(settings: &ProjectSettings, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    let theme = cx.theme();
    
    v_flex()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme.foreground)
                .child("Integrations")
        )
        .child(Divider::horizontal())
        .child(
            div()
                .text_sm()
                .text_color(theme.muted_foreground)
                .child("Connect your project with external tools and services")
        )
        .child(render_integration_card("GitHub", "View repository, issues, and pull requests", IconName::GitHub, settings.remote_url.is_some(), cx))
        .child(render_integration_card("VS Code", "Open project in Visual Studio Code", IconName::Code, true, cx))
        .child(render_integration_card("Git GUI", "Launch Git graphical interface", IconName::GitHub, true, cx))
        .child(render_integration_card("Terminal", "Open in system terminal", IconName::Terminal, true, cx))
}

fn render_integration_card(name: &str, desc: &str, icon: IconName, available: bool, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    let theme = cx.theme();
    
    h_flex()
        .p_4()
        .gap_3()
        .border_1()
        .border_color(theme.border)
        .rounded_lg()
        .bg(theme.sidebar)
        .hover(|this| this.bg(theme.muted.opacity(0.1)))
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
            div()
                .px_2()
                .py_1()
                .rounded_md()
                .bg(if available { theme.accent.opacity(0.1) } else { theme.muted_foreground.opacity(0.1) })
                .text_xs()
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(if available { theme.accent } else { theme.muted_foreground })
                .child(if available { "Available" } else { "N/A" })
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
