use super::state::*;
use super::panel::DawPanel;
use super::track_header;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::*, h_flex, v_flex, Icon, IconName, Sizable, StyledExt, ActiveTheme,
    scroll::Scrollable, PixelsExt};
use std::path::PathBuf;
use crate::ui::panels::daw_editor::audio_types::SAMPLE_RATE;

const TIMELINE_HEADER_HEIGHT: f32 = 40.0;
const TRACK_HEADER_WIDTH: f32 = 200.0;
const MIN_TRACK_HEIGHT: f32 = 60.0;
const MAX_TRACK_HEIGHT: f32 = 300.0;

pub fn render_timeline(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let panel_entity = cx.entity().clone();
    
    v_flex()
        .size_full()
        .bg(cx.theme().background)
        // Capture timeline element bounds for coordinate conversion
        .on_children_prepainted({
            let panel_entity = panel_entity.clone();
            move |children_bounds, _window, cx| {
                if !children_bounds.is_empty() {
                    // Calculate bounding box from children (all in window coordinates)
                    let mut min_x = f32::MAX;
                    let mut min_y = f32::MAX;
                    let mut max_x = f32::MIN;
                    let mut max_y = f32::MIN;

                    for child_bounds in &children_bounds {
                        min_x = min_x.min(child_bounds.origin.x.as_f32());
                        min_y = min_y.min(child_bounds.origin.y.as_f32());
                        max_x = max_x.max((child_bounds.origin.x + child_bounds.size.width).as_f32());
                        max_y = max_y.max((child_bounds.origin.y + child_bounds.size.height).as_f32());
                    }

                    let origin = gpui::Point { x: px(min_x), y: px(min_y) };
                    let size = gpui::Size {
                        width: px(max_x - min_x),
                        height: px(max_y - min_y),
                    };

                    // Store bounds in panel
                    panel_entity.update(cx, |panel, _cx| {
                        panel.timeline_element_bounds = Some(gpui::Bounds { origin, size });
                    });
                }
            }
        })
        // Ruler/timeline header
        .child(render_ruler(state, cx))
        // Scrollable track area
        .child(render_track_area(state, cx))
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
                .overflow_x_hidden()
                .child(render_ruler_markings(state, cx))
        )
}

fn render_ruler_markings(state: &DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let zoom = state.viewport.zoom;
    let scroll_x = state.viewport.scroll_x;
    
    // Calculate visible range
    let visible_width = 1000.0; // Would be actual viewport width
    let visible_beats = visible_width / zoom as f32;
    let start_beat = scroll_x.max(0.0);
    let end_beat = start_beat + visible_beats as f64;
    
    let start_bar = (start_beat / 4.0).floor() as i32;
    let end_bar = (end_beat / 4.0).ceil() as i32;
    
    div()
        .h_full()
        .w(px(end_beat as f32 * zoom as f32))
        .relative()
        .children((start_bar..=end_bar).map(|bar| {
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
        .h_full()
        .child(
            div()
                .w_px()
                .h_full()
                .bg(cx.theme().accent)
        )
}

fn render_track_area(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    // Clone track list to avoid borrow issues
    let tracks: Vec<_> = state.project.as_ref()
        .map(|p| p.tracks.clone())
        .unwrap_or_default();
    
    let scroll_y_offset = state.viewport.scroll_y;
    
    h_flex()
        .flex_1()
        .min_w_0()  // Allow shrinking to fit within container
        .overflow_hidden()
        // Track headers column - synchronized vertical scroll
        .child(
            div()
                .w(px(TRACK_HEADER_WIDTH))
                .h_full()
                .overflow_hidden()
                .border_r_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().muted.opacity(0.3))
                .child(
                    div()
                        .id("track-headers-scroll-container")
                        .absolute()
                        .top(px(-scroll_y_offset as f32))
                        .left_0()
                        .w(px(TRACK_HEADER_WIDTH))
                        .child(
                            v_flex()
                                .children(tracks.iter().map(|track| {
                                    track_header::render_track_header(track, state, cx)
                                }))
                        )
                )
        )
        // Timeline content - scrollable and constrained
        .child(
            div()
                .flex_1()
                .min_w_0()  // Allow shrinking to fit
                .max_w_full()  // Don't exceed available width
                .h_full()
                .overflow_hidden()  // Clip content, scroll handlers below
                .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _window, cx| {
                    let delta = match event.delta {
                        ScrollDelta::Pixels(p) => p,
                        ScrollDelta::Lines(l) => gpui::Point::new(px(l.x * 20.0), px(l.y * 20.0)),
                    };
                    
                    // Horizontal scroll with bounds
                    let new_scroll_x = (this.state.viewport.scroll_x + delta.x.as_f64()).max(0.0);
                    this.state.viewport.scroll_x = new_scroll_x;
                    
                    // Vertical scroll with bounds (synchronized with headers)
                    let max_scroll_y = this.state.project.as_ref()
                        .map(|p| {
                            let total_height: f32 = p.tracks.iter()
                                .map(|t| *this.state.track_heights.get(&t.id).unwrap_or(&this.state.viewport.track_height))
                                .sum();
                            (total_height - 400.0).max(0.0) as f64  // Assume ~400px visible height
                        })
                        .unwrap_or(0.0);
                    
                    let new_scroll_y = (this.state.viewport.scroll_y + delta.y.as_f64()).clamp(0.0, max_scroll_y);
                    this.state.viewport.scroll_y = new_scroll_y;
                    
                    cx.notify();
                }))
                .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _window, cx| {
                    // Handle dragging clips
                    if let DragState::DraggingClip { clip_id, track_id, start_beat, mouse_offset } = &this.state.drag_state {
                        // Use proper coordinate conversion: window → element → timeline
                        let element_pos = DawPanel::window_to_timeline_pos(event.position, this);
                        let timeline_x = element_pos.x.as_f32();
                        
                        let mouse_x = timeline_x - mouse_offset.0;
                        let new_beat = this.state.pixels_to_beats(mouse_x);
                        let snapped_beat = this.state.snap_beat(new_beat);
                        
                        // Update clip position in project
                        let clip_id = *clip_id;
                        let track_id = *track_id;
                        if let Some(ref mut project) = this.state.project {
                            if let Some(track) = project.tracks.iter_mut().find(|t| t.id == track_id) {
                                if let Some(clip) = track.clips.iter_mut().find(|c| c.id == clip_id) {
                                    let tempo = project.transport.tempo;
                                    // Keep the duration the same, just update start_time
                                    clip.start_time = (snapped_beat * 60.0 / tempo as f64 * SAMPLE_RATE as f64) as u64;
                                }
                            }
                        }
                        cx.notify();
                    }
                }))
                .child(render_timeline_scroll_content(&tracks, state, cx))
        )
}

