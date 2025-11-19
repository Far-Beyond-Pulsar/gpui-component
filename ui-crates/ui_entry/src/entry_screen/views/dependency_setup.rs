use gpui::*;
use gpui::prelude::*;
use ui::{
    button::{Button, ButtonVariants},
    h_flex, v_flex, ActiveTheme, Icon, IconName,
};
use crate::entry_screen::{EntryScreen, InstallProgress, InstallStatus};
use std::process::Command;
use std::sync::{Arc, Mutex};

#[cfg(target_os = "windows")]
const RUSTUP_URL: &str = "https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe";

#[cfg(any(target_os = "linux", target_os = "macos"))]
const RUSTUP_URL: &str = "https://sh.rustup.rs";

pub fn render_dependency_setup(screen: &mut EntryScreen, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    let theme = cx.theme();
    
    // Use cached dependency status (checked on background thread)
    let (rust_installed, build_tools_installed) = if let Some(ref status) = screen.dependency_status {
        (status.rust_installed, status.build_tools_installed)
    } else {
        // Still checking in background, show as not installed
        (false, false)
    };
    
    div()
        .absolute()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(gpui::black().opacity(0.7))
        .child(
            v_flex()
                .w(px(600.))
                .gap_6()
                .p_8()
                .bg(theme.background)
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .shadow_lg()
                // Header
                .child(
                    v_flex()
                        .gap_2()
                        .child(
                            h_flex()
                                .items_center()
                                .gap_3()
                                .child(
                                    Icon::new(IconName::Settings)
                                        .size_6()
                                        .text_color(theme.accent)
                                )
                                .child(
                                    div()
                                        .text_2xl()
                                        .font_weight(FontWeight::BOLD)
                                        .text_color(theme.foreground)
                                        .child("Development Dependencies")
                                )
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(theme.muted_foreground)
                                .child("Manage required development tools and dependencies")
                        )
                )
                // Status section
                .child(
                    v_flex()
                        .gap_3()
                        .child(render_dependency_item("Rust Toolchain".to_string(), rust_installed, theme))
                        .child(render_dependency_item("Build Tools (MSVC/GCC/Clang)".to_string(), build_tools_installed, theme))
                )
                // Progress section
                .children(screen.install_progress.clone().map(|progress| {
                    render_install_progress(progress, theme)
                }))
                // Actions
                .child(
                    v_flex()
                        .gap_3()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::BOLD)
                                .text_color(theme.foreground)
                                .child("Actions")
                        )
                        .child(
                            h_flex()
                                .gap_3()
                                .child(
                                    Button::new("install-deps")
                                        .label("Install Missing Dependencies")
                                        .primary()
                                        .when(screen.install_progress.as_ref().map(|p| {
                                            matches!(p.status, InstallStatus::Downloading | InstallStatus::Installing)
                                        }).unwrap_or(false), |btn| btn.ghost())
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            run_setup_script(this, cx);
                                            cx.notify();
                                        }))
                                )
                                .children(rust_installed.then(|| {
                                    Button::new("uninstall-rust")
                                        .label("Uninstall Rust (Testing)")
                                        .ghost()
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            uninstall_rust();
                                            this.check_dependencies_async(cx);
                                            cx.notify();
                                        }))
                                }))
                        )
                )
                // Close button
                .child(
                    h_flex()
                        .justify_end()
                        .child(
                            Button::new("close")
                                .label("Close")
                                .ghost()
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.show_dependency_setup = false;
                                    cx.notify();
                                }))
                        )
                )
        )
}

fn render_install_progress(progress: InstallProgress, theme: &ui::Theme) -> impl IntoElement {
    let (icon, color, status_text) = match &progress.status {
        InstallStatus::Idle => (IconName::Circle, theme.accent, "Ready".to_string()),
        InstallStatus::Downloading => (IconName::Download, theme.accent, "Downloading installer...".to_string()),
        InstallStatus::Installing => (IconName::Settings, theme.accent, "Installing dependencies...".to_string()),
        InstallStatus::Complete => (IconName::Check, theme.success_foreground, "Installation complete!".to_string()),
        InstallStatus::Error(e) => (IconName::WarningTriangle, gpui::red(), e.clone()),
    };
    
    let progress_val = progress.progress;
    let logs = progress.logs.clone();
    
    v_flex()
        .gap_3()
        .p_4()
        .bg(theme.secondary.opacity(0.2))
        .rounded_lg()
        .border_1()
        .border_color(theme.border)
        .child(
            h_flex()
                .items_center()
                .gap_2()
                .child(
                    Icon::new(icon)
                        .size_4()
                        .text_color(color)
                )
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.foreground)
                        .child(status_text)
                )
        )
        // Progress bar
        .child(
            div()
                .w_full()
                .h(px(8.))
                .bg(theme.secondary.opacity(0.3))
                .rounded_sm()
                .child(
                    div()
                        .h_full()
                        .rounded_sm()
                        .bg(if matches!(progress.status, InstallStatus::Error(_)) {
                            gpui::red()
                        } else {
                            theme.accent
                        })

                        .w(relative(progress.progress.max(0.0).min(1.0)))
                )
        )
        // Log output
        .child(
            div()
                .w_full()
                .max_h(px(200.))
                .p_2()
                .bg(gpui::black().opacity(0.3))
                .rounded_sm()
                .text_xs()
                .children(
                    progress.logs.iter().rev().take(20).rev().map(|log| {
                        div()
                            .text_color(theme.muted_foreground)
                            .child(log.clone())
                    })
                )
        )
}

