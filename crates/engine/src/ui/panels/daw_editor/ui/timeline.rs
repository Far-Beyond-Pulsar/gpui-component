use super::state::*;
use super::panel::DawPanel;
use super::track_header;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::*, h_flex, v_flex, Icon, IconName, Sizable, StyledExt, ActiveTheme,
    scroll::{Scrollbar, ScrollbarAxis, ScrollbarState}, PixelsExt, v_virtual_list, VirtualListScrollHandle};
use std::path::PathBuf;
use std::rc::Rc;
use std::ops::Range;
use crate::ui::panels::daw_editor::audio_types::SAMPLE_RATE;

const TIMELINE_HEADER_HEIGHT: f32 = 40.0;
const TRACK_HEADER_WIDTH: f32 = 200.0;
const MIN_TRACK_HEIGHT: f32 = 60.0;
const MAX_TRACK_HEIGHT: f32 = 300.0;

pub fn render_timeline(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    // Prepare virtualization item sizes for tracks
    let track_sizes: Rc<Vec<Size<Pixels>>> = {
        let tracks = state.project.as_ref()
            .map(|p| p.tracks.as_slice())
            .unwrap_or(&[]);
        
        Rc::new(
            tracks.iter()
                .map(|track| {
                    let height = *state.track_heights.get(&track.id)
                        .unwrap_or(&state.viewport.track_height);
                    Size {
                        width: px(9999.0), // Will be constrained by layout
                        height: px(height),
                    }
                })
                .collect()
        )
    };

    let panel_entity = cx.entity().clone();

    v_flex()
        .size_full()
        .bg(cx.theme().background)
        // Ruler/timeline header
        .child(render_ruler(state, cx))
        // Scrollable track area with virtualization
        .child(render_virtual_track_area(state, track_sizes, cx))
}

fn render_ruler(state: &DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let _tempo = state.project.as_ref().map(|p| p.transport.tempo).unwrap_or(120.0);
    let _zoom = state.viewport.zoom;

    h_flex()
        .w_full()
        .h(px(TIMELINE_HEADER_HEIGHT))
        .bg(cx.theme().muted)
        .border_b_1()
        .border_color(cx.theme().border)
        // Track header spacer
        .child(
            div()
                .w(px(TRACK_HEADER_WIDTH))
                .h_full()
                .border_r_1()
                .border_color(cx.theme().border)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    div()
                        .text_xs()
                        .font_semibold()
                        .text_color(cx.theme().muted_foreground)
                        .child("TRACKS")
                )
        )
        // Timeline ruler with beat markings
        .child(
            div()
                .flex_1()
                .h_full()
                .relative()
                .child(render_ruler_markings(state, cx))
        )
}

fn render_ruler_markings(state: &DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let zoom = state.viewport.zoom;
    let total_beats = 500.0; // Total beats in project
    let total_width = state.beats_to_pixels(total_beats);

    div()
        .h_full()
        .w(px(total_width))
        .relative()
        // Render beat markings
        .children((0..=125).map(|bar| {  // 500 beats / 4 = 125 bars
            let beat = (bar * 4) as f64;
            let x = state.beats_to_pixels(beat);

            div()
                .absolute()
                .left(px(x))
                .h_full()
                .child(
                    v_flex()
                        .h_full()
                        .child(
                            div()
                                .px_2()
                                .text_xs()
                                .font_family("monospace")
                                .text_color(cx.theme().foreground)
                                .child(format!("{}", bar + 1))
                        )
                        .child(
                            div()
                                .w_px()
                                .flex_1()
                                .bg(cx.theme().border)
                        )
                )
        }))
        // Playhead
        .child(render_playhead(state, cx))
}

fn render_playhead(state: &DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let x = state.beats_to_pixels(state.selection.playhead_position);

    div()
        .absolute()
        .left(px(x))
        .top_0()
        .bottom_0()
        .w(px(2.0))
        .bg(cx.theme().accent)
        
        .child(
            div()
                .absolute()
                .top_0()
                .left(px(-6.0))
                .w(px(14.0))
                .h(px(14.0))
                .bg(cx.theme().accent)
                .child(
                    Icon::new(IconName::Play)
                        .size_3()
                        .text_color(cx.theme().accent_foreground)
                )
        )
}

