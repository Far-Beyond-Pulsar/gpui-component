use gpui::*;
use ui::{prelude::*, Button, IconName, Selectable};
use crate::state::TreeNode;

pub struct TreeNodeView;

impl TreeNodeView {
    pub fn render(
        node: &TreeNode,
        is_selected: bool,
        indent_level: usize,
        on_click: impl Fn(&mut Window, &mut Context<impl Send>) + 'static,
        cx: &mut Context<impl Send>,
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

                Button::new(format!("crate-{}", name), name.clone())
                    .full_width()
                    .style(ButtonStyle::Subtle)
                    .icon(Some(icon))
                    .icon_position(IconPosition::Start)
                    .icon_size(IconSize::Small)
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
                        Button::new(format!("category-{}", name), display_name)
                            .full_width()
                            .style(ButtonStyle::Subtle)
                            .icon(Some(icon))
                            .icon_position(IconPosition::Start)
                            .icon_size(IconSize::XSmall)
                            .selected(is_selected)
                            .on_click(move |_event, window, cx| on_click(window, cx))
                    )
                    .into_any_element()
            }
            TreeNode::Item { name, .. } => {
                div()
                    .pl(indent + px(8.0))
                    .child(
                        Button::new(format!("item-{}", name), name.clone())
                            .full_width()
                            .style(ButtonStyle::Subtle)
                            .icon(Some(IconName::FileText))
                            .icon_position(IconPosition::Start)
                            .icon_size(IconSize::XSmall)
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
