use gpui::{prelude::*, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    v_flex, Icon, IconName, ActiveTheme as _,
};
use crate::ui::entry_screen::{EntryScreen, EntryScreenView};
use crate::OpenSettings;

pub fn render_sidebar(screen: &EntryScreen, cx: &mut Context<EntryScreen>) -> impl IntoElement {
    let theme = cx.theme();
    
    v_flex()
        .w(px(72.))
        .h_full()
        .bg(theme.sidebar)
        .border_r_1()
        .border_color(theme.border)
        .gap_2()
        .items_center()
        .pt_8()
        .pb_4()
        .child(
            Button::new("recent-projects")
                .icon(IconName::FolderClosed)
                .label("")
                .tooltip("Recent Projects")
                .with_variant(if screen.view == EntryScreenView::Recent {
                    gpui_component::button::ButtonVariant::Primary
                } else {
                    gpui_component::button::ButtonVariant::Ghost
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.view = EntryScreenView::Recent;
                    cx.notify();
                }))
        )
        .child(
            Button::new("templates")
                .icon(IconName::Star)
                .label("")
                .tooltip("Project Templates")
                .with_variant(if screen.view == EntryScreenView::Templates {
                    gpui_component::button::ButtonVariant::Primary
                } else {
                    gpui_component::button::ButtonVariant::Ghost
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.view = EntryScreenView::Templates;
                    cx.notify();
                }))
        )
        .child(
            Button::new("new-project")
                .icon(IconName::Plus)
                .label("")
                .tooltip("Create New Project")
                .with_variant(if screen.view == EntryScreenView::NewProject {
                    gpui_component::button::ButtonVariant::Primary
                } else {
                    gpui_component::button::ButtonVariant::Ghost
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.view = EntryScreenView::NewProject;
                    cx.notify();
                }))
        )
        .child(
            Button::new("clone-git")
                .icon(IconName::GitHub)
                .label("")
                .tooltip("Clone from Git")
                .with_variant(if screen.view == EntryScreenView::CloneGit {
                    gpui_component::button::ButtonVariant::Primary
                } else {
                    gpui_component::button::ButtonVariant::Ghost
                })
                .on_click(cx.listener(|this, _, _, cx| {
                    this.view = EntryScreenView::CloneGit;
                    cx.notify();
                }))
        )
        .child(div().flex_1())
        .child(
            Button::new("open-existing")
                .icon(IconName::FolderOpen)
                .label("")
                .tooltip("Open Existing Project")
                .with_variant(gpui_component::button::ButtonVariant::Ghost)
                .on_click(cx.listener(|this, _, window, cx| {
                    this.open_folder_dialog(window, cx);
                }))
        )
        .child(
            Button::new("settings")
                .icon(IconName::Settings)
                .label("")
                .tooltip("Settings")
                .with_variant(gpui_component::button::ButtonVariant::Ghost)
                .on_click(cx.listener(|_, _, window, cx| {
                    window.dispatch_action(Box::new(OpenSettings), cx);
                }))
        )
}
