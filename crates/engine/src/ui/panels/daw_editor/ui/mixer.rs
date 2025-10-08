/// Mixer View Component
/// Professional channel strips with faders, pan, sends, meters, and insert effects

use super::state::*;
use super::panel::DawPanel;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::*, h_flex, v_flex, Icon, IconName, Sizable, StyledExt, ActiveTheme,
    slider::{Slider, SliderState},
};
use crate::ui::panels::daw_editor::audio_types::{Track, TrackId};

pub fn render_mixer(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let num_tracks = state.project.as_ref()
        .map(|p| p.tracks.len())
        .unwrap_or(0);
    
    // Ensure we have enough tracks for a nice display (minimum 8)
    let display_count = num_tracks.max(8);
    
    div()
        .size_full()
        .overflow_hidden()
        .child(
            h_flex()
                .id("mixer-scroll-content")
                .scrollable(Axis::Horizontal)
                .h_full()
                .gap_3()
                .p_4()
                .bg(hsla(220.0 / 360.0, 0.15, 0.08, 1.0))
                .children((0..display_count).filter_map(|idx| {
                    if let Some(ref project) = state.project {
                        if idx < project.tracks.len() {
                            let track = &project.tracks[idx];
                            return Some(render_channel_strip(track, idx, state, cx).into_any_element());
                        }
                    }
                    // Render empty channel strip if no track exists
                    Some(render_empty_channel_strip(idx, cx).into_any_element())
                }))
                // Master channel at the end
                .child(render_master_channel(state, cx))
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
        .gap_1p5()
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
                .h(px(32.0))
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
        // Vertical fader (main content area)
        .child(render_vertical_fader(track, track_id, cx))
        // Volume readout
        .child(
            div()
                .w_full()
                .h(px(24.0))
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
        // Pan knob
        .child(render_pan_control(track, track_id, pan_percent, cx))
        // Send knobs (2 sends: A and B)
        .child(render_send_controls(track, track_id, cx))
        // Mute / Solo / Record buttons
        .child(render_channel_buttons(track, track_id, is_muted, is_solo, cx))
}

fn render_empty_channel_strip(idx: usize, cx: &mut Context<DawPanel>) -> impl IntoElement {
    // Beautiful color per track
    let track_hue = (idx as f32 * 137.5) % 360.0; // Golden angle distribution
    let track_color = hsla(track_hue / 360.0, 0.6, 0.5, 1.0);
    
    v_flex()
        .w(px(90.0))
        .h_full()
        .gap_1p5()
        .p_2()
        .bg(cx.theme().muted.opacity(0.15))
        .rounded_lg()
        .border_1()
        .border_color(cx.theme().border.opacity(0.3))
        .shadow_sm()
        // Track color indicator at top
        .child(
            div()
                .w_full()
                .h(px(3.0))
                .bg(track_color.opacity(0.3))
                .rounded_sm()
        )
        // Track name
        .child(
            div()
                .w_full()
                .h(px(32.0))
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_xs()
                        .font_semibold()
                        .text_center()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("Track {}", idx + 1))
                )
        )
        // Filler
        .child(
            div()
                .flex_1()
                .w_full()
                .flex()
                .items_center()
                .justify_center()
                .text_color(cx.theme().muted_foreground.opacity(0.5))
                .text_xs()
                .child("Empty")
        )
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
                        .h(px(24.0))
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
        .h(px(60.0))
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
                .h(px(4.0))
                .rounded_sm()
                .bg(if is_lit {
                    color
                } else {
                    cx.theme().secondary.opacity(0.2)
                })
        }))
}

fn render_vertical_fader(track: &Track, track_id: TrackId, cx: &mut Context<DawPanel>) -> impl IntoElement {
    // Vertical fader representation
    let fader_height = 180.0;
    let knob_height = 30.0;
    let fader_pos = track.volume.clamp(0.0, 1.5); // Allow boost to 150%
    let knob_y = (1.0 - fader_pos.min(1.0)) * (fader_height - knob_height);
    
    div()
        .w_full()
        .h(px(fader_height))
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .id(SharedString::from(format!("fader-{}", track_id)))
                .w(px(24.0))
                .h_full()
                .relative()
                .bg(cx.theme().secondary.opacity(0.3))
                .rounded_md()
                .border_1()
                .border_color(cx.theme().border)
                .cursor_pointer()
                .on_mouse_down(MouseButton::Left, cx.listener(move |_panel, _event: &MouseDownEvent, _window, cx| {
                    // Start drag for fader
                    // TODO: Implement drag handling
                    cx.notify();
                }))
                // Center line (unity gain)
                .child(
                    div()
                        .absolute()
                        .left(px(8.0))
                        .right(px(8.0))
                        .top(px(fader_height * 0.33)) // Unity at 0 dB
                        .h(px(2.0))
                        .bg(cx.theme().accent.opacity(0.5))
                )
                // Fader track fill (below knob)
                .child(
                    div()
                        .absolute()
                        .left(px(6.0))
                        .right(px(6.0))
                        .top(px(knob_y + knob_height / 2.0))
                        .bottom(px(4.0))
                        .bg(hsla(200.0 / 360.0, 0.7, 0.5, 0.6))
                        .rounded_sm()
                )
                // Fader knob
                .child(
                    div()
                        .absolute()
                        .left(px(0.0))
                        .right(px(0.0))
                        .top(px(knob_y))
                        .h(px(knob_height))
                        .bg(cx.theme().accent)
                        .rounded_md()
                        .border_2()
                        .border_color(cx.theme().accent_foreground.opacity(0.3))
                        .shadow_lg()
                        .cursor_pointer()
                        .hover(|style| style.bg(cx.theme().accent.opacity(0.9)))
                        .on_mouse_down(MouseButton::Left, cx.listener(move |_panel, _event: &MouseDownEvent, _window, cx| {
                            // TODO: Start dragging fader knob
                            cx.stop_propagation();
                        }))
                )
        )
}

