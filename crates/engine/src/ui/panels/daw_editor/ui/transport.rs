/// Transport Controls Component
/// Play, stop, record, loop, metronome, and timeline position

use super::state::*;
use super::panel::DawPanel;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::*, h_flex, Icon, IconName, Sizable, StyledExt, ActiveTheme, 
    Selectable, divider::Divider, tooltip::Tooltip,
};

pub fn render_transport(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    h_flex()
        .w_full()
        .h(px(56.0))
        .px_4()
        .gap_3()
        .items_center()
        .bg(cx.theme().background)
        .border_b_1()
        .border_color(cx.theme().border)
        // Transport buttons
        .child(render_transport_buttons(state, cx))
        .child(Divider::vertical().h(px(32.0)))
        // Timeline position display
        .child(render_position_display(state, cx))
        .child(Divider::vertical().h(px(32.0)))
        // Tempo and time signature
        .child(render_tempo_section(state, cx))
        .child(div().flex_1())
        // Loop section
        .child(render_loop_section(state, cx))
        .child(Divider::vertical().h(px(32.0)))
        // Metronome and count-in
        .child(render_metronome_section(state, cx))
}

fn render_transport_buttons(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    h_flex()
        .gap_1()
        .items_center()
        // Go to start
        .child(
            Button::new("transport-start")
                .icon(Icon::new(IconName::ChevronLeft))
                .ghost()
                .small()
                .tooltip(move |_window, _cx| {
                    Tooltip::new("Go to Start")
                })
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.state.set_playhead(0.0);
                    cx.notify();
                }))
        )
        // Stop
        .child(
            Button::new("transport-stop")
                .icon(Icon::new(IconName::Square))
                .ghost()
                .small()
                .tooltip(move |_window, _cx| {
                    Tooltip::new("Stop")
                })
                .on_click(cx.listener(|this, _, window, cx| {
                    handle_stop(&mut this.state, window, cx);
                }))
        )
        // Play/Pause
        .child(
            Button::new("transport-play")
                .icon(Icon::new(if state.is_playing { 
                    IconName::Pause 
                } else { 
                    IconName::Play 
                }))
                .primary()
                .small()
                .tooltip(move |_window, _cx| {
                    if state.is_playing { 
                        Tooltip::new("Pause") 
                    } else { 
                        Tooltip::new("Play")
                    }
                })
                .on_click(cx.listener(|this, _, window, cx| {
                    handle_play_pause(&mut this.state, window, cx);
                }))
        )
        // Record
        .child(
            Button::new("transport-record")
                .icon(Icon::new(IconName::Circle))
                .danger()
                .ghost()
                .when(state.is_recording, |b| b.danger())
                .small()
                
                .tooltip(move |_window, _cx| {
                    Tooltip::new("Record")
                })
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.state.is_recording = !this.state.is_recording;
                    cx.notify();
                }))
        )
}

fn render_position_display(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let position = state.selection.playhead_position;
    let tempo = state.project.as_ref()
        .map(|p| p.transport.tempo)
        .unwrap_or(120.0);
    
    // Convert beats to time format
    let seconds = (position / tempo) * 60.0;
    let minutes = (seconds / 60.0).floor() as u32;
    let secs = (seconds % 60.0).floor() as u32;
    let millis = ((seconds % 1.0) * 1000.0).floor() as u32;
    
    // Convert to bars:beats format
    let bars = (position / 4.0).floor() as u32 + 1;
    let beats = (position % 4.0).floor() as u32 + 1;
    let subdivisions = ((position % 1.0) * 100.0).floor() as u32;
    
    h_flex()
        .gap_2()
        .items_center()
        .child(
            div()
                .px_3()
                .py_1()
                .rounded_md()
                .bg(cx.theme().muted)
                .child(
                    div()
                        .text_sm()
                        .font_family("monospace")
                        .child(format!("{:02}:{:02}.{:03}", minutes, secs, millis))
                )
        )
        .child(
            div()
                .px_3()
                .py_1()
                .rounded_md()
                .bg(cx.theme().muted)
                .child(
                    div()
                        .text_sm()
                        .font_family("monospace")
                        .child(format!("{:03}.{}.{:02}", bars, beats, subdivisions))
                )
        )
}

