use gpui::*;
use gpui::prelude::{FluentBuilder, InteractiveElement};
use gpui_component::{
    h_flex, v_flex,
    ActiveTheme as _, StyledExt,
    Icon, IconName,
};
use std::collections::HashMap;

use super::{NodeDefinitions, NodeCategory, NodeDefinition, BlueprintNode};
use crate::graph::Position;

/// Unreal-like node creation context menu
pub struct NodeCreationMenu {
    focus_handle: FocusHandle,
    search_query: String,
    search_input_focus: FocusHandle,
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
    pub fn new(target_position: Point<f32>, cx: &mut App) -> Self {
        let focus_handle = cx.focus_handle();
        let search_input_focus = cx.focus_handle();

        let mut menu = Self {
            focus_handle,
            search_input_focus,
            search_query: String::new(),
            expanded_categories: HashMap::new(),
            target_position,
            filtered_categories: Vec::new(),
        };

        menu.update_filtered_categories();
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
        h_flex()
            .w_full()
            .p_2()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .w_full()
                    .child(
                        div()
                            .w_full()
                            .relative()
                            .child(
                                div()
                                    .w_full()
                                    .p_2()
                                    .bg(cx.theme().input)
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .rounded(cx.theme().radius)
                                    .text_sm()
                                    .text_color(cx.theme().foreground)
                                    .child(
                                        if self.search_query.is_empty() {
                                            div()
                                                .text_color(cx.theme().muted_foreground)
                                                .child("Search nodes...")
                                        } else {
                                            div().child(self.search_query.clone())
                                        }
                                    )
                            )
                            .child(
                                div()
                                    .absolute()
                                    .right_2()
                                    .top_1_2()
                                    .child(
                                        Icon::new(IconName::Search)
                                            .size(px(16.0))
                                            .text_color(cx.theme().muted_foreground)
                                    )
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
            .gap_2()
            .p_2()
            .hover(|style| style.bg(cx.theme().muted.opacity(0.5)))
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                this.toggle_category(&category_name);
                cx.notify();
            }))
            .child(
                Icon::new(if is_expanded {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                })
                .size(px(14.0))
                .text_color(cx.theme().muted_foreground)
            )
            .child(
                div()
                    .flex_1()
                    .text_sm()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .child(format!("{} ({})", category.name, match_count))
            )
    }

    fn render_node_item(&self, node: &FilteredNode, cx: &mut Context<Self>) -> impl IntoElement {
        let node_def = node.definition.clone();

        h_flex()
            .w_full()
            .items_center()
            .gap_2()
            .p_2()
            .pl_6() // Indent under category
            .hover(|style| style.bg(cx.theme().accent.opacity(0.1)))
            .cursor_pointer()
            .on_mouse_down(MouseButton::Left, cx.listener(move |this, _, _, cx| {
                let new_node = this.create_node(&node_def);
                cx.emit(NodeCreationEvent::CreateNode(new_node));
            }))
            .child(
                Icon::new(IconName::CircleX) // TODO: Use node type specific icon
                    .size(px(16.0))
                    .text_color(cx.theme().accent)
            )
            .child(
                v_flex()
                    .flex_1()
                    .gap_0p5()
                    .child(
                        div()
                            .text_sm()
                            .font_medium()
                            .text_color(cx.theme().foreground)
                            .child(self.render_highlighted_text(&node.highlighted_name, cx))
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(self.render_highlighted_text(&node.highlighted_description, cx))
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
                        // Highlighted part
                        div()
                            .font_bold()
                            .text_color(cx.theme().accent)
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
        v_flex()
            .key_context("NodeCreationMenu")
            .track_focus(&self.focus_handle)
            .min_w(px(300.0))
            .max_w(px(500.0))
            .max_h(px(600.0))
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .shadow_lg()
            .overflow_hidden()
            .child(self.render_search_box(cx))
            .child(
                div()
                    .flex_1()
                    .overflow_y_hidden()
                    .child(
                        v_flex()
                            .w_full()
                            .children(
                                self.filtered_categories.iter().map(|category| {
                                    self.render_category(category, cx)
                                })
                            )
                    )
            )
    }
}