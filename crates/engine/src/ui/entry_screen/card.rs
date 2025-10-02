use gpui::*;
use gpui_component::{h_flex, v_flex, ActiveTheme as _, Icon, IconName, StyledExt};

use super::models::CardItem;

/// Render a polished project card with hover effects and animations
pub fn render_card<V: 'static>(
    card: &CardItem,
    index: usize,
    is_selected: bool,
    cx: &mut Context<V>,
) -> impl IntoElement {
    let card_clone = card.clone();
    let theme = cx.theme().clone();

    let shadow = if is_selected {
        vec![gpui::BoxShadow {
            color: theme.primary.opacity(0.3),
            offset: Point::default(),
            blur_radius: px(12.),
            spread_radius: px(2.),
        }]
    } else {
        vec![gpui::BoxShadow {
            color: Hsla::black().opacity(0.1),
            offset: Point { x: px(0.), y: px(2.) },
            blur_radius: px(8.),
            spread_radius: px(0.),
        }]
    };

    let hover_shadow = vec![gpui::BoxShadow {
        color: theme.primary.opacity(0.2),
        offset: Point::default(),
        blur_radius: px(16.),
        spread_radius: px(0.),
    }];

    div()
        .id(SharedString::from(format!("card-{}", index)))
        .w(px(300.))
        .h(px(220.))
        .bg(theme.background)
        .border_1()
        .border_color(if is_selected {
            theme.primary
        } else {
            theme.border
        })
        .rounded(px(16.))
        .overflow_hidden()
        .cursor_pointer()
        .shadow(shadow)
        .hover(move |style| {
            style
                .border_color(theme.primary.opacity(0.6))
                .shadow(hover_shadow.clone())
        })
        .child(
            v_flex()
                .size_full()
                .child(render_card_image(card, cx))
                .child(render_card_content(card, cx))
        )
}

/// Render the card image/thumbnail section
fn render_card_image<V: 'static>(card: &CardItem, cx: &mut Context<V>) -> impl IntoElement {
    let icon = match card {
        CardItem::Project(_) => IconName::Folder,
        CardItem::Template(_) => IconName::BookOpen,
        CardItem::BlankProject => IconName::Plus,
    };

    let gradient_start = match card {
        CardItem::Project(_) => cx.theme().primary.opacity(0.2),
        CardItem::Template(_) => cx.theme().accent.opacity(0.2),
        CardItem::BlankProject => cx.theme().muted.opacity(0.3),
    };

    let is_blank = matches!(card, CardItem::BlankProject);
    let theme = cx.theme().clone();

    div()
        .w_full()
        .h(px(130.))
        .relative()
        .bg(gradient_start)
        .flex()
        .items_center()
        .justify_center()
        .child(
            Icon::new(icon)
                .size(px(48.))
                .text_color(theme.foreground.opacity(0.4))
        )
        .children(if is_blank {
            Some(
                div()
                    .absolute()
                    .bottom(px(8.))
                    .right(px(8.))
                    .px_2()
                    .py_1()
                    .rounded(px(6.))
                    .bg(theme.primary.opacity(0.9))
                    .text_xs()
                    .font_semibold()
                    .text_color(theme.primary_foreground)
                    .child("New")
            )
        } else {
            None
        })
}

/// Render the card content section (title, description, tags)
fn render_card_content<V: 'static>(card: &CardItem, cx: &mut Context<V>) -> impl IntoElement {
    let tags = card.tags();
    let has_tags = !tags.is_empty();
    let theme = cx.theme().clone();

    v_flex()
        .p_4()
        .gap_2()
        .flex_1()
        .child(
            div()
                .text_base()
                .font_bold()
                .text_color(theme.foreground)
                .line_height(rems(1.2))
                .child(card.name().to_string())
        )
        .child(
            div()
                .text_xs()
                .line_height(rems(1.4))
                .text_color(theme.muted_foreground)
                .overflow_hidden()
                .max_h(px(40.))
                .child(card.description().to_string())
        )
        .children(if has_tags {
            Some(
                h_flex()
                    .gap_1()
                    .mt_1()
                    .flex_wrap()
                    .children(tags.into_iter().take(3).map(|tag| {
                        div()
                            .px_2()
                            .py(px(2.))
                            .rounded(px(4.))
                            .bg(theme.muted)
                            .text_xs()
                            .text_color(theme.muted_foreground)
                            .child(tag.to_string())
                    }))
            )
        } else {
            None
        })
}

/// Render empty state when no cards are available
pub fn render_empty_state<V: 'static>(cx: &mut Context<V>) -> impl IntoElement {
    let theme = cx.theme().clone();

    div()
        .p(px(64.))
        .w_full()
        .flex()
        .items_center()
        .justify_center()
        .child(
            v_flex()
                .gap_6()
                .items_center()
                .max_w(px(400.))
                .child(
                    div()
                        .w(px(80.))
                        .h(px(80.))
                        .rounded(px(40.))
                        .bg(theme.muted.opacity(0.5))
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(
                            Icon::new(IconName::Inbox)
                                .size(px(40.))
                                .text_color(theme.muted_foreground)
                        )
                )
                .child(
                    v_flex()
                        .gap_2()
                        .items_center()
                        .child(
                            div()
                                .text_xl()
                                .font_bold()
                                .text_color(theme.foreground)
                                .child("No projects yet")
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_center()
                                .text_color(theme.muted_foreground)
                                .child("Get started by opening an existing project or creating a new one")
                        )
                )
        )
}
