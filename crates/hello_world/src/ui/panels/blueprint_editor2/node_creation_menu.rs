use gpui::*;
use gpui::ElementId;
use gpui::Axis;
use gpui::prelude::{FluentBuilder, InteractiveElement, StatefulInteractiveElement, Styled};
use gpui_component::{
    h_flex, v_flex,
    ActiveTheme as _, StyledExt,
    Icon, IconName,
    input::{InputState, InputEvent, TextInput},
    tooltip::Tooltip,
    text::TextView,
};
use gpui::div;
use std::collections::HashMap;
use std::rc::Rc;

use super::{NodeDefinitions, NodeCategory, NodeDefinition, BlueprintNode};
use crate::graph::Position;

/// Helper to create tooltip function that properly manages memory
fn create_markdown_tooltip(description: Rc<str>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    move |window, cx| {
        let desc_clone = description.clone();
        Tooltip::element(move |window, cx| {
            v_flex()
                .w(px(400.0))  // Fixed width for better text wrapping
                .h(px(300.0))  // Increased height for more content
                .p_2()
                .overflow_hidden()  // Ensure content doesn't escape
                .child(
                    v_flex()
                        .w_full()
                        .h_full()
                        .scrollable(Axis::Vertical)  // Enable vertical scrolling
                        .child(
                            TextView::markdown(
                                "node-tooltip",
                                desc_clone.as_ref(),
                                window,
                                cx
                            )
                        )
                )
        }).build(window, cx)
    }
}


/// Unreal-like node creation context menu
pub struct NodeCreationMenu {
    focus_handle: FocusHandle,
    search_query: String,
    search_input_state: Entity<InputState>,
    expanded_categories: HashMap<String, bool>,
    target_position: Point<f32>,
    filtered_categories: Vec<FilteredCategory>,
}

#[derive(Clone)]
struct FilteredCategory {
    name: String,
    color: String,
    nodes: Vec<FilteredNode>,
    is_expanded: bool,
    match_count: usize,
}

#[derive(Clone)]
struct FilteredNode {
    definition: NodeDefinition,
    highlighted_name: String,
    highlighted_description: String,
}

impl NodeCreationMenu {
    pub fn new(target_position: Point<f32>, search_input_state: Entity<InputState>, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let mut menu = Self {
            focus_handle,
            search_input_state: search_input_state.clone(),
            search_query: String::new(),
            expanded_categories: HashMap::new(),
            target_position,
            filtered_categories: Vec::new(),
        };

        // Subscribe to search input events for real-time filtering
        cx.subscribe(&search_input_state, |this, input_state, event: &InputEvent, cx| {
            match event {
                InputEvent::Change => {
                    let text = input_state.read(cx).value().to_string();
                    this.search_query = text;
                    this.update_filtered_categories();
                    cx.notify();
                }
                _ => {}
            }
        }).detach();

        menu.update_filtered_categories();

        // Auto-focus the search input when menu opens
        // The focus will be handled when the menu is rendered

        menu
    }


    pub fn with_search_query(mut self, query: String) -> Self {
        self.search_query = query;
        self.update_filtered_categories();
        self
    }

    fn update_filtered_categories(&mut self) {
        let node_definitions = NodeDefinitions::load();
        let search_query = self.search_query.to_lowercase();

        self.filtered_categories = node_definitions.categories
            .iter()
            .filter_map(|category| {
                let filtered_nodes: Vec<_> = category.nodes
                    .iter()
                    .filter_map(|node| {
                        if search_query.is_empty() {
                            // If no search query, include all nodes
                            Some(FilteredNode {
                                definition: node.clone(),
                                highlighted_name: node.name.clone(),
                                highlighted_description: node.description.clone(),
                            })
                        } else {
                            // Check if node matches search
                            let name_matches = node.name.to_lowercase().contains(&search_query);
                            let desc_matches = node.description.to_lowercase().contains(&search_query);
                            let category_matches = category.name.to_lowercase().contains(&search_query);

                            if name_matches || desc_matches || category_matches {
                                Some(FilteredNode {
                                    definition: node.clone(),
                                    highlighted_name: self.highlight_text(&node.name, &search_query),
                                    highlighted_description: self.highlight_text(&node.description, &search_query),
                                })
                            } else {
                                None
                            }
                        }
                    })
                    .collect();

                if !filtered_nodes.is_empty() {
                    let is_expanded = if search_query.is_empty() {
                        // By default, categories are collapsed
                        self.expanded_categories.get(&category.name).copied().unwrap_or(false)
                    } else {
                        // When searching, show matching categories expanded
                        true
                    };

                    Some(FilteredCategory {
                        name: category.name.clone(),
                        color: category.color.clone(),
                        nodes: filtered_nodes.clone(),
                        is_expanded,
                        match_count: filtered_nodes.len(),
                    })
                } else {
                    None
                }
            })
            .collect();
    }

    fn highlight_text(&self, text: &str, query: &str) -> String {
        if query.is_empty() {
            return text.to_string();
        }

        let lower_text = text.to_lowercase();
        let lower_query = query.to_lowercase();

        if let Some(start) = lower_text.find(&lower_query) {
            let end = start + query.len();
            format!(
                "{}**{}**{}",
                &text[..start],
                &text[start..end],
                &text[end..]
            )
        } else {
            text.to_string()
        }
    }

    fn toggle_category(&mut self, category_name: &str) {
        let is_expanded = self.expanded_categories.get(category_name).copied().unwrap_or(false);
        self.expanded_categories.insert(category_name.to_string(), !is_expanded);
        self.update_filtered_categories();
    }

