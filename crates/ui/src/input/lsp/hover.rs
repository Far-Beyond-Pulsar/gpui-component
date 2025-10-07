use anyhow::Result;
use gpui::{App, Context, MouseMoveEvent, Point, Pixels, Task, Window};
use ropey::Rope;
use std::time::Duration;

use crate::input::{popovers::HoverPopover, InputState, RopeExt};

const HOVER_REQUEST_DELAY: Duration = Duration::from_millis(500); // Delay before requesting hover from LSP

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
    /// Handle hover trigger LSP request with delay.
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

        // Start async task with delay before requesting hover from LSP
        let text = self.text.clone();
        let editor = cx.entity();
        
        self.lsp._hover_task = cx.spawn_in(window, async move |_, cx| {
            // Wait before requesting hover info
            cx.background_executor().timer(HOVER_REQUEST_DELAY).await;
            
            // Request hover info from LSP (this is async and non-blocking)
            // Note: We can't pass window/cx into the async task, so we recreate the call
            let result = {
                // Get the provider again in the async context
                let provider_task = cx.update(|window, cx| {
                    editor.read(cx).lsp.hover_provider.clone()
                        .map(|p| p.hover(&text, offset, window, cx))
                }).ok().flatten();
                
                if let Some(task) = provider_task {
                    task.await.ok().flatten()
                } else {
                    None
                }
            };
            
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
                        
                        // Create hover popover (it will show after its own internal delay)
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
