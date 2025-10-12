use gpui::ParentElement;
use gpui::{
    div, px, size, AnyElement, AnyView, App, Bounds, Context, DragMoveEvent, ElementId,
    EventEmitter, FocusHandle, Focusable, IntoElement, Pixels, Point, Render, SharedString,
    StatefulInteractiveElement, Window, WindowBounds, WindowKind, WindowOptions,
};

use super::{DraggableTab, DraggedTab};
use crate::{h_flex, v_flex, ActiveTheme, Root, StyledExt};
use gpui::prelude::FluentBuilder;
use gpui::AppContext;
use gpui::Styled;

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

    /// Check if drag position is outside window bounds
    fn check_drag_outside(
        &mut self,
        position: Point<Pixels>,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let bounds = window.bounds();
        let margin = px(20.0);

        let is_outside = position.x < bounds.left() - margin
            || position.x > bounds.right() + margin
            || position.y < bounds.top() - margin
            || position.y > bounds.bottom() + margin;

        if is_outside != self.dragging_outside {
            self.dragging_outside = is_outside;
            cx.notify();
        }

        is_outside
    }

    /// Create a new window with the given tab
    fn create_window_with_tab(
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
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Opaque,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            ..Default::default()
        };

        let _ = cx.open_window(window_options, move |window, cx| {
            // Create new tab bar with just this tab
            let new_tab_bar = cx.new(|cx| {
                let mut bar = Self::new("new-tab-bar", window, cx);
                bar.add_tab(tab_id, label, content, true);
                bar
            });

            cx.new(|cx| Root::new(new_tab_bar.into(), window, cx))
        });
    }

    /// Reorder tab within this bar
    fn reorder_tab(&mut self, from_index: usize, to_index: usize, cx: &mut Context<Self>) {
        if from_index >= self.tabs.len() || to_index >= self.tabs.len() {
            return;
        }

        if from_index != to_index {
            let tab = self.tabs.remove(from_index);
            self.tabs.insert(to_index, tab);

            // Update selected index
            if self.selected_index == from_index {
                self.selected_index = to_index;
            } else if from_index < self.selected_index && to_index >= self.selected_index {
                self.selected_index -= 1;
            } else if from_index > self.selected_index && to_index <= self.selected_index {
                self.selected_index += 1;
            }

            cx.emit(TabBarEvent::TabReordered {
                from: from_index,
                to: to_index,
            });
            cx.notify();
        }
    }
}

impl Focusable for DraggableTabBar {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<TabBarEvent> for DraggableTabBar {}

impl Render for DraggableTabBar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity().clone();

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
                        // Tabs container (scrollable)
                        h_flex().flex_1().overflow_x_hidden().items_end().children(
                            self.tabs.iter().enumerate().map(
                                |(ix, (tab_id, label, content, closable))| {
                                    let is_selected = ix == self.selected_index;
                                    let tab_id = tab_id.clone();
                                    let label = label.clone();
                                    let content = content.clone();
                                    let closable = *closable;
                                    let view_clone = view.clone();

                                    DraggableTab::new(("tab", ix), label.clone(), content.clone())
                                        .selected(is_selected)
                                        .closable(closable)
                                        .on_click(cx.listener(move |this, _event, _window, cx| {
                                            this.set_selected(ix);
                                            cx.emit(TabBarEvent::TabSelected(ix));
                                            cx.notify();
                                        }))
                                        .on_close(cx.listener(move |this, _event, _window, cx| {
                                            this.remove_tab(ix);
                                            cx.emit(TabBarEvent::TabClosed(ix));
                                            cx.notify();
                                        }))
                                        .on_drag(
                                            DraggedTab {
                                                tab_id: tab_id.clone(),
                                                content: content.clone(),
                                                label: label.clone(),
                                                tab_bar_id: ElementId::Name(SharedString::from(format!("tab-bar-{}", view.entity_id().as_u64()))),
                                                source_index: ix,
                                                drag_start_position: None,
                                            },
                                            move |mut drag, position, _, cx| {
                                                drag.drag_start_position = Some(position);
                                                cx.stop_propagation();
                                                cx.new(|_| drag)
                                            },
                                        )
                                        .on_drag_move(cx.listener(
                                            move |this,
                                                  event: &DragMoveEvent<DraggedTab>,
                                                  window,
                                                  cx| {
                                                this.check_drag_outside(
                                                    event.event.position,
                                                    window,
                                                    cx,
                                                );
                                            },
                                        ))
                                        .drag_over::<DraggedTab>(|this, _, _, cx| {
                                            this.border_l_2().border_color(cx.theme().drag_border)
                                        })
                                        .on_drop(cx.listener(
                                            move |this, drag: &DraggedTab, window, cx| {
                                                if this.dragging_outside {
                                                    // Create new window
                                                    if let Some(position) = drag.drag_start_position
                                                    {
                                                        // Remove tab from this bar
                                                        this.remove_tab(ix);

                                                        // Create new window
                                                        let drag_clone = drag.clone();
                                                        window.defer(cx, move |_window, cx| {
                                                            Self::create_window_with_tab(
                                                                drag_clone.tab_id.clone(),
                                                                drag_clone.label.clone(),
                                                                drag_clone.content.clone(),
                                                                position,
                                                                cx,
                                                            );
                                                        });
                                                    }
                                                } else if drag.tab_bar_id
                                                    == ElementId::Name(SharedString::from(format!("tab-bar-{}", view_clone.entity_id().as_u64())))
                                                {
                                                    // Reorder within same bar
                                                    this.reorder_tab(drag.source_index, ix, cx);
                                                } else {
                                                    // Drop from another window
                                                    cx.emit(TabBarEvent::TabDropped {
                                                        tab: drag.clone(),
                                                        at_index: ix,
                                                    });
                                                }
                                            },
                                        ))
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