    fn update_search(&mut self, query: String) {
        self.search_query = query;
        self.update_filtered_categories();
    }

    fn create_node(&self, node_def: &NodeDefinition) -> BlueprintNode {
        BlueprintNode::from_definition(node_def, self.target_position)
    }

    fn render_search_box(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .w_full()
            .p_1p5()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .w_full()
                    .relative()
                    .child(
                        TextInput::new(&self.search_input_state)
                            .cleanable()
                            .w_full()
                            .text_xs()
                    )
                    .child(
                        div()
                            .absolute()
                            .right_1p5()
                            .top_1_2()
                            .child(
                                Icon::new(IconName::Search)
                                    .size(px(14.0))
                                    .text_color(cx.theme().muted_foreground)
                            )
                    )
            )
    }

    fn render_category_header(&self, category: &FilteredCategory, cx: &mut Context<Self>) -> impl IntoElement {
        let category_name = category.name.clone();
        let is_expanded = category.is_expanded;
        let match_count = category.match_count;

        h_flex()
            .w_full()
            .items_center()
            .gap_1p5()
            .px_1p5()
            .py_1()
            .hover(|style| style.bg(cx.theme().muted.opacity(0.5)))
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.toggle_category(&category_name);
                cx.notify();
            }))
            .child(
                Icon::new(if is_expanded {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                })
                .size(px(12.0))
                .text_color(cx.theme().muted_foreground)
            )
            .child(
                div()
                    .flex_1()
                    .text_xs()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .child(format!("{} ({})", category.name, match_count))
            )
    }

    fn render_node_item(&self, node: &FilteredNode, cx: &mut Context<Self>) -> impl IntoElement {
        let node_def = node.definition.clone();

        // Pre-compute highlighted text to avoid borrowing issues
        let highlighted_name_element = self.render_highlighted_text(&node.highlighted_name, cx);

        // Use Box::leak for now to get 'static lifetime
        // Memory will be cleaned up when the process exits
        let tooltip_description: &'static str = Box::leak(node.definition.description.clone().into_boxed_str());
        let element_id = format!("node-item-{}", node.definition.id);

        h_flex()
            .w_full()
            .items_center()
            .gap_1p5()
            .px_1p5()
            .py_1()
            .pl_6() // Indent under category
            .hover(|style| style.bg(cx.theme().accent.opacity(0.1)))
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                let new_node = this.create_node(&node_def);
                // Emit the event and also close the menu
                cx.emit(NodeCreationEvent::CreateNode(new_node));
            }))
            .id(ElementId::Name(element_id.into()))
            .tooltip(create_markdown_tooltip(tooltip_description.clone()))
            .child(
                Icon::new(IconName::CircleX) // TODO: Use node type specific icon
                    .size(px(14.0))
                    .text_color(cx.theme().accent)
            )
            .child(
                v_flex()
                    .flex_1()
                    .gap_0()
                    .child(
                        div()
                            .text_xs()
                            .font_medium()
                            .text_color(cx.theme().foreground)
                            .child(highlighted_name_element)
                    )
            )
    }


    fn render_highlighted_text(&self, text: &str, cx: &mut Context<Self>) -> impl IntoElement {
        if text.contains("**") {
            // Parse highlighted text (simple markdown-like bold)
            let parts: Vec<&str> = text.split("**").collect();
            h_flex().children(
                parts.iter().enumerate().map(|(i, part)| {
                    if i % 2 == 1 {
                        // Highlighted part - bright yellow for visibility
                        div()
                            .font_bold()
                            .text_color(rgb(0xFFFF00)) // Bright yellow
                            .child(part.to_string())
                            .into_any_element()
                    } else {
                        // Normal part
                        div()
                            .child(part.to_string())
                            .into_any_element()
                    }
                })
            )
        } else {
            div().child(text.to_string())
        }
    }

    fn render_category(&self, category: &FilteredCategory, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .child(self.render_category_header(category, cx))
            .when(category.is_expanded, |div| {
                div.child(
                    v_flex()
                        .w_full()
                        .children(
                            category.nodes.iter().map(|node| {
                                self.render_node_item(node, cx)
                            })
                        )
                )
            })
    }
}

#[derive(Clone)]
pub enum NodeCreationEvent {
    CreateNode(BlueprintNode),
    Dismiss,
}

impl EventEmitter<NodeCreationEvent> for NodeCreationMenu {}

impl Focusable for NodeCreationMenu {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for NodeCreationMenu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {

        div()
            .absolute()
            .left(px(self.target_position.x))
            .top(px(self.target_position.y))
            .child(
                v_flex()
                    .key_context("NodeCreationMenu")
                    .track_focus(&self.focus_handle)
                    .w(px(280.0))
                    .max_h(px(350.0))
                    .bg(cx.theme().popover)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .shadow_lg()
                    .overflow_hidden()
                    .on_mouse_down(MouseButton::Left, cx.listener(|_, _, _, cx| {
                        cx.stop_propagation(); // Prevent clicks inside menu from bubbling up
                    }))
                    .on_scroll_wheel(cx.listener(|_, _, _, cx| {
                        cx.stop_propagation(); // Prevent scroll events from reaching canvas
                    }))
                    .child(self.render_search_box(cx))
                    .child(
                        div()
                            .flex_1()
                            .overflow_y_hidden()
                            .child(
                                v_flex()
                                    .w_full()
                                    .gap_0p5()
                                    .scrollable(Axis::Vertical)
                                    .children(
                                        self.filtered_categories.iter().map(|category| {
                                            self.render_category(category, cx)
                                        })
                                    )
                            )
                    )
            )
    }
}