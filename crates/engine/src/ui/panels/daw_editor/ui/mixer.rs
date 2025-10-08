/// Mixer View Component
/// Professional channel strips with faders, pan, sends, meters, and insert effects

use super::state::*;
use super::panel::DawPanel;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::*, h_flex, v_flex, Icon, IconName, Sizable, StyledExt, ActiveTheme,
};
use crate::ui::panels::daw_editor::audio_types::{Track, TrackId};

pub fn render_mixer(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let num_tracks = state.project.as_ref()
        .map(|p| p.tracks.len())
        .unwrap_or(0);

    div()
        .size_full()
        .overflow_hidden()
        .child(
            h_flex()
                .id("mixer-scroll-content")
                .overflow_x_scroll()
                .h_full()
                .gap_2()
                .px_3()
                .py_2()
                .bg(hsla(220.0 / 360.0, 0.15, 0.08, 1.0))
                // Render all tracks
                .children((0..num_tracks).map(|idx| {
                    if let Some(ref project) = state.project {
                        if idx < project.tracks.len() {
                            let track = &project.tracks[idx];
                            return render_channel_strip(track, idx, state, cx).into_any_element();
                        }
                    }
                    div().into_any_element()
                }))
                // Add channel button
                .child(render_add_channel_button(cx))
                // Master channel at the end
                .child(render_master_channel(state, cx))
        )
}

fn render_add_channel_button(cx: &mut Context<DawPanel>) -> impl IntoElement {
    v_flex()
        .w(px(90.0))
        .h_full()
        .gap_1()
        .p_2()
        .bg(cx.theme().accent.opacity(0.1))
        .rounded_lg()
        .border_1()
        .border_color(cx.theme().accent.opacity(0.3))
        .cursor_pointer()
        .hover(|style| style.bg(cx.theme().accent.opacity(0.2)))
        .on_mouse_down(MouseButton::Left, cx.listener(|panel, _event: &MouseDownEvent, _window, cx| {
            // Add a new track
            if let Some(ref mut project) = panel.state.project {
                let new_track_id = uuid::Uuid::new_v4();
                let new_track = Track {
                    id: new_track_id,
                    name: format!("Track {}", project.tracks.len() + 1),
                    track_type: super::super::audio_types::TrackType::Audio,
                    volume: 1.0,
                    pan: 0.0,
                    muted: false,
                    solo: false,
                    record_armed: false,
                    clips: Vec::new(),
                    sends: Vec::new(),
                    automation: Vec::new(),
                    color: [0.5, 0.5, 0.8],
                };
                project.tracks.push(new_track);
                cx.notify();
            }
        }))
        .child(
            div()
                .flex_1()
                .w_full()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_2()
                .child(
                    div()
                        .w(px(48.0))
                        .h(px(48.0))
                        .rounded_full()
                        .bg(cx.theme().accent.opacity(0.3))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            Icon::new(IconName::Plus)
                                .size_6()
                                .text_color(cx.theme().accent)
                        )
                )
                .child(
                    div()
                        .text_xs()
                        .font_semibold()
                        .text_color(cx.theme().accent)
                        .child("Add Track")
                )
        )
}

