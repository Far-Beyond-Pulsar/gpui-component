/// Timeline/Arrange View Component
/// Main timeline with tracks, clips, and automation lanes

use super::state::*;
use super::panel::DawPanel;
use super::track_header;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::*, h_flex, v_flex, Icon, IconName, Sizable, StyledExt, ActiveTheme,
    scroll::Scrollable,
};
use std::path::PathBuf;

const TIMELINE_HEADER_HEIGHT: f32 = 40.0;
const TRACK_HEADER_WIDTH: f32 = 200.0;
const MIN_TRACK_HEIGHT: f32 = 60.0;
const MAX_TRACK_HEIGHT: f32 = 300.0;

pub fn render_timeline(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    v_flex()
        .size_full()
        .bg(cx.theme().background)
        // Ruler/timeline header
        .child(render_ruler(state, cx))
        // Scrollable track area
        .child(render_track_area(state, cx))
}

fn render_ruler(state: &DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let tempo = state.project.as_ref().map(|p| p.transport.tempo).unwrap_or(120.0);
    let zoom = state.viewport.zoom;
    
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
    let tracks = state.project.as_ref()
        .map(|p| &p.tracks)
        .map(|t| t.as_slice())
        .unwrap_or(&[]);
    
    h_flex()
        .flex_1()
        .overflow_hidden()
        // Track headers column
        .child(
            div()
                .w(px(TRACK_HEADER_WIDTH))
                .h_full()
                .overflow_hidden()
                .border_r_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().muted.opacity(0.3))
                .child(
                    v_flex()
                        .children(tracks.iter().map(|track| {
                            track_header::render_track_header(track, state, cx)
                        }))
                )
        )
        // Timeline content
        .child(
            div()
                .flex_1()
                .h_full()
                .overflow_scroll()
                
                .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _window, cx| {
                    let delta = match event.delta {
                        ScrollDelta::Pixels(p) => p,
                        ScrollDelta::Lines(l) => gpui::Point::new(px(l.x * 20.0), px(l.y * 20.0)),
                    };
                    this.state.viewport.scroll_x += delta.x.as_f32() as f64;
                    this.state.viewport.scroll_y += delta.y.as_f32() as f64;
                    cx.notify();
                }))
                .child(render_timeline_content(tracks, state, cx))
        )
}

fn render_timeline_content(
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
    
    // Calculate timeline width (e.g., 500 beats)
    let timeline_width = state.beats_to_pixels(500.0);
    
    div()
        .w(px(timeline_width))
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
                .w_full()
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
    
    div()
        .id(format!("track-lane-{}", track_id))
        .size_full()
        .relative()
        .bg(if is_selected {
            cx.theme().accent.opacity(0.05)
        } else {
            cx.theme().background
        })
        .when(is_dragging_over, |d| {
            d.bg(cx.theme().accent.opacity(0.1))
        })
        .border_b_1()
        .border_color(cx.theme().border)
        .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
            this.state.select_track(track_id, event.modifiers.shift);
            cx.notify();
        }))
        // Handle mouse up for dropping files
        .on_mouse_up(gpui::MouseButton::Left, cx.listener(move |this, event: &MouseUpEvent, _window, cx| {
            if let DragState::DraggingFile { ref file_path, .. } = this.state.drag_state {
                // Get mouse position relative to timeline
                let mouse_x = event.position.x.as_f32() - TRACK_HEADER_WIDTH;
                let beat = this.state.pixels_to_beats(mouse_x);
                let snapped_beat = this.state.snap_beat(beat);
                
                if let Some(_clip_id) = this.state.add_clip(track_id, snapped_beat, file_path.clone()) {
                    eprintln!("✅ Added clip '{}' to track at beat {}", 
                        file_path.file_name().and_then(|n| n.to_str()).unwrap_or("?"),
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
        .and_then(|s| s.to_str())
        .unwrap_or("Clip");
    
    div()
        .id(format!("clip-{}", clip_id))
        .absolute()
        .left(px(x))
        .top(px(4.0))
        .w(px(width))
        .h(px(state.viewport.track_height - 8.0))
        .rounded_sm()
        .overflow_hidden()
        .cursor_pointer()
        .when(is_selected, |d| {
            d.border_2().border_color(cx.theme().accent)
        })
        .when(!is_selected, |d| {
            d.border_1().border_color(cx.theme().border)
        })
        .bg(cx.theme().accent.opacity(0.3))
        .hover(|d| d.bg(cx.theme().accent.opacity(0.4)))
        .on_click(cx.listener(move |this, event: &ClickEvent, _window, cx| {
            this.state.select_clip(clip_id, event.modifiers.shift);
            cx.notify();
        }))
        // Make clips draggable with mouse down
        .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |this, event: &MouseDownEvent, _window, cx| {
            // Start dragging the clip
            let mouse_x = event.position.x.as_f32();
            let clip_x = x;
            this.state.drag_state = DragState::DraggingClip {
                clip_id,
                track_id,
                start_beat: clip.start_beat(tempo),
                mouse_offset: (mouse_x - clip_x, 0.0),
            };
            // Also select the clip
            this.state.select_clip(clip_id, event.modifiers.shift);
            cx.notify();
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
                        .text_color(cx.theme().accent_foreground)
                        .child(file_name)
                )
                .child(
                    div()
                        .flex_1()
                        .relative()
                        // Placeholder waveform
                        .child(render_waveform_placeholder(cx))
                )
        )
}

fn render_waveform_placeholder(cx: &mut Context<DawPanel>) -> impl IntoElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            Icon::new(IconName::Activity)
                .size_4()
                .text_color(cx.theme().accent_foreground.opacity(0.3))
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
