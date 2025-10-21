use std::rc::Rc;

use gpui::{
    actions, div, prelude::FluentBuilder as _, px, AnyElement, App, AppContext, ClickEvent,
    Context, Corner, Entity, FocusHandle, InteractiveElement as _, IntoElement, Menu, MenuItem,
    MouseButton, ParentElement as _, Render, SharedString, Styled as _, Subscription, Window,
};
use gpui_component::{
    badge::Badge,
    button::{Button, ButtonVariants as _},
    locale,
    menu::AppMenuBar,
    popup_menu::PopupMenuExt as _,
    scroll::ScrollbarShow,
    set_locale, ActiveTheme as _, ContextModal as _, IconName, PixelsExt, Sizable as _, Theme,
    ThemeMode, TitleBar,
};

use crate::{themes::ThemeSwitcher, SelectFont, SelectLocale, SelectRadius, SelectScrollbarShow};

// Define actions for the main menu
actions!(
    menu,
    [
        // File menu
        NewFile,
        NewProject,
        OpenFile,
        OpenFolder,
        OpenRecent,
        SaveFile,
        SaveAs,
        SaveAll,
        CloseFile,
        CloseFolder,
        CloseAll,
        // Edit menu
        Undo,
        Redo,
        Cut,
        Copy,
        Paste,
        SelectAll,
        Find,
        FindReplace,
        FindInFiles,
        // Selection menu
        SelectLine,
        SelectWord,
        ExpandSelection,
        ShrinkSelection,
        AddCursorAbove,
        AddCursorBelow,
        // Build menu
        Build,
        Rebuild,
        Clean,
        BuildAndRun,
        RunTests,
        // View menu
        ToggleExplorer,
        ToggleTerminal,
        ToggleOutput,
        ToggleProblems,
        ZoomIn,
        ZoomOut,
        ResetZoom,
        ToggleFullscreen,
        // Go menu
        GoToFile,
        GoToLine,
        GoToSymbol,
        GoToDefinition,
        GoToReferences,
        GoBack,
        GoForward,
        // Run menu
        RunProject,
        DebugProject,
        RunWithoutDebugging,
        StopDebugging,
        RestartDebugging,
        // Terminal menu
        NewTerminal,
        SplitTerminal,
        ClearTerminal,
        // Help menu
        ShowCommands,
        OpenDocumentation,
        ReportIssue,
        AboutApp
    ]
);