fn render_channel_strip(
    track: &Track,
    idx: usize,
    state: &DawUiState,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    let is_selected = state.selection.selected_track_ids.contains(&track.id);
    let is_muted = track.muted || state.is_track_effectively_muted(track.id);
    let is_solo = state.solo_tracks.contains(&track.id);
    let volume_db = 20.0 * track.volume.log10(); // Convert linear to dB
    let pan_percent = (track.pan * 100.0) as i32;
    let track_id = track.id;

    // Beautiful color per track
    let track_hue = (idx as f32 * 137.5) % 360.0; // Golden angle distribution
    let track_color = hsla(track_hue / 360.0, 0.6, 0.5, 1.0);

    v_flex()
        .w(px(90.0))
        .h_full()
        .gap_1()
        .p_2()
        .bg(if is_selected {
            cx.theme().accent.opacity(0.15)
        } else {
            cx.theme().muted.opacity(0.25)
        })
        .rounded_lg()
        .border_1()
        .border_color(if is_selected {
            cx.theme().accent
        } else {
            cx.theme().border
        })
        .shadow_md()
        .cursor_pointer()
        .on_mouse_down(MouseButton::Left, cx.listener(move |panel, _event: &MouseDownEvent, _window, cx| {
            panel.state.select_track(track_id, false);
            cx.notify();
        }))
        // Track color indicator at top
        .child(
            div()
                .w_full()
                .h(px(3.0))
                .bg(track_color)
                .rounded_sm()
        )
        // Track name
        .child(
            div()
                .w_full()
                .h(px(28.0))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_xs()
                        .font_semibold()
                        .text_center()
                        .text_color(cx.theme().foreground)
                        .line_clamp(2)
                        .child(track.name.clone())
                )
        )
        // Insert slots (3 effect slots)
        .child(render_insert_slots(track, cx))
        // Peak meter LEDs
        .child(render_peak_meters(track, cx))
        // Vertical fader slider
        .child(render_fader_slider(track, track_id, cx))
        // Volume readout
        .child(
            div()
                .w_full()
                .h(px(20.0))
                .flex()
                .items_center()
                .justify_center()
                .text_xs()
                .text_color(if volume_db > 0.0 {
                    hsla(0.0, 0.8, 0.5, 1.0) // Red if clipping
                } else {
                    cx.theme().muted_foreground
                })
                .child(format!("{:+.1} dB", volume_db))
        )
        // Pan knob/slider
        .child(render_pan_slider(track, track_id, cx))
        // Send knobs (2 sends: A and B)
        .child(render_send_controls(track, track_id, cx))
        // Mute / Solo / Record buttons
        .child(render_channel_buttons(track, track_id, is_muted, is_solo, cx))
}

fn render_insert_slots(track: &Track, cx: &mut Context<DawPanel>) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_0p5()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("INSERTS")
        )
        .child(
            h_flex()
                .w_full()
                .gap_0p5()
                .children((0..3).map(|slot_idx| {
                    div()
                        .w(px(24.0))
                        .h(px(20.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(cx.theme().secondary.opacity(0.4))
                        .rounded_sm()
                        .border_1()
                        .border_color(cx.theme().border.opacity(0.5))
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .cursor_pointer()
                        .hover(|style| style.bg(cx.theme().secondary.opacity(0.6)))
                        .child(format!("{}", slot_idx + 1))
                }))
        )
}

fn render_peak_meters(track: &Track, cx: &mut Context<DawPanel>) -> impl IntoElement {
    // Simulate stereo peak meters
    let left_peak = track.volume * 0.8;
    let right_peak = track.volume * 0.75;

    h_flex()
        .w_full()
        .h(px(48.0))
        .gap_1()
        .child(render_meter_bar(left_peak, cx))
        .child(render_meter_bar(right_peak, cx))
}

fn render_meter_bar(level: f32, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let level_clamped = level.clamp(0.0, 1.0);
    let segments = 12;

    v_flex()
        .flex_1()
        .gap_0p5()
        .flex_col_reverse() // Bottom to top
        .children((0..segments).map(move |seg| {
            let threshold = seg as f32 / segments as f32;
            let is_lit = level_clamped >= threshold;

            // Color gradient: green -> yellow -> orange -> red
            let color = if seg > 10 {
                hsla(0.0, 0.9, 0.5, 1.0) // Red
            } else if seg > 8 {
                hsla(30.0 / 360.0, 0.9, 0.5, 1.0) // Orange
            } else if seg > 6 {
                hsla(60.0 / 360.0, 0.9, 0.5, 1.0) // Yellow
            } else {
                hsla(120.0 / 360.0, 0.7, 0.5, 1.0) // Green
            };

            div()
                .w_full()
                .h(px(3.0))
                .rounded_sm()
                .bg(if is_lit {
                    color
                } else {
                    cx.theme().secondary.opacity(0.2)
                })
        }))
}

fn render_fader_slider(
    track: &Track,
    track_id: TrackId,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    let volume = track.volume;
    let volume_percent = ((volume / 1.5) * 100.0).clamp(0.0, 100.0);
    
    v_flex()
        .w_full()
        .flex_1()
        .min_h(px(100.0))
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .text_center()
                .child("VOLUME")
        )
        .child(
            div()
                .flex_1()
                .w_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    // Vertical fader track
                    div()
                        .relative()
                        .w(px(8.0))
                        .h_full()
                        .min_h(px(80.0))
                        .bg(cx.theme().secondary.opacity(0.3))
                        .rounded_sm()
                        .cursor_ns_resize()
                        .child(
                            // Volume fill
                            div()
                                .absolute()
                                .bottom_0()
                                .left_0()
                                .w_full()
                                .h(relative(volume_percent / 100.0))
                                .bg(cx.theme().accent.opacity(0.4))
                                .rounded_sm()
                        )
                        .child(
                            // Fader thumb - clickable to adjust
                            div()
                                .absolute()
                                .w(px(20.0))
                                .h(px(12.0))
                                .left(px(-6.0))
                                .bottom(relative(volume_percent / 100.0))
                                .bg(cx.theme().accent)
                                .rounded_sm()
                                .border_1()
                                .border_color(cx.theme().accent)
                                .shadow_sm()
                                .cursor_pointer()
                                .on_mouse_down(MouseButton::Left, cx.listener(move |panel, _event: &MouseDownEvent, _window, cx| {
                                    // Increase volume on click
                                    if let Some(ref mut project) = panel.state.project {
                                        if let Some(track) = project.tracks.iter_mut().find(|t| t.id == track_id) {
                                            track.volume = (track.volume + 0.05).min(1.5);
                                            cx.notify();
                                        }
                                    }
                                }))
                                .on_mouse_down(MouseButton::Right, cx.listener(move |panel, _event: &MouseDownEvent, _window, cx| {
                                    // Decrease volume on right-click
                                    if let Some(ref mut project) = panel.state.project {
                                        if let Some(track) = project.tracks.iter_mut().find(|t| t.id == track_id) {
                                            track.volume = (track.volume - 0.05).max(0.0);
                                            cx.notify();
                                        }
                                    }
                                }))
                        )
                )
        )
}

