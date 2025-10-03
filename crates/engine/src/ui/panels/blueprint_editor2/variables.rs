use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dropdown::{Dropdown, DropdownState, DropdownItem, SearchableVec},
    input::InputState,
    h_flex, v_flex,
    ActiveTheme as _, StyledExt, Icon, IconName, AxisExt as _,
};

use super::*;
use super::panel::BlueprintEditorPanel;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClassVariable {
    pub name: String,
    pub var_type: String,
    pub default_value: Option<String>,
}

// Wrapper type for dropdown items with colors
#[derive(Clone, Debug)]
pub struct TypeItem {
    type_str: SharedString,
    display_name: SharedString,
}

impl TypeItem {
    pub fn new(type_str: String) -> Self {
        let display_name = crate::compiler::type_extractor::get_type_display_name(&type_str);
        Self {
            type_str: type_str.into(),
            display_name: display_name.into(),
        }
    }
}

impl DropdownItem for TypeItem {
    type Value = SharedString;

    fn title(&self) -> SharedString {
        self.display_name.clone()
    }

    fn display_title(&self) -> Option<AnyElement> {
        // Get the color for this type
        let type_info = crate::graph::TypeInfo::parse(&self.type_str);
        let pin_color = type_info.generate_color();

        Some(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    // Colored dot
                    div()
                        .w(px(8.))
                        .h(px(8.))
                        .rounded_full()
                        .bg(gpui::Rgba { r: pin_color.r, g: pin_color.g, b: pin_color.b, a: pin_color.a })
                )
                .child(
                    div()
                        .child(self.display_name.clone())
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(gpui::Rgba { r: 0.6, g: 0.6, b: 0.6, a: 1.0 })
                        .child(format!("({})", self.type_str))
                )
                .into_any_element()
        )
    }

    fn value(&self) -> &Self::Value {
        &self.type_str
    }
}

pub struct VariablesRenderer;

impl VariablesRenderer {
    pub fn render(panel: &BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        v_flex()
            .size_full()
            .gap_2()
            .child(
                h_flex()
                    .w_full()
                    .p_2()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child("Variables")
                    )
                    .child(
                        Button::new("add-variable")
                            .ghost()
                            .icon(IconName::Plus)
                            .on_click(cx.listener(|panel, _, window, cx| {
                                panel.start_creating_variable(window, cx);
                            }))
                    )
            )
            .child(
                div()
                    .flex_1()
                    .overflow_y_hidden()
                    .child(
                        v_flex()
                            .w_full()
                            .p_2()
                            .bg(cx.theme().background)
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .scrollable(Axis::Vertical)
                            .child(Self::render_variables_list(panel, cx))
                    )
            )
    }

    fn render_variables_list(panel: &BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .children(if panel.is_creating_variable {
                vec![Self::render_variable_creation_form(panel, cx).into_any_element()]
            } else {
                Vec::new()
            })
            .children(
                if panel.class_variables.is_empty() && !panel.is_creating_variable {
                    vec![
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .h(px(100.))
                            .text_color(cx.theme().muted_foreground)
                            .text_sm()
                            .child("No variables defined")
                            .into_any_element()
                    ]
                } else {
                    panel.class_variables.iter().map(|var| {
                        Self::render_variable_row(var, cx)
                    }).collect()
                }
            )
    }

    fn render_variable_row(var: &ClassVariable, cx: &mut Context<BlueprintEditorPanel>) -> AnyElement {
        let var_name = var.name.clone();
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        var.name.hash(&mut hasher);
        let var_hash = hasher.finish() as usize;

        // Get the color for this type
        let type_info = crate::graph::TypeInfo::parse(&var.var_type);
        let pin_color = type_info.generate_color();

        h_flex()
            .w_full()
            .p_2()
            .gap_2()
            .bg(cx.theme().sidebar)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .hover(|style| style.bg(cx.theme().muted.opacity(0.3)))
            .child(
                v_flex()
                    .flex_1()
                    .gap_1()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .text_sm()
                                    .font_medium()
                                    .text_color(cx.theme().foreground)
                                    .child(var.name.clone())
                            )
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                div()
                                    .w(px(8.))
                                    .h(px(8.))
                                    .rounded_full()
                                    .bg(gpui::Rgba { r: pin_color.r, g: pin_color.g, b: pin_color.b, a: pin_color.a })
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(crate::compiler::type_extractor::get_type_display_name(&var.var_type))
                            )
                    )
                    .children(var.default_value.as_ref().map(|default| {
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Default: {}", default))
                    }))
            )
            .child(
                Button::new(("delete-var", var_hash))
                    .ghost()
                    .icon(IconName::Close)
                    .on_click(cx.listener(move |panel, _, _, cx| {
                        panel.remove_variable(&var_name, cx);
                    }))
            )
            .into_any_element()
    }

    fn render_variable_creation_form(panel: &BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        use gpui_component::input::TextInput;

        v_flex()
            .w_full()
            .p_3()
            .gap_3()
            .bg(cx.theme().sidebar)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .child(
                div()
                    .text_sm()
                    .font_semibold()
                    .text_color(cx.theme().foreground)
                    .child("New Variable")
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Name")
                            )
                            .child(
                                TextInput::new(&panel.variable_name_input)
                            )
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Type")
                            )
                            .child(
                                Dropdown::new(&panel.variable_type_dropdown)
                            )
                    )
            )
            .child(
                h_flex()
                    .gap_2()
                    .justify_end()
                    .child(
                        Button::new("cancel-var")
                            .ghost()
                            .label("Cancel")
                            .on_click(cx.listener(|panel, _, _, cx| {
                                panel.cancel_creating_variable(cx);
                            }))
                    )
                    .child(
                        Button::new("create-var")
                            .primary()
                            .label("Create")
                            .on_click(cx.listener(|panel, _, _, cx| {
                                panel.complete_creating_variable(cx);
                            }))
                    )
            )
    }

}
