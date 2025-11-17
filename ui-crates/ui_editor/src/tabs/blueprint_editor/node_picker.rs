use gpui::{
    prelude::*,
    div,
    px,
    point,
    Axis,
    Context,
    DismissEvent,
    Entity,
    EventEmitter,
    KeyDownEvent,
    MouseButton,
    Render,
    Window,
};
use ui::{
    h_flex,
    input::{ InputState, InputEvent },
    input::TextInput,
    v_flex,
    ActiveTheme as _,
    Icon,
    IconName,
    StyledExt,
};
use crate::tabs::blueprint_editor::{ NodeDefinition, NodeCategory, NodeDefinitions };

#[derive(Clone, Debug)]
pub struct NodeSelected {
    pub node_def: NodeDefinition,
    pub position: (f32, f32), // Graph space position where node should be created
}

pub struct CategoryState {
    pub category: NodeCategory,
    pub expanded: bool,
    pub filtered_nodes: Vec<NodeDefinition>,
}

pub struct NodePicker {
    pub search_input: Entity<InputState>,
    categories: Vec<CategoryState>,
    selected_index: usize,
    spawn_position: (f32, f32),
    show_docs: bool,
}

impl EventEmitter<NodeSelected> for NodePicker {}
impl EventEmitter<DismissEvent> for NodePicker {}

impl NodePicker {
    pub fn new(spawn_position: (f32, f32), window: &mut Window, cx: &mut Context<Self>) -> Self {
        let search_input = cx.new(|cx| {
            let mut state = InputState::new(window, cx);
            state.set_placeholder("Search nodes...", window, cx);
            state
        });

        // Load nodes from definitions
        let node_defs = NodeDefinitions::load();
        let categories: Vec<CategoryState> = node_defs.categories
            .iter()
            .map(|cat| CategoryState {
                category: cat.clone(),
                expanded: false, // Start with all categories collapsed
                filtered_nodes: cat.nodes.clone(),
            })
            .collect();

        // Subscribe to input changes to update the filter
        cx.subscribe(&search_input, |this, _input, event: &InputEvent, cx| {
            match event {
                InputEvent::Change => {
                    let query = this.search_input.read(cx).text().to_string();
                    this.update_filter(&query);
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => {
                    this.select_node(cx);
                }
                _ => {}
            }
        }).detach();

        Self {
            search_input,
            categories,
            selected_index: 0,
            spawn_position,
            show_docs: false,
        }
    }

    fn update_filter(&mut self, query: &str) {
        if query.is_empty() {
            // Reset to show all nodes in each category
            for cat_state in &mut self.categories {
                cat_state.filtered_nodes = cat_state.category.nodes.clone();
                cat_state.expanded = true;
            }
        } else {
            let query_lower = query.to_lowercase();
            for cat_state in &mut self.categories {
                cat_state.filtered_nodes = cat_state.category.nodes
                    .iter()
                    .filter(|node| {
                        // Search in name and description
                        node.name.to_lowercase().contains(&query_lower) ||
                            node.description.to_lowercase().contains(&query_lower)
                    })
                    .cloned()
                    .collect();
                // Auto-expand categories with matches when searching
                cat_state.expanded = !cat_state.filtered_nodes.is_empty();
            }
        }
        self.selected_index = 0;
    }

    fn get_all_visible_nodes(&self) -> Vec<NodeDefinition> {
        self.categories
            .iter()
            .filter(|cat| cat.expanded)
            .flat_map(|cat| cat.filtered_nodes.iter().cloned())
            .collect()
    }

    fn select_node(&mut self, cx: &mut Context<Self>) {
        let visible_nodes = self.get_all_visible_nodes();
        if let Some(node) = visible_nodes.get(self.selected_index) {
            cx.emit(NodeSelected {
                node_def: node.clone(),
                position: self.spawn_position,
            });
        }
    }

    fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let visible_nodes = self.get_all_visible_nodes();
        if visible_nodes.is_empty() {
            return;
        }

        let new_index = ((self.selected_index as isize) + delta).rem_euclid(
            visible_nodes.len() as isize
        ) as usize;

        self.selected_index = new_index;
        cx.notify();
    }

    fn toggle_category(&mut self, category_index: usize, cx: &mut Context<Self>) {
        if let Some(cat_state) = self.categories.get_mut(category_index) {
            cat_state.expanded = !cat_state.expanded;
            cx.notify();
        }
    }

    fn get_icon_for_category(_category: &NodeCategory) -> IconName {
        IconName::Code
    }
}

