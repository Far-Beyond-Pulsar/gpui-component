use gpui::{prelude::*, Axis, MouseButton, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
    input::{InputState, TextInput},
    ActiveTheme as _, Icon, IconName, StyledExt,
};

use super::models::CardItem;

/// Render the polished detail sidebar with project information and actions
pub fn render_sidebar<V: 'static>(
    card: &CardItem,
    project_name_input: &Entity<InputState>,
    project_path_input: &Entity<InputState>,
    on_close: impl Fn(&mut V, &ClickEvent, &mut Window, &mut Context<V>) + 'static,
    on_browse: impl Fn(&mut V, &ClickEvent, &mut Window, &mut Context<V>) + 'static,
    on_action: impl Fn(&mut V, &ClickEvent, &mut Window, &mut Context<V>) + 'static,
    cx: &mut Context<V>,
) -> impl IntoElement {
    let shadow = vec![gpui::BoxShadow {
        color: Hsla::black().opacity(0.1),
        offset: Point { x: px(-4.), y: px(0.) },
        blur_radius: px(16.),
        spread_radius: px(0.),
    }];

    v_flex()
        .w(px(380.))
        .h_full()
        .bg(cx.theme().sidebar)
        .border_l_1()
        .border_color(cx.theme().border)
        .shadow(shadow)
        .on_mouse_down(MouseButton::Left, cx.listener(|_view, _event, _window, cx| {
            // Stop propagation to prevent clicks from falling through to background
            cx.stop_propagation();
        }))
        .on_mouse_move(cx.listener(|_view, _event, _window, cx| {
            // Stop propagation for mouse moves as well
            cx.stop_propagation();
        }))
        .child(render_sidebar_header(on_close, cx))
        .child(
            div()
                .flex_1()
                .overflow_y_hidden()
                .child(
                    v_flex()
                        .w_full()
                        .scrollable(Axis::Vertical)
                        .child(
                            v_flex()
                                .px_6()
                                .pb_6()
                                .gap_6()
                                .child(render_sidebar_preview(card, cx))
                                .child(render_sidebar_info(card, cx))
                                .child(render_sidebar_form(
                                    project_name_input,
                                    project_path_input,
                                    on_browse,
                                    cx,
                                ))
                                .child(render_sidebar_action(card, on_action, cx))
                        )
                )
        )
}

/// Render sidebar header with close button
fn render_sidebar_header<V: 'static>(
    on_close: impl Fn(&mut V, &ClickEvent, &mut Window, &mut Context<V>) + 'static,
    cx: &mut Context<V>,
) -> impl IntoElement {
    let theme = cx.theme().clone();

    div()
        .w_full()
        .px_6()
        .pt_6()
        .pb_4()
        .border_b_1()
        .border_color(theme.border)
        .flex()
        .items_center()
        .justify_between()
        .child(
            div()
                .text_lg()
                .font_bold()
                .text_color(theme.foreground)
                .child("Project Details")
        )
        .child(
            Button::new("close-sidebar")
                .ghost()
                .icon(IconName::Close)
                .on_click(cx.listener(on_close))
        )
}

/// Render the project preview/thumbnail
fn render_sidebar_preview<V: 'static>(card: &CardItem, cx: &mut Context<V>) -> impl IntoElement {
    let icon = match card {
        CardItem::Project(_) => IconName::Folder,
        CardItem::Template(_) => IconName::Frame,
        CardItem::BlankProject => IconName::Plus,
    };

    let bg_color = match card {
        CardItem::Project(_) => cx.theme().primary.opacity(0.1),
        CardItem::Template(_) => cx.theme().accent.opacity(0.1),
        CardItem::BlankProject => cx.theme().muted.opacity(0.5),
    };

    let theme = cx.theme().clone();

    div()
        .w_full()
        .h(px(200.))
        .bg(bg_color)
        .rounded(px(12.))
        .overflow_hidden()
        .border_1()
        .border_color(theme.border)
        .flex()
        .items_center()
        .justify_center()
        .child(
            Icon::new(icon)
                .size(px(64.))
                .text_color(theme.foreground.opacity(0.3))
        )
}