fn render_pan_slider(
    track: &Track,
    track_id: TrackId,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    let pan = track.pan;
    let pan_percent = (pan * 100.0) as i32;
    let pan_position = ((pan + 1.0) / 2.0 * 100.0).clamp(0.0, 100.0);
    
    v_flex()
        .w_full()
        .gap_0p5()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("PAN")
        )
        .child(
            div()
                .w_full()
                .h(px(32.0))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    // Horizontal pan slider
                    div()
                        .relative()
                        .w_full()
                        .h(px(6.0))
                        .bg(cx.theme().secondary.opacity(0.3))
                        .rounded_sm()
                        .cursor_ew_resize()
                        .child(
                            // Center indicator
                            div()
                                .absolute()
                                .w(px(2.0))
                                .h_full()
                                .left(relative(0.5))
                                .bg(cx.theme().border)
                        )
                        .child(
                            // Pan thumb - clickable to adjust
                            div()
                                .absolute()
                                .w(px(12.0))
                                .h(px(16.0))
                                .left(relative(pan_position / 100.0))
                                .top(px(-5.0))
                                .bg(cx.theme().accent)
                                .rounded_sm()
                                .border_1()
                                .border_color(cx.theme().accent)
                                .shadow_sm()
                                .cursor_pointer()
                                .on_mouse_down(MouseButton::Left, cx.listener(move |panel, _event: &MouseDownEvent, _window, cx| {
                                    // Pan right on click
                                    if let Some(ref mut project) = panel.state.project {
                                        if let Some(track) = project.tracks.iter_mut().find(|t| t.id == track_id) {
                                            track.pan = (track.pan + 0.1).min(1.0);
                                            cx.notify();
                                        }
                                    }
                                }))
                                .on_mouse_down(MouseButton::Right, cx.listener(move |panel, _event: &MouseDownEvent, _window, cx| {
                                    // Pan left on right-click
                                    if let Some(ref mut project) = panel.state.project {
                                        if let Some(track) = project.tracks.iter_mut().find(|t| t.id == track_id) {
                                            track.pan = (track.pan - 0.1).max(-1.0);
                                            cx.notify();
                                        }
                                    }
                                }))
                        )
                )
        )
        .child(
            div()
                .w_full()
                .text_xs()
                .text_center()
                .text_color(cx.theme().muted_foreground)
                .child(if pan_percent == 0 {
                    "C".to_string()
                } else if pan_percent < 0 {
                    format!("{}L", -pan_percent)
                } else {
                    format!("{}R", pan_percent)
                })
        )
}

