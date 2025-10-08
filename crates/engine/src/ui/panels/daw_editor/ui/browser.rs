/// Browser Panel Component
/// Left sidebar with files, instruments, effects, loops, and samples

use super::state::*;
use super::panel::DawPanel;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::*, h_flex, v_flex, Icon, IconName, Sizable, StyledExt, ActiveTheme,
    Selectable, input::*, scroll::Scrollable, divider::Divider, tooltip::Tooltip,
};

const BROWSER_WIDTH: f32 = 280.0;

pub fn render_browser(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    v_flex()
        .w(px(BROWSER_WIDTH))
        .h_full()
        .bg(cx.theme().muted.opacity(0.3))
        .border_r_1()
        .border_color(cx.theme().border)
        // Tab bar
        .child(render_browser_tabs(state, cx))
        // Search bar
        .child(render_search_bar(state, cx))
        // Content area
        .child(render_browser_content(state, cx))
}

fn render_browser_tabs(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    h_flex()
        .w_full()
        .h(px(40.0))
        .px_2()
        .gap_1()
        .items_center()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(render_tab_button("Files", BrowserTab::Files, state, cx))
        .child(render_tab_button("Instruments", BrowserTab::Instruments, state, cx))
        .child(render_tab_button("Effects", BrowserTab::Effects, state, cx))
        .child(render_tab_button("Loops", BrowserTab::Loops, state, cx))
}

fn render_tab_button(
    label: &'static str,
    tab: BrowserTab,
    state: &mut DawUiState,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    Button::new(ElementId::Name(format!("browser-tab-{:?}", tab).into()))
        .label(label)
        .ghost()
        .compact()
        .small()
        
        .on_click(cx.listener(move |this, _, _window, cx| {
            this.state.browser_tab = tab;
            cx.notify();
        }))
}

fn render_search_bar(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let search_state = cx.new(|cx| {
        InputState::new(cx)
            .placeholder("Search...")
    });
    
    h_flex()
        .w_full()
        .px_3()
        .py_2()
        .border_b_1()
        .border_color(cx.theme().border)
        .child(
            Input::new(search_state.clone())
                .prefix(Icon::new(IconName::Search).size_4())
                .small()
                .cleanable()
        )
}

fn render_browser_content(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    div()
        .flex_1()
        .overflow_hidden()
        .child(
            div()
                .size_full()
                .overflow_hidden()
                .child(match state.browser_tab {
                    BrowserTab::Files => render_files_tab(state, cx).into_any_element(),
                    BrowserTab::Instruments => render_instruments_tab(state, cx).into_any_element(),
                    BrowserTab::Effects => render_effects_tab(state, cx).into_any_element(),
                    BrowserTab::Loops => render_loops_tab(state, cx).into_any_element(),
                    BrowserTab::Samples => render_samples_tab(state, cx).into_any_element(),
                })
        )
}

fn render_files_tab(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_1()
        .p_2()
        // Import button
        .child(
            Button::new("import-audio")
                .label("Import Audio")
                .icon(Icon::new(IconName::Upload))
                .w_full()
                .small()
                .on_click(cx.listener(|this, _, window, cx| {
                    handle_import_audio(&mut this.state, window, cx);
                }))
        )
        .child(Divider::horizontal().my_2())
        // File list
        .children(
            state.filtered_files.iter().filter_map(|&idx| {
                state.audio_files.get(idx).map(|file| {
                    render_audio_file_item(file, state, cx)
                })
            })
        )
        .when(state.audio_files.is_empty(), |flex| {
            flex.child(render_empty_state("No audio files", "Import files to get started", cx))
        })
}

fn render_audio_file_item(
    file: &AudioFile,
    state: &DawUiState,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    let file_path = file.path.clone();
    let file_name = file.name.clone();
    let file_type = file.file_type.clone();
    let size_kb = file.size_bytes / 1024;
    
    div()
        .w_full()
        .px_2()
        .py_2()
        .rounded_sm()
        .cursor_pointer()
        .hover(|d| d.bg(cx.theme().accent.opacity(0.1)))
        .active(|d| d.bg(cx.theme().accent.opacity(0.2)))
        .on_drag(file_path.clone(), |drag, window, cx| {
            drag.child(
                div()
                    .px_3()
                    .py_2()
                    .rounded_md()
                    .bg(cx.theme().accent)
                    .text_color(cx.theme().accent_foreground)
                    .child(file_name.clone())
            )
        })
        .child(
            v_flex()
                .gap_1()
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(Icon::new(IconName::Heart).size_4().text_color(cx.theme().muted_foreground))
                        .child(
                            div()
                                .flex_1()
                                .text_sm()
                                .child(file_name.clone())
                        )
                )
                .child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .justify_between()
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(file_type.clone())
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("{} KB", size_kb))
                        )
                )
        )
}

fn render_instruments_tab(_state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    render_empty_state("No Instruments", "Virtual instruments will appear here", cx)
}

fn render_effects_tab(_state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    render_empty_state("No Effects", "Audio effects will appear here", cx)
}

fn render_loops_tab(_state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    render_empty_state("No Loops", "Loop files will appear here", cx)
}

fn render_samples_tab(_state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    render_empty_state("No Samples", "One-shot samples will appear here", cx)
}

fn render_empty_state(
    title: &'static str,
    description: &'static str,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    v_flex()
        .w_full()
        .p_6()
        .gap_2()
        .items_center()
        .justify_center()
        .child(
            Icon::new(IconName::Inbox)
                .size(px(48.0))
                .text_color(cx.theme().text_color(cx.theme().muted_foreground).opacity(0.5))
        )
        .child(
            div()
                .text_sm()
                .font_semibold()
                .text_color(cx.theme().muted_foreground)
                .child(title)
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().text_color(cx.theme().muted_foreground).opacity(0.7))
                .text_center()
                .child(description)
        )
}

// Event handlers

fn handle_import_audio(state: &mut DawUiState, window: &mut Window, cx: &mut Context<DawPanel>) {
    let project_dir = state.project_dir.clone();
    
    cx.spawn(|this, mut cx| async move {
        let files = rfd::AsyncFileDialog::new()
            .add_filter("Audio Files", &["wav", "mp3", "ogg", "flac", "aiff"])
            .pick_files()
            .await;

        if let Some(files) = files {
            for file in files {
                let source = file.path().to_path_buf();
                
                if let Some(ref proj_dir) = project_dir {
                    cx.update(|cx| {
                        this.update(cx, |this, cx| {
                            if let Err(e) = this.state.import_audio_file(source) {
                                eprintln!("❌ Import failed: {}", e);
                            } else {
                                eprintln!("✅ Audio file imported");
                            }
                            cx.notify();
                        }).ok();
                    }).ok();
                }
            }
        }
    }).detach();
}