/// Render project information section
fn render_sidebar_info<V: 'static>(card: &CardItem, cx: &mut Context<V>) -> impl IntoElement {
    let tags = card.tags();
    let has_tags = !tags.is_empty();
    let theme = cx.theme().clone();

    v_flex()
        .gap_3()
        .child(
            div()
                .text_xl()
                .font_bold()
                .text_color(theme.foreground)
                .child(card.name().to_string())
        )
        .child(
            div()
                .text_sm()
                .line_height(rems(1.5))
                .text_color(theme.muted_foreground)
                .child(card.description().to_string())
        )
        .children(if has_tags {
            Some(
                h_flex()
                    .gap_2()
                    .flex_wrap()
                    .mt_1()
                    .children(tags.into_iter().map(|tag| {
                        div()
                            .px_3()
                            .py(px(4.))
                            .rounded(px(6.))
                            .bg(theme.primary.opacity(0.1))
                            .border_1()
                            .border_color(theme.primary.opacity(0.2))
                            .text_xs()
                            .font_medium()
                            .text_color(theme.primary)
                            .child(tag.to_string())
                    }))
            )
        } else {
            None
        })
}

/// Render the project configuration form
fn render_sidebar_form<V: 'static>(
    project_name_input: &Entity<InputState>,
    project_path_input: &Entity<InputState>,
    on_browse: impl Fn(&mut V, &ClickEvent, &mut Window, &mut Context<V>) + 'static,
    cx: &mut Context<V>,
) -> impl IntoElement {
    let theme = cx.theme().clone();

    v_flex()
        .gap_4()
        .p_4()
        .rounded(px(12.))
        .bg(theme.muted.opacity(0.3))
        .border_1()
        .border_color(theme.border)
        .child(render_form_field::<V>(
            "Project Name",
            project_name_input,
            None::<fn(&mut V, &ClickEvent, &mut Window, &mut Context<V>)>,
            cx,
        ))
        .child(render_form_field(
            "Project Path",
            project_path_input,
            Some(on_browse),
            cx,
        ))
}

/// Render a form field with label and input
fn render_form_field<V: 'static>(
    label: impl Into<SharedString>,
    input: &Entity<InputState>,
    on_browse: Option<impl Fn(&mut V, &ClickEvent, &mut Window, &mut Context<V>) + 'static>,
    cx: &mut Context<V>,
) -> impl IntoElement {
    let theme = cx.theme().clone();
    let label_text = label.into();

    v_flex()
        .gap_2()
        .child(
            div()
                .text_sm()
                .font_semibold()
                .text_color(theme.foreground)
                .child(label_text)
        )
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(TextInput::new(input))
                .children(on_browse.map(|on_browse_fn| {
                    Button::new("browse-path")
                        .ghost()
                        .icon(IconName::Folder)
                        .on_click(cx.listener(on_browse_fn))
                }))
        )
}

/// Render the action button
fn render_sidebar_action<V: 'static>(
    card: &CardItem,
    on_action: impl Fn(&mut V, &ClickEvent, &mut Window, &mut Context<V>) + 'static,
    cx: &mut Context<V>,
) -> impl IntoElement {
    let (button_text, icon) = match card {
        CardItem::Project(_) => ("Open Project", IconName::FolderOpen),
        CardItem::Template(_) => ("Create from Template", IconName::PagePlusIn),
        CardItem::BlankProject => ("Create Project", IconName::Plus),
    };

    Button::new("sidebar-action")
        .primary()
        .w_full()
        .h(px(44.))
        .text_base()
        .font_semibold()
        .icon(icon)
        .label(button_text)
        .on_click(cx.listener(on_action))
}
