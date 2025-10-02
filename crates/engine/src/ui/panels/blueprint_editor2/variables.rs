use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
    ActiveTheme as _, StyledExt, Icon, IconName,
};

use super::*;
use super::panel::BlueprintEditorPanel;

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ClassVariable {
    pub name: String,
    pub var_type: String,
    pub default_value: Option<String>,
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
                            .on_click(cx.listener(|panel, _, _, cx| {
                                panel.add_variable(cx);
                            }))
                    )
            )
            .child(
                div()
                    .flex_1()
                    .overflow_y_scroll()
                    .p_2()
                    .bg(cx.theme().background)
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded(cx.theme().radius)
                    .child(Self::render_variables_list(panel, cx))
            )
    }

    fn render_variables_list(panel: &BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        v_flex()
            .gap_2()
            .children(
                if panel.class_variables.is_empty() {
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
                    panel.class_variables.iter().enumerate().map(|(index, var)| {
                        Self::render_variable_row(index, var, cx)
                    }).collect()
                }
            )
    }

    fn render_variable_row(index: usize, var: &ClassVariable, cx: &mut Context<BlueprintEditorPanel>) -> AnyElement {
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
                        div()
                            .text_sm()
                            .font_medium()
                            .text_color(cx.theme().foreground)
                            .child(var.name.clone())
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("Type: {}", var.var_type))
                    )
                    .when_some(var.default_value.clone(), |this, default| {
                        this.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!("Default: {}", default))
                        )
                    })
            )
            .child(
                Button::new(format!("delete-var-{}", index))
                    .ghost()
                    .icon(IconName::Trash)
                    .on_click(cx.listener(move |panel, _, _, cx| {
                        panel.remove_variable(index, cx);
                    }))
            )
            .into_any_element()
    }
}
