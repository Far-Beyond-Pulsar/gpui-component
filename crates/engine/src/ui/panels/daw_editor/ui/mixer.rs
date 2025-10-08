/// Mixer View Component
/// Channel strips with faders, pan, sends, and inserts

use super::state::*;
use super::panel::DawPanel;
use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::*, h_flex, v_flex, Icon, IconName, Sizable, StyledExt, ActiveTheme,
    slider::{Slider, SliderState}, scroll::Scrollable,
};

pub fn render_mixer(state: &mut DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    let tracks = state.project.as_ref()
        .map(|p| &p.tracks)
        .map(|t| t.as_slice())
        .unwrap_or(&[]);
    
    h_flex()
        .size_full()
        .overflow_x_hidden()
        .gap_2()
        .p_4()
        .children(tracks.iter().map(|track| {
            render_channel_strip(track, state, cx)
        }))
        // Master channel
        .child(render_master_channel(state, cx))
}

fn render_channel_strip(
    track: &crate::ui::panels::daw_editor::audio_types::Track,
    state: &DawUiState,
    cx: &mut Context<DawPanel>,
) -> impl IntoElement {
    v_flex()
        .w(px(state.mixer_width))
        .h_full()
        .gap_2()
        .p_2()
        .bg(cx.theme().muted.opacity(0.3))
        .rounded_md()
        .border_1()
        .border_color(cx.theme().border)
        // Track name
        .child(
            div()
                .w_full()
                .text_sm()
                .font_semibold()
                .text_center()
                .child(&track.name)
        )
        // Fader (vertical slider)
        .child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child("Fader") // Would be actual slider
        )
        // Pan control
        .child(
            div()
                .w_full()
                .text_xs()
                .text_center()
                .text_color(cx.theme().muted_foreground)
                .child(format!("Pan: {:.0}%", track.pan * 100.0))
        )
        // Meters and controls
        .child(
            h_flex()
                .w_full()
                .gap_1()
                .child(
                    Button::new(ElementId::Name(format!("mixer-mute-{}", track.id).into()))
                        .label("M")
                        .small()
                        .compact()
                        
                )
                .child(
                    Button::new(ElementId::Name(format!("mixer-solo-{}", track.id).into()))
                        .label("S")
                        .small()
                        .compact()
                )
        )
}

fn render_master_channel(state: &DawUiState, cx: &mut Context<DawPanel>) -> impl IntoElement {
    v_flex()
        .w(px(state.mixer_width))
        .h_full()
        .gap_2()
        .p_2()
        .bg(cx.theme().accent.opacity(0.1))
        .rounded_md()
        .border_2()
        .border_color(cx.theme().accent)
        .child(
            div()
                .w_full()
                .text_sm()
                .font_bold()
                .text_center()
                .child("MASTER")
        )
        .child(
            div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .child("Master Fader")
        )
}
