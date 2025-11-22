use gpui::{prelude::*, *};
use ui::{h_flex, v_flex, divider::Divider, ActiveTheme, StyledExt};
use pulsar_std::{get_all_type_constructors, get_type_constructors_by_category, TypeConstructorMetadata};
use std::collections::HashMap;
use crate::type_block::TypeBlock;

/// Palette of available type constructors (like Scratch's block palette)
/// Now dynamically loaded from the type constructor registry!
pub struct ConstructorPalette {
    categories: Vec<CategoryData>,
}

struct CategoryData {
    name: String,
    icon: &'static str,
    constructors: Vec<&'static TypeConstructorMetadata>,
}

impl ConstructorPalette {
    pub fn new() -> Self {
        // Load all type constructors from the registry
        let all_constructors = get_all_type_constructors();

        // Group by category
        let mut categories_map: HashMap<String, Vec<&'static TypeConstructorMetadata>> = HashMap::new();

        for constructor in all_constructors {
            categories_map
                .entry(constructor.category.to_string())
                .or_insert_with(Vec::new)
                .push(constructor);
        }

        // Map categories to icons
        let category_icons: HashMap<&str, &'static str> = [
            ("Smart Pointers", "📦"),
            ("Option & Result", "🎯"),
            ("Collections", "📚"),
            ("Interior Mutability", "🔒"),
            ("Other", "🔧"),
        ]
        .iter()
        .copied()
        .collect();

        // Convert to CategoryData with consistent ordering
        let category_order = [
            "Smart Pointers",
            "Option & Result",
            "Collections",
            "Interior Mutability",
            "Other",
        ];

        let mut categories = Vec::new();

        for &category_name in &category_order {
            if let Some(constructors) = categories_map.get(category_name) {
                categories.push(CategoryData {
                    name: category_name.to_string(),
                    icon: category_icons.get(category_name).copied().unwrap_or("🔧"),
                    constructors: constructors.clone(),
                });
            }
        }

        // Add any categories not in the predefined order
        for (category_name, constructors) in categories_map {
            if !category_order.contains(&category_name.as_str()) {
                categories.push(CategoryData {
                    name: category_name.clone(),
                    icon: "🔧",
                    constructors,
                });
            }
        }

        Self { categories }
    }

    pub fn render(&self, cx: &mut WindowContext) -> impl IntoElement {
        v_flex()
            .w(px(280.0))
            .h_full()
            .bg(cx.theme().sidebar)
            .border_r_1()
            .border_color(cx.theme().border)
            .child(
                // Header
                h_flex()
                    .w_full()
                    .px_4()
                    .py_3()
                    .bg(cx.theme().secondary)
                    .border_b_2()
                    .border_color(cx.theme().border)
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .font_bold()
                            .text_color(cx.theme().foreground)
                            .child("Type Constructors")
                    )
            )
            .child(
                // Scrollable categories
                v_flex()
                    .flex_1()
                    .overflow_y_scroll()
                    .p_3()
                    .gap_3()
                    .children(
                        self.categories
                            .iter()
                            .map(|category| self.render_category(category, cx))
                    )
            )
            .child(
                // Footer hint
                div()
                    .px_4()
                    .py_3()
                    .bg(cx.theme().secondary.opacity(0.5))
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child("Click to add blocks to your type")
                    )
            )
    }

    fn render_category(&self, category: &CategoryData, cx: &mut WindowContext) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_2()
            .child(
                // Category header
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_base()
                            .child(category.icon)
                    )
                    .child(
                        div()
                            .text_xs()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child(&category.name)
                    )
            )
            .child(
                // Blocks
                v_flex()
                    .gap_1()
                    .pl_4()
                    .children(
                        category.constructors.iter().map(|constructor| {
                            self.render_palette_block(constructor, cx)
                        })
                    )
            )
    }

    fn render_palette_block(&self, constructor: &TypeConstructorMetadata, cx: &mut WindowContext) -> impl IntoElement {
        let color = hsla(0.08, 0.8, 0.6, 1.0); // Orange for constructors

        v_flex()
            .w_full()
            .gap_1()
            .child(
                h_flex()
                    .w_full()
                    .px_3()
                    .py_2()
                    .gap_2()
                    .bg(color)
                    .rounded(px(6.0))
                    .border_1()
                    .border_color(color.lighten(0.1))
                    .shadow_sm()
                    .cursor_pointer()
                    .hover(|style| {
                        style.bg(color.lighten(0.1)).shadow_md()
                    })
                    .active(|style| {
                        style.bg(color.darken(0.1))
                    })
                    .child(
                        div()
                            .text_sm()
                            .font_bold()
                            .text_color(gpui::white())
                            .child(constructor.name)
                    )
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(gpui::white().opacity(0.7))
                            .child(format!("<{}params>", constructor.params_count))
                    )
            )
            .child(
                div()
                    .pl_3()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(constructor.description)
            )
    }
}

impl Default for ConstructorPalette {
    fn default() -> Self {
        Self::new()
    }
}
