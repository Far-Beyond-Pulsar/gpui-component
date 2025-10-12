use std::sync::Arc;

use gpui::{
    div, px, AnyElement, AnyView, App, ClickEvent, Div, DragMoveEvent, ElementId,
    InteractiveElement, IntoElement, ParentElement, Pixels, Point, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, StyleRefinement, Window,
};

use crate::{h_flex, ActiveTheme, Icon, IconName, Sizable, Size, StyledExt};

/// Data carried during tab drag operations
#[derive(Clone)]
pub struct DraggedTab {
    pub tab_id: ElementId,
    pub content: AnyView,
    pub label: SharedString,
    pub tab_bar_id: ElementId,
    pub source_index: usize,
    pub drag_start_position: Option<Point<Pixels>>,
}

/// A single draggable tab with Chrome-like behavior
#[derive(IntoElement)]
pub struct DraggableTab {
    id: ElementId,
    base: Div,
    style: StyleRefinement,
    label: SharedString,
    icon: Option<Icon>,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    content: AnyView,
    size: Size,
    selected: bool,
    closable: bool,
    on_click: Option<Arc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>>,
    on_close: Option<Arc<dyn Fn(&mut Window, &mut App) + 'static>>,
}

impl DraggableTab {
    /// Create a new draggable tab
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>, content: AnyView) -> Self {
        Self {
            id: id.into(),
            base: div(),
            style: StyleRefinement::default(),
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

    /// Set the click handler
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Arc::new(handler));
        self
    }

    /// Set the close handler
    pub fn on_close(mut self, handler: impl Fn(&mut Window, &mut App) + 'static) -> Self {
        self.on_close = Some(Arc::new(handler));
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

impl Styled for DraggableTab {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

impl RenderOnce for DraggableTab {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let height = px(36.);
        let padding_x = px(16.);
        let padding_y = px(8.);

        let (bg, fg, border_color, border_top_width) = if self.selected {
            (
                cx.theme().tab_active,
                cx.theme().tab_active_foreground,
                cx.theme().border,
                px(2.),
            )
        } else {
            (
                cx.theme().transparent,
                cx.theme().tab_foreground.opacity(0.7),
                cx.theme().transparent,
                px(1.),
            )
        };

        let tab_id = self.id.clone();
        let label = self.label.clone();
        let on_close = self.on_close.clone();

        self.base
            .id(self.id)
            .flex()
            .items_center()
            .gap_2()
            .h(height)
            .px(padding_x)
            .py(padding_y)
            .bg(bg)
            .text_color(fg)
            .border_t(border_top_width)
            .border_l_1()
            .border_r_1()
            .border_color(border_color)
            .rounded_tl(px(8.))
            .rounded_tr(px(8.))
            .cursor_default()
            .hover(|this| {
                if self.selected {
                    this
                } else {
                    this.bg(cx.theme().tab_active.opacity(0.3))
                        .border_color(cx.theme().border.opacity(0.6))
                }
            })
            .when_some(self.icon, |this, icon| this.child(icon))
            .when_some(self.prefix, |this, prefix| this.child(prefix))
            .child(
                div()
                    .flex_1()
                    .overflow_hidden()
                    .text_ellipsis()
                    .whitespace_nowrap()
                    .child(label),
            )
            .when_some(self.suffix, |this, suffix| this.child(suffix))
            .when(self.closable, |this| {
                this.child(
                    div()
                        .id(("close", tab_id))
                        .flex()
                        .items_center()
                        .justify_center()
                        .size(px(16.))
                        .rounded(px(4.))
                        .cursor_pointer()
                        .hover(|this| this.bg(cx.theme().secondary_hover))
                        .active(|this| this.bg(cx.theme().secondary_active))
                        .child(Icon::new(IconName::Close).size_3())
                        .on_click(move |_, window, cx| {
                            if let Some(on_close) = on_close.as_ref() {
                                on_close(window, cx);
                            }
                        }),
                )
            })
            .when_some(self.on_click, |this, on_click| {
                this.on_click(move |event, window, cx| on_click(event, window, cx))
            })
            .refine_style(&self.style)
    }
}