fn render_dependency_item(name: String, installed: bool, theme: &ui::Theme) -> impl IntoElement {
    let (icon, color, status_text) = if installed {
        (IconName::Check, theme.success_foreground, "Installed")
    } else {
        (IconName::WarningTriangle, gpui::yellow(), "Missing")
    };
    
    h_flex()
        .gap_3()
        .items_center()
        .p_3()
        .bg(theme.secondary.opacity(0.3))
        .rounded_md()
        .child(
            Icon::new(icon)
                .size_5()
                .text_color(color)
        )
        .child(
            v_flex()
                .flex_1()
                .child(
                    div()
                        .text_sm()
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(theme.foreground)
                        .child(name)
                )
        )
        .child(
            div()
                .text_xs()
                .font_weight(FontWeight::MEDIUM)
                .text_color(color)
                .child(status_text)
        )
}

fn run_setup_script(screen: &mut EntryScreen, cx: &mut Context<EntryScreen>) {
    // Initialize progress
    screen.install_progress = Some(InstallProgress {
        logs: vec!["Starting installation...".to_string()],
        progress: 0.0,
        status: InstallStatus::Downloading,
    });
    
    let progress = Arc::new(Mutex::new(screen.install_progress.clone().unwrap()));
    let progress_clone = progress.clone();
    
    let progress_for_thread = Arc::clone(&progress);
    
    cx.spawn(async move |this, cx| {
        std::thread::spawn(move || {
            if let Err(e) = install_rust_with_progress(progress_clone) {
                let mut prog = progress_for_thread.lock().unwrap();
                prog.status = InstallStatus::Error(format!("Installation failed: {}", e));
                prog.logs.push(format!("Error: {}", e));
            }
        });
        
        // Poll progress updates
        loop {
            cx.background_executor().timer(std::time::Duration::from_millis(100)).await;
            
            let should_break = cx.update(|cx| {
                this.update(cx, |screen, cx| {
                    if let Ok(prog) = progress.lock() {
                        screen.install_progress = Some(prog.clone());
                        cx.notify();
                        
                        matches!(prog.status, InstallStatus::Complete | InstallStatus::Error(_))
                    } else {
                        false
                    }
                }).unwrap_or(false)
            }).unwrap_or(false);
            
            if should_break {
                // Re-check dependencies after install
                cx.update(|cx| {
                    this.update(cx, |screen, cx| {
                        screen.check_dependencies_async(cx);
                    }).ok();
                }).ok();
                break;
            }
        }
    }).detach();
}

fn install_rust_with_progress(progress: Arc<Mutex<InstallProgress>>) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(target_os = "windows")]
    {
        install_rust_windows(progress)
    }
    
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        install_rust_unix(progress)
    }
}

