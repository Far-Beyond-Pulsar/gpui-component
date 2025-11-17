use gpui::{prelude::*, div, px, Axis, Context, DismissEvent, Entity, EventEmitter, KeyDownEvent, MouseButton, Render, Window};
use ui::{h_flex, input::{InputEvent, InputState, TextInput}, v_flex, ActiveTheme as _, Icon, IconName, StyledExt};

use super::palette::{PaletteDelegate, PaletteItem};

struct CategoryState {
    name: String,
    expanded: bool,
}

/// Generic palette component that works with any PaletteDelegate
/// Handles all rendering - delegates just provide data
pub struct GenericPalette<D: PaletteDelegate> {
    pub search_input: Entity<InputState>,
    delegate: D,
    filtered_categories: Vec<(String, Vec<D::Item>)>,
    category_states: Vec<CategoryState>,
    selected_index: usize,
    show_docs: bool,
}

impl<D: PaletteDelegate> EventEmitter<DismissEvent> for GenericPalette<D> {}

impl<D: PaletteDelegate> GenericPalette<D> {
    pub fn new(mut delegate: D, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Get all the data we need from delegate before moving it
        let placeholder = delegate.placeholder().to_string();
        let categories = delegate.categories();
        let collapsed = delegate.categories_collapsed_by_default();

        let search_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder(&placeholder, window, cx);
            state
        });

        let category_states: Vec<CategoryState> = categories
            .iter()
            .map(|(name, _)| CategoryState {
                name: name.clone(),
                expanded: !collapsed,
            })
            .collect();

        let filtered_categories = categories.clone();

        // Subscribe to input changes
        cx.subscribe(&search_input, |this, _input, event: &InputEvent, cx| {
            match event {
                InputEvent::Change => {
                    let query = this.search_input.read(cx).text().to_string();
                    this.update_filter(&query);
                    cx.notify();
                }
                _ => {}
            }
        })
        .detach();

        Self {
            search_input,
            delegate,
            filtered_categories,
            category_states,
            selected_index: 0,
            show_docs: false,
        }
    }

    pub fn delegate(&self) -> &D {
        &self.delegate
    }

    pub fn delegate_mut(&mut self) -> &mut D {
        &mut self.delegate
    }

    fn update_filter(&mut self, query: &str) {
        self.filtered_categories = self.delegate.filter(query);

        // Update category states
        let collapsed = self.delegate.categories_collapsed_by_default();
        self.category_states = self.filtered_categories
            .iter()
            .map(|(name, items)| CategoryState {
                name: name.clone(),
                // Auto-expand categories with matches when searching, or respect default
                expanded: if query.is_empty() { !collapsed } else { !items.is_empty() },
            })
            .collect();

        self.selected_index = 0;
    }

    fn get_all_visible_items(&self) -> Vec<D::Item> {
        self.filtered_categories
            .iter()
            .enumerate()
            .filter(|(idx, _)| self.category_states.get(*idx).map(|s| s.expanded).unwrap_or(true))
            .flat_map(|(_, (_, items))| items.iter().cloned())
            .collect()
    }

    fn select_item(&mut self, cx: &mut Context<Self>) {
        let visible_items = self.get_all_visible_items();
        if let Some(item) = visible_items.get(self.selected_index) {
            self.delegate.confirm(item);
            cx.emit(DismissEvent);
        }
    }

    fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let visible_items = self.get_all_visible_items();
        if visible_items.is_empty() {
            return;
        }

        let new_index = ((self.selected_index as isize) + delta)
            .rem_euclid(visible_items.len() as isize) as usize;

        self.selected_index = new_index;
        cx.notify();
    }

    fn toggle_category(&mut self, category_index: usize, cx: &mut Context<Self>) {
        if let Some(state) = self.category_states.get_mut(category_index) {
            state.expanded = !state.expanded;
            cx.notify();
        }
    }
}

impl<D: PaletteDelegate> Render for GenericPalette<D> {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_index = self.selected_index;
        let visible_items = self.get_all_visible_items();
        let selected_item = visible_items.get(selected_index).cloned();
        let show_docs = self.show_docs && self.delegate.supports_docs();

