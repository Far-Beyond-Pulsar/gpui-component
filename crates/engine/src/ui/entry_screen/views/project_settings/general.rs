use gpui::{prelude::*, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    v_flex, divider::Divider, ActiveTheme as _, StyledExt, IconName,
};
use super::{types::ProjectSettings, helpers::render_info_section};
use crate::ui::entry_screen::EntryScreen;

pub fn render_general_tab(settings: &ProjectSettings, cx: &mut Context<EntryScreen>) -> impl IntoElement {
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
        .child(
            v_flex()
                .gap_2()
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child("Project Actions")
                )
                .child(
                    v_flex()
                        .gap_3()
                        .child(
                            Button::new("open-in-explorer")
                                .label("Open in File Manager")
                                .icon(IconName::FolderOpen)
                                .w_full()
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
                                .w_full()
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
                                })
                        )
                        .child(
                            Button::new("copy-path")
                                .label("Copy Project Path")
                                .icon(IconName::Copy)
                                .w_full()
                                .with_variant(gpui_component::button::ButtonVariant::Secondary)
                                .on_click({
                                    let path = settings.project_path.to_string_lossy().to_string();
                                    move |_, _, cx| {
                                        cx.write_to_clipboard(gpui::ClipboardItem::new_string(path.clone()));
                                    }
                                })
                        )
                )
        )
}
