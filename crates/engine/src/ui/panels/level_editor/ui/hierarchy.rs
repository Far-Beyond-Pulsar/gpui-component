use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _}, h_flex, v_flex, scroll::ScrollbarAxis, ActiveTheme, IconName, Selectable, Sizable, StyledExt,
};

use super::state::{LevelEditorState, ObjectType, SceneObject};

/// Hierarchy Panel - Scene outliner showing all objects in a tree structure
pub struct HierarchyPanel;

impl HierarchyPanel {
    pub fn new() -> Self {
        Self
    }

    pub fn render(&self, state: &LevelEditorState, cx: &mut App) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_2()
            .child(
                // Header
                h_flex()
                    .w_full()
                    .p_2()
                    .justify_between()
                    .items_center()
                    .bg(cx.theme().sidebar)
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Hierarchy")
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                Button::new("add_object")
                                    .icon(IconName::Plus)
                                    .ghost()
                                    .xsmall()
                                    .tooltip("Add Object")
                            )
                            .child(
                                Button::new("delete_object")
                                    .icon(IconName::Trash)
                                    .ghost()
                                    .xsmall()
                                    .tooltip("Delete Selected")
                            )
                    )
            )
            .child(
                // Object tree
                div()
                    .flex_1()
                    .overflow_hidden()
                    .p_2()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(
                        v_flex()
                            .size_full()
                            .scrollable(ScrollbarAxis::Vertical)
                            .child(
                                v_flex()
                                    .gap_1()
                                    .children(
                                        state.scene_objects.iter().map(|obj| {
                                            Self::render_object_tree_item(obj, state, 0, cx)
                                        })
                                    )
                            )
                    )
            )
    }

    fn render_object_tree_item(
        object: &SceneObject,
        state: &LevelEditorState,
        depth: usize,
        cx: &App,
    ) -> impl IntoElement {
        let is_selected = state.selected_object.as_ref() == Some(&object.id);
        let has_children = !object.children.is_empty();

        let mut base_div = div()
            .w_full()
            .pl(px((depth * 16) as f32))
            .pr_2()
            .py_1p5()
            .rounded(cx.theme().radius);

        if is_selected {
            base_div = base_div
                .bg(cx.theme().primary.opacity(0.2))
                .border_l_2()
                .border_color(cx.theme().primary);
        } else {
            base_div = base_div.hover(|style| style.bg(cx.theme().muted.opacity(0.5)));
        }

        v_flex()
            .gap_px()
            .child(
                base_div.child(
                    h_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            // Expand/collapse arrow for items with children
                            if has_children {
                                div()
                                    .w_4()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("▼")
                                    .into_any_element()
                            } else {
                                div()
                                    .w_4()
                                    .into_any_element()
                            }
                        )
                        .child(
                            // Icon based on object type
                            div()
                                .text_color(if is_selected {
                                    cx.theme().primary
                                } else {
                                    cx.theme().foreground
                                })
                                .child(Self::get_icon_for_object_type(object.object_type))
                        )
                        .child(
                            // Object name
                            div()
                                .text_sm()
                                .text_color(if is_selected {
                                    cx.theme().primary
                                } else {
                                    cx.theme().foreground
                                })
                                .child(object.name.clone())
                        )
                        .child(
                            // Visibility toggle
                            div()
                                .ml_auto()
                                .text_xs()
                                .text_color(if object.visible {
                                    cx.theme().muted_foreground
                                } else {
                                    cx.theme().danger
                                })
                                .child(if object.visible { "●" } else { "○" })
                        )
                )
            )
            .children(
                // Render children recursively
                object.children.iter().map(|child| {
                    Self::render_object_tree_item(child, state, depth + 1, cx)
                })
            )
    }

    fn get_icon_for_object_type(object_type: ObjectType) -> &'static str {
        match object_type {
            ObjectType::Camera => "◆",
            ObjectType::Light => "○",
            ObjectType::Mesh => "▪",
            ObjectType::Empty => "◦",
        }
    }
}
