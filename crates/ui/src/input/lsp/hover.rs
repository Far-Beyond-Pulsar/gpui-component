use anyhow::Result;
use gpui::{App, Context, MouseMoveEvent, Point, Pixels, Task, Window};
use ropey::Rope;

use crate::input::{popovers::HoverPopover, InputState, RopeExt};

/// Hover provider
///
/// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_hover
pub trait HoverProvider {
    /// textDocument/hover
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_hover
    fn hover(
        &self,
        _text: &Rope,
        _offset: usize,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Task<Result<Option<lsp_types::Hover>>>;
}

impl InputState {
    /// Handle hover trigger LSP request.
    /// Requests immediately from LSP and uses popup delay to mask response time.
    pub(super) fn handle_hover_popover(
        &mut self,
        offset: usize,
        mouse_position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) {
        if self.selecting {
            return;
        }

        let Some(provider) = self.lsp.hover_provider.clone() else {
            return;
        };

        // Check if we already have a hover popover for this location
        if let Some(hover_popover) = self.hover_popover.as_ref() {
            if hover_popover.read(cx).is_same(offset) {
                return;
            }
            
            // Check if mouse is inside the current hover popover
            if hover_popover.read(cx).contains_point(mouse_position) {
                // Don't hide if mouse is in the popover
                return;
            }
        }

        // Clear any existing hover popover when moving to a new location
        self.hover_popover = None;

        // Request hover info from LSP IMMEDIATELY (async, non-blocking)
        // The popup delay will mask the response time
        let text = self.text.clone();
        let task = provider.hover(&text, offset, window, cx);
        let editor = cx.entity();
        
        self.lsp._hover_task = cx.spawn_in(window, async move |_, cx| {
            // LSP request is already in flight, just wait for it
            let result = task.await?;
            
            // Process the result
            _ = editor.update(cx, |editor, cx| {
                match result {
                    Some(hover) => {
                        let mut symbol_range = text.word_range(offset).unwrap_or(offset..offset);
                        
                        if let Some(range) = hover.range {
                            let start = text.position_to_offset(&range.start);
                            let end = text.position_to_offset(&range.end);
                            symbol_range = start..end;
                        }
                        
                        // Create hover popover (it will show after its internal delay)
                        // By the time the delay is done, LSP has likely already responded
                        let hover_popover = HoverPopover::new(cx.entity(), symbol_range, &hover, mouse_position, cx);
                        editor.hover_popover = Some(hover_popover);
                        cx.notify();
                    }
                    None => {
                        editor.hover_popover = None;
                    }
                }
            });

            Ok(())
        });
    }
}