        // Outer wrapper: full-screen darkened background overlay
        div()
            .absolute()
            .top_0()
            .left_0()
            .w_full()
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::rgba(0x00000099))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|_this, _event, _window, cx| {
                    cx.emit(DismissEvent);
                    cx.stop_propagation();
                }),
            )
            .child(
                h_flex()
                    .gap_0()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    .on_key_down(cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                        match event.keystroke.key.as_str() {
                            "down" | "arrowdown" => {
                                this.move_selection(1, cx);
                                cx.stop_propagation();
                            }
                            "up" | "arrowup" => {
                                this.move_selection(-1, cx);
                                cx.stop_propagation();
                            }
                            "enter" | "return" => {
                                this.select_item(cx);
                                cx.stop_propagation();
                            }
                            "escape" => {
                                cx.emit(DismissEvent);
                                cx.stop_propagation();
                            }
                            " " | "space" if this.delegate.supports_docs() => {
                                this.show_docs = !this.show_docs;
                                cx.notify();
                                cx.stop_propagation();
                            }
                            _ => {}
                        }
                    }))
                    // Documentation panel (shown on the LEFT when space is pressed)
                    .when(show_docs, |this| {
                        this.child(
                            v_flex()
                                .w(px(400.0))
                                .max_h(px(600.0))
                                .bg(cx.theme().background)
                                .border_1()
                                .border_r_0()
                                .border_color(cx.theme().border)
                                .rounded_l(px(8.0))
                                .shadow_lg()
                                .overflow_hidden()
                                .child(
                                    // Header
                                    h_flex()
                                        .p_3()
                                        .border_b_1()
                                        .border_color(cx.theme().border)
                                        .gap_2()
                                        .items_center()
                                        .child(
                                            Icon::new(IconName::SubmitDocument)
                                                .size(px(18.0))
                                                .text_color(cx.theme().muted_foreground),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .font_semibold()
                                                .text_color(cx.theme().foreground)
                                                .child("Documentation"),
                                        ),
                                )
                                .child(
                                    // Documentation content
                                    div()
                                        .flex_1()
                                        .overflow_hidden()
                                        .child(
                                            v_flex()
                                                .p_4()
                                                .gap_4()
                                                .scrollable(Axis::Vertical)
                                                .when_some(selected_item.as_ref().and_then(|item| item.documentation()), |this, doc_text| {
                                                    this.child(
                                                        div()
                                                            .text_sm()
                                                            .text_color(cx.theme().foreground)
                                                            .child(doc_text)
                                                    )
                                                })
                                                .when(selected_item.as_ref().and_then(|item| item.documentation()).is_none(), |this| {
                                                    this.child(
                                                        div()
                                                            .flex_1()
                                                            .flex()
                                                            .items_center()
                                                            .justify_center()
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .text_color(
                                                                        cx.theme().muted_foreground,
                                                                    )
                                                                    .child(
                                                                        "No documentation available",
                                                                    ),
                                                            )
                                                    )
                                                }),
                                        ),
                                )
                                .child(
                                    // Footer hint
                                    div()
                                        .p_2()
                                        .border_t_1()
                                        .border_color(cx.theme().border)
                                        .bg(cx.theme().muted.opacity(0.05))
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_center()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("Press Space to toggle"),
                                        ),
                                ),
                        )
                    })
                    // Main list panel
                    .child(
                        v_flex()
                            .w(px(600.0))
                            .max_h(px(500.0))
                            .bg(cx.theme().background)
                            .border_1()
                            .border_color(cx.theme().border)
                            .when(show_docs, |this| this.border_l_0().rounded_r(px(8.0)))
                            .when(!show_docs, |this| this.rounded(px(8.0)))
                            .shadow_lg()
                            .overflow_hidden()
                            .child(
                                // Search input
                                h_flex()
                                    .p_3()
                                    .border_b_1()
                                    .border_color(cx.theme().border)
                                    .child(
                                        TextInput::new(&self.search_input)
                                            .appearance(false)
                                            .bordered(false)
                                            .prefix(
                                                Icon::new(IconName::Search)
                                                    .size(px(18.0))
                                                    .text_color(cx.theme().muted_foreground),
                                            )
                                            .w_full(),
                                    ),
                            )
                            .child(
                                // Item list with categories
                                div()
                                    .flex_1()
                                    .overflow_hidden()
                                    .child(
                                        v_flex()
                                            .gap_0p5()
                                            .p_2()
                                            .scrollable(Axis::Vertical)
                                            .children({
                                                let mut item_index = 0;
                                                let has_categories = self
                                                    .filtered_categories
                                                    .iter()
                                                    .any(|(name, _)| !name.is_empty());

                                                self.filtered_categories
                                                    .iter()
                                                    .enumerate()
                                                    .flat_map(|(cat_idx, (cat_name, items))| {
                                                        let mut elements = Vec::new();

                                                        // Category header (only if category name is not empty)
                                                        if !cat_name.is_empty() && has_categories {
                                                            let expanded = self
                                                                .category_states
                                                                .get(cat_idx)
                                                                .map(|s| s.expanded)
                                                                .unwrap_or(true);

                                                            elements.push(
                                                                h_flex()
                                                                    .w_full()
                                                                    .px_2()
                                                                    .py_2()
                                                                    .gap_2()
                                                                    .items_center()
                                                                    .cursor_pointer()
                                                                    .hover(|s| {
                                                                        s.bg(cx
                                                                            .theme()
                                                                            .muted
                                                                            .opacity(0.1))
                                                                    })
                                                                    .on_mouse_down(
                                                                        MouseButton::Left,
                                                                        cx.listener(
                                                                            move |this,
                                                                                  _,
                                                                                  _,
                                                                                  cx| {
                                                                                this.toggle_category(
                                                                                    cat_idx, cx,
                                                                                );
                                                                            },
                                                                        ),
                                                                    )
                                                                    .child(
                                                                        Icon::new(if expanded {
                                                                            IconName::ChevronDown
                                                                        } else {
                                                                            IconName::ChevronRight
                                                                        })
                                                                        .size(px(14.0))
                                                                        .text_color(
                                                                            cx.theme().muted_foreground,
                                                                        ),
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .text_xs()
                                                                            .font_semibold()
                                                                            .text_color(
                                                                                cx.theme().foreground,
                                                                            )
                                                                            .child(cat_name.clone()),
                                                                    )
                                                                    .child(
                                                                        div()
                                                                            .text_xs()
                                                                            .text_color(
                                                                                cx.theme().muted_foreground,
                                                                            )
                                                                            .child(format!(
                                                                                "({})",
                                                                                items.len()
                                                                            )),
                                                                    )
                                                                    .into_any_element(),
                                                            );

                                                            // Items (if expanded)
                                                            if expanded {
                                                                for item in items {
                                                                    let is_selected =
                                                                        item_index == selected_index;
                                                                    let current_item_index = item_index;

                                                                    elements.push(
                                                                        self.render_item(item, is_selected, current_item_index, true, cx)
                                                                    );

                                                                    item_index += 1;
                                                                }
                                                            }
                                                        } else {
                                                            // No category name, render items directly
                                                            for item in items {
                                                                let is_selected =
                                                                    item_index == selected_index;
                                                                let current_item_index = item_index;

                                                                elements.push(
                                                                    self.render_item(item, is_selected, current_item_index, false, cx)
                                                                );

                                                                item_index += 1;
                                                            }
                                                        }

                                                        elements
                                                    })
                                                    .collect::<Vec<_>>()
                                            }),
                                    ),
                            )
                            .when(self.get_all_visible_items().is_empty(), |this| {
                                this.child(
                                    div()
                                        .flex_1()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .p_8()
                                        .child(
                                            v_flex()
                                                .items_center()
                                                .gap_2()
                                                .child(
                                                    Icon::new(IconName::Search)
                                                        .size(px(48.0))
                                                        .text_color(
                                                            cx.theme().muted_foreground.opacity(0.3),
                                                        ),
                                                )
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .text_color(cx.theme().muted_foreground)
                                                        .child("No items found"),
                                                ),
                                        ),
                                )
                            }),
                    ),
            )
    }
}

