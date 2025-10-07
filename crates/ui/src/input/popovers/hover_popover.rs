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
const POPOVER_GAP: Pixels = px(4.);

pub struct HoverPopover {
    editor: Entity<InputState>,
    /// The symbol range byte of the hover trigger.
    pub(crate) symbol_range: Range<usize>,
    pub(crate) hover: Rc<lsp_types::Hover>,
}

impl HoverPopover {
    pub fn new(
        editor: Entity<InputState>,
        symbol_range: Range<usize>,
        hover: &lsp_types::Hover,
        cx: &mut App,
    ) -> Entity<Self> {
        let hover = Rc::new(hover.clone());

        cx.new(|_| Self {
            editor,
            symbol_range,
            hover,
        })
    }

    pub(crate) fn is_same(&self, offset: usize) -> bool {
        self.symbol_range.contains(&offset)
    }

    /// Get the position where the popover should be rendered
    fn origin(&self, cx: &App) -> Option<Point<Pixels>> {
        let editor = self.editor.read(cx);
        let Some(last_layout) = editor.last_layout.as_ref() else {
            return None;
        };

        let Some(_last_bounds) = editor.last_bounds else {
            return None;
        };

        // Get the position of the start of the hovered symbol
        let (_, _, start_pos) = editor.line_and_position_for_offset(self.symbol_range.start);
        let Some(start_pos) = start_pos else {
            return None;
        };

        let scroll_origin = editor.scroll_handle.offset();

        // Position popover below the hovered text
        Some(
            scroll_origin + start_pos - editor.input_bounds.origin
                + Point::new(px(0.), last_layout.line_height + POPOVER_GAP),
        )
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

        let Some(pos) = self.origin(cx) else {
            return div().into_any_element();
        };

        let max_width = MAX_HOVER_WIDTH.min(window.bounds().size.width - pos.x - px(20.));

        deferred(
            div()
                .id("hover-popover")
                .absolute()
                .left(pos.x)
                .top(pos.y)
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