/// Render virtualized track area - using Table pattern with uniform_list
fn render_virtual_track_area(
    state: &DawUiState,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    let panel_entity = cx.entity().clone();
    let num_tracks = state.project.as_ref().map(|p| p.tracks.len()).unwrap_or(0);
    let total_beats = 500.0;
    let total_width = state.beats_to_pixels(total_beats);
    let vertical_scroll_handle = state.timeline_vertical_scroll_handle.clone();
    
    div()
        .flex_1()
        .w_full()
        .relative()
        .overflow_hidden()
        .child(
            uniform_list(
                "timeline-track-rows",
                num_tracks,
                cx.processor(move |panel: &mut DawPanel, visible_range: Range<usize>, _window, cx| {
                    let tracks = panel.state.project.as_ref()
                        .map(|p| &p.tracks)
                        .map(|t| t.as_slice())
                        .unwrap_or(&[]);
                    let total_width = panel.state.beats_to_pixels(500.0);
                    
                    visible_range.into_iter().filter_map(|track_idx| {
                        tracks.get(track_idx).map(|track| {
                            render_track_row(track, &panel.state, total_width, cx)
                        })
                    }).collect()
                })
            )
            .size_full()
            .track_scroll(vertical_scroll_handle)
            .with_sizing_behavior(ListSizingBehavior::Infer)
        )
        // Scrollbar overlay for horizontal scrolling
        .child(
            div()
                .absolute()
                .inset_0()
                .child(
                    Scrollbar::both(
                        &state.timeline_scroll_state,
                        &state.timeline_scroll_handle,
                    )
                    .axis(ScrollbarAxis::Horizontal)
                )
        )
}

/// Render a single track row - fixed header on left, scrollable content on right (Table pattern)
fn render_track_row(
    track: &crate::ui::panels::daw_editor::audio_types::Track,
    state: &DawUiState,
    total_width: f32,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    let track_id = track.id;
    let track_height = *state.track_heights.get(&track_id)
        .unwrap_or(&state.viewport.track_height);
    
    h_flex()
        .w_full()
        .h(px(track_height))
        .border_b_1()
        .border_color(cx.theme().border)
        // Fixed left: track header (like Table's fixed left columns)
        .child(
            div()
                .w(px(TRACK_HEADER_WIDTH))
                .h_full()
                .border_r_1()
                .border_color(cx.theme().border)
                .child(track_header::render_track_header(track, state, cx))
        )
        // Scrollable right: track content (like Table's scrollable columns)
        .child(
            div()
                .flex_1()
                .h_full()
                .overflow_hidden()
                .relative()
                .child(
                    div()
                        .w(px(total_width))
                        .h_full()
                        .child(render_track_content(track, state, total_width, cx))
                )
        )
}

/// Render track timeline content (clips, automation, etc.)
fn render_track_content(
    track: &crate::ui::panels::daw_editor::audio_types::Track,
    state: &DawUiState,
    total_width: f32,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    let track_id = track.id;
    let track_height = *state.track_heights.get(&track_id)
        .unwrap_or(&state.viewport.track_height);

    div()
        .w(px(total_width))
        .h_full()
        .relative()
        .bg(cx.theme().background)
        // Grid lines
        .child(render_grid_lines(state, cx))
        // Drop zone for dragging files/clips
        .child(render_drop_zone(track_id, state, cx))
        // Render clips
        .children(track.clips.iter().map(|clip| {
            render_clip(clip, track_id, state, cx)
        }))
}