fn render_send_controls(
    track: &Track,
    track_id: TrackId,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_0p5()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("SENDS")
        )
        .child(
            h_flex()
                .w_full()
                .gap_1()
                .child(render_send_knob("A", 0.0, track_id, cx))
                .child(render_send_knob("B", 0.0, track_id, cx))
        )
}

fn render_send_knob(
    label: &'static str,
    value: f32,
    _track_id: TrackId,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    v_flex()
        .flex_1()
        .gap_0p5()
        .items_center()
        .child(
            div()
                .w(px(32.0))
                .h(px(32.0))
                .rounded_full()
                .bg(cx.theme().secondary.opacity(0.4))
                .border_2()
                .border_color(cx.theme().border)
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .hover(|style| style.bg(cx.theme().secondary.opacity(0.6)))
                .child(
                    div()
                        .text_xs()
                        .font_semibold()
                        .text_color(cx.theme().foreground)
                        .child(format!("{:.0}", value * 100.0))
                )
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(label)
        )
}

fn render_channel_buttons(
    track: &Track,
    track_id: TrackId,
    is_muted: bool,
    is_solo: bool,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    h_flex()
        .w_full()
        .gap_0p5()
        .child(
            div()
                .flex_1()
                .h(px(24.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .bg(if is_muted {
                    hsla(0.0, 0.7, 0.4, 0.8)
                } else {
                    cx.theme().secondary.opacity(0.4)
                })
                .border_1()
                .border_color(cx.theme().border)
                .cursor_pointer()
                .hover(|style| {
                    style.bg(if is_muted {
                        hsla(0.0, 0.7, 0.5, 0.9)
                    } else {
                        cx.theme().secondary.opacity(0.6)
                    })
                })
                .on_mouse_down(MouseButton::Left, cx.listener(move |panel, _event: &MouseDownEvent, _window, cx| {
                    if let Some(ref mut project) = panel.state.project {
                        if let Some(track) = project.tracks.iter_mut().find(|t| t.id == track_id) {
                            track.muted = !track.muted;
                            cx.notify();
                        }
                    }
                }))
                .child(
                    div()
                        .text_xs()
                        .font_semibold()
                        .text_color(if is_muted {
                            gpui::white()
                        } else {
                            cx.theme().muted_foreground
                        })
                        .child("M")
                )
        )
        .child(
            div()
                .flex_1()
                .h(px(24.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .bg(if is_solo {
                    hsla(60.0 / 360.0, 0.9, 0.5, 0.8)
                } else {
                    cx.theme().secondary.opacity(0.4)
                })
                .border_1()
                .border_color(cx.theme().border)
                .cursor_pointer()
                .hover(|style| {
                    style.bg(if is_solo {
                        hsla(60.0 / 360.0, 0.9, 0.6, 0.9)
                    } else {
                        cx.theme().secondary.opacity(0.6)
                    })
                })
                .on_mouse_down(MouseButton::Left, cx.listener(move |panel, _event: &MouseDownEvent, _window, cx| {
                    panel.state.toggle_solo(track_id);
                    cx.notify();
                }))
                .child(
                    div()
                        .text_xs()
                        .font_semibold()
                        .text_color(if is_solo {
                            gpui::black()
                        } else {
                            cx.theme().muted_foreground
                        })
                        .child("S")
                )
        )
        .child(
            div()
                .flex_1()
                .h(px(24.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_sm()
                .bg(if track.record_armed {
                    hsla(0.0, 0.9, 0.5, 0.8)
                } else {
                    cx.theme().secondary.opacity(0.4)
                })
                .border_1()
                .border_color(cx.theme().border)
                .cursor_pointer()
                .hover(|style| {
                    style.bg(if track.record_armed {
                        hsla(0.0, 0.9, 0.6, 0.9)
                    } else {
                        cx.theme().secondary.opacity(0.6)
                    })
                })
                .on_mouse_down(MouseButton::Left, cx.listener(move |panel, _event: &MouseDownEvent, _window, cx| {
                    if let Some(ref mut project) = panel.state.project {
                        if let Some(track) = project.tracks.iter_mut().find(|t| t.id == track_id) {
                            track.record_armed = !track.record_armed;
                            cx.notify();
                        }
                    }
                }))
                .child(
                    div()
                        .text_xs()
                        .font_semibold()
                        .text_color(if track.record_armed {
                            gpui::white()
                        } else {
                            cx.theme().muted_foreground
                        })
                        .child("R")
                )
        )
}

fn render_master_channel(state: &DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let master_volume = state.project.as_ref()
        .map(|p| p.master_track.volume)
        .unwrap_or(1.0);
    let volume_db = 20.0 * master_volume.log10();
    let volume_percent = ((master_volume / 1.5) * 100.0).clamp(0.0, 100.0);
    
    v_flex()
        .w(px(90.0))
        .h_full()
        .gap_1()
        .p_2()
        .bg(cx.theme().accent.opacity(0.2))
        .rounded_lg()
        .border_2()
        .border_color(cx.theme().accent)
        .shadow_lg()
        // Master label
        .child(
            div()
                .w_full()
                .h(px(3.0))
                .bg(cx.theme().accent)
                .rounded_sm()
        )
        .child(
            div()
                .w_full()
                .h(px(28.0))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_sm()
                        .font_bold()
                        .text_color(cx.theme().accent)
                        .child("MASTER")
                )
        )
        // Spacer for insert slots (master has no inserts in this simple version)
        .child(div().h(px(44.0)))
        // Master peak meters
        .child(render_master_meters(state, cx))
        // Master fader
        .child(render_master_fader(master_volume, cx))
        // Master volume readout
        .child(
            div()
                .w_full()
                .h(px(24.0))
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .font_bold()
                .text_color(if volume_db > 0.0 {
                    hsla(0.0, 0.9, 0.5, 1.0)
                } else {
                    cx.theme().accent
                })
                .child(format!("{:+.1} dB", volume_db))
        )
}

fn render_master_meters(state: &DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let master_volume = state.project.as_ref()
        .map(|p| p.master_track.volume)
        .unwrap_or(1.0);
    
    h_flex()
        .w_full()
        .h(px(48.0))
        .gap_1()
        .child(render_meter_bar(master_volume * 0.9, cx))
        .child(render_meter_bar(master_volume * 0.85, cx))
}

fn render_master_fader(master_volume: f32, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let volume_percent = ((master_volume / 1.5) * 100.0).clamp(0.0, 100.0);
    
    v_flex()
        .w_full()
        .flex_1()
        .min_h(px(100.0))
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().accent)
                .font_semibold()
                .text_center()
                .child("OUTPUT")
        )
        .child(
            div()
                .flex_1()
                .w_full()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .relative()
                        .w(px(10.0))
                        .h_full()
                        .min_h(px(80.0))
                        .bg(cx.theme().secondary.opacity(0.4))
                        .rounded_sm()
                        .cursor_ns_resize()
                        .child(
                            div()
                                .absolute()
                                .bottom_0()
                                .left_0()
                                .w_full()
                                .h(relative(volume_percent / 100.0))
                                .bg(cx.theme().accent.opacity(0.5))
                                .rounded_sm()
                        )
                        .child(
                            div()
                                .absolute()
                                .w(px(24.0))
                                .h(px(14.0))
                                .left(px(-7.0))
                                .bottom(relative(volume_percent / 100.0))
                                .bg(cx.theme().accent)
                                .rounded_md()
                                .border_2()
                                .border_color(cx.theme().accent)
                                .shadow_md()
                                .cursor_pointer()
                                .on_mouse_down(MouseButton::Left, cx.listener(move |panel, _event: &MouseDownEvent, _window, cx| {
                                    // Increase master volume
                                    if let Some(ref mut project) = panel.state.project {
                                        project.master_track.volume = (project.master_track.volume + 0.05).min(1.5);
                                        cx.notify();
                                    }
                                }))
                                .on_mouse_down(MouseButton::Right, cx.listener(move |panel, _event: &MouseDownEvent, _window, cx| {
                                    // Decrease master volume
                                    if let Some(ref mut project) = panel.state.project {
                                        project.master_track.volume = (project.master_track.volume - 0.05).max(0.0);
                                        cx.notify();
                                    }
                                }))
                        )
                )
        )
}
