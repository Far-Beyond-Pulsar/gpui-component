# Mixer Refactor Plan

## Current Issues
1. Faders don't respond to dragging (manual drag code removed due to private Pixels fields)
2. Pan controls don't respond to dragging
3. Colors have poor contrast
4. No proper Slider component integration

## Solution: Use SliderState from UI Crate

### Pattern from slider_story.rs:
```rust
// 1. Create SliderState entities in DawUiState
pub struct MixerChannelState {
    pub volume_slider: Entity<SliderState>,
    pub pan_slider: Entity<SliderState>,
    pub send_a_slider: Entity<SliderState>,
    pub send_b_slider: Entity<SliderState>,
}

// 2. Subscribe to slider events
cx.subscribe(&slider, |this, _, event: &SliderEvent, cx| {
    match event {
        SliderEvent::Change(value) => {
            // Update track volume/pan
            cx.notify();
        }
    }
})

// 3. Render with Slider component
Slider::new(&state.volume_slider)
    .vertical()
    .h(px(150.0))
    .bg(cx.theme().slider_bar)
```

## Implementation Steps
1. Add mixer_channel_states: HashMap<TrackId, MixerChannelState> to DawUiState
2. Initialize SliderState for each track when project loads
3. Subscribe to SliderEvent and update track volumes/pans
4. Replace manual fader rendering with Slider::new().vertical()
5. Replace manual pan rendering with Slider::new().horizontal()
6. Improve colors using theme colors with proper contrast

## Color Improvements
- Use cx.theme().slider_bar for track backgrounds
- Use track-specific accent colors with better saturation
- Ensure text has >= 4.5:1 contrast ratio
- Use theme.border for separators

## Layout Improvements
- Set mixer panel to fixed height (e.g., h(px(280.0)))
- Allow horizontal scrolling for many channels
- Ensure mixer doesn't push other panels off screen
