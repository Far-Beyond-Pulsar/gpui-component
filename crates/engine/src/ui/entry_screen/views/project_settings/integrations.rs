use gpui::{prelude::*, *};
use gpui_component::{
    h_flex, v_flex, Icon, IconName, divider::Divider, ActiveTheme as _, StyledExt,
    button::{Button, ButtonVariants as _},
    indicator::Indicator,
};
use super::types::{ProjectSettings, ToolInfo, AvailableTools};
use crate::ui::entry_screen::EntryScreen;

pub fn render_integrations_tab(settings: &ProjectSettings, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    let theme = cx.theme();
    let project_path = settings.project_path.clone();
    let is_loading = settings.available_tools_cache.is_none();
    
    // Use cached tools if available, otherwise show loading or use default
    let tools = settings.available_tools_cache.clone().unwrap_or_else(|| {
        // Return a default with no tools - will trigger detection on first render
        AvailableTools {
            editors: vec![],
            git_tools: vec![],
            terminals: vec![],
        }
    });
    
    // If cache is empty, trigger detection asynchronously
    if is_loading {
        cx.spawn(async move |this, mut cx| {
            // Detect tools in background
            let detected = std::thread::spawn(|| AvailableTools::detect())
                .join()
                .ok();
            
            if let Some(tools) = detected {
                cx.update(|cx| {
                    this.update(cx, |screen, cx| {
                        if let Some(ref mut proj_settings) = screen.project_settings {
                            proj_settings.available_tools_cache = Some(tools);
                            cx.notify();
                        }
                    }).ok();
                }).ok();
            }
        }).detach();
    }
    
    v_flex()
        .gap_6()
        .child(
            div()
                .text_2xl()
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme.foreground)
                .child("Tool Integrations & Preferences")
        )
        .child(Divider::horizontal())
        .child(
            div()
                .text_sm()
                .text_color(theme.foreground.opacity(0.8))
                .child("Configure default tools to use with this project. Available tools are shown at the top.")
        )
        .when(is_loading, |this| {
            this.child(
                h_flex()
                    .items_center()
                    .gap_3()
                    .p_6()
                    .justify_center()
                    .border_1()
                    .border_color(theme.border)
                    .rounded_lg()
                    .bg(theme.sidebar.opacity(0.5))
                    .child(
                        div()
                            .id("tools-spinner")
                            .child(Indicator::new().color(theme.primary))
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(theme.foreground.opacity(0.7))
                            .child("Detecting available tools...")
                    )
            )
        })
        .when(!is_loading, |this| {
            this.child(render_tool_group("Code Editors", IconName::Code, tools.editors.clone(), &project_path, settings.preferred_editor.clone(), &theme))
                .child(render_tool_group("Git Tools", IconName::GitHub, tools.git_tools.clone(), &project_path, settings.preferred_git_tool.clone(), &theme))
                .child(render_tool_group("Terminals", IconName::Terminal, tools.terminals.clone(), &project_path, None, &theme))
        })
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
                                .text_color(theme.foreground.opacity(0.7))
                                .child("• Available tools (detected on your system) appear at the top")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.foreground.opacity(0.7))
                                .child("• Unavailable tools appear at the bottom and are disabled")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.foreground.opacity(0.7))
                                .child("• Set a default tool by clicking 'Set as Default' next to it")
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(theme.foreground.opacity(0.7))
                                .child("• Preferences are saved per-project in Pulsar.toml")
                        )
                )
        )
}

fn render_tool_group(
    title: &str,
    icon: IconName,
    tools: Vec<ToolInfo>,
    project_path: &std::path::PathBuf,
    current_default: Option<String>,
    theme: &gpui_component::theme::Theme,
) -> impl IntoElement {
    let mut available: Vec<_> = tools.iter().filter(|t| t.available).cloned().collect();
    let mut unavailable: Vec<_> = tools.iter().filter(|t| !t.available).cloned().collect();
    
    v_flex()
        .gap_3()
        .child(
            h_flex()
                .items_center()
                .gap_2()
                .child(
                    Icon::new(icon)
                        .size(px(20.))
                        .text_color(theme.primary)
                )
                .child(
                    div()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(theme.foreground)
                        .child(title.to_string())
                )
                .child(
                    div()
                        .px_2()
                        .py_1()
                        .rounded_md()
                        .bg(theme.primary.opacity(0.1))
                        .text_xs()
                        .text_color(theme.primary)
                        .child(format!("{} available", available.len()))
                )
        )
        .child(
            v_flex()
                .gap_2()
                .p_4()
                .border_1()
                .border_color(theme.border)
                .rounded_lg()
                .bg(theme.sidebar.opacity(0.5))
                .children(
                    available.iter().map(|tool| {
                        render_tool_item(tool, project_path, &current_default, true, theme)
                    })
                )
                .when(available.is_empty(), |this| {
                    this.child(
                        div()
                            .text_sm()
                            .text_color(theme.foreground.opacity(0.5))
                            .child("No tools detected")
                    )
                })
                .when(!unavailable.is_empty(), |this| {
                    this.child(
                        div()
                            .w_full()
                            .h(px(1.))
                            .bg(theme.border)
                            .my_2()
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(theme.foreground.opacity(0.5))
                            .mb_2()
                            .child("Not Installed")
                    )
                    .children(
                        unavailable.iter().map(|tool| {
                            render_tool_item(tool, project_path, &current_default, false, theme)
                        })
                    )
                })
        )
}

