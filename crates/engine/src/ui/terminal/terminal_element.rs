//! Complete terminal element with full interactivity
//! Adapted from Zed's terminal_element.rs

use super::terminal_core::{Terminal, TerminalBounds, TerminalContent};
use super::rendering::{layout_grid, BatchedTextRun, LayoutRect};
use gpui::*;
use gpui_component::ActiveTheme;

/// Simple terminal input handler (simplified from Zed)
struct TerminalInputHandler {
    terminal: Entity<Terminal>,
}

impl InputHandler for TerminalInputHandler {
    fn selected_text_range(&mut self, _ignore_disabled_input: bool, _window: &mut Window, _cx: &mut App) -> Option<UTF16Selection> {
        // Return empty selection
        Some(UTF16Selection {
            range: 0..0,
            reversed: false,
        })
    }

    fn marked_text_range(&mut self, _window: &mut Window, _cx: &mut App) -> Option<std::ops::Range<usize>> {
        None
    }

    fn text_for_range(&mut self, _range: std::ops::Range<usize>, _actual_range: &mut Option<std::ops::Range<usize>>, _window: &mut Window, _cx: &mut App) -> Option<String> {
        None
    }

    fn replace_text_in_range(&mut self, _replacement_range: Option<std::ops::Range<usize>>, text: &str, window: &mut Window, cx: &mut App) {
        // This is the key method - send text to terminal!
        self.terminal.update(cx, |terminal, cx| {
            terminal.handle_input(text, window, cx);
        });
    }

    fn replace_and_mark_text_in_range(&mut self, _range_utf16: Option<std::ops::Range<usize>>, _new_text: &str, _new_marked_range: Option<std::ops::Range<usize>>, _window: &mut Window, _cx: &mut App) {
        // Not needed for basic terminal
    }

    fn unmark_text(&mut self, _window: &mut Window, _cx: &mut App) {
        // Not needed for basic terminal
    }

    fn bounds_for_range(&mut self, _range_utf16: std::ops::Range<usize>, _window: &mut Window, _cx: &mut App) -> Option<Bounds<Pixels>> {
        None
    }

    fn apple_press_and_hold_enabled(&mut self) -> bool {
        false
    }

    fn character_index_for_point(&mut self, _point: Point<Pixels>, _window: &mut Window, _cx: &mut App) -> Option<usize> {
        None
    }
}

/// Layout state for terminal rendering (from Zed)
pub struct LayoutState {
    pub hitbox: Hitbox,
    pub batched_text_runs: Vec<BatchedTextRun>,
    pub rects: Vec<LayoutRect>,
    pub background_color: Hsla,
    pub dimensions: TerminalBounds,
    pub text_style: TextStyle,
}

/// Terminal element with full interactivity (adapted from Zed)
pub struct TerminalElement {
    terminal: Entity<Terminal>,
    focus: FocusHandle,
    focused: bool,
    interactivity: Interactivity,
}

impl InteractiveElement for TerminalElement {
    fn interactivity(&mut self) -> &mut Interactivity {
        &mut self.interactivity
    }
}

impl StatefulInteractiveElement for TerminalElement {}

impl TerminalElement {
    pub fn new(terminal: Entity<Terminal>, focus: FocusHandle, focused: bool) -> Self {
        TerminalElement {
            terminal,
            focus: focus.clone(),
            focused,
            interactivity: Interactivity::default(),
        }
        .track_focus(&focus)
    }

    // Adapted from Zed's register_mouse_listeners
    fn register_mouse_listeners(&mut self, hitbox: &Hitbox, window: &mut Window) {
        let focus = self.focus.clone();
        let terminal = self.terminal.clone();

        // Left mouse button down - focus terminal
        self.interactivity.on_mouse_down(MouseButton::Left, {
            let focus = focus.clone();
            
            move |_e, window, _cx| {
                window.focus(&focus);
            }
        });

        // Mouse move for hover effects
        window.on_mouse_event({
            let hitbox = hitbox.clone();
            
            move |_e: &MouseMoveEvent, phase, _window, _cx| {
                if phase != DispatchPhase::Bubble {
                    return;
                }
                // Hover handling would go here
            }
        });

        // Scroll wheel - from Zed
        self.interactivity.on_scroll_wheel({
            let terminal = terminal.clone();
            
            move |event: &ScrollWheelEvent, _phase, cx| {
                terminal.update(cx, |terminal, cx| {
                    if let Some(session) = terminal.active_session_mut() {
                        let delta_y = event.delta.pixel_delta(px(20.0)).y;
                        let lines_to_scroll = (delta_y / session.last_content.terminal_bounds.line_height).abs() as usize;
                        
                        if lines_to_scroll > 0 {
                            if delta_y > px(0.0) {
                                // Scroll up (into history)
                                terminal.scroll_up(lines_to_scroll, cx);
                            } else {
                                // Scroll down
                                terminal.scroll_down(lines_to_scroll, cx);
                            }
                        }
                    }
                });
            }
        });
    }
}

impl Element for TerminalElement {
    type RequestLayoutState = ();
    type PrepaintState = LayoutState;

