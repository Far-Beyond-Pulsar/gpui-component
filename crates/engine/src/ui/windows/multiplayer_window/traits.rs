//! Trait implementations for MultiplayerWindow

use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::Button,
    h_flex, v_flex,
    ActiveTheme as _, StyledExt, TitleBar,
};

use super::state::MultiplayerWindow;

impl Focusable for MultiplayerWindow {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for MultiplayerWindow {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let pending_diff = self.pending_file_sync.as_ref().map(|(diff, _)| diff.clone());

        v_flex()
            .size_full()
            .bg(cx.theme().background)
            .child(
                TitleBar::new()
                    .child(
                        h_flex()
                            .w_full()
                            .items_center()
                            .justify_between()
                            .child(div().text_sm().child("Multiplayer"))
                            .when_some(self.active_session.as_ref(), |this, session| {
                                this.child(
                                    h_flex()
                                        .gap_2()
                                        .items_center()
                                        .child(
                                            div()
                                                .w(px(6.))
                                                .h(px(6.))
                                                .rounded(px(3.))
                                                .bg(cx.theme().success)
                                        )
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().foreground)
                                                .child(format!("{} users", session.connected_users.len()))
                                        )
                                )
                            })
                    )
            )
            .child(
                if let Some(ref session) = self.active_session {
                    self.render_active_session(session, cx).into_any_element()
                } else {
                    self.render_connection_form(cx).into_any_element()
                }
            )
            .when_some(pending_diff, |parent, diff| {
                let has_changes = diff.has_changes();
                let total_added = diff.added.len();
                let total_modified = diff.modified.len();
                let total_deleted = diff.deleted.len();

                parent.child(
                    div()
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(gpui::black().opacity(0.5))
                        .child(
                            v_flex()
                                .gap_4()
                                .w(px(600.))
                                .p_4()
                                .bg(cx.theme().background)
                                .rounded(px(8.))
                                .border_1()
                                .border_color(cx.theme().border)
                                .child(
                                    div()
                                        .text_lg()
                                        .font_bold()
                                        .text_color(cx.theme().foreground)
                                        .child("File Synchronization Required")
                                )
                                .child(
                                    div()
                                        .p_3()
                                        .rounded(px(6.))
                                        .bg(cx.theme().accent.opacity(0.1))
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().foreground)
                                                .child(format!(
                                                    "{} added, {} modified, {} deleted",
                                                    total_added, total_modified, total_deleted
                                                ))
                                        )
                                )
                                .child(
                                    div()
                                        .p_2()
                                        .rounded(px(4.))
                                        .bg(cx.theme().warning.opacity(0.1))
                                        .border_1()
                                        .border_color(cx.theme().warning)
                                        .child(
                                            div()
                                                .text_xs()
                                                .text_color(cx.theme().warning)
                                                .child("⚠ Local changes will be overwritten!")
                                        )
                                )
                                .child(
                                    h_flex()
                                        .gap_2()
                                        .justify_end()
                                        .child(
                                            Button::new("cancel-sync")
                                                .label("Cancel")
                                                .on_click(cx.listener(|this, _, _window, cx| {
                                                    this.cancel_file_sync(cx);
                                                }))
                                        )
                                        .child(
                                            Button::new("approve-sync")
                                                .label(if has_changes {
                                                    format!("Sync {} Files", diff.change_count())
                                                } else {
                                                    "Continue".to_string()
                                                })
                                                .on_click(cx.listener(|this, _, _window, cx| {
                                                    this.approve_file_sync(cx);
                                                }))
                                        )
                                )
                        )
                )
            })
    }
}
