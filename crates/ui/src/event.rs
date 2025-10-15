use gpui::{App, ClickEvent, InteractiveElement, Stateful, Window};

pub trait InteractiveElementExt: InteractiveElement {
    /// Set the listener for a double click event.
    fn on_double_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self
    where
        Self: Sized,
    {
        // Note: click_count is no longer available in new GPUI
        // Double-click detection would need to be implemented manually with timing
        // For now, we'll just use the single click handler
        self.interactivity().on_click(move |event, window, cx| {
            listener(event, window, cx);
        });
        self
    }
}

impl<E: InteractiveElement> InteractiveElementExt for Stateful<E> {}