fn render_pan_control(track: &Track, track_id: TrackId, pan_percent: i32, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let pan_hue = if track.pan < 0.0 {
        240.0 / 360.0 // Blue for left
    } else {
        30.0 / 360.0 // Orange for right
    };
    
    v_flex()
        .w_full()
        .gap_1()
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("PAN")
        )
        .child(
            div()
                .w_full()
                .h(px(40.0))
                .flex()
                .items_center()
                .justify_center()
                // Circular pan knob
                .child(
                    div()
                        .id(SharedString::from(format!("pan-{}", track_id)))
                        .w(px(36.0))
                        .h(px(36.0))
                        .rounded_full()
                        .bg(cx.theme().secondary.opacity(0.5))
                        .border_2()
                        .border_color(cx.theme().border)
                        .flex()
                        .items_center()
                        .justify_center()
                        .cursor_pointer()
                        .hover(|style| style.bg(cx.theme().secondary.opacity(0.7)))
                        .on_mouse_down(MouseButton::Left, cx.listener(move |panel, _event: &MouseDownEvent, _window, cx| {
                            // TODO: Implement pan dragging - for now, double-click simulation to center
                            if let Some(track) = panel.state.get_track_mut(track_id) {
                                track.pan = 0.0;
                                cx.notify();
                            }
                        }))
                        // Pan indicator line
                        .child(
                            div()
                                .w(px(2.0))
                                .h(px(12.0))
                                .bg(hsla(pan_hue, 0.8, 0.5, 1.0))
                                .rounded_sm()
                                // Rotate based on pan value (-45° to +45°)
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
                    format!("L{}", pan_percent.abs())
                } else {
                    format!("R{}", pan_percent)
                })
        )
}

fn render_send_controls(track: &Track, track_id: TrackId, cx: &mut Context<DawPanel>) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_1()
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
                // Send A
                .child(render_send_knob("A".to_string(), track_id, 0, 0.0, hsla(280.0 / 360.0, 0.7, 0.5, 1.0), cx))
                // Send B
                .child(render_send_knob("B".to_string(), track_id, 1, 0.0, hsla(320.0 / 360.0, 0.7, 0.5, 1.0), cx))
        )
}

fn render_send_knob(label: String, track_id: TrackId, send_idx: usize, _level: f32, color: Hsla, cx: &mut Context<DawPanel>) -> impl IntoElement {
    v_flex()
        .flex_1()
        .gap_0p5()
        .child(
            div()
                .w_full()
                .h(px(28.0))
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .id(SharedString::from(format!("send-{}-{}", track_id, send_idx)))
                        .w(px(26.0))
                        .h(px(26.0))
                        .rounded_full()
                        .bg(cx.theme().secondary.opacity(0.4))
                        .border_1()
                        .border_color(color.opacity(0.5))
                        .cursor_pointer()
                        .hover(|style| style.border_color(color))
                        .on_mouse_down(MouseButton::Left, cx.listener(move |_panel, _event: &MouseDownEvent, _window, cx| {
                            // TODO: Start send drag
                            cx.notify();
                        }))
                )
        )
        .child(
            div()
                .w_full()
                .text_xs()
                .text_center()
                .text_color(cx.theme().muted_foreground)
                .child(label)
        )
}

