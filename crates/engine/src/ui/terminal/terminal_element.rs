//! Terminal rendering element
//! Based on Zed's terminal_element.rs implementation

use super::terminal_core::{Terminal, TerminalBounds};
use super::rendering::{layout_grid, BatchedTextRun, LayoutRect};
use alacritty_terminal::term::cell::Cell;
use gpui::*;
use gpui_component::ActiveTheme;

/// Wrapper for indexed cell
#[derive(Debug, Clone)]
pub struct IndexedCell {
    pub point: alacritty_terminal::index::Point,
    pub cell: Cell,
}

impl std::ops::Deref for IndexedCell {
    type Target = Cell;

    fn deref(&self) -> &Self::Target {
        &self.cell
    }
}

/// Layout state for terminal rendering
pub struct LayoutState {
    pub hitbox: Hitbox,
    pub batched_text_runs: Vec<BatchedTextRun>,
    pub rects: Vec<LayoutRect>,
    pub background_color: Hsla,
    pub dimensions: TerminalBounds,
    pub text_style: TextStyle,
}

/// Terminal element that renders the terminal content
pub struct TerminalElement {
    terminal: Entity<Terminal>,
    focus: FocusHandle,
}

impl TerminalElement {
    pub fn new(terminal: Entity<Terminal>, focus: FocusHandle) -> Self {
        Self { terminal, focus }
    }
}

impl Element for TerminalElement {
    type RequestLayoutState = ();
    type PrepaintState = LayoutState;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = relative(1.).into();
        
        let layout_id = window.request_layout(style, None, cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let hitbox = window.insert_hitbox(bounds, true);
        let theme = cx.theme();
        
        // Create text style for terminal - exactly like Zed
        let text_style = TextStyle {
            font_family: "monospace".into(),
            font_features: FontFeatures::default(),
            font_weight: FontWeight::NORMAL,
            font_fallbacks: None,
            font_size: px(13.0).into(),
            font_style: FontStyle::Normal,
            line_height: px(20.0).into(),
            background_color: Some(hsla(0.0, 0.0, 0.05, 1.0)),
            white_space: WhiteSpace::Normal,
            color: hsla(0.0, 0.0, 0.9, 1.0),
            ..Default::default()  // Like Zed does
        };

        // Calculate terminal dimensions
        let font_id = cx.text_system().resolve_font(&text_style.font());
        let font_pixels = text_style.font_size.to_pixels(window.rem_size());
        let cell_width = cx
            .text_system()
            .advance(font_id, font_pixels, 'm')
            .unwrap()
            .width;
        let line_height = font_pixels * 1.5;

        let dimensions = TerminalBounds::new(line_height, cell_width, bounds);

        // Get terminal content and layout it
        let (rects, batched_text_runs) = self.terminal.read(cx).active_session()
            .map(|session| {
                // Resize terminal to fit bounds
                let term = session.term().lock();
                let content = term.renderable_content();
                
                // Convert display_iter to our IndexedCell format
                let cells: Vec<IndexedCell> = content.display_iter
                    .map(|ic| IndexedCell {
                        point: ic.point,
                        cell: Cell::clone(&ic.cell),
                    })
                    .collect();
                
                // Layout the grid
                layout_grid(
                    cells.into_iter(),
                    0,
                    &text_style,
                    &theme,
                )
            })
            .unwrap_or_else(|| (Vec::new(), Vec::new()));

        LayoutState {
            hitbox,
            batched_text_runs,
            rects,
            background_color: hsla(0.0, 0.0, 0.05, 1.0),
            dimensions,
            text_style,
        }
    }

    fn paint(
        &mut self,
        _global_id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        layout: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        // Paint background
        window.paint_quad(fill(bounds, layout.background_color));

        let origin = bounds.origin;

        // Paint background rectangles
        for rect in &layout.rects {
            rect.paint(origin, &layout.dimensions, window);
        }

        // Paint batched text runs
        for batch in &layout.batched_text_runs {
            batch.paint(origin, &layout.dimensions, window, cx);
        }
    }
}

impl IntoElement for TerminalElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}