fn render_tool_item(
    tool: &ToolInfo,
    project_path: &std::path::PathBuf,
    current_default: &Option<String>,
    available: bool,
    theme: &gpui_component::theme::Theme,
) -> impl IntoElement {
    let is_default = current_default.as_ref().map(|d| d == &tool.command).unwrap_or(false);
    let tool_name = tool.name.clone();
    let tool_command = tool.command.clone();
    let path = project_path.clone();
    
    h_flex()
        .items_center()
        .justify_between()
        .px_3()
        .py_2()
        .rounded_md()
        .bg(if is_default { 
            theme.primary.opacity(0.1) 
        } else if available { 
            gpui::transparent_black() 
        } else { 
            theme.muted.opacity(0.05) 
        })
        .when(available, |this| {
            this.hover(|style| style.bg(theme.muted.opacity(0.1)))
                .cursor_pointer()
        })
        .child(
            h_flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_sm()
                        .font_weight(if is_default { gpui::FontWeight::SEMIBOLD } else { gpui::FontWeight::NORMAL })
                        .text_color(if available { 
                            if is_default { theme.primary } else { theme.foreground }
                        } else { 
                            theme.foreground.opacity(0.4)
                        })
                        .child(tool_name.clone())
                )
                .when(is_default, |this| {
                    this.child(
                        h_flex()
                            .items_center()
                            .gap_1()
                            .child(
                                Icon::new(IconName::Check)
                                    .size(px(12.))
                                    .text_color(theme.primary)
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(theme.primary.opacity(0.2))
                                    .text_xs()
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .text_color(theme.primary)
                                    .child("Default")
                            )
                    )
                })
        )
        .child(
            h_flex()
                .gap_2()
                .when(available && !is_default, |this| {
                    let cmd_clone = tool_command.clone();
                    let path_clone = path.clone();
                    this.child(
                        Button::new(SharedString::from(format!("set-default-{}", tool_command)))
                            .label("Set as Default")
                            .with_variant(gpui_component::button::ButtonVariant::Ghost)
                            .on_click(move |_, _, _| {
                                save_tool_preference(&path_clone, &cmd_clone);
                            })
                    )
                })
                .when(available, |this| {
                    let cmd_clone = tool_command.clone();
                    let path_clone = path.clone();
                    this.child(
                        Button::new(SharedString::from(format!("launch-{}", tool_command)))
                            .label("Launch")
                            .icon(IconName::ArrowUp)
                            .with_variant(gpui_component::button::ButtonVariant::Secondary)
                            .on_click(move |_, _, _| {
                                let _ = std::process::Command::new(&cmd_clone)
                                    .arg(path_clone.to_str().unwrap_or(""))
                                    .spawn();
                            })
                    )
                })
                .when(!available, |this| {
                    this.child(
                        div()
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .bg(theme.muted.opacity(0.1))
                            .text_xs()
                            .text_color(theme.foreground.opacity(0.4))
                            .child("Not Installed")
                    )
                })
        )
}

fn save_tool_preference(project_path: &std::path::PathBuf, tool_command: &str) {
    let config_path = project_path.join("Pulsar.toml");
    
    // Read existing config or create new one
    let mut config = if config_path.exists() {
        std::fs::read_to_string(&config_path)
            .ok()
            .and_then(|content| toml::from_str::<toml::Value>(&content).ok())
            .unwrap_or_else(|| toml::Value::Table(toml::map::Map::new()))
    } else {
        toml::Value::Table(toml::map::Map::new())
    };
    
    // Ensure tools table exists
    if !config.as_table().map(|t| t.contains_key("tools")).unwrap_or(false) {
        if let Some(table) = config.as_table_mut() {
            table.insert("tools".to_string(), toml::Value::Table(toml::map::Map::new()));
        }
    }
    
    // Determine if this is an editor or git tool
    let editors = ["code", "devenv", "subl", "vim", "nvim", "emacs", "idea", "clion", "notepad++"];
    let git_tools = ["git", "github", "gitkraken", "sourcetree", "git-cola", "lazygit"];
    
    let key = if editors.contains(&tool_command) {
        "editor"
    } else if git_tools.contains(&tool_command) {
        "git_tool"
    } else {
        return; // Unknown tool type
    };
    
    // Update the preference
    if let Some(table) = config.as_table_mut() {
        if let Some(tools_table) = table.get_mut("tools").and_then(|v| v.as_table_mut()) {
            tools_table.insert(key.to_string(), toml::Value::String(tool_command.to_string()));
        }
    }
    
    // Write back to file
    if let Ok(toml_string) = toml::to_string_pretty(&config) {
        let _ = std::fs::write(&config_path, toml_string);
    }
}

