use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    context_menu::ContextMenuExt,
    h_flex, v_flex, scroll::ScrollbarAxis, ActiveTheme, Icon, IconName, Sizable, StyledExt,
};

use super::state::{LevelEditorState, SceneObject};
use super::actions::*;
use ui_editor::tabs::level_editor::scene_database::ObjectType;

/// Hierarchy Panel - Scene outliner showing all objects in a tree structure
pub struct HierarchyPanel;

impl HierarchyPanel {
    pub fn new() -> Self {
        Self
    }

    pub fn render<V: 'static>(
        &self,
        state: &LevelEditorState,
        cx: &mut Context<V>
    ) -> impl IntoElement
    where
        V: EventEmitter<PanelEvent> + Render,
    {
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
                                            .on_click(cx.listener(|_, _, _, cx| {
                                                cx.dispatch_action(&AddObject);
                                            }))
                                    )
                                    .child(
                                        Button::new("delete_object")
                                            .icon(IconName::Trash)
                                            .ghost()
                                            .xsmall()
                                            .tooltip("Delete Selected")
                                            .on_click(cx.listener(|_, _, _, cx| {
                                                cx.dispatch_action(&DeleteObject);
                                            }))
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
                                state.scene_objects().iter().map(|obj| {
                                    Self::render_object_tree_item(obj, state, 0, cx)
                                })
                            )
                    )
            )
    }

    fn render_object_tree_item<V: 'static>(
        object: &SceneObject,
        state: &LevelEditorState,
        depth: usize,
        cx: &mut Context<V>,
    ) -> impl IntoElement
    where
        V: EventEmitter<PanelEvent> + Render,
    {
        let is_selected = state.selected_object().as_ref() == Some(&object.id);
        let has_children = !object.children.is_empty();
        let is_expanded = state.is_object_expanded(&object.id);
        let indent = px(depth as f32 * 16.0);
        let icon = Self::get_icon_for_object_type(object.object_type);
        let object_id = object.id.clone();
        let object_id_for_expand = object.id.clone();

        // Build item div base
        let item_id = SharedString::from(format!("object-{}", object.id));
        let mut item_div = div()
            .id(item_id)
            .w_full()
            .flex()
            .items_center()
            .gap_2()
            .h(px(24.0))
            .pl(indent + px(8.0))
            .pr_2()
            .rounded_md()
            .cursor_pointer();

        // Apply conditional styling
        item_div = if is_selected {
            item_div.bg(cx.theme().accent)
        } else {
            item_div.hover(|style| style.bg(cx.theme().accent.opacity(0.1)))
        };

        div()
            .w_full()
            .flex()
            .flex_col()
            .child(
                item_div
                    .on_mouse_down(MouseButton::Left, cx.listener(move |view, _, _, cx| {
                        cx.dispatch_action(&SelectObject {
                            object_id: object_id.clone()
                        });
                    }))
                    .context_menu(move |menu, _window, _cx| {
                        menu
                            .menu("Add Child Object", Box::new(AddObject) as Box<dyn gpui::Action>)
                            .menu("Duplicate", Box::new(DuplicateObject) as Box<dyn gpui::Action>)
                            .separator()
                            .menu("Rename", Box::new(RenameObject) as Box<dyn gpui::Action>)
                            .menu("Delete", Box::new(DeleteObject) as Box<dyn gpui::Action>)
                    })
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
                                .child(if is_expanded { "▼" } else { "▶" })
                                .on_mouse_down(MouseButton::Left, cx.listener(move |view, _, _, cx| {
                                    cx.stop_propagation();
                                    cx.dispatch_action(&ToggleObjectExpanded {
                                        object_id: object_id_for_expand.clone()
                                    });
                                }))
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
            )
            // Note: Children rendering removed for now since object.children
            // contains ObjectIds, not full SceneObjects. Would need to look them up from database.
    }

    fn get_icon_for_object_type(object_type: ObjectType) -> IconName {
        match object_type {
            ObjectType::Camera => IconName::Camera,
            ObjectType::Light(_) => IconName::LightBulb,
            ObjectType::Mesh(_) => IconName::Box,
            ObjectType::Empty => IconName::Circle,
            ObjectType::ParticleSystem => IconName::Play,
            ObjectType::AudioSource => IconName::MusicNote,
        }
    }
}

use gpui_component::dock::PanelEvent;
