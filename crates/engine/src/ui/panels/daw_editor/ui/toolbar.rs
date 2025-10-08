/// DAW Toolbar Component
/// Top toolbar with file operations, edit tools, and view options

use super::state::*;
use super::panel::DawPanel;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::*, h_flex, v_flex, Icon, IconName, Sizable, StyledExt, ActiveTheme, Disableable,
    Selectable, divider::Divider, tooltip::Tooltip,
};

pub fn render_toolbar(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    h_flex()
        .w_full()
        .h(px(48.0))
        .px_3()
        .gap_2()
        .items_center()
        .bg(cx.theme().muted)
        .border_b_1()
        .border_color(cx.theme().border)
        // File operations
        .child(render_file_section(state, cx))
        .child(Divider::vertical().h(px(24.0)))
        // Edit tools
        .child(render_tools_section(state, cx))
        .child(Divider::vertical().h(px(24.0)))
        // View options
        .child(render_view_section(state, cx))
        .child(Divider::vertical().h(px(24.0)))
        // Snap settings
        .child(render_snap_section(state, cx))
        // Spacer
        .child(div().flex_1())
        // Project info
        .child(render_project_info(state, cx))
}

fn render_file_section(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    h_flex()
        .gap_1()
        .items_center()
        .child(
            Button::new("toolbar-new")
                .icon(Icon::new(IconName::Plus))
                .ghost()
                .small()
                .tooltip(move |_window, _cx| {
                    Tooltip::new("New Project")
                })
                .on_click(cx.listener(|this, _, window, cx| {
                    handle_new_project(&mut this.state, window, cx);
                }))
        )
        .child(
            Button::new("toolbar-open")
                .icon(Icon::new(IconName::FolderOpen))
                .ghost()
                .small()
                .tooltip(move |_window, _cx| {
                    Tooltip::new("Open Project")
                })
                .on_click(cx.listener(|this, _, window, cx| {
                    handle_open_project(&mut this.state, window, cx);
                }))
        )
        .child(
            Button::new("toolbar-save")
                .icon(Icon::new(IconName::Save))
                .ghost()
                .small()
                .disabled(state.project.is_none())
                .tooltip(move |_window, _cx| {
                    Tooltip::new("Save Project")
                })
                .on_click(cx.listener(|this, _, _window, cx| {
                    if let Err(e) = this.state.save_project() {
                        eprintln!("❌ Save failed: {}", e);
                    } else {
                        eprintln!("✅ Project saved");
                    }
                    cx.notify();
                }))
        )
        .child(
            Button::new("toolbar-export")
                .icon(Icon::new(IconName::Download))
                .ghost()
                .small()
                .disabled(state.project.is_none())
                .tooltip(move |_window, _cx| {
                    Tooltip::new("Export Audio")
                })
        )
}

fn render_tools_section(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    h_flex()
        .gap_1()
        .items_center()
        .child(
            Button::new("tool-select")
                .icon(Icon::new(IconName::CursorPointer))
                .ghost()
                .small()
                
                .tooltip(move |_window, _cx| {
                    Tooltip::new("Select Tool")
                })
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.state.current_tool = EditTool::Select;
                    cx.notify();
                }))
        )
        .child(
            Button::new("tool-draw")
                .icon(Icon::new(IconName::EditPencil))
                .ghost()
                .small()
                
                .tooltip(move |_window, _cx| {
                    Tooltip::new("Draw Tool")
                })
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.state.current_tool = EditTool::Draw;
                    cx.notify();
                }))
        )
        .child(
            Button::new("tool-cut")
                .icon(Icon::new(IconName::Scissor))
                .ghost()
                .small()
                
                .tooltip(move |_window, _cx| {
                    Tooltip::new("Cut Tool")
                })
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.state.current_tool = EditTool::Cut;
                    cx.notify();
                }))
        )
        .child(
            Button::new("tool-erase")
                .icon(Icon::new(IconName::Eraser))
                .ghost()
                .small()
                
                .tooltip(move |_window, _cx| {
                    Tooltip::new("Erase Tool")
                })
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.state.current_tool = EditTool::Erase;
                    cx.notify();
                }))
        )
}

fn render_view_section(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    h_flex()
        .gap_1()
        .items_center()
        .child(
            Button::new("view-arrange")
                .label("Arrange")
                .ghost()
                .small()
                
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.state.view_mode = ViewMode::Arrange;
                    cx.notify();
                }))
        )
        .child(
            Button::new("view-mix")
                .label("Mix")
                .ghost()
                .small()
                
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.state.view_mode = ViewMode::Mix;
                    cx.notify();
                }))
        )
        .child(
            Button::new("view-edit")
                .label("Edit")
                .ghost()
                .small()
                
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.state.view_mode = ViewMode::Edit;
                    cx.notify();
                }))
        )
}

fn render_snap_section(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    h_flex()
        .gap_1()
        .items_center()
        .child(
            Button::new("snap-toggle")
                .icon(Icon::new(IconName::Grid3x3))
                .ghost()
                .small()
                
                .tooltip(move |_window, _cx| {
                    Tooltip::new("Toggle Snap")
                })
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.state.snap_mode = match this.state.snap_mode {
                        SnapMode::Off => SnapMode::Grid,
                        _ => SnapMode::Off,
                    };
                    cx.notify();
                }))
        )
        .child(
            Button::new("snap-value")
                .label(state.snap_value.label())
                .ghost()
                .small()
                .tooltip(move |_window, _cx| {
                    Tooltip::new("Snap Value")
                })
                .on_click(cx.listener(|this, _, _window, cx| {
                    // Cycle through snap values
                    this.state.snap_value = match this.state.snap_value {
                        SnapValue::Bar => SnapValue::Half,
                        SnapValue::Half => SnapValue::Quarter,
                        SnapValue::Quarter => SnapValue::Eighth,
                        SnapValue::Eighth => SnapValue::Sixteenth,
                        SnapValue::Sixteenth => SnapValue::ThirtySecond,
                        SnapValue::ThirtySecond => SnapValue::Bar,
                    };
                    cx.notify();
                }))
        )
}

fn render_project_info(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let project_name = state.project.as_ref()
        .map(|p| p.name.clone())
        .unwrap_or_else(|| "No Project".to_string());
    
    let track_count = state.project.as_ref()
        .map(|p| p.tracks.len())
        .unwrap_or(0);
    
    h_flex()
        .gap_3()
        .items_center()
        .child(
            div()
                .text_sm()
                .font_semibold()
                .text_color(cx.theme().foreground)
                .child(project_name)
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(format!("{} tracks", track_count))
        )
}

// Event handlers

fn handle_new_project(state: &mut DawUiState, window: &mut Window, cx: &mut Context<DawPanel>) {
    // In a real implementation, this would open a dialog
    // For now, create a default project
    use std::env;
    let default_dir = env::temp_dir().join("pulsar_projects");
    state.new_project("New Project".to_string(), default_dir);
    cx.notify();
}

fn handle_open_project(state: &mut DawUiState, window: &mut Window, cx: &mut Context<DawPanel>) {
    cx.spawn(|this, mut cx| async move {
        let file = rfd::AsyncFileDialog::new()
            .add_filter("Pulsar DAW Project", &["pdaw"])
            .pick_file()
            .await;

        if let Some(file) = file {
            let path = file.path().to_path_buf();
            cx.update(|cx| {
                this.update(cx, |this, cx| {
                    this.load_project(path, cx);
                }).ok();
            }).ok();
        }
    }).detach();
}
