/// Track Header Component
/// Left sidebar showing track controls (mute, solo, volume, etc.)

use super::state::*;
use super::panel::DawPanel;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::*, h_flex, v_flex, Icon, IconName, Sizable, StyledExt, ActiveTheme,
    Selectable, slider::{Slider, SliderState}, tooltip::Tooltip,
};

pub fn render_track_header(
    track: &crate::ui::panels::daw_editor::audio_types::Track,
    state: &DawUiState,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    let track_height = *state.track_heights.get(&track.id)
        .unwrap_or(&state.viewport.track_height);
    
    let is_selected = state.selection.selected_track_ids.contains(&track.id);
    let is_muted = state.is_track_effectively_muted(track.id);
    let is_soloed = state.solo_tracks.contains(&track.id);
    let track_id = track.id;
    
    v_flex()
        .w_full()
        .h(px(track_height))
        .px_2()
        .py_2()
        .gap_2()
        .bg(if is_selected {
            cx.theme().accent.opacity(0.1)
        } else {
            cx.theme().muted.opacity(0.1)
        })
        .border_b_1()
        .border_color(cx.theme().border)
        // Track name and color
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    div()
                        .w(px(4.0))
                        .h(px(24.0))
                        .rounded_sm()
                        .bg(cx.theme().accent) // Would be track color
                )
                .child(
                    div()
                        .flex_1()
                        .text_sm()
                        .font_semibold()
                        .child(track.name.clone())
                )
        )
        // Controls
        .child(
            h_flex()
                .gap_1()
                .child(
                    Button::new(format!("track-{}-mute", track_id))
                        .label("M")
                        .compact()
                        .small()
                        .when(is_muted, |b| b.warning())
                        
                        .tooltip("Mute")
                        .on_click(cx.listener(move |this, _, _window, cx| {
                            if let Some(t) = this.state.get_track_mut(track_id) {
                                t.muted = !t.muted;
                                cx.notify();
                            }
                        }))
                )
                .child(
                    Button::new(format!("track-{}-solo", track_id))
                        .label("S")
                        .compact()
                        .small()
                        .when(is_soloed, |b| b.primary())
                        
                        .tooltip("Solo")
                        .on_click(cx.listener(move |this, _, _window, cx| {
                            this.state.toggle_solo(track_id);
                            cx.notify();
                        }))
                )
                .child(
                    Button::new(format!("track-{}-record", track_id))
                        .icon(Icon::new(IconName::Circle))
                        .compact()
                        .small()
                        .ghost()
                        .tooltip("Record Arm")
                )
        )
        // Volume indicator
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(format!("{:.1} dB", track.volume_db()))
        )
}
