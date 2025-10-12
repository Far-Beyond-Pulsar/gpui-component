use std::sync::Arc;
use std::rc::Rc;

use gpui::{
    div, px, AnyElement, AnyView, App, AppContext, ClickEvent, Div, DragMoveEvent, ElementId,
    InteractiveElement, IntoElement, ParentElement, Pixels, Point, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, StyleRefinement, Window,
};

use crate::{h_flex, ActiveTheme, Icon, IconName, IconButton, Sizable, Size, StyledExt};

/// Data carried during tab drag operations
#[derive(Clone, Debug)]
pub struct DraggedTab {
    pub tab_id: ElementId,
    pub content: AnyView,
    pub label: SharedString,
    pub tab_bar_id: ElementId,
    pub source_index: usize,
    pub drag_start_position: Option<Point<Pixels>>,
}

/// A single draggable tab with Chrome-like behavior
pub struct DraggableTab {
    pub id: ElementId,
    pub label: SharedString,
    pub icon: Option<Icon>,
    pub prefix: Option<AnyElement>,
    pub suffix: Option<AnyElement>,
    pub content: AnyView,
    pub size: Size,
    pub selected: bool,
    pub closable: bool,
    pub on_click: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
    pub on_close: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>,
}

impl DraggableTab {
    /// Create a new draggable tab
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, content: AnyView) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            icon: None,
            prefix: None,
            suffix: None,
            content,
            size: Size::default(),
            selected: false,
            closable: true,
            on_click: None,
            on_close: None,
        }
    }

    /// Set the icon for the tab
    pub fn icon(mut self, icon: impl Into<Icon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Set a prefix element (shown before label)
    pub fn prefix(mut self, prefix: impl IntoElement) -> Self {
        self.prefix = Some(prefix.into_any_element());
        self
    }

    /// Set a suffix element (shown after label, before close button)
    pub fn suffix(mut self, suffix: impl IntoElement) -> Self {
        self.suffix = Some(suffix.into_any_element());
        self
    }

    /// Set whether this tab is selected
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Set whether this tab can be closed
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Set on_click handler
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// Set on_close handler
    pub fn on_close(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_close = Some(Rc::new(handler));
        self
    }

    pub fn content(&self) -> &AnyView {
        &self.content
    }

    pub fn label(&self) -> &SharedString {
        &self.label
    }
}

impl Sizable for DraggableTab {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}

impl RenderOnce for DraggableTab {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let on_click = self.on_click.clone();
        let on_close = self.on_close.clone();
        
        h_flex()
            .id(self.id)
            .h(px(32.))
            .px_3()
            .gap_2()
            .items_center()
            .rounded_t_md()
            .border_1()
            .border_b_0()
            .border_color(if self.selected {
                cx.theme().border
            } else {
                cx.theme().transparent
            })
            .bg(if self.selected {
                cx.theme().tab_active
            } else {
                cx.theme().tab
            })
            .hover(|this| this.bg(cx.theme().tab_active))
            .when_some(on_click, |this, handler| {
                this.on_click(move |event, window, cx| {
                    (handler)(event, window, cx);
                })
            })
            .when_some(self.icon, |this, icon| this.child(icon))
            .when_some(self.prefix, |this, prefix| this.child(prefix))
            .child(self.label)
            .when_some(self.suffix, |this, suffix| this.child(suffix))
            .when(self.closable, |this| {
                this.child(
                    IconButton::new("close", IconName::Close)
                        .small()
                        .ghost()
                        .when_some(on_close, |btn, handler| {
                            btn.on_click(move |event, window, cx| {
                                cx.stop_propagation();
                                (handler)(event, window, cx);
                            })
                        })
                )
            })
    }
}
