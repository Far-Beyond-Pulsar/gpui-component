use gpui::*;
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
    ActiveTheme as _, StyledExt, IconName,
};

use super::panel::BlueprintEditorPanel;

pub struct MacrosRenderer;

impl MacrosRenderer {
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
                                                    .child("📦")
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
                                                    .child("Macro Library")
                                            )
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child(format!("{} macro{}",
                                                        Self::count_macros(panel),
                                                        if Self::count_macros(panel) == 1 { "" } else { "s" }
                                                    ))
                                            )
                                    )
                            )
                            .child(
                                Button::new("create-macro")
                                    .icon(IconName::Plus)
                                    .primary()
                                    .tooltip("Create New Macro")
                                    .on_click(cx.listener(|panel, _, _window, cx| {
                                        panel.create_new_local_macro(cx);
                                    }))
                            )
                    )
                    .child(
                        // Category/Section bar
                        h_flex()
                            .w_full()
                            .px_4()
                            .py_2()
                            .bg(cx.theme().sidebar)
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
                                            .child("MACROS")
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
                                    .child(format!("{}", Self::count_macros(panel)))
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
                    .child(Self::render_macros_list(panel, cx))
            )
    }

    fn count_macros(panel: &BlueprintEditorPanel) -> usize {
        panel.library_manager.get_libraries()
            .values()
            .map(|lib| lib.subgraphs.len())
            .sum()
    }

    fn render_macros_list(panel: &BlueprintEditorPanel, cx: &mut Context<BlueprintEditorPanel>) -> impl IntoElement {
        let libraries = panel.library_manager.get_libraries();

        v_flex()
            .gap_3()
            .children(
                if libraries.is_empty() {
                    vec![
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .h(px(100.))
                            .text_color(cx.theme().muted_foreground)
                            .text_sm()
                            .child("No macro libraries loaded")
                            .into_any_element()
                    ]
                } else {
                    libraries.values().flat_map(|library| {
                        let mut items = vec![
                            // Library header
                            v_flex()
                                .w_full()
                                .gap_1()
                                .child(
                                    div()
                                        .w_full()
                                        .px_2()
                                        .py_2()
                                        .text_xs()
                                        .font_bold()
                                        .text_color(cx.theme().foreground.opacity(0.7))
                                        .child(library.name.clone())
                                )
                                .into_any_element()
                        ];

                        // Add macro items
                        for subgraph in &library.subgraphs {
                            items.push(Self::render_macro_row(subgraph, cx));
                        }

                        items
                    }).collect()
                }
            )
    }

    fn render_macro_row(subgraph: &crate::graph::SubGraphDefinition, cx: &mut Context<BlueprintEditorPanel>) -> AnyElement {
        let subgraph_id = subgraph.id.clone();
        let subgraph_name = subgraph.name.clone();

        // STUDIO-QUALITY MACRO ROW
        h_flex()
            .w_full()
            .px_3()
            .py_3()
            .gap_3()
            .bg(cx.theme().background)
            .border_1()
            .border_color(cx.theme().border.opacity(0.4))
            .rounded(px(8.0))
            .cursor_pointer()
            .hover(|style| {
                style
                    .bg(cx.theme().accent.opacity(0.08))
                    .border_color(cx.theme().accent.opacity(0.6))
                    .shadow_md()
            })
            .on_mouse_down(gpui::MouseButton::Left, cx.listener(move |panel, _, _window, cx| {
                panel.open_subgraph(subgraph_id.clone(), subgraph_name.clone(), cx);
            }))
            .child(
                // Macro icon
                div()
                    .flex_shrink_0()
                    .w(px(14.))
                    .h(px(14.))
                    .rounded_full()
                    .bg(gpui::Rgba { r: 0.61, g: 0.35, b: 0.71, a: 1.0 }) // Purple for macros
                    .border_2()
                    .border_color(cx.theme().border)
                    .shadow_sm()
            )
            .child(
                // Macro info section
                v_flex()
                    .flex_1()
                    .gap_1p5()
                    .child(
                        // Macro name
                        div()
                            .text_sm()
                            .font_semibold()
                            .text_color(cx.theme().foreground)
                            .child(subgraph.name.clone())
                    )
                    .child(
                        // Description
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(subgraph.description.clone())
                    )
                    .child(
                        // Input/Output count
                        h_flex()
                            .gap_3()
                            .items_center()
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded(px(4.0))
                                    .bg(cx.theme().success.opacity(0.15))
                                    .text_xs()
                                    .font_family("JetBrainsMono-Regular")
                                    .text_color(cx.theme().success)
                                    .child(format!("→ {}", subgraph.interface.inputs.len()))
                            )
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .rounded(px(4.0))
                                    .bg(cx.theme().warning.opacity(0.15))
                                    .text_xs()
                                    .font_family("JetBrainsMono-Regular")
                                    .text_color(cx.theme().warning)
                                    .child(format!("← {}", subgraph.interface.outputs.len()))
                            )
                    )
            )
            .child(
                // Arrow icon indicating clickable
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground.opacity(0.4))
                    .child("›")
            )
            .into_any_element()
    }
}