#[cfg(target_os = "windows")]
fn install_rust_windows(progress: Arc<Mutex<InstallProgress>>) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    
    let exe_path = std::env::temp_dir().join("rustup-init.exe");
    
    // Check if rustup already exists
    let rustup_exists = Command::new("rustup")
        .arg("--version")
        .output()
        .is_ok();
    
    if rustup_exists {
        let mut prog = progress.lock().unwrap();
        prog.logs.push("Existing Rust installation detected".to_string());
        prog.logs.push("Uninstalling old version...".to_string());
        prog.progress = 0.05;
        drop(prog);
        
        // Try to uninstall existing rustup
        let _ = Command::new("rustup")
            .args(&["self", "uninstall", "-y"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output();
        
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    
    // Update progress: Downloading
    {
        let mut prog = progress.lock().unwrap();
        prog.logs.push("Downloading rustup installer...".to_string());
        prog.progress = 0.1;
        prog.status = InstallStatus::Downloading;
    }
    
    // Download using reqwest
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;
    
    let response = client.get(RUSTUP_URL).send()?;
    let bytes = response.bytes()?;
    
    {
        let mut prog = progress.lock().unwrap();
        prog.logs.push(format!("Downloaded {} bytes", bytes.len()));
        prog.progress = 0.3;
    }
    
    // Write to temp file
    let mut file = std::fs::File::create(&exe_path)?;
    file.write_all(&bytes)?;
    file.flush()?;
    drop(file);
    
    {
        let mut prog = progress.lock().unwrap();
        prog.logs.push("Running rustup installer with elevated privileges...".to_string());
        prog.logs.push("Please accept the UAC prompt if it appears".to_string());
        prog.progress = 0.4;
        prog.status = InstallStatus::Installing;
    }
    
    // Run installer with admin privileges using runas verb
    let status = runas::Command::new(&exe_path)
        .args(&["-y", "--default-toolchain", "stable", "--profile", "default"])
        .show(false) // Don't show console window
        .status()?;
    
    if status.success() {
        let mut prog = progress.lock().unwrap();
        prog.logs.push("✅ Rust installed successfully!".to_string());
        prog.progress = 1.0;
        prog.status = InstallStatus::Complete;
    } else {
        return Err(format!("Rustup installer exited with status: {:?}", status).into());
    }
    
    // Clean up
    let _ = std::fs::remove_file(&exe_path);
    
    Ok(())
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn install_rust_unix(progress: Arc<Mutex<InstallProgress>>) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::Write;
    use std::os::unix::fs::PermissionsExt;
    
    let script_path = std::env::temp_dir().join("rustup-init.sh");
    
    // Check if rustup already exists
    let rustup_exists = Command::new("rustup")
        .arg("--version")
        .output()
        .is_ok();
    
    if rustup_exists {
        let mut prog = progress.lock().unwrap();
        prog.logs.push("Existing Rust installation detected".to_string());
        prog.logs.push("Uninstalling old version...".to_string());
        prog.progress = 0.05;
        drop(prog);
        
        // Try to uninstall existing rustup
        let _ = Command::new("rustup")
            .args(&["self", "uninstall", "-y"])
            .output();
        
        std::thread::sleep(std::time::Duration::from_secs(2));
    }
    
    // Update progress: Downloading
    {
        let mut prog = progress.lock().unwrap();
        prog.logs.push("Downloading rustup installer...".to_string());
        prog.progress = 0.1;
        prog.status = InstallStatus::Downloading;
    }
    
    // Download using reqwest
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(300))
        .build()?;
    
    let response = client.get(RUSTUP_URL).send()?;
    let bytes = response.bytes()?;
    
    {
        let mut prog = progress.lock().unwrap();
        prog.logs.push(format!("Downloaded {} bytes", bytes.len()));
        prog.progress = 0.3;
    }
    
    // Write to temp file
    let mut file = std::fs::File::create(&script_path)?;
    file.write_all(&bytes)?;
    file.flush()?;
    drop(file);
    
    // Make executable
    let mut perms = std::fs::metadata(&script_path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&script_path, perms)?;
    
    {
        let mut prog = progress.lock().unwrap();
        prog.logs.push("Running rustup installer...".to_string());
        prog.logs.push("May require sudo password".to_string());
        prog.progress = 0.4;
        prog.status = InstallStatus::Installing;
    }
    
    // Run installer silently with -y
    let status = Command::new("sh")
        .args(&[script_path.to_str().unwrap(), "-y", "--default-toolchain", "stable", "--profile", "default"])
        .status()?;
    
    if status.success() {
        let mut prog = progress.lock().unwrap();
        prog.logs.push("✅ Rust installed successfully!".to_string());
        prog.progress = 1.0;
        prog.status = InstallStatus::Complete;
    } else {
        return Err(format!("Rustup installer exited with status: {:?}", status).into());
    }
    
    // Clean up
    let _ = std::fs::remove_file(&script_path);
    
    Ok(())
}

fn uninstall_rust() {
    #[cfg(target_os = "windows")]
    {
        // Run rustup self uninstall
        let _ = Command::new("rustup")
            .args(&["self", "uninstall", "-y"])
            .spawn();
    }
    
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        // Run rustup self uninstall
        let _ = Command::new("rustup")
            .args(&["self", "uninstall", "-y"])
            .spawn();
    }
}
