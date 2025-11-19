use gpui::*;
use ui::{
    button::{Button, ButtonVariants},
    h_flex, v_flex, ActiveTheme, Icon, IconName,
};
use crate::entry_screen::EntryScreen;
use std::process::Command;

pub fn render_dependency_setup(screen: &mut EntryScreen, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    let theme = cx.theme();
    
    // Check current dependency status
    let rust_installed = check_rust_installed();
    let build_tools_installed = check_build_tools_installed();
    
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
                                        .on_click(cx.listener(|_this, _, _, cx| {
                                            run_setup_script();
                                            cx.notify();
                                        }))
                                )
                                .children(rust_installed.then(|| {
                                    Button::new("uninstall-rust")
                                        .label("Uninstall Rust (Testing)")
                                        .ghost()
                                        .on_click(cx.listener(|_this, _, _, cx| {
                                            uninstall_rust();
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

fn check_rust_installed() -> bool {
    Command::new("rustc")
        .arg("--version")
        .output()
        .is_ok()
}

fn check_build_tools_installed() -> bool {
    #[cfg(target_os = "windows")]
    {
        Command::new("cl")
            .arg("/?")
            .output()
            .is_ok()
    }
    
    #[cfg(target_os = "linux")]
    {
        Command::new("gcc")
            .arg("--version")
            .output()
            .is_ok()
    }
    
    #[cfg(target_os = "macos")]
    {
        Command::new("clang")
            .arg("--version")
            .output()
            .is_ok()
    }
}

fn run_setup_script() {
    #[cfg(target_os = "windows")]
    {
        let script_path = std::path::PathBuf::from("script/setup-dev-environment.ps1");
        if script_path.exists() {
            let _ = Command::new("powershell")
                .arg("-ExecutionPolicy")
                .arg("Bypass")
                .arg("-File")
                .arg(&script_path)
                .spawn();
        }
    }
    
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        let script_path = std::path::PathBuf::from("script/setup-dev-environment.sh");
        if script_path.exists() {
            let _ = Command::new("bash")
                .arg(&script_path)
                .spawn();
        }
    }
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