fn render_drop_zone(
    track_id: uuid::Uuid,
    state: &DawUiState,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    let is_drag_target = matches!(
        &state.drag_state,
        DragState::DraggingFile { .. }
    );

    div()
        .absolute()
        .inset_0()
        .when(is_drag_target, |d| {
            d.border_2()
                .border_color(cx.theme().accent.opacity(0.3))
                .bg(cx.theme().accent.opacity(0.05))
        })
        // Handle mouse up to drop files onto track
        .on_mouse_up(gpui::MouseButton::Left, cx.listener(move |this, event: &MouseUpEvent, _window, cx| {
            if let DragState::DraggingFile { file_path, file_name } = &this.state.drag_state.clone() {
                // Convert window position to element-local position
                let element_pos = DawPanel::window_to_timeline_pos(event.position, this);
                let mouse_x = element_pos.x.as_f32();

                // Calculate beat position from mouse X
                let beat = this.state.pixels_to_beats(mouse_x);
                let tempo = this.state.get_tempo();
                let snap_mode = this.state.snap_mode;
                let snap_value = this.state.snap_value;

                // Apply snap if enabled
                let snapped_beat = if snap_mode == SnapMode::Grid {
                    let snap_beats = snap_value.to_beats();
                    (beat / snap_beats).round() * snap_beats
                } else {
                    beat
                };

                // Create new clip
                if let Some(project) = &mut this.state.project {
                    if let Some(track) = project.tracks.iter_mut().find(|t| t.id == track_id) {
                        // Convert beats to samples: samples = beats * 60 * sample_rate / tempo
                        let start_time = ((snapped_beat * 60.0 * SAMPLE_RATE as f64) / tempo as f64) as u64;
                        let duration = ((10.0 * 60.0 * SAMPLE_RATE as f64) / tempo as f64) as u64; // 10 beats duration
                        
                        let clip = crate::ui::panels::daw_editor::audio_types::AudioClip::new(
                            file_path.clone(),
                            start_time,
                            duration,
                        );
                        track.clips.push(clip);
                        eprintln!("📎 Created clip '{}' at beat {} on track '{}'",
                            file_name, snapped_beat, track.name);
                    }
                }

                // Clear drag state
                this.state.drag_state = DragState::None;
                cx.notify();
            }
        }))
        // Handle mouse move for clip dragging
        .on_mouse_move(cx.listener(move |this, event: &MouseMoveEvent, _window, cx| {
            if let DragState::DraggingClip { clip_id, track_id: drag_track_id, start_beat, mouse_offset } = &this.state.drag_state.clone() {
                // Only update if dragging on THIS track
                if drag_track_id == &track_id {
                    // Convert window position to element-local position
                    let element_pos = DawPanel::window_to_timeline_pos(event.position, this);
                    let mouse_x = element_pos.x.as_f32() - mouse_offset.0;

                    // Calculate new beat position
                    let new_beat = this.state.pixels_to_beats(mouse_x);
                    let snap_mode = this.state.snap_mode;
                    let snap_value = this.state.snap_value;
                    let tempo = this.state.get_tempo();

                    // Apply snap if enabled
                    let snapped_beat = if snap_mode == SnapMode::Grid {
                        let snap_beats = snap_value.to_beats();
                        (new_beat / snap_beats).round() * snap_beats
                    } else {
                        new_beat
                    }.max(0.0);

                    // Update clip position
                    if let Some(project) = &mut this.state.project {
                        if let Some(track) = project.tracks.iter_mut().find(|t| t.id == track_id) {
                            if let Some(clip) = track.clips.iter_mut().find(|c| c.id == *clip_id) {
                                clip.set_start_beat(snapped_beat, tempo);
                            }
                        }
                    }
                    cx.notify();
                }
            }
        }))
        // Handle mouse up for clip drop
        .on_mouse_up(gpui::MouseButton::Left, cx.listener(move |this, event: &MouseUpEvent, _window, cx| {
            if let DragState::DraggingClip { clip_id, track_id: drag_track_id, start_beat, mouse_offset } = &this.state.drag_state.clone() {
                // Convert window position to element-local position
                let element_pos = DawPanel::window_to_timeline_pos(event.position, this);
                let mouse_x = element_pos.x.as_f32() - mouse_offset.0;

                // Calculate final beat position
                let new_beat = this.state.pixels_to_beats(mouse_x);
                let snap_mode = this.state.snap_mode;
                let snap_value = this.state.snap_value;
                let tempo = this.state.get_tempo();

                // Apply snap if enabled
                let snapped_beat = if snap_mode == SnapMode::Grid {
                    let snap_beats = snap_value.to_beats();
                    (new_beat / snap_beats).round() * snap_beats
                } else {
                    new_beat
                }.max(0.0);

                eprintln!("📍 Dropped clip at beat {} (snapped from {})",
                    snapped_beat, new_beat);

                // Finalize clip position
                if let Some(project) = &mut this.state.project {
                    if let Some(track) = project.tracks.iter_mut().find(|t| t.id == track_id) {
                        if let Some(clip) = track.clips.iter_mut().find(|c| c.id == *clip_id) {
                            clip.set_start_beat(snapped_beat, tempo);
                            eprintln!("✅ Final clip position: beat {}",
                                snapped_beat);
                        }
                    }
                }

                // Clear drag state
                this.state.drag_state = DragState::None;
                cx.notify();
            }
        }))
}