impl<D: PaletteDelegate> GenericPalette<D> {
    fn render_item(
        &self,
        item: &D::Item,
        is_selected: bool,
        item_index: usize,
        indented: bool,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        h_flex()
            .w_full()
            .px_3()
            .py_2()
            .when(indented, |this| this.ml_4())
            .rounded(px(6.0))
            .gap_3()
            .items_center()
            .cursor_pointer()
            .when(is_selected, |this| this.bg(cx.theme().primary.opacity(0.15)))
            .hover(|s| s.bg(cx.theme().muted.opacity(0.2)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    this.selected_index = item_index;
                    this.select_item(cx);
                }),
            )
            .on_mouse_move(cx.listener(move |this, _, _, cx| {
                if this.selected_index != item_index {
                    this.selected_index = item_index;
                    cx.notify();
                }
            }))
            .child(
                Icon::new(item.icon())
                    .size(px(20.0))
                    .text_color(if is_selected {
                        cx.theme().primary
                    } else {
                        cx.theme().muted_foreground
                    }),
            )
            .child(
                v_flex()
                    .flex_1()
                    .gap_0p5()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(if is_selected {
                                cx.theme().foreground
                            } else {
                                cx.theme().foreground.opacity(0.9)
                            })
                            .child(item.name().to_string()),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(item.description().to_string()),
                    ),
            )
            .into_any_element()
    }
}
