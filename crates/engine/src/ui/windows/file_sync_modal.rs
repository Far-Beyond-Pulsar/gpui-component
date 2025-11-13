//! File sync diff approval modal
//!
//! Shows the user what files will be added/modified/deleted when joining a session

use gpui::*;
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::Button, h_flex, v_flex, modal::Modal, ActiveTheme as _, Icon, IconName, Sizable as _,
    StyledExt,
};

use crate::ui::file_sync::TreeDiff;

gpui::actions!(file_sync_modal, [ApproveFileSync, CancelFileSync]);

/// Modal for approving file sync changes
pub struct FileSyncModal {
    diff: TreeDiff,
}

impl FileSyncModal {
    pub fn new(diff: TreeDiff) -> Self {
        Self { diff }
    }

    fn format_size(bytes: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if bytes >= GB {
            format!("{:.2} GB", bytes as f64 / GB as f64)
        } else if bytes >= MB {
            format!("{:.2} MB", bytes as f64 / MB as f64)
        } else if bytes >= KB {
            format!("{:.2} KB", bytes as f64 / KB as f64)
        } else {
            format!("{} bytes", bytes)
        }
    }
}

impl RenderOnce for FileSyncModal {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let total_added_size: u64 = self.diff.added.iter().map(|f| f.size).sum();
        let total_modified_size: u64 = self.diff.modified.iter().map(|f| f.size).sum();
        let has_changes = self.diff.has_changes();