fn render_channel_buttons(track: &Track, track_id: TrackId, is_muted: bool, is_solo: bool, cx: &mut Context<DawPanel>) -> impl IntoElement {
    v_flex()
        .w_full()
        .gap_1()
        .child(
            h_flex()
                .w_full()
                .gap_1()
                // Mute button
                .child(
                    div()
                        .id(SharedString::from(format!("mute-{}", track_id)))
                        .flex_1()
                        .h(px(24.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_md()
                        .border_1()
                        .bg(if is_muted {
                            hsla(0.0, 0.7, 0.4, 1.0)
                        } else {
                            cx.theme().secondary.opacity(0.4)
                        })
                        .border_color(if is_muted {
                            hsla(0.0, 0.7, 0.6, 1.0)
                        } else {
                            cx.theme().border
                        })
                        .text_xs()
                        .font_semibold()
                        .text_color(if is_muted {
                            white()
                        } else {
                            cx.theme().muted_foreground
                        })
                        .cursor_pointer()
                        .hover(|style| style.bg(if is_muted {
                            hsla(0.0, 0.7, 0.5, 1.0)
                        } else {
                            cx.theme().secondary.opacity(0.6)
                        }))
                        .on_mouse_down(MouseButton::Left, cx.listener(move |panel, _event: &MouseDownEvent, _window, cx| {
                            if let Some(track) = panel.state.get_track_mut(track_id) {
                                track.muted = !track.muted;
                                cx.notify();
                            }
                        }))
                        .child("M")
                )
                // Solo button
                .child(
                    div()
                        .id(SharedString::from(format!("solo-{}", track_id)))
                        .flex_1()
                        .h(px(24.0))
                        .flex()
                        .items_center()
                        .justify_center()
                        .rounded_md()
                        .border_1()
                        .bg(if is_solo {
                            hsla(50.0 / 360.0, 0.9, 0.5, 1.0)
                        } else {
                            cx.theme().secondary.opacity(0.4)
                        })
                        .border_color(if is_solo {
                            hsla(50.0 / 360.0, 0.9, 0.7, 1.0)
                        } else {
                            cx.theme().border
                        })
                        .text_xs()
                        .font_semibold()
                        .text_color(if is_solo {
                            hsla(50.0 / 360.0, 0.9, 0.1, 1.0)
                        } else {
                            cx.theme().muted_foreground
                        })
                        .cursor_pointer()
                        .hover(|style| style.bg(if is_solo {
                            hsla(50.0 / 360.0, 0.9, 0.6, 1.0)
                        } else {
                            cx.theme().secondary.opacity(0.6)
                        }))
                        .on_mouse_down(MouseButton::Left, cx.listener(move |panel, _event: &MouseDownEvent, _window, cx| {
                            panel.state.toggle_solo(track_id);
                            cx.notify();
                        }))
                        .child("S")
                )
        )
        // Record arm button
        .child(
            div()
                .id(SharedString::from(format!("record-{}", track_id)))
                .w_full()
                .h(px(24.0))
                .flex()
                .items_center()
                .justify_center()
                .rounded_md()
                .border_1()
                .bg(if track.record_armed {
                    hsla(0.0, 0.8, 0.5, 1.0)
                } else {
                    cx.theme().secondary.opacity(0.4)
                })
                .border_color(if track.record_armed {
                    hsla(0.0, 0.8, 0.7, 1.0)
                } else {
                    cx.theme().border
                })
                .text_xs()
                .font_semibold()
                .text_color(if track.record_armed {
                    white()
                } else {
                    cx.theme().muted_foreground
                })
                .cursor_pointer()
                .hover(|style| style.bg(if track.record_armed {
                    hsla(0.0, 0.8, 0.6, 1.0)
                } else {
                    cx.theme().secondary.opacity(0.6)
                }))
                .on_mouse_down(MouseButton::Left, cx.listener(move |panel, _event: &MouseDownEvent, _window, cx| {
                    if let Some(track) = panel.state.get_track_mut(track_id) {
                        track.record_armed = !track.record_armed;
                        cx.notify();
                    }
                }))
                .child("R")
        )
}

fn render_master_channel(state: &DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    // Use a default master volume since Transport doesn't have it
    let master_volume = 1.0f32;
    let volume_db = 20.0 * master_volume.log10();
    
    v_flex()
        .w(px(100.0))
        .h_full()
        .gap_1p5()
        .p_2()
        .bg(hsla(200.0 / 360.0, 0.3, 0.15, 1.0))
        .rounded_lg()
        .border_2()
        .border_color(cx.theme().accent)
        .shadow_lg()
        // Master label
        .child(
            div()
                .w_full()
                .h(px(3.0))
                .bg(hsla(200.0 / 360.0, 0.8, 0.5, 1.0))
                .rounded_sm()
        )
        .child(
            div()
                .w_full()
                .h(px(32.0))
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .font_bold()
                .text_color(cx.theme().accent)
                .child("MASTER")
        )
        // Master meters
        .child(
            h_flex()
                .w_full()
                .h(px(80.0))
                .gap_1()
                .child(render_meter_bar(master_volume * 0.9, cx))
                .child(render_meter_bar(master_volume * 0.85, cx))
        )
        // Master fader
        .child(
            div()
                .w_full()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .w(px(32.0))
                        .h_full()
                        .relative()
                        .bg(cx.theme().secondary.opacity(0.3))
                        .rounded_md()
                        .border_2()
                        .border_color(cx.theme().accent.opacity(0.5))
                        // Unity line
                        .child(
                            div()
                                .absolute()
                                .left(px(8.0))
                                .right(px(8.0))
                                .top(px(100.0))
                                .h(px(2.0))
                                .bg(cx.theme().accent)
                        )
                )
        )
        // Master volume readout
        .child(
            div()
                .w_full()
                .h(px(28.0))
                .flex()
                .items_center()
                .justify_center()
                .text_sm()
                .font_bold()
                .text_color(if volume_db > 0.0 {
                    hsla(0.0, 0.8, 0.5, 1.0)
                } else {
                    cx.theme().accent
                })
                .child(format!("{:+.1} dB", volume_db))
        )
}