/// Initialize the app menus
pub fn init_app_menus(title: impl Into<SharedString>, cx: &mut App) {
    cx.set_menus(vec![
        Menu {
            name: title.into(),
            items: vec![MenuItem::action("About", AboutApp)],
        },
        Menu {
            name: "File".into(),
            items: vec![
                MenuItem::action("New File", NewFile),
                MenuItem::action("New Project", NewProject),
                MenuItem::separator(),
                MenuItem::action("Open File", OpenFile),
                MenuItem::action("Open Folder", OpenFolder),
                MenuItem::action("Open Recent", OpenRecent),
                MenuItem::separator(),
                MenuItem::action("Save", SaveFile),
                MenuItem::action("Save As...", SaveAs),
                MenuItem::action("Save All", SaveAll),
                MenuItem::separator(),
                MenuItem::action("Close File", CloseFile),
                MenuItem::action("Close Folder", CloseFolder),
                MenuItem::action("Close All", CloseAll),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::action("Undo", Undo),
                MenuItem::action("Redo", Redo),
                MenuItem::separator(),
                MenuItem::action("Cut", Cut),
                MenuItem::action("Copy", Copy),
                MenuItem::action("Paste", Paste),
                MenuItem::separator(),
                MenuItem::action("Select All", SelectAll),
                MenuItem::separator(),
                MenuItem::action("Find", Find),
                MenuItem::action("Find & Replace", FindReplace),
                MenuItem::action("Find in Files", FindInFiles),
            ],
        },
        Menu {
            name: "Selection".into(),
            items: vec![
                MenuItem::action("Select Line", SelectLine),
                MenuItem::action("Select Word", SelectWord),
                MenuItem::separator(),
                MenuItem::action("Expand Selection", ExpandSelection),
                MenuItem::action("Shrink Selection", ShrinkSelection),
                MenuItem::separator(),
                MenuItem::action("Add Cursor Above", AddCursorAbove),
                MenuItem::action("Add Cursor Below", AddCursorBelow),
            ],
        },
        Menu {
            name: "Build".into(),
            items: vec![
                MenuItem::action("Build", Build),
                MenuItem::action("Rebuild", Rebuild),
                MenuItem::action("Clean", Clean),
                MenuItem::separator(),
                MenuItem::action("Build & Run", BuildAndRun),
                MenuItem::separator(),
                MenuItem::action("Run Tests", RunTests),
            ],
        },
        Menu {
            name: "View".into(),
            items: vec![
                MenuItem::action("Toggle Explorer", ToggleExplorer),
                MenuItem::action("Toggle Terminal", ToggleTerminal),
                MenuItem::action("Toggle Output", ToggleOutput),
                MenuItem::action("Toggle Problems", ToggleProblems),
                MenuItem::separator(),
                MenuItem::action("Zoom In", ZoomIn),
                MenuItem::action("Zoom Out", ZoomOut),
                MenuItem::action("Reset Zoom", ResetZoom),
                MenuItem::separator(),
                MenuItem::action("Toggle Fullscreen", ToggleFullscreen),
            ],
        },
        Menu {
            name: "Go".into(),
            items: vec![
                MenuItem::action("Go to File", GoToFile),
                MenuItem::action("Go to Line", GoToLine),
                MenuItem::action("Go to Symbol", GoToSymbol),
                MenuItem::separator(),
                MenuItem::action("Go to Definition", GoToDefinition),
                MenuItem::action("Go to References", GoToReferences),
                MenuItem::separator(),
                MenuItem::action("Go Back", GoBack),
                MenuItem::action("Go Forward", GoForward),
            ],
        },
        Menu {
            name: "Run".into(),
            items: vec![
                MenuItem::action("Run Project", RunProject),
                MenuItem::action("Debug Project", DebugProject),
                MenuItem::action("Run without Debugging", RunWithoutDebugging),
                MenuItem::separator(),
                MenuItem::action("Stop Debugging", StopDebugging),
                MenuItem::action("Restart Debugging", RestartDebugging),
            ],
        },
        Menu {
            name: "Terminal".into(),
            items: vec![
                MenuItem::action("New Terminal", NewTerminal),
                MenuItem::action("Split Terminal", SplitTerminal),
                MenuItem::separator(),
                MenuItem::action("Clear Terminal", ClearTerminal),
            ],
        },
        Menu {
            name: "Help".into(),
            items: vec![
                MenuItem::action("Show Commands", ShowCommands),
                MenuItem::separator(),
                MenuItem::action("Documentation", OpenDocumentation),
                MenuItem::action("Report Issue", ReportIssue),
                MenuItem::separator(),
                MenuItem::action("About", AboutApp),
            ],
        },
    ]);
}

pub struct AppTitleBar {
    app_menu_bar: Entity<AppMenuBar>,
    locale_selector: Entity<LocaleSelector>,
    font_size_selector: Entity<FontSizeSelector>,
    theme_switcher: Entity<ThemeSwitcher>,
    child: Rc<dyn Fn(&mut Window, &mut App) -> AnyElement>,
    _subscriptions: Vec<Subscription>,
}

impl AppTitleBar {
    pub fn new(
        title: impl Into<SharedString>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        init_app_menus(title, cx);

        let app_menu_bar = AppMenuBar::new(window, cx);
        let locale_selector = cx.new(|cx| LocaleSelector::new(window, cx));
        let font_size_selector = cx.new(|cx| FontSizeSelector::new(window, cx));
        let theme_switcher = cx.new(|cx| ThemeSwitcher::new(cx));

        Self {
            app_menu_bar,
            locale_selector,
            font_size_selector,
            theme_switcher,
            child: Rc::new(|_, _| div().into_any_element()),
            _subscriptions: vec![],
        }
    }

    pub fn child<F, E>(mut self, f: F) -> Self
    where
        E: IntoElement,
        F: Fn(&mut Window, &mut App) -> E + 'static,
    {
        self.child = Rc::new(move |window, cx| f(window, cx).into_any_element());
        self
    }

    fn change_color_mode(&mut self, _: &ClickEvent, _: &mut Window, cx: &mut Context<Self>) {
        let mode = match cx.theme().mode.is_dark() {
            true => ThemeMode::Light,
            false => ThemeMode::Dark,
        };

        Theme::change(mode, None, cx);
    }
}