fn render_timeline_scroll_content(
    tracks: &[crate::ui::panels::daw_editor::audio_types::Track],
    state: &mut DawUiState,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    let total_height: f32 = tracks.iter()
        .map(|track| {
            *state.track_heights.get(&track.id)
                .unwrap_or(&state.viewport.track_height)
        })
        .sum();
    
    // Calculate timeline width (e.g., 500 beats) - constrained to viewport + scroll
    let timeline_total_width = state.beats_to_pixels(500.0);
    let scroll_x = state.viewport.scroll_x;
    let scroll_y = state.viewport.scroll_y;
    
    div()
        .id("timeline-scroll-wrapper")
        .absolute()
        .top(px(-scroll_y as f32))
        .left(px(-state.beats_to_pixels(scroll_x)))
        .w(px(timeline_total_width))
        .h(px(total_height))
        .relative()
        .bg(cx.theme().background)
        .children(tracks.iter().enumerate().map(|(idx, track)| {
            let y_offset: f32 = tracks[..idx].iter()
                .map(|t| {
                    *state.track_heights.get(&t.id)
                        .unwrap_or(&state.viewport.track_height)
                })
                .sum();
            
            let track_height = *state.track_heights.get(&track.id)
                .unwrap_or(&state.viewport.track_height);
            
            div()
                .absolute()
                .top(px(y_offset))
                .left_0()
                .w(px(timeline_total_width))
                .h(px(track_height))
                .child(render_track_lane(track, state, cx))
        }))
        // Grid lines
        .child(render_grid_lines(state, cx))
}

fn render_track_lane(
    track: &crate::ui::panels::daw_editor::audio_types::Track,
    state: &mut DawUiState,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    let is_selected = state.selection.selected_track_ids.contains(&track.id);
    let track_id = track.id;
    let is_dragging_over = matches!(&state.drag_state, DragState::DraggingFile { .. });
    
    // Get track index for color
    let track_idx = state.project.as_ref()
        .and_then(|p| p.tracks.iter().position(|t| t.id == track_id))
        .unwrap_or(0);
    
    // Generate consistent color per track
    let track_hue = (track_idx as f32 * 137.5) % 360.0; // Golden angle
    let track_color = hsla(track_hue / 360.0, 0.3, 0.12, 1.0);
    
    div()
        .id(ElementId::Name(format!("track-lane-{}", track_id).into()))
        .size_full()
        .relative()
        .bg(if is_selected {
            hsla(track_hue / 360.0, 0.4, 0.15, 1.0)
        } else {
            track_color
        })
        .when(is_dragging_over, |d| {
            d.bg(cx.theme().accent.opacity(0.1))
        })
        .border_b_1()
        .border_color(cx.theme().border.opacity(0.5))
        .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
            this.state.select_track(track_id, event.modifiers().shift);
            cx.notify();
        }))
        // Handle mouse up for dropping files
        .on_mouse_up(gpui::MouseButton::Left, cx.listener(move |this, event: &MouseUpEvent, _window, cx| {
            if let DragState::DraggingFile { ref file_path, .. } = this.state.drag_state {
                // Clone file_path to avoid holding an immutable borrow
                let file_path_cloned = file_path.clone();
                
                // Use proper coordinate conversion: window → element
                let element_pos = DawPanel::window_to_timeline_pos(event.position, this);
                
                // Convert x position to beats
                let mouse_x = element_pos.x.as_f32();
                let beat = this.state.pixels_to_beats(mouse_x);
                let snapped_beat = this.state.snap_beat(beat);
                
                if let Some(_clip_id) = this.state.add_clip(track_id, snapped_beat, file_path_cloned.clone()) {
                    eprintln!("✅ Added clip '{}' to track at beat {}", 
                        file_path_cloned.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
                        snapped_beat);
                }
                
                // Clear drag state
                this.state.drag_state = DragState::None;
                cx.notify();
            }
        }))
        // Render clips
        .children(track.clips.iter().map(|clip| {
            render_clip(clip, track_id, state, cx)
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
    
    div()
        .id(ElementId::Name(format!("clip-{}", clip_id).into()))
        .absolute()
        .left(px(x))
        .top(px(4.0))
        .w(px(width))
        .h(px(state.viewport.track_height - 8.0))
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
        .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
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
    let zoom = state.viewport.zoom;
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
