use std::{ops::Range, rc::Rc};

use gpui::{
    deferred, div, px, App, AppContext, Entity, InteractiveElement, IntoElement, ParentElement, 
    Pixels, Point, Render, StatefulInteractiveElement, Styled, Window,
};

use crate::{
    input::{popovers::render_markdown, InputState},
    v_flex, ActiveTheme, StyledExt,
};

const MAX_HOVER_WIDTH: Pixels = px(500.);
const MAX_HOVER_HEIGHT: Pixels = px(400.);
const HOVER_OFFSET_X: Pixels = px(15.); // Offset to the right of cursor

pub struct HoverPopover {
    editor: Entity<InputState>,
    /// The symbol range byte of the hover trigger.
    pub(crate) symbol_range: Range<usize>,
    pub(crate) hover: Rc<lsp_types::Hover>,
    /// Mouse position where hover was triggered (in window coordinates)
    pub(crate) mouse_position: Point<Pixels>,
}

impl HoverPopover {
    pub fn new(
        editor: Entity<InputState>,
        symbol_range: Range<usize>,
        hover: &lsp_types::Hover,
        mouse_position: Point<Pixels>,
        cx: &mut App,
    ) -> Entity<Self> {
        let hover = Rc::new(hover.clone());

        cx.new(|_| Self {
            editor,
            symbol_range,
            hover,
            mouse_position,
        })
    }

    pub(crate) fn is_same(&self, offset: usize) -> bool {
        self.symbol_range.contains(&offset)
    }
}

impl Render for HoverPopover {
    fn render(&mut self, window: &mut Window, cx: &mut gpui::Context<Self>) -> impl IntoElement {
        let contents = match self.hover.contents.clone() {
            lsp_types::HoverContents::Scalar(scalar) => match scalar {
                lsp_types::MarkedString::String(s) => s,
                lsp_types::MarkedString::LanguageString(ls) => {
                    // Format code blocks properly
                    format!("```{}\n{}\n```", ls.language, ls.value)
                },
            },
            lsp_types::HoverContents::Array(arr) => arr
                .into_iter()
                .map(|item| match item {
                    lsp_types::MarkedString::String(s) => s,
                    lsp_types::MarkedString::LanguageString(ls) => {
                        format!("```{}\n{}\n```", ls.language, ls.value)
                    },
                })
                .collect::<Vec<_>>()
                .join("\n\n"),
            lsp_types::HoverContents::Markup(markup) => markup.value,
        };

        // Get the editor's input_bounds to convert window coords to element coords
        let editor = self.editor.read(cx);
        let element_origin = editor.input_bounds.origin;
        
        // Convert window coordinates to element coordinates
        // Window coords - element origin = element-relative coords
        let element_mouse_x = self.mouse_position.x - element_origin.x;
        let element_mouse_y = self.mouse_position.y - element_origin.y;
        
        // Position popover to the right of the mouse cursor (in element coordinates)
        let mut pos_x = element_mouse_x + HOVER_OFFSET_X;
        let mut pos_y = element_mouse_y;

        // Get element size for boundary checking
        let element_width = editor.input_bounds.size.width;
        let element_height = editor.input_bounds.size.height;
        
        // If popover would go off the right edge of the element, show it to the left of cursor
        if pos_x + MAX_HOVER_WIDTH > element_width {
            pos_x = (element_mouse_x - MAX_HOVER_WIDTH - HOVER_OFFSET_X).max(px(10.));
        }
        
        // If popover would go off the bottom of the element, move it up
        if pos_y + MAX_HOVER_HEIGHT > element_height {
            pos_y = (element_height - MAX_HOVER_HEIGHT - px(10.)).max(px(10.));
        }

        let max_width = MAX_HOVER_WIDTH.min(element_width - pos_x - px(20.));

        deferred(
            div()
                .id("hover-popover")
                .absolute()
                .left(pos_x)
                .top(pos_y)
                .on_scroll_wheel(|_, _, cx| {
                    // Stop scroll events from propagating
                    cx.stop_propagation();
                })
                .child(
                    v_flex()
                        .w(max_width)
                        .max_h(MAX_HOVER_HEIGHT)
                        .min_w(px(200.))
                        .p_2()
                        .bg(cx.theme().popover)
                        .text_color(cx.theme().popover_foreground)
                        .border_1()
                        .border_color(cx.theme().border)
                        .rounded(cx.theme().radius)
                        .shadow_lg()
                        .overflow_hidden()
                        .child(
                            div()
                                .id("hover-popover-content")
                                .w_full()
                                .h_full()
                                .overflow_y_scroll()
                                .overflow_x_hidden()
                                .child(render_markdown("hover-content", contents, window, cx))
                        )
                )
                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                    // Close hover on click outside
                    this.editor.update(cx, |editor, cx| {
                        editor.hover_popover = None;
                        cx.notify();
                    });
                })),
        )
        .into_any_element()
    }
}

