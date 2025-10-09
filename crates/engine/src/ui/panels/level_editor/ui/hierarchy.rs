use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _}, h_flex, v_flex, scroll::ScrollbarAxis, ActiveTheme, Icon, IconName, Selectable, Sizable, StyledExt,
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
            .child(
                // Header
                div()
                    .w_full()
                    .px_4()
                    .py_3()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        h_flex()
                            .w_full()
                            .justify_between()
                            .items_center()
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
            )
            .child(
                // Object tree
                div()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        v_flex()
                            .size_full()
                            .scrollable(ScrollbarAxis::Vertical)
                            .children(
                                state.scene_objects.iter().map(|obj| {
                                    Self::render_object_tree_item(obj, state, 0, cx)
                                })
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
        let indent = px(depth as f32 * 16.0);
        let icon = Self::get_icon_for_object_type(object.object_type);

        div()
            .flex()
            .flex_col()
            .child({
                let mut item_div = div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .h(px(24.0))
                    .pl(indent + px(12.0))
                    .pr_3()
                    .rounded_md()
                    .cursor_pointer();

                if is_selected {
                    item_div = item_div.bg(cx.theme().accent);
                } else {
                    item_div = item_div.hover(|style| style.bg(cx.theme().accent.opacity(0.1)));
                }

                item_div
                    .child(
                        // Expand/collapse arrow for items with children
                        if has_children {
                            div()
                                .w_4()
                                .text_xs()
                                .text_color(if is_selected {
                                    cx.theme().accent_foreground
                                } else {
                                    cx.theme().muted_foreground
                                })
                                .child("▼")
                                .into_any_element()
                        } else {
                            div()
                                .w_4()
                                .into_any_element()
                        }
                    )
                    .child(Icon::new(icon).size_4())
                    .child({
                        let mut text_div = div().text_sm();
                        if is_selected {
                            text_div = text_div.text_color(cx.theme().accent_foreground);
                        } else {
                            text_div = text_div.text_color(cx.theme().foreground);
                        }
                        text_div.child(object.name.clone())
                    })
                    .child(
                        // Visibility toggle
                        div()
                            .ml_auto()
                            .text_xs()
                            .text_color(if object.visible {
                                if is_selected {
                                    cx.theme().accent_foreground.opacity(0.7)
                                } else {
                                    cx.theme().muted_foreground
                                }
                            } else {
                                cx.theme().danger
                            })
                            .child(if object.visible { "●" } else { "○" })
                    )
            })
            .children(
                // Render children recursively
                object.children.iter().map(|child| {
                    Self::render_object_tree_item(child, state, depth + 1, cx)
                })
            )
    }

    fn get_icon_for_object_type(object_type: ObjectType) -> IconName {
        match object_type {
            ObjectType::Camera => IconName::Camera,
            ObjectType::Light => IconName::LightBulb,
            ObjectType::Mesh => IconName::Box,
            ObjectType::Empty => IconName::Circle,
        }
    }
}