// TODO: (From @tristanpoland) Near as I can tell this println! call is never executed. Look into this when debugging the titlebar
impl Render for AppTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let notifications_count = window.notifications(cx).len();

        TitleBar::new()
            // left side with app menu bar
            .child(div().flex().items_center().child(self.app_menu_bar.clone()))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_end()
                    .px_2()
                    .gap_2()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child((self.child.clone())(window, cx))
                    .child(self.theme_switcher.clone())
                    .child(
                        Button::new("theme-mode")
                            .map(|this| {
                                if cx.theme().mode.is_dark() {
                                    this.icon(IconName::Sun)
                                } else {
                                    this.icon(IconName::Moon)
                                }
                            })
                            .small()
                            .ghost()
                            .on_click(cx.listener(Self::change_color_mode)),
                    )
                    .child(self.locale_selector.clone())
                    .child(self.font_size_selector.clone())
                    .child(
                        Button::new("github")
                            .icon(IconName::GitHub)
                            .small()
                            .ghost()
                            .on_click(|_, _, cx| {
                                cx.open_url("https://github.com/longbridge/gpui-component")
                            }),
                    )
                    .child(
                        div().relative().child(
                            Badge::new().count(notifications_count).max(99).child(
                                Button::new("bell")
                                    .small()
                                    .ghost()
                                    .compact()
                                    .icon(IconName::Bell),
                            ),
                        ),
                    ),
            )
    }
}

struct LocaleSelector {
    focus_handle: FocusHandle,
}

impl LocaleSelector {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    fn on_select_locale(
        &mut self,
        locale: &SelectLocale,
        window: &mut Window,
        _: &mut Context<Self>,
    ) {
        set_locale(&locale.0);
        window.refresh();
    }
}

impl Render for LocaleSelector {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.focus_handle.clone();
        let locale = locale().to_string();

        div()
            .id("locale-selector")
            .track_focus(&focus_handle)
            .on_action(cx.listener(Self::on_select_locale))
            .child(
                Button::new("btn")
                    .small()
                    .ghost()
                    .icon(IconName::Globe)
                    .popup_menu(move |this, _, _| {
                        this.menu_with_check(
                            "English",
                            locale == "en",
                            Box::new(SelectLocale("en".into())),
                        )
                        .menu_with_check(
                            "简体中文",
                            locale == "zh-CN",
                            Box::new(SelectLocale("zh-CN".into())),
                        )
                    })
                    .anchor(Corner::TopRight),
            )
    }
}

struct FontSizeSelector {
    focus_handle: FocusHandle,
}

impl FontSizeSelector {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    fn on_select_font(
        &mut self,
        font_size: &SelectFont,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        Theme::global_mut(cx).font_size = px(font_size.0 as f32);
        window.refresh();
    }

    fn on_select_radius(
        &mut self,
        radius: &SelectRadius,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        Theme::global_mut(cx).radius = px(radius.0 as f32);
        window.refresh();
    }

    fn on_select_scrollbar_show(
        &mut self,
        show: &SelectScrollbarShow,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        Theme::global_mut(cx).scrollbar_show = show.0;
        window.refresh();
    }
}

impl Render for FontSizeSelector {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let focus_handle = self.focus_handle.clone();
        let font_size = cx.theme().font_size.as_f32();
        let radius = cx.theme().radius.as_f32();
        let scroll_show = cx.theme().scrollbar_show;

        div()
            .id("font-size-selector")
            .track_focus(&focus_handle)
            .on_action(cx.listener(Self::on_select_font))
            .on_action(cx.listener(Self::on_select_radius))
            .on_action(cx.listener(Self::on_select_scrollbar_show))
            .child(
                Button::new("btn")
                    .small()
                    .ghost()
                    .icon(IconName::Settings2)
                    .popup_menu(move |this, _, _| {
                        this.scrollable()
                            .max_h(px(480.))
                            .label("Font Size")
                            .menu_with_check("Large", font_size == 18.0, Box::new(SelectFont(18)))
                            .menu_with_check(
                                "Medium (default)",
                                font_size == 16.0,
                                Box::new(SelectFont(16)),
                            )
                            .menu_with_check("Small", font_size == 14.0, Box::new(SelectFont(14)))
                            .separator()
                            .label("Border Radius")
                            .menu_with_check("8px", radius == 8.0, Box::new(SelectRadius(8)))
                            .menu_with_check(
                                "6px (default)",
                                radius == 6.0,
                                Box::new(SelectRadius(6)),
                            )
                            .menu_with_check("4px", radius == 4.0, Box::new(SelectRadius(4)))
                            .menu_with_check("0px", radius == 0.0, Box::new(SelectRadius(0)))
                            .separator()
                            .label("Scrollbar")
                            .menu_with_check(
                                "Scrolling to show",
                                scroll_show == ScrollbarShow::Scrolling,
                                Box::new(SelectScrollbarShow(ScrollbarShow::Scrolling)),
                            )
                            .menu_with_check(
                                "Hover to show",
                                scroll_show == ScrollbarShow::Hover,
                                Box::new(SelectScrollbarShow(ScrollbarShow::Hover)),
                            )
                            .menu_with_check(
                                "Always show",
                                scroll_show == ScrollbarShow::Always,
                                Box::new(SelectScrollbarShow(ScrollbarShow::Always)),
                            )
                    })
                    .anchor(Corner::TopRight),
            )
    }
}