        Modal::new(window, cx)
            .title("File Synchronization Required")
            .child(
                v_flex()
                    .gap_4()
                    .w(px(600.))
                    .child(
                        // Summary
                        div()
                            .p_3()
                            .rounded(px(6.))
                            .bg(cx.theme().accent.opacity(0.1))
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_sm()
                                            .font_bold()
                                            .text_color(cx.theme().foreground)
                                            .child(format!(
                                                "The host has {} file(s) that differ from your local project",
                                                self.diff.change_count()
                                            ))
                                    )
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(self.diff.summary())
                                    )
                            )
                    )
                    .when(has_changes, |this| {
                        this.child(
                            v_flex()
                                .gap_3()
                                .max_h(px(400.))
                                .overflow_y_hidden()
                                .child(
                                    v_flex()
                                        .gap_2()
                                        // Added files
                                        .when(!self.diff.added.is_empty(), |this| {
                                            this.child(
                                                v_flex()
                                                    .gap_1()
                                                    .child(
                                                        h_flex()
                                                            .gap_2()
                                                            .items_center()
                                                            .child(
                                                                Icon::new(IconName::Plus)
                                                                    .size(px(14.))
                                                                    .text_color(cx.theme().success),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .font_bold()
                                                                    .text_color(cx.theme().success)
                                                                    .child(format!(
                                                                        "{} FILES TO DOWNLOAD ({})",
                                                                        self.diff.added.len(),
                                                                        Self::format_size(total_added_size)
                                                                    )),
                                                            ),
                                                    )
                                                    .child(
                                                        v_flex()
                                                            .gap_0p5()
                                                            .pl_4()
                                                            .children(self.diff.added.iter().take(10).map(|file| {
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(cx.theme().foreground)
                                                                    .child(format!(
                                                                        "  + {} ({})",
                                                                        file.path.display(),
                                                                        Self::format_size(file.size)
                                                                    ))
                                                                    .into_any_element()
                                                            })),
                                                    )
                                                    .when(self.diff.added.len() > 10, |this| {
                                                        this.child(
                                                            div()
                                                                .pl_4()
                                                                .text_xs()
                                                                .text_color(cx.theme().muted_foreground)
                                                                .child(format!("...and {} more", self.diff.added.len() - 10)),
                                                        )
                                                    }),
                                            )
                                        })
                                        // Modified files
                                        .when(!self.diff.modified.is_empty(), |this| {
                                            this.child(
                                                v_flex()
                                                    .gap_1()
                                                    .child(
                                                        h_flex()
                                                            .gap_2()
                                                            .items_center()
                                                            .child(
                                                                Icon::new(IconName::Edit)
                                                                    .size(px(14.))
                                                                    .text_color(cx.theme().warning),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .font_bold()
                                                                    .text_color(cx.theme().warning)
                                                                    .child(format!(
                                                                        "{} FILES TO UPDATE ({})",
                                                                        self.diff.modified.len(),
                                                                        Self::format_size(total_modified_size)
                                                                    )),
                                                            ),
                                                    )
                                                    .child(
                                                        v_flex()
                                                            .gap_0p5()
                                                            .pl_4()
                                                            .children(self.diff.modified.iter().take(10).map(|file| {
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(cx.theme().foreground)
                                                                    .child(format!(
                                                                        "  ~ {} ({})",
                                                                        file.path.display(),
                                                                        Self::format_size(file.size)
                                                                    ))
                                                                    .into_any_element()
                                                            })),
                                                    )
                                                    .when(self.diff.modified.len() > 10, |this| {
                                                        this.child(
                                                            div()
                                                                .pl_4()
                                                                .text_xs()
                                                                .text_color(cx.theme().muted_foreground)
                                                                .child(format!("...and {} more", self.diff.modified.len() - 10)),
                                                        )
                                                    }),
                                            )
                                        })
                                        // Deleted files
                                        .when(!self.diff.deleted.is_empty(), |this| {
                                            this.child(
                                                v_flex()
                                                    .gap_1()
                                                    .child(
                                                        h_flex()
                                                            .gap_2()
                                                            .items_center()
                                                            .child(
                                                                Icon::new(IconName::Trash)
                                                                    .size(px(14.))
                                                                    .text_color(cx.theme().danger),
                                                            )
                                                            .child(
                                                                div()
                                                                    .text_xs()
                                                                    .font_bold()
                                                                    .text_color(cx.theme().danger)
                                                                    .child(format!("{} FILES TO DELETE", self.diff.deleted.len())),
                                                            ),
                                                    )
                                                    .child(
                                                        v_flex()
                                                            .gap_0p5()
                                                            .pl_4()
                                                            .children(self.diff.deleted.iter().take(10).map(|path| {
                                                                div()
                                                                    .text_xs()
                                                                    .text_color(cx.theme().foreground)
                                                                    .child(format!("  - {}", path.display()))
                                                                    .into_any_element()
                                                            })),
                                                    )
                                                    .when(self.diff.deleted.len() > 10, |this| {
                                                        this.child(
                                                            div()
                                                                .pl_4()
                                                                .text_xs()
                                                                .text_color(cx.theme().muted_foreground)
                                                                .child(format!("...and {} more", self.diff.deleted.len() - 10)),
                                                        )
                                                    }),
                                            )
                                        }),
                                )
                        )
                    })
                    .when(!has_changes, |this| {
                        this.child(
                            div()
                                .p_4()
                                .text_center()
                                .child(
                                    v_flex()
                                        .gap_2()
                                        .items_center()
                                        .child(
                                            Icon::new(IconName::Check)
                                                .size(px(32.))
                                                .text_color(cx.theme().success),
                                        )
                                        .child(
                                            div()
                                                .text_sm()
                                                .text_color(cx.theme().foreground)
                                                .child("Your project is already in sync!"),
                                        ),
                                ),
                        )
                    })
                    .child(
                        // Warning
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
                                    .child("⚠ Local changes will be overwritten. Make sure you have backups!"),
                            ),
                    )
                    .child(
                        // Buttons
                        h_flex()
                            .gap_2()
                            .justify_end()
                            .child(
                                Button::new("cancel")
                                    .label("Cancel")
                                    .on_click(|_, _window, cx| {
                                        cx.dispatch_action(&CancelFileSync);
                                    }),
                            )
                            .child(
                                Button::new("approve")
                                    .label(if has_changes {
                                        format!("Sync {} Files", self.diff.change_count())
                                    } else {
                                        "Continue".to_string()
                                    })
                                    .on_click(|_, _window, cx| {
                                        cx.dispatch_action(&ApproveFileSync);
                                    }),
                            ),
                    ),
            )
    }
}
