use gpui::{prelude::*, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    h_flex, v_flex, StyledExt, TitleBar, Icon, IconName, ActiveTheme as _,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum EntryScreenView {
    Recent,
    Templates,
}

/// EntryScreen: Modern entry UI with sidebar navigation for recent projects and templates.
pub struct EntryScreen {
    view: EntryScreenView,
}

impl EntryScreen {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { view: EntryScreenView::Recent }
    }

    //
}

impl Render for EntryScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        v_flex()
            .size_full()
            .bg(theme.background)
            // Hero/branding section with gradient and glass effect
            .child(
                div()
                    .h(px(180.))
                    .w_full()
                    .rounded_lg()
                    .bg_linear_gradient(
                        90.0,
                        theme.primary.opacity(0.85),
                        theme.secondary.opacity(0.85),
                    )
                    .backdrop_blur(px(16.))
                    .shadow_lg()
                    .flex()
                    .items_center()
                    .justify_between()
                    .px_12()
                    .child(
                        h_flex()
                            .gap_4()
                            .items_center()
                            .child(
                                Icon::new(IconName::Rocket)
                                    .size(px(48.))
                                    .text_color(theme.primary_foreground)
                                    .shadow_md()
                            )
                            .child(
                                div()
                                    .text_4xl()
                                    .font_extrabold()
                                    .text_color(theme.primary_foreground)
                                    .child("Pulsar Engine Launcher")
                            )
                    )
                    .child(
                        Button::new("launch-btn")
                            .label("Launch Project")
                            .large()
                            .with_variant(gpui_component::button::ButtonVariant::Primary)
                            .icon(IconName::Play)
                            .tooltip("Launch the selected project")
                    )
            )
            // Main content area
            .child(
                h_flex()
                    .size_full()
                    .child(
                        // Sidebar with icons and tooltips
                        v_flex()
                            .w(px(72.))
                            .h_full()
                            .bg(theme.sidebar)
                            .border_r_1()
                            .border_color(theme.border)
                            .gap_4()
                            .items_center()
                            .pt_8()
                            .child(
                                Button::new("recent-projects")
                                    .icon(IconName::FolderClosed)
                                    .label("")
                                    .tooltip("Recent Projects")
                                    .with_variant(gpui_component::button::ButtonVariant::Ghost)
                                    .on_click(cx.listener(|this: &mut Self, _, _, _| this.view = EntryScreenView::Recent))
                            )
                            .child(
                                Button::new("templates")
                                    .icon(IconName::Star)
                                    .label("")
                                    .tooltip("Templates")
                                    .with_variant(gpui_component::button::ButtonVariant::Ghost)
                                    .on_click(cx.listener(|this: &mut Self, _, _, _| this.view = EntryScreenView::Templates))
                            )
                            .child(
                                Button::new("settings")
                                    .icon(IconName::Settings)
                                    .label("")
                                    .tooltip("Settings")
                                    .with_variant(gpui_component::button::ButtonVariant::Ghost)
                            )
                    )
                    .child(
                        // Main area: grid of cards, progress, dividers, glass, etc.
                        v_flex()
                            .flex_1()
                            .h_full()
                            .scrollable(gpui_component::scroll::ScrollbarAxis::Vertical)
                            .bg(theme.background)
                            .gap_y_8()
                            .p_12()
                            .child(
                                match self.view {
                                    EntryScreenView::Recent => {
                                        v_flex()
                                            .gap_y_6()
                                            .child(div().text_2xl().font_bold().child("Recent Projects"))
                                            .child(gpui_component::divider::Divider::horizontal())
                                            .child(
                                                h_flex()
                                                    .gap_x_8()
                                                    .h(px(220.))
                                                    .child(
                                                        v_flex()
                                                            .flex_1()
                                                            .h_full()
                                                            .border_1()
                                                            .border_color(theme.border)
                                                            .rounded_lg()
                                                            .p_6()
                                                            .bg(theme.sidebar)
                                                            .shadow_lg()
                                                            .child(
                                                                h_flex()
                                                                    .gap_2()
                                                                    .items_center()
                                                                    .child(Icon::new(IconName::Gamepad2).size(px(28.)).text_color(theme.primary))
                                                                    .child(div().font_semibold().child("Project 1"))
                                                                    .child(
                                                                        gpui_component::badge::Badge::new("v1.2.3")
                                                                            .color(theme.primary)
                                                                    )
                                                            )
                                                            .child(div().text_color(theme.muted_foreground).text_sm().child("/path/to/project1"))
                                                            .child(
                                                                gpui_component::progress::Progress::new().value(75.)
                                                            )
                                                            .child(
                                                                Button::new("open1")
                                                                    .label("Open")
                                                                    .icon(IconName::ArrowRight)
                                                                    .tooltip("Open this project")
                                                                    .with_variant(gpui_component::button::ButtonVariant::Secondary)
                                                            )
                                                    )
                                                    .child(
                                                        v_flex()
                                                            .flex_1()
                                                            .h_full()
                                                            .border_1()
                                                            .border_color(theme.border)
                                                            .rounded_lg()
                                                            .p_6()
                                                            .bg(theme.sidebar)
                                                            .shadow_lg()
                                                            .child(
                                                                h_flex()
                                                                    .gap_2()
                                                                    .items_center()
                                                                    .child(Icon::new(IconName::Gamepad2).size(px(28.)).text_color(theme.primary))
                                                                    .child(div().font_semibold().child("Project 2"))
                                                                    .child(
                                                                        gpui_component::badge::Badge::new("v2.0.0-beta")
                                                                            .color(theme.secondary)
                                                                    )
                                                            )
                                                            .child(div().text_color(theme.muted_foreground).text_sm().child("/path/to/project2"))
                                                            .child(
                                                                gpui_component::progress::Progress::new().value(40.)
                                                            )
                                                            .child(
                                                                Button::new("open2")
                                                                    .label("Open")
                                                                    .icon(IconName::ArrowRight)
                                                                    .tooltip("Open this project")
                                                                    .with_variant(gpui_component::button::ButtonVariant::Secondary)
                                                            )
                                                    )
                                            )
                                    }
                                    EntryScreenView::Templates => {
                                        v_flex()
                                            .gap_y_6()
                                            .child(div().text_2xl().font_bold().child("Templates"))
                                            .child(gpui_component::divider::Divider::horizontal())
                                            .child(
                                                h_flex()
                                                    .gap_x_8()
                                                    .h(px(220.))
                                                    .child(
                                                        v_flex()
                                                            .flex_1()
                                                            .h_full()
                                                            .border_1()
                                                            .border_color(theme.border)
                                                            .rounded_lg()
                                                            .p_6()
                                                            .bg(theme.sidebar)
                                                            .shadow_lg()
                                                            .child(
                                                                h_flex()
                                                                    .gap_2()
                                                                    .items_center()
                                                                    .child(Icon::new(IconName::Sparkles).size(px(28.)).text_color(theme.primary))
                                                                    .child(div().font_semibold().child("Blank Project"))
                                                                    .child(
                                                                        gpui_component::badge::Badge::new("New")
                                                                            .color(theme.primary)
                                                                    )
                                                            )
                                                            .child(div().text_color(theme.muted_foreground).text_sm().child("A new empty project"))
                                                            .child(
                                                                Button::new("create_blank")
                                                                    .label("Create")
                                                                    .icon(IconName::Plus)
                                                                    .tooltip("Create a blank project")
                                                                    .with_variant(gpui_component::button::ButtonVariant::Primary)
                                                            )
                                                    )
                                                    .child(
                                                        v_flex()
                                                            .flex_1()
                                                            .h_full()
                                                            .border_1()
                                                            .border_color(theme.border)
                                                            .rounded_lg()
                                                            .p_6()
                                                            .bg(theme.sidebar)
                                                            .shadow_lg()
                                                            .child(
                                                                h_flex()
                                                                    .gap_2()
                                                                    .items_center()
                                                                    .child(Icon::new(IconName::Sparkles).size(px(28.)).text_color(theme.primary))
                                                                    .child(div().font_semibold().child("2D Platformer"))
                                                                    .child(
                                                                        gpui_component::badge::Badge::new("Popular")
                                                                            .color(theme.secondary)
                                                                    )
                                                            )
                                                            .child(div().text_color(theme.muted_foreground).text_sm().child("A 2D platformer template"))
                                                            .child(
                                                                Button::new("create_2d")
                                                                    .label("Create")
                                                                    .icon(IconName::Plus)
                                                                    .tooltip("Create a 2D platformer project")
                                                                    .with_variant(gpui_component::button::ButtonVariant::Primary)
                                                            )
                                                    )
                                                    .child(
                                                        v_flex()
                                                            .flex_1()
                                                            .h_full()
                                                            .border_1()
                                                            .border_color(theme.border)
                                                            .rounded_lg()
                                                            .p_6()
                                                            .bg(theme.sidebar)
                                                            .shadow_lg()
                                                            .child(
                                                                h_flex()
                                                                    .gap_2()
                                                                    .items_center()
                                                                    .child(Icon::new(IconName::Sparkles).size(px(28.)).text_color(theme.primary))
                                                                    .child(div().font_semibold().child("3D First-Person"))
                                                                    .child(
                                                                        gpui_component::badge::Badge::new("Beta")
                                                                            .color(theme.primary)
                                                                    )
                                                            )
                                                            .child(div().text_color(theme.muted_foreground).text_sm().child("A 3D FPS template"))
                                                            .child(
                                                                Button::new("create_3d")
                                                                    .label("Create")
                                                                    .icon(IconName::Plus)
                                                                    .tooltip("Create a 3D FPS project")
                                                                    .with_variant(gpui_component::button::ButtonVariant::Primary)
                                                            )
                                                    )
                                            )
                                    }
                                }
                            )
                    )
            )
    }
}
