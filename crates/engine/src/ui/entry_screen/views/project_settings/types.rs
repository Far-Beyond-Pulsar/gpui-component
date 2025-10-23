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
    pub current_branch: Option<String>,
    pub stash_count: Option<usize>,
    pub untracked_files: Option<usize>,
    pub preferred_editor: Option<String>,
    pub preferred_git_tool: Option<String>,
    // Cached tool lists to avoid re-detecting on every render
    pub available_tools_cache: Option<AvailableTools>,
}

#[derive(Clone, Debug)]
pub struct AvailableTools {
    pub editors: Vec<ToolInfo>,
    pub git_tools: Vec<ToolInfo>,
    pub terminals: Vec<ToolInfo>,
}

#[derive(Clone, Debug)]
pub struct ToolInfo {
    pub name: String,
    pub command: String,
    pub available: bool,
}

impl ToolInfo {
    pub fn new(name: &str, command: &str) -> Self {
        let available = check_tool_available(command);
        Self {
            name: name.to_string(),
            command: command.to_string(),
            available,
        }
    }
}

fn check_tool_available(command: &str) -> bool {
    #[cfg(windows)]
    {
        std::process::Command::new("where")
            .arg(command)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(not(windows))]
    {
        std::process::Command::new("which")
            .arg(command)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
}

impl AvailableTools {
    pub fn detect() -> Self {
        Self {
            editors: vec![
                ToolInfo::new("Visual Studio Code", "code"),
                ToolInfo::new("Visual Studio", "devenv"),
                ToolInfo::new("Sublime Text", "subl"),
                ToolInfo::new("Vim", "vim"),
                ToolInfo::new("Neovim", "nvim"),
                ToolInfo::new("Emacs", "emacs"),
                ToolInfo::new("IntelliJ IDEA", "idea"),
                ToolInfo::new("CLion", "clion"),
                ToolInfo::new("Notepad++", "notepad++"),
            ],
            git_tools: vec![
                ToolInfo::new("Git GUI", "git"),
                ToolInfo::new("GitHub Desktop", "github"),
                ToolInfo::new("GitKraken", "gitkraken"),
                ToolInfo::new("SourceTree", "sourcetree"),
                ToolInfo::new("Git Cola", "git-cola"),
                ToolInfo::new("Lazygit", "lazygit"),
                ToolInfo::new("Magit (Emacs)", "emacs"),
            ],
            terminals: vec![
                ToolInfo::new("Windows Terminal", "wt"),
                ToolInfo::new("PowerShell", "pwsh"),
                ToolInfo::new("Command Prompt", "cmd"),
                ToolInfo::new("Git Bash", "bash"),
                ToolInfo::new("Alacritty", "alacritty"),
                ToolInfo::new("Kitty", "kitty"),
            ],
        }
    }
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
            current_branch: None,
            stash_count: None,
            untracked_files: None,
            preferred_editor: None,
            preferred_git_tool: None,
            available_tools_cache: None,
        }
    }

    pub fn load_all_data(&mut self) {
        self.load_disk_info();
        self.load_git_info();
        self.load_git_ci_info();
        self.load_tool_preferences();
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

        // Get current branch
        if let Ok(output) = std::process::Command::new("git")
            .current_dir(&self.project_path)
            .args(&["branch", "--show-current"])
            .output()
        {
            if let Ok(branch) = String::from_utf8(output.stdout) {
                self.current_branch = Some(branch.trim().to_string());
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

        // Get uncommitted changes and untracked files
        if let Ok(output) = std::process::Command::new("git")
            .current_dir(&self.project_path)
            .args(&["status", "--porcelain"])
            .output()
        {
            if let Ok(status) = String::from_utf8(output.stdout) {
                let lines: Vec<&str> = status.lines().collect();
                let modified_count = lines.iter().filter(|l| !l.starts_with("??")).count();
                let untracked_count = lines.iter().filter(|l| l.starts_with("??")).count();
                self.uncommitted_changes = Some(modified_count);
                self.untracked_files = Some(untracked_count);
            }
        }

        // Get stash count
        if let Ok(output) = std::process::Command::new("git")
            .current_dir(&self.project_path)
            .args(&["stash", "list"])
            .output()
        {
            if let Ok(stash_list) = String::from_utf8(output.stdout) {
                self.stash_count = Some(stash_list.lines().count());
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
    
    fn load_tool_preferences(&mut self) {
        let config_path = self.project_path.join("Pulsar.toml");
        if !config_path.exists() {
            return;
        }
        
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            if let Ok(parsed) = toml::from_str::<toml::Value>(&content) {
                if let Some(tools_table) = parsed.get("tools").and_then(|v| v.as_table()) {
                    self.preferred_editor = tools_table.get("editor")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    
                    self.preferred_git_tool = tools_table.get("git_tool")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                }
            }
        }
    }
}

pub fn format_size(size: Option<u64>) -> String {
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

/// Load tool preferences for a project without loading full settings
pub fn load_project_tool_preferences(project_path: &std::path::PathBuf) -> (Option<String>, Option<String>) {
    let config_path = project_path.join("Pulsar.toml");
    if !config_path.exists() {
        return (None, None);
    }
    
    if let Ok(content) = std::fs::read_to_string(&config_path) {
        if let Ok(parsed) = toml::from_str::<toml::Value>(&content) {
            if let Some(tools_table) = parsed.get("tools").and_then(|v| v.as_table()) {
                let editor = tools_table.get("editor")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                let git_tool = tools_table.get("git_tool")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                
                return (editor, git_tool);
            }
        }
    }
    
    (None, None)
}
