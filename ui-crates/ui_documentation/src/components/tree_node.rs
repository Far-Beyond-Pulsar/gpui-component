use gpui::*;
use ui::{ActiveTheme, button::{Button, ButtonVariants}, h_flex, v_flex, IconName, Selectable, Sizable, Size, StyledExt};
use crate::state::TreeNode;

pub struct TreeNodeView;

impl TreeNodeView {
    pub fn render<V: Send + 'static>(
        node: &TreeNode,
        node_idx: usize,
        is_selected: bool,
        indent_level: usize,
        on_click: impl Fn(&mut Window, &mut App) + 'static,
        cx: &mut Context<V>,
    ) -> impl IntoElement {
        let _theme = cx.theme();
        let indent = px((indent_level * 16) as f32);

        match node {
            TreeNode::Crate { name, expanded } => {
                let icon = if *expanded {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                };

                Button::new(("crate", node_idx))
                    .label(name.clone())
                    .w_full()
                    .ghost()
                    .icon(icon)
                    .with_size(Size::Small)
                    .selected(is_selected)
                    .on_click(move |_, window, cx| on_click(window, cx))
                    .into_any_element()
            }
            TreeNode::Category { name, expanded, .. } => {
                let icon = if *expanded {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                };

                let display_name = match name.as_str() {
                    "functions" => "Functions".to_string(),
                    "structs" => "Structs".to_string(),
                    "enums" => "Enums".to_string(),
                    "traits" => "Traits".to_string(),
                    "macros" => "Macros".to_string(),
                    "modules" => "Modules".to_string(),
                    "constants" => "Constants".to_string(),
                    "type_aliases" => "Type Aliases".to_string(),
                    _ => name.clone(),
                };

                div()
                    .pl(indent)
                    .child(
                        Button::new(("category", node_idx))
                            .label(display_name)
                            .w_full()
                            .ghost()
                            .icon(icon)
                            .with_size(Size::XSmall)
                            .selected(is_selected)
                            .on_click(move |_, window, cx| on_click(window, cx))
                    )
                    .into_any_element()
            }
            TreeNode::Item { name, .. } => {
                div()
                    .pl(indent + px(8.0))
                    .child(
                        Button::new(("item", node_idx))
                            .label(name.clone())
                            .w_full()
                            .ghost()
                            .with_size(Size::XSmall)
                            .selected(is_selected)
                            .on_click(move |_, window, cx| on_click(window, cx))
                    )
                    .into_any_element()
            }
        }
    }

    pub fn get_indent_level(node: &TreeNode) -> usize {
        match node {
            TreeNode::Crate { .. } => 0,
            TreeNode::Category { .. } => 1,
            TreeNode::Item { .. } => 2,
        }
    }
}
