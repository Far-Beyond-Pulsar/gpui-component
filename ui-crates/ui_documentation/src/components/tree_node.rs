use gpui::*;
use ui::{ActiveTheme, button::Button, h_flex, v_flex, div, IconName, Selectable, Sizable, ButtonVariants, Size, StyledExt};
use crate::state::TreeNode;

pub struct TreeNodeView;

impl TreeNodeView {
    pub fn render<V: Send + 'static>(
        node: &TreeNode,
        is_selected: bool,
        indent_level: usize,
        on_click: impl Fn(&mut Window, &mut Context<V>) + 'static,
        cx: &mut Context<V>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let indent = px((indent_level * 16) as f32);

        match node {
            TreeNode::Crate { name, expanded } => {
                let icon = if *expanded {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                };

                Button::new(("crate", name.as_str()))
                    .label(name.clone())
                    .w_full()
                    .ghost()
                    .icon(icon)
                    .with_size(Size::Small)
                    .selected(is_selected)
                    .on_click(move |_event, window, cx| on_click(window, cx))
                    .into_any_element()
            }
            TreeNode::Category { name, expanded, .. } => {
                let icon = if *expanded {
                    IconName::ChevronDown
                } else {
                    IconName::ChevronRight
                };
                
                let display_name = match name.as_str() {
                    "functions" => "Functions",
                    "structs" => "Structs",
                    "enums" => "Enums",
                    "traits" => "Traits",
                    "macros" => "Macros",
                    "modules" => "Modules",
                    "constants" => "Constants",
                    "type_aliases" => "Type Aliases",
                    _ => name.as_str(),
                };

                div()
                    .pl(indent)
                    .child(
                        Button::new(("category", name.as_str()))
                            .label(display_name)
                            .w_full()
                            .ghost()
                            .icon(icon)
                            .with_size(Size::XSmall)
                            .selected(is_selected)
                            .on_click(move |_event, window, cx| on_click(window, cx))
                    )
                    .into_any_element()
            }
            TreeNode::Item { name, .. } => {
                div()
                    .pl(indent + px(8.0))
                    .child(
                        Button::new(("item", name.as_str()))
                            .label(name.clone())
                            .w_full()
                            .ghost()
                            .with_size(Size::XSmall)
                            .selected(is_selected)
                            .on_click(move |_event, window, cx| on_click(window, cx))
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
