use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    dropdown::{Dropdown, DropdownItem},
    h_flex, v_flex,
    ActiveTheme as _, StyledExt, IconName, Colorize,
};

use super::*;
use super::panel::BlueprintEditorPanel;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClassVariable {
    pub name: String,
    pub var_type: String,
    pub default_value: Option<String>,
}

// Drag data for variables
#[derive(Clone, Debug)]
pub struct VariableDrag {
    pub var_name: String,
    pub var_type: String,
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
                        .w(px(10.))
                        .h(px(10.))
                        .rounded_full()
                        .bg(gpui::Rgba { r: pin_color.r, g: pin_color.g, b: pin_color.b, a: pin_color.a })
                        .border_1()
                        .border_color(gpui::Rgba { r: 0.3, g: 0.3, b: 0.3, a: 1.0 })
                )
                .child(
                    div()
                        .flex_1()
                        .child(self.display_name.clone())
                )
                .child(
                    div()
                        .text_xs()
                        .text_color(gpui::Rgba { r: 0.5, g: 0.5, b: 0.5, a: 1.0 })
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
            .bg(cx.theme().sidebar)
            .child(
                // STUDIO-QUALITY HEADER
                v_flex()
                    .w_full()
                    .child(
                        // Main header with gradient background
                        h_flex()
                            .w_full()
                            .px_4()
                            .py_3()
                            .bg(cx.theme().secondary)
                            .border_b_2()
                            .border_color(cx.theme().border)
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .gap_3()
                                    .items_center()
                                    .child(
                                        // Icon with subtle glow effect
                                        div()
                                            .flex_shrink_0()
                                            .w(px(32.0))
                                            .h(px(32.0))
                                            .rounded(px(6.0))
                                            .bg(cx.theme().accent.opacity(0.15))
                                            .border_1()
                                            .border_color(cx.theme().accent.opacity(0.3))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                div()
                                                    .text_lg()
                                                    .child("📋")
                                            )
                                    )
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .font_bold()
                                                    .text_color(cx.theme().foreground)
                                                    .child("My Blueprint")
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(format!("{} variable{}", 
                                                        panel.class_variables.len(),
                                                        if panel.class_variables.len() == 1 { "" } else { "s" }
                                                    ))
                                            )
                                    )
                            )
                            .child(
                                Button::new("add-variable")
                                    .icon(IconName::Plus)
                                    .primary()
                                    .tooltip("Add New Variable")
                                    .on_click(cx.listener(|panel, _, window, cx| {
                                        panel.start_creating_variable(window, cx);
                                    }))
                            )
                    )
                    .child(
                        // Category/Section bar
                        h_flex()
                            .w_full()
                            .px_4()
                            .py_2()
                            .bg(cx.theme().sidebar.darken(0.03))
                            .border_b_1()
                            .border_color(cx.theme().border.opacity(0.3))
                            .items_center()
                            .justify_between()
                            .child(
                                h_flex()
                                    .gap_2()
                                    .items_center()
                                    .child(
                                        div()
                                            .text_xs()
                                            .font_semibold()
                                            .text_color(cx.theme().accent)
                                            .child("VARIABLES")
                                    )
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded(px(4.0))
                                    .bg(cx.theme().accent.opacity(0.15))
                                    .text_xs()
                                    .font_family("JetBrainsMono-Regular")
                                    .text_color(cx.theme().accent)
                                    .child(format!("{}", panel.class_variables.len()))
                            )
                    )
            )
            .child(
                // CONTENT AREA - clean scrollable list
                v_flex()
                    .flex_1()
                    .overflow_hidden()
                    .p_3()
                    .gap_2()
                    .scrollable(Axis::Vertical)
                    .child(Self::render_variables_list(panel, cx))
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
        let var_type = var.var_type.clone();
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        var.name.hash(&mut hasher);
        let var_hash = hasher.finish() as usize;

        // Get the color for this type
        let type_info = crate::graph::TypeInfo::parse(&var.var_type);
        let pin_color = type_info.generate_color();

        // Clone for the mouse down handler
        let drag_var_name = var_name.clone();
        let drag_var_type = var_type.clone();

        // STUDIO-QUALITY VARIABLE ROW (Unreal Engine style)
        h_flex()
            .w_full()
            .px_3()
            .py_3()
            .gap_3()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border.opacity(0.4))
            .rounded(px(8.0))
            .cursor_grab()
            .hover(|style| {
                style
                    .bg(cx.theme().accent.opacity(0.08))
                    .border_color(cx.theme().accent.opacity(0.6))
                    .shadow_md()
            })
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |panel, _event, _window, cx| {
                panel.start_dragging_variable(drag_var_name.clone(), drag_var_type.clone(), cx);
            }))
            .child(
                // Type indicator with enhanced styling
                div()
                    .flex_shrink_0()
                    .w(px(14.))
                    .h(px(14.))
                    .rounded_full()
                    .bg(gpui::Rgba { r: pin_color.r, g: pin_color.g, b: pin_color.b, a: pin_color.a })
                    .border_2()
                    .border_color(cx.theme().border)
                    .shadow_sm()
            )
            .child(
                // Variable info section
                v_flex()
                    .flex_1()
                    .gap_1p5()
                    .child(
                        // Variable name with drag indicator
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_semibold()
                                    .text_color(cx.theme().foreground)
                                    .child(var.name.clone())
                            )
                            .child(
                                // Drag handle icon
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground.opacity(0.4))
                                    .child("⋮⋮")
                            )
                    )
                    .child(
                        // Type information with formatted name
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded(px(4.0))
                                    .bg(cx.theme().muted.opacity(0.2))
                                    .text_xs()
                                    .font_family("JetBrainsMono-Regular")
                                    .text_color(cx.theme().muted_foreground)
                                    .child(crate::compiler::type_extractor::get_type_display_name(&var.var_type))
                            )
                    )
                    .children(var.default_value.as_ref().map(|default| {
                        div()
                            .px_2()
                            .py_1()
                            .rounded(px(4.0))
                            .bg(cx.theme().success.opacity(0.1))
                            .text_xs()
                            .text_color(cx.theme().success)
                            .child(format!("= {}", default))
                    }))
            )
            .child(
                // Action buttons (on hover)
                h_flex()
                    .gap_1()
                    .child(
                        Button::new(("delete-var", var_hash))
                            .icon(IconName::Close)
                            .ghost()
                            .tooltip("Remove Variable")
                            .on_click(cx.listener(move |panel, _, _, cx| {
                                panel.remove_variable(&var_name, cx);
                            }))
                    )
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