fn render_tempo_section(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let tempo = state.project.as_ref()
        .map(|p| p.transport.tempo)
        .unwrap_or(120.0);
    
    let time_sig_num = state.project.as_ref()
        .map(|p| p.transport.time_signature_numerator)
        .unwrap_or(4);
    
    let time_sig_denom = state.project.as_ref()
        .map(|p| p.transport.time_signature_denominator)
        .unwrap_or(4);
    
    h_flex()
        .gap_2()
        .items_center()
        .child(
            div()
                .px_2()
                .py_1()
                .rounded_sm()
                .cursor_pointer()
                .hover(|d| d.bg(cx.theme().muted))
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(Icon::new(IconName::Timer).size_4())
                        .child(
                            div()
                                .text_sm()
                                .font_semibold()
                                .child(format!("{:.1}", tempo))
                        )
                        .child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground.into())
                                .child("BPM")
                        )
                )
        )
        .child(
            div()
                .px_2()
                .py_1()
                .rounded_sm()
                .cursor_pointer()
                .hover(|d| d.bg(cx.theme().muted))
                .child(
                    h_flex()
                        .gap_1()
                        .items_center()
                        .child(Icon::new(IconName::Heart).size_4())
                        .child(
                            div()
                                .text_sm()
                                .font_family("monospace")
                                .child(format!("{}/{}", time_sig_num, time_sig_denom))
                        )
                )
        )
}

fn render_loop_section(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    h_flex()
        .gap_1()
        .items_center()
        // Loop
        .child(
            Button::new("transport-loop")
                .icon(Icon::new(IconName::Repeat))
                .ghost()
                .small()
                .when(state.is_looping, |b| b.primary())
                .tooltip(move |_window, _cx| {
                    Tooltip::new("Loop")
                })
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.state.is_looping = !this.state.is_looping;
                    cx.notify();
                }))
        )
        .when(state.is_looping, |flex| {
            let loop_start = state.selection.loop_start.unwrap_or(0.0);
            let loop_end = state.selection.loop_end.unwrap_or(16.0);
            
            flex.child(
                div()
                    .px_2()
                    .py_1()
                    .rounded_sm()
                    .bg(cx.theme().accent.opacity(0.1))
                    .border_1()
                    .border_color(cx.theme().accent)
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().accent_foreground)
                            .child(format!("{:.1} - {:.1}", loop_start, loop_end))
                    )
            )
        })
}

fn render_metronome_section(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    h_flex()
        .gap_1()
        .items_center()
        .child(
            Button::new("transport-metronome")
                .icon(Icon::new(IconName::Heart))
                .ghost()
                .small()
                .when(state.metronome_enabled, |b| b.primary())
                .tooltip(move |_window, _cx| {
                    Tooltip::new("Metronome")
                })
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.state.metronome_enabled = !this.state.metronome_enabled;
                    cx.notify();
                }))
        )
        .child(
            Button::new("transport-countin")
                .icon(Icon::new(IconName::Clock))
                .ghost()
                .small()
                .when(state.count_in_enabled, |b| b.primary())
                .tooltip(move |_window, _cx| {
                    Tooltip::new("Count-In")
                })
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.state.count_in_enabled = !this.state.count_in_enabled;
                    cx.notify();
                }))
        )
}

// Event handlers

fn handle_play_pause(state: &mut DawUiState, window: &mut Window, cx: &mut Context<DawPanel>) {
    state.is_playing = !state.is_playing;

    if let Some(ref service) = state.audio_service {
        let service = service.clone();
        let playing = state.is_playing;

        cx.spawn(async move |_this, _cx| {
            if playing {
                let _ = service.play().await;
            } else {
                let _ = service.pause().await;
            }
        }).detach();
    }

    cx.notify();
}

fn handle_stop(state: &mut DawUiState, window: &mut Window, cx: &mut Context<DawPanel>) {
    state.is_playing = false;
    state.set_playhead(0.0);

    if let Some(ref service) = state.audio_service {
        let service = service.clone();

        cx.spawn(async move |_this, _cx| {
            let _ = service.stop().await;
        }).detach();
    }

    cx.notify();
}