impl Render for NodePicker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected_index = self.selected_index;
        let visible_nodes = self.get_all_visible_nodes();
        let selected_node = visible_nodes.get(selected_index).cloned();
        let show_docs = self.show_docs;

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
            .on_mouse_down(MouseButton::Left, cx.listener(|_this, _event, _window, cx| {
                cx.emit(DismissEvent);
                cx.stop_propagation();
            }))
            .child(h_flex()
            .gap_0()
            .on_mouse_down(MouseButton::Left, |_, _, cx| {
                cx.stop_propagation();
            })
            .on_key_down(
                cx.listener(|this, event: &KeyDownEvent, _window, cx| {
                    match event.keystroke.key.as_str() {
                        "down" | "arrowdown" => {
                            this.move_selection(1, cx);
                            cx.stop_propagation();
                        }
                        "up" | "arrowup" => {
                            this.move_selection(-1, cx);
                            cx.stop_propagation();
                        }
                        "escape" => {
                            cx.emit(DismissEvent);
                            cx.stop_propagation();
                        }
                        " " | "space" => {
                            this.show_docs = !this.show_docs;
                            cx.notify();
                            cx.stop_propagation();
                        }
                        _ => {}
                    }
                })
            )
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
                                        .text_color(cx.theme().muted_foreground)
                                )
                                .child(
                                    div()
                                        .text_sm()
                                        .font_semibold()
                                        .text_color(cx.theme().foreground)
                                        .child("Documentation")
                                )
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
                                        .when_some(selected_node.clone(), |this, node| {
                                    this.child(
                                        v_flex()
                                            .gap_3()
                                            // Node name
                                            .child(
                                                div()
                                                    .text_lg()
                                                    .font_bold()
                                                    .text_color(cx.theme().foreground)
                                                    .child(node.name.clone())
                                            )
                                            // Description
                                            .child(
                                                v_flex()
                                                    .gap_1()
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .font_semibold()
                                                            .text_color(cx.theme().foreground)
                                                            .child("Description")
                                                    )
                                                    .child(
                                                        div()
                                                            .text_sm()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child(node.description.clone())
                                                    )
                                            )
                                            // Input pins
                                            .when(!node.inputs.is_empty(), |this| {
                                                this.child(
                                                    v_flex()
                                                        .gap_2()
                                                        .child(
                                                            div()
                                                                .text_sm()
                                                                .font_semibold()
                                                                .text_color(cx.theme().foreground)
                                                                .child("Input Pins")
                                                        )
                                                        .children(
                                                            node.inputs.iter().map(|pin| {
                                                                h_flex()
                                                                    .gap_2()
                                                                    .items_start()
                                                                    .child(
                                                                        div()
                                                                            .w(px(8.0))
                                                                            .h(px(8.0))
                                                                            .rounded_full()
                                                                            .bg(
                                                                                cx
                                                                                    .theme()
                                                                                    .primary.opacity(
                                                                                        0.6
                                                                                    )
                                                                            )
                                                                            .mt(px(4.0))
                                                                    )
                                                                    .child(
                                                                        v_flex()
                                                                            .gap_0p5()
                                                                            .child(
                                                                                div()
                                                                                    .text_sm()
                                                                                    .font_medium()
                                                                                    .text_color(
                                                                                        cx.theme().foreground
                                                                                    )
                                                                                    .child(
                                                                                        pin.name.clone()
                                                                                    )
                                                                            )
                                                                            .child(
                                                                                div()
                                                                                    .text_xs()
                                                                                    .text_color(
                                                                                        cx.theme().muted_foreground
                                                                                    )
                                                                                    .child(
                                                                                        format!(
                                                                                            "Type: {:?}",
                                                                                            pin.pin_type
                                                                                        )
                                                                                    )
                                                                            )
                                                                    )
                                                            })
                                                        )
                                                )
                                            })
                                            // Output pins
                                            .when(!node.outputs.is_empty(), |this| {
                                                this.child(
                                                    v_flex()
                                                        .gap_2()
                                                        .child(
                                                            div()
                                                                .text_sm()
                                                                .font_semibold()
                                                                .text_color(cx.theme().foreground)
                                                                .child("Output Pins")
                                                        )
                                                        .children(
                                                            node.outputs.iter().map(|pin| {
                                                                h_flex()
                                                                    .gap_2()
                                                                    .items_start()
                                                                    .child(
                                                                        div()
                                                                            .w(px(8.0))
                                                                            .h(px(8.0))
                                                                            .rounded_full()
                                                                            .bg(
                                                                                cx
                                                                                    .theme()
                                                                                    .accent.opacity(
                                                                                        0.6
                                                                                    )
                                                                            )
                                                                            .mt(px(4.0))
                                                                    )
                                                                    .child(
                                                                        v_flex()
                                                                            .gap_0p5()
                                                                            .child(
                                                                                div()
                                                                                    .text_sm()
                                                                                    .font_medium()
                                                                                    .text_color(
                                                                                        cx.theme().foreground
                                                                                    )
                                                                                    .child(
                                                                                        pin.name.clone()
                                                                                    )
                                                                            )
                                                                            .child(
                                                                                div()
                                                                                    .text_xs()
                                                                                    .text_color(
                                                                                        cx.theme().muted_foreground
                                                                                    )
                                                                                    .child(
                                                                                        format!(
                                                                                            "Type: {:?}",
                                                                                            pin.pin_type
                                                                                        )
                                                                                    )
                                                                            )
                                                                    )
                                                            })
                                                        )
                                                )
                                            })
                                            // Usage hint
                                            .child(
                                                div()
                                                    .p_3()
                                                    .rounded(px(6.0))
                                                    .bg(cx.theme().muted.opacity(0.1))
                                                    .border_1()
                                                    .border_color(cx.theme().border)
                                                    .child(
                                                        div()
                                                            .text_xs()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child(
                                                                "Press Enter to place this node"
                                                            )
                                                    )
                                            )
                                    )
                                })
                                .when(selected_node.is_none(), |this| {
                                    this.child(
                                        div()
                                            .flex_1()
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("Select a node to view documentation")
                                            )
                                    )
                                })
                                )
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
                                        .child("Press Space to toggle")
                                )
                        )
                )
            })
            // Main node list panel
            .child(
                v_flex()
                    .w(px(500.0))
                    .max_h(px(600.0))
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
                                            .text_color(cx.theme().muted_foreground)
                                    )
                                    .w_full()
                            )
                    )
                    .child(
                        // Node list with categories
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .child(
                                v_flex()
                                    .gap_0p5()
                                    .p_2()
                                    .scrollable(Axis::Vertical)
                                    .children({
                                let visible_nodes = self.get_all_visible_nodes();
                                let mut node_index = 0;

                                self.categories.iter().enumerate().flat_map(|(cat_idx, cat_state)| {
                                    let mut elements = Vec::new();

                                    // Category header
                                    let expanded = cat_state.expanded;
                                    let has_nodes = !cat_state.filtered_nodes.is_empty();

                                    elements.push(
                                        h_flex()
                                            .w_full()
                                            .px_2()
                                            .py_2()
                                            .gap_2()
                                            .items_center()
                                            .cursor_pointer()
                                            .hover(|s| s.bg(cx.theme().muted.opacity(0.1)))
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(move |this, _, _, cx| {
                                                    this.toggle_category(cat_idx, cx);
                                                })
                                            )
                                            .child(
                                                Icon::new(if expanded { IconName::ChevronDown } else { IconName::ChevronRight })
                                                    .size(px(14.0))
                                                    .text_color(cx.theme().muted_foreground)
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .font_semibold()
                                                    .text_color(cx.theme().foreground)
                                                    .child(cat_state.category.name.clone())
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(format!("({})", cat_state.filtered_nodes.len()))
                                            )
                                            .into_any_element()
                                    );

                                    // Nodes in category (if expanded)
                                    if expanded && has_nodes {
                                        for node in &cat_state.filtered_nodes {
                                            let is_selected = node_index == selected_index;
                                            let node_def = node.clone();
                                            let current_node_index = node_index;
                                            let icon = IconName::Code;

                                            elements.push(
                                                h_flex()
                                                    .w_full()
                                                    .px_3()
                                                    .py_2()
                                                    .ml_4()
                                                    .rounded(px(6.0))
                                                    .gap_3()
                                                    .items_center()
                                                    .cursor_pointer()
                                                    .when(is_selected, |this| {
                                                        this.bg(cx.theme().primary.opacity(0.15))
                                                    })
                                                    .hover(|s| s.bg(cx.theme().muted.opacity(0.2)))
                                                    .on_mouse_down(
                                                        MouseButton::Left,
                                                        cx.listener(move |this, _, _, cx| {
                                                            this.selected_index = current_node_index;
                                                            this.select_node(cx);
                                                        })
                                                    )
                                                    .on_mouse_move(
                                                        cx.listener(move |this, _, _, cx| {
                                                            if this.selected_index != current_node_index {
                                                                this.selected_index = current_node_index;
                                                                cx.notify();
                                                            }
                                                        })
                                                    )
                                                    .child(
                                                        Icon::new(icon)
                                                            .size(px(20.0))
                                                            .text_color(
                                                                if is_selected {
                                                                    cx.theme().primary
                                                                } else {
                                                                    cx.theme().muted_foreground
                                                                }
                                                            )
                                                    )
                                                    .child(
                                                        v_flex()
                                                            .flex_1()
                                                            .gap_0p5()
                                                            .child(
                                                                div()
                                                                    .text_sm()
                                                                    .font_semibold()
                                                                    .text_color(
                                                                        if is_selected {
                                                                            cx.theme().foreground
                                                                        } else {
                                                                            cx.theme().foreground.opacity(0.9)
                                                                        }
                                                                    )
                                                                    .child(node_def.name.clone())
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(cx.theme().muted_foreground)
                                                                    .child(node_def.description.clone())
                                                            )
                                                    )
                                                    .into_any_element()
                                            );

                                            node_index += 1;
                                        }
                                    }

                                    elements
                                }).collect::<Vec<_>>()
                            })
                    )
                    .when(self.get_all_visible_nodes().is_empty(), |this| {
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
                                                    cx.theme().muted_foreground.opacity(0.3)
                                                )
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("No nodes found")
                                        )
                                )
                        )
                    })
                    )
            ))
    }
}