fn render_clip(
    clip: &crate::ui::panels::daw_editor::audio_types::AudioClip,
    track_id: uuid::Uuid,
    state: &DawUiState,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    let tempo = state.get_tempo();
    let x = state.beats_to_pixels(clip.start_beat(tempo));
    let width = state.beats_to_pixels(clip.duration_beats(tempo));
    let is_selected = state.selection.selected_clip_ids.contains(&clip.id);
    let clip_id = clip.id;

    let file_name = std::path::Path::new(&clip.asset_path)
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Clip".to_string());

    // Get track color for clip coloring
    let track_idx = state.project.as_ref()
        .and_then(|p| p.tracks.iter().position(|t| t.id == track_id))
        .unwrap_or(0);

    // Generate consistent color per track
    let track_hue = (track_idx as f32 * 137.5) % 360.0; // Golden angle
    let clip_color = hsla(track_hue / 360.0, 0.5, 0.45, 1.0);
    let clip_border_color = hsla(track_hue / 360.0, 0.7, 0.35, 1.0);

    let track_height = *state.track_heights.get(&track_id)
        .unwrap_or(&state.viewport.track_height);

    div()
        .id(ElementId::Name(format!("clip-{}", clip_id).into()))
        .absolute()
        .left(px(x))
        .top(px(4.0))
        .w(px(width))
        .h(px(track_height - 8.0))
        .rounded_sm()
        .overflow_hidden()
        .cursor_pointer()
        .when(is_selected, |d| {
            d.border_2().border_color(cx.theme().accent).shadow_lg()
        })
        .when(!is_selected, |d| {
            d.border_1().border_color(clip_border_color)
        })
        .bg(clip_color)
        .hover(|d| d.bg(clip_color.opacity(0.9)))
        .on_click(cx.listener(move |this, _event: &ClickEvent, _window, cx| {
            this.state.select_clip(clip_id, false);
            cx.notify();
        }))
        // Make clips draggable with mouse down
        .on_mouse_down(gpui::MouseButton::Left, cx.listener({
            let start_beat = clip.start_beat(tempo);
            move |this, event: &MouseDownEvent, _window, cx| {
                // Use proper coordinate conversion: window → element
                let element_pos = DawPanel::window_to_timeline_pos(event.position, this);
                let mouse_x = element_pos.x.as_f32();
                let clip_x = x;

                this.state.drag_state = DragState::DraggingClip {
                    clip_id,
                    track_id,
                    start_beat,
                    mouse_offset: (mouse_x - clip_x, 0.0),
                };
                // Also select the clip
                this.state.select_clip(clip_id, false);
                cx.notify();
            }
        }))
        .child(
            v_flex()
                .size_full()
                .px_2()
                .py_1()
                .gap_1()
                .child(
                    div()
                        .text_xs()
                        .font_semibold()
                        .text_color(cx.theme().background) // Contrast with clip color
                        .child(file_name)
                )
                .child(
                    div()
                        .flex_1()
                        .relative()
                        // Placeholder waveform with track-colored tint
                        .child(render_waveform_placeholder(clip_color, cx))
                )
        )
}

fn render_waveform_placeholder(tint_color: Hsla, cx: &mut Context<DawPanel>) -> impl IntoElement {
    // Darken the tint color manually
    let darkened_color = hsla(
        tint_color.h,
        tint_color.s,
        (tint_color.l * 0.7).max(0.0),
        tint_color.a
    );

    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            Icon::new(IconName::Activity)
                .size_4()
                .text_color(darkened_color)
        )
}

fn render_grid_lines(state: &DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let _zoom = state.viewport.zoom;
    let num_beats = 500; // Total beats to show

    div()
        .absolute()
        .inset_0()
        
        .children((0..num_beats).step_by(4).map(|beat| {
            let x = state.beats_to_pixels(beat as f64);

            div()
                .absolute()
                .left(px(x))
                .top_0()
                .bottom_0()
                .w_px()
                .bg(cx.theme().border.opacity(0.3))
        }))
}