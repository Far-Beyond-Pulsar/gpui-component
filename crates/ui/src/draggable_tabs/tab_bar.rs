use super::DraggedTab;
use crate::button::ButtonVariant;
use crate::styled::Sizable;
use crate::{button::Button, h_flex, v_flex, ActiveTheme, IconName, Root, StyledExt};
use gpui::prelude::FluentBuilder;
use gpui::AppContext;
use gpui::ParentElement;
use gpui::Styled;
use gpui::{
    div, px, size, AnyElement, AnyView, App, Bounds, Context, ElementId, EventEmitter, FocusHandle,
    Focusable, IntoElement, Pixels, Point, Render, SharedString, Window, WindowBounds, WindowKind,
    WindowOptions,
};
/// Events emitted by DraggableTabBar
#[derive(Clone, Debug)]
pub enum TabBarEvent {
    /// Tab was selected
    TabSelected(usize),
    /// Tab was closed
    TabClosed(usize),
    /// Tab was reordered within this bar
    TabReordered { from: usize, to: usize },
    /// Tab was dropped from another window
    TabDropped { tab: DraggedTab, at_index: usize },
}

/// A Chrome-like draggable tab bar
pub struct DraggableTabBar {
    _id: ElementId,
    focus_handle: FocusHandle,
    tabs: Vec<(ElementId, SharedString, AnyView, bool)>, // (id, label, content, closable)
    selected_index: usize,
    prefix: Option<AnyElement>,
    suffix: Option<AnyElement>,
    /// Track if currently dragging outside window bounds
    dragging_outside: bool,
}

impl DraggableTabBar {
    pub fn new(id: impl Into<ElementId>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            _id: id.into(),
            focus_handle: cx.focus_handle(),
            tabs: Vec::new(),
            selected_index: 0,
            prefix: None,
            suffix: None,
            dragging_outside: false,
        }
    }

    /// Add a tab to the bar
    pub fn add_tab(
        &mut self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        content: AnyView,
        closable: bool,
    ) {
        self.tabs.push((id.into(), label.into(), content, closable));
    }

    /// Remove a tab by index
    pub fn remove_tab(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.tabs.remove(index);
            if self.selected_index >= self.tabs.len() && !self.tabs.is_empty() {
                self.selected_index = self.tabs.len() - 1;
            }
        }
    }

    /// Set the selected tab index
    pub fn set_selected(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.selected_index = index;
        }
    }

    /// Get the currently selected tab content
    pub fn selected_content(&self) -> Option<&AnyView> {
        self.tabs
            .get(self.selected_index)
            .map(|(_, _, content, _)| content)
    }

    /// Set prefix element (shown before tabs)
    pub fn set_prefix(&mut self, prefix: impl IntoElement) {
        self.prefix = Some(prefix.into_any_element());
    }

    /// Set suffix element (shown after tabs)
    pub fn set_suffix(&mut self, suffix: impl IntoElement) {
        self.suffix = Some(suffix.into_any_element());
    }

    /// Create a new window with the given tab
    /*fn create_window_with_tab(
        tab_id: ElementId,
        label: SharedString,
        content: AnyView,
        position: Point<Pixels>,
        cx: &mut App,
    ) {
        let window_size = size(px(800.), px(600.));
        let title_bar_height = px(36.0);

        let window_bounds = Bounds::new(
            Point {
                x: position.x - px(100.0),
                y: position.y - title_bar_height - px(4.0),
            },
            window_size,
        );

        let window_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: None,
            window_min_size: Some(gpui::Size {
                width: px(400.),
                height: px(300.),
            }),
            kind: WindowKind::Normal,
            ..Default::default()
        };

        cx.open_window(window_options, |window, cx| {
            cx.new(|cx| {
                let mut bar = DraggableTabBar::new("window-tab-bar", window, cx);
                bar.add_tab(tab_id, label, content, true);
                bar
            })
        })
        .ok();
    }*/

    fn reorder_tab(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if from != to && from < self.tabs.len() && to < self.tabs.len() {
            let tab = self.tabs.remove(from);
            self.tabs.insert(to, tab);

            // Update selected index
            if self.selected_index == from {
                self.selected_index = to;
            } else if from < self.selected_index && to >= self.selected_index {
                self.selected_index -= 1;
            } else if from > self.selected_index && to <= self.selected_index {
                self.selected_index += 1;
            }

            cx.emit(TabBarEvent::TabReordered { from, to });
            cx.notify();
        }
    }
}

impl EventEmitter<TabBarEvent> for DraggableTabBar {}
impl Focusable for DraggableTabBar {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for DraggableTabBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                // Tab bar container
                h_flex()
                    .h(px(36.))
                    .bg(cx.theme().tab_bar)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .when_some(self.prefix.take(), |this, prefix| this.child(prefix))
                    .child(
                        // Tabs container
                        h_flex().flex_1().overflow_x_hidden().items_end().children(
                            self.tabs.iter().enumerate().map(
                                |(ix, (_tab_id, label, _content, closable))| {
                                    let is_selected = ix == self.selected_index;
                                    let label = label.clone();
                                    let closable = *closable;

                                    // Render simple tab using Button
                                    h_flex()
                                        .h(px(32.))
                                        .px_3()
                                        .gap_2()
                                        .items_center()
                                        .rounded_t_md()
                                        .border_1()
                                        .border_b_0()
                                        .border_color(if is_selected {
                                            cx.theme().border
                                        } else {
                                            cx.theme().transparent
                                        })
                                        .bg(if is_selected {
                                            cx.theme().tab_active
                                        } else {
                                            cx.theme().tab
                                        })
                                        .child(
                                            Button::new(SharedString::from(format!("tab-{}", ix)))
                                                .label(label.clone())
                                                .on_click(cx.listener(
                                                    move |this, _event, _window, cx| {
                                                        this.set_selected(ix);
                                                        cx.emit(TabBarEvent::TabSelected(ix));
                                                        cx.notify();
                                                    },
                                                )),
                                        )
                                        .when(closable, |this| {
                                            this.child(
                                                Button::new(SharedString::from(format!(
                                                    "tab-close-{}",
                                                    ix
                                                )))
                                                .icon(IconName::Close)
                                                .small()
                                                .on_click(cx.listener(
                                                    move |this, _event, _window, cx| {
                                                        this.remove_tab(ix);
                                                        cx.emit(TabBarEvent::TabClosed(ix));
                                                        cx.notify();
                                                    },
                                                )),
                                            )
                                        })
                                },
                            ),
                        ),
                    )
                    .when_some(self.suffix.take(), |this, suffix| this.child(suffix)),
            )
            .child(
                // Content area
                div()
                    .flex_1()
                    .overflow_hidden()
                    .when_some(self.selected_content().cloned(), |this, content| {
                        this.child(content)
                    }),
            )
    }
}
