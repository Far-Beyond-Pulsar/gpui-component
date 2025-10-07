use anyhow::Result;
use gpui::{Context, Entity, EntityInputHandler, Task, Window};
use lsp_types::{request::Completion, CompletionContext, CompletionItem, CompletionResponse};
use ropey::Rope;
use std::{cell::RefCell, ops::Range, rc::Rc};

use crate::input::{
    popovers::{CompletionMenu, ContextMenu},
    InputState,
};

/// A trait for providing code completions based on the current input state and context.
pub trait CompletionProvider {
    /// Fetches completions based on the given byte offset.
    ///
    /// - The `offset` is in bytes of current cursor.
    ///
    /// textDocument/completion
    ///
    /// https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#textDocument_completion
    fn completions(
        &self,
        text: &Rope,
        offset: usize,
        trigger: CompletionContext,
        window: &mut Window,
        cx: &mut Context<InputState>,
    ) -> Task<Result<CompletionResponse>>;

    fn resolve_completions(
        &self,
        _completion_indices: Vec<usize>,
        _completions: Rc<RefCell<Box<[Completion]>>>,
        _: &mut Context<InputState>,
    ) -> Task<Result<bool>> {
        Task::ready(Ok(false))
    }

    /// Determines if the completion should be triggered based on the given byte offset.
    ///
    /// This is called on the main thread.
    fn is_completion_trigger(
        &self,
        offset: usize,
        new_text: &str,
        cx: &mut Context<InputState>,
    ) -> bool;
}

impl InputState {
    pub(crate) fn handle_completion_trigger(
        &mut self,
        range: &Range<usize>,
        new_text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.completion_inserting {
            return;
        }

        let Some(provider) = self.lsp.completion_provider.clone() else {
            return;
        };

        let start = range.end;
        let new_offset = self.cursor();

        // Check if we should trigger completions (VSCode-style: trigger on most typing)
        let should_trigger = provider.is_completion_trigger(start, new_text, cx);

        if !should_trigger {
            // Not a completion trigger - but check if we need to close an open menu
            if let Some(ContextMenu::Completion(menu)) = self.context_menu.as_ref() {
                if menu.read(cx).is_open() {
                    let start_offset = menu.read(cx).trigger_start_offset.unwrap_or(start);
                    // If cursor moved before the trigger point, close the menu
                    if new_offset < start_offset {
                        menu.update(cx, |menu, cx| {
                            menu.hide(cx);
                        });
                    }
                }
            }
            return;
        }

        // Debouncing for identifier typing (not for trigger characters)
        // TODO: Add debouncing back - for now just request immediately
        let last_char = new_text.chars().last().unwrap_or(' ');
        let is_trigger_char = matches!(last_char, '.' | ':' | '<' | '(');
        
        if is_trigger_char {
            println!("🚀 Requesting completions at offset {} (trigger char)", new_offset);
        } else {
            println!("🚀 Requesting completions at offset {} (identifier)", new_offset);
        }
        
        self.request_completions_now(new_offset, start, new_text, provider, self.text.clone(), None, window, cx);
    }
    
    fn request_completions_now(
        &mut self,
        new_offset: usize,
        start: usize,
        new_text: &str,
        provider: Rc<dyn CompletionProvider>,
        text: Rope,
        existing_menu: Option<Entity<CompletionMenu>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {

        println!("🚀 Requesting completions NOW at offset {}", new_offset);

        // Create or get the menu
        let menu = existing_menu.or_else(|| {
            match self.context_menu.as_ref() {
                Some(ContextMenu::Completion(menu)) => Some(menu.clone()),
                _ => None,
            }
        }).unwrap_or_else(|| {
            let new_menu = CompletionMenu::new(cx.entity(), window, cx);
            self.context_menu = Some(ContextMenu::Completion(new_menu.clone()));
            new_menu
        });

        // Show loading state immediately (non-blocking)
        menu.update(cx, |menu, cx| {
            menu.show_loading(new_offset, cx);
        });

        // For trigger characters like :: or ., we want to filter from the CURRENT position
        // For identifier completion, we want to filter from the start of the identifier
        let last_char = new_text.chars().last().unwrap_or(' ');
        let filter_start = if matches!(last_char, '.' | ':' | '<') {
            // Trigger character - filter from current position (show all initially)
            new_offset
        } else {
            // Identifier - filter from start of the word
            menu.read(cx).trigger_start_offset.unwrap_or(start)
        };

        if new_offset < filter_start {
            return;
        }

        let query = self
            .text_for_range(
                self.range_to_utf16(&(filter_start..new_offset)),
                &mut None,
                window,
                cx,
            )
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
            
        println!("   Filter range: {}..{}, query: '{}'", filter_start, new_offset, query);
            
        _ = menu.update(cx, |menu, _| {
            menu.update_query(filter_start, query.clone());
        });

        // Determine trigger kind based on what was typed
        let last_char = new_text.chars().last();
        let (trigger_kind, trigger_char) = if last_char.map_or(false, |c| matches!(c, '.' | ':' | '<' | '(')) {
            (lsp_types::CompletionTriggerKind::TRIGGER_CHARACTER, last_char.map(|c| c.to_string()))
        } else {
            (lsp_types::CompletionTriggerKind::INVOKED, None)
        };
        
        let completion_context = CompletionContext {
            trigger_kind,
            trigger_character: trigger_char,
        };

        let provider_responses =
            provider.completions(&text, new_offset, completion_context, window, cx);
        self._context_menu_task = cx.spawn_in(window, async move |editor, cx| {
            let mut completions: Vec<CompletionItem> = vec![];
            if let Some(provider_responses) = provider_responses.await.ok() {
                match provider_responses {
                    CompletionResponse::Array(items) => completions.extend(items),
                    CompletionResponse::List(list) => completions.extend(list.items),
                }
            }

            if completions.is_empty() {
                _ = menu.update(cx, |menu, cx| {
                    menu.hide(cx);
                    cx.notify();
                });

                return Ok(());
            }

            editor
                .update_in(cx, |editor, window, cx| {
                    if !editor.focus_handle.is_focused(window) {
                        return;
                    }

                    _ = menu.update(cx, |menu, cx| {
                        menu.show(new_offset, completions, window, cx);
                        // Initially show all items (empty query)
                        menu.update_query_only("", cx);
                    });

                    cx.notify();
                })
                .ok();

            Ok(())
        });
    }
}