    fn id(&self) -> Option<ElementId> {
        self.interactivity.element_id.clone()
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    // Adapted from Zed's request_layout
    fn request_layout(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let layout_id = self.interactivity.request_layout(
            global_id,
            inspector_id,
            window,
            cx,
            |mut style, window, cx| {
                style.size.width = relative(1.).into();
                style.size.height = relative(1.).into();
                window.request_layout(style, None, cx)
            },
        );
        (layout_id, ())
    }

    // Adapted from Zed's prepaint - this is where the magic happens
    fn prepaint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.interactivity.prepaint(
            global_id,
            inspector_id,
            bounds,
            bounds.size,
            window,
            cx,
            |_, _, hitbox, window, cx| {
                let hitbox = hitbox.unwrap();
                
                // Clone theme colors we need before any borrows
                let (background_color, foreground_color) = {
                    let theme = cx.theme();
                    (theme.background, theme.foreground)
                };
                
                // Create text style exactly like script editor (JetBrains Mono)
                let text_style = TextStyle {
                    font_family: "JetBrains Mono".into(),
                    font_features: FontFeatures::default(),
                    font_weight: FontWeight::NORMAL,
                    font_fallbacks: Some(FontFallbacks::from_fonts(vec!["monospace".to_string()])),
                    font_size: px(13.0).into(),
                    font_style: FontStyle::Normal,
                    line_height: px(20.0).into(),
                    background_color: Some(background_color),
                    white_space: WhiteSpace::Normal,
                    color: foreground_color,
                    ..Default::default()
                };

                // Use JetBrains Mono font like script editor - EXACT match
                let font = Font {
                    family: "JetBrains Mono".to_string().into(),
                    weight: FontWeight::NORMAL,
                    style: FontStyle::Normal,
                    features: FontFeatures::default(),
                    fallbacks: Some(FontFallbacks::from_fonts(vec!["monospace".to_string()])),
                };

                // Calculate terminal dimensions exactly like Zed does
                let font_id = cx.text_system().resolve_font(&font);
                let rem_size = window.rem_size();
                let font_pixels = text_style.font_size.to_pixels(rem_size);
                
                // Use lowercase 'm' for cell width (proper monospace calculation like Zed)
                let cell_width = cx
                    .text_system()
                    .advance(font_id, font_pixels, 'm')
                    .unwrap()
                    .width;
                    
                // Line height - use font_pixels * 1.3 for better spacing
                let line_height = font_pixels * 1.3;

                let dimensions = TerminalBounds::new(line_height, cell_width, bounds);

                // Update terminal size and sync content (release borrow immediately)
                {
                    self.terminal.update(cx, |terminal, cx| {
                        if let Some(session) = terminal.active_session_mut() {
                            session.set_size(dimensions);
                            // Sync to process any new events and update content
                            session.sync(window, cx);
                        }
                    });
                }
                
                // Request another frame to keep checking for updates (like Zed does)
                // This ensures the terminal updates continuously
                window.on_next_frame(|window, cx| {
                    window.refresh();
                });

                // Get terminal content - read only in prepaint (Zed approach)
                // Need to get theme again for layout_grid
                let theme = cx.theme();
                let (rects, batched_text_runs) = {
                    let terminal_read = self.terminal.read(cx);
                    if let Some(session) = terminal_read.active_session() {
                        // Use cached last_content (like Zed)
                        layout_grid(
                            session.last_content.cells.iter().cloned(),
                            session.last_content.display_offset,
                            &text_style,
                            &font,
                            &theme,
                        )
                    } else {
                        (Vec::new(), Vec::new())
                    }
                };

                LayoutState {
                    hitbox,
                    batched_text_runs,
                    rects,
                    background_color,
                    dimensions,
                    text_style,
                }
            },
        )
    }

    // Adapted from Zed's paint method
    fn paint(
        &mut self,
        global_id: Option<&GlobalElementId>,
        inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        layout: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.register_mouse_listeners(&layout.hitbox, window);
        
        // Set cursor style like Zed does
        if self.focused {
            window.set_cursor_style(CursorStyle::IBeam, &layout.hitbox);
        }

        // Paint using interactivity (Zed's approach)
        self.interactivity.paint(
            global_id,
            inspector_id,
            bounds,
            Some(&layout.hitbox),
            window,
            cx,
            |_, window, cx| {
                // Register input handler - THIS IS KEY!
                let input_handler = TerminalInputHandler {
                    terminal: self.terminal.clone(),
                };
                window.handle_input(&self.focus, input_handler, cx);

                // Register key handler for special keys (from Zed)
                window.on_key_event({
                    let terminal = self.terminal.clone();
                    move |event: &KeyDownEvent, phase, _window, cx| {
                        if phase != DispatchPhase::Bubble {
                            return;
                        }
                        
                        // Try to handle as keystroke (special keys)
                        terminal.update(cx, |term, cx| {
                            term.try_keystroke(&event.keystroke, false, cx);
                        });
                    }
                });

                // Paint background
                window.paint_quad(fill(bounds, layout.background_color));

                let origin = bounds.origin;

                // Paint background rectangles (Zed does this)
                for rect in &layout.rects {
                    rect.paint(origin, &layout.dimensions, window);
                }

                // Paint batched text runs (Zed's optimization)
                for batch in &layout.batched_text_runs {
                    batch.paint(origin, &layout.dimensions, window, cx);
                }
            },
        );
    }
}

impl IntoElement for TerminalElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

