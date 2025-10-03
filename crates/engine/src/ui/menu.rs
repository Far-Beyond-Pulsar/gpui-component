use std::rc::Rc;

use gpui::{
    actions, div, prelude::FluentBuilder as _, px, Action, AnyElement, App, AppContext, ClickEvent, Context, Corner,
    Entity, FocusHandle, InteractiveElement as _, IntoElement, MouseButton, ParentElement as _,
    Render, SharedString, Styled as _, Subscription, Window,
};
use gpui_component::{
    badge::Badge,
    button::{Button, ButtonVariants as _},
    locale,
    popup_menu::PopupMenuExt as _,
    scroll::ScrollbarShow,
    set_locale, ActiveTheme as _, ContextModal as _, IconName, Sizable as _, Theme, ThemeMode,
    TitleBar, h_flex,
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

pub struct AppTitleBar {
    title: SharedString,
    main_menu: Entity<MainMenu>,
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
        let main_menu = cx.new(|cx| MainMenu::new(window, cx));
        let locale_selector = cx.new(|cx| LocaleSelector::new(window, cx));
        let font_size_selector = cx.new(|cx| FontSizeSelector::new(window, cx));
        let theme_switcher = cx.new(|cx| ThemeSwitcher::new(cx));

        Self {
            title: title.into(),
            main_menu,
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

impl Render for AppTitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let notifications_count = window.notifications(cx).len();

        TitleBar::new()
            // left side with title and main menu
            .child(
                h_flex()
                    .items_center()
                    .gap_4()
                    .child(self.title.clone())
                    .child(self.main_menu.clone())
            )
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

struct MainMenu {
    focus_handle: FocusHandle,
}

impl MainMenu {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
        }
    }

    fn on_menu_action(&mut self, action: &dyn Action, _: &mut Window, cx: &mut Context<Self>) {
        // Handle menu actions here
        cx.notify();
    }
}

impl Render for MainMenu {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .items_center()
            .gap_0()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, action: &NewFile, window, cx| {
                this.on_menu_action(action, window, cx);
            }))
            .on_action(cx.listener(|this, action: &OpenFile, window, cx| {
                this.on_menu_action(action, window, cx);
            }))
            .on_action(cx.listener(|this, action: &SaveFile, window, cx| {
                this.on_menu_action(action, window, cx);
            }))
            .on_action(cx.listener(|this, action: &Copy, window, cx| {
                this.on_menu_action(action, window, cx);
            }))
            .on_action(cx.listener(|this, action: &Paste, window, cx| {
                this.on_menu_action(action, window, cx);
            }))
            .on_action(cx.listener(|this, action: &Cut, window, cx| {
                this.on_menu_action(action, window, cx);
            }))
            .on_action(cx.listener(|this, action: &Find, window, cx| {
                this.on_menu_action(action, window, cx);
            }))
            .on_action(cx.listener(|this, action: &Build, window, cx| {
                this.on_menu_action(action, window, cx);
            }))
            .on_action(cx.listener(|this, action: &RunProject, window, cx| {
                this.on_menu_action(action, window, cx);
            }))
            // File Menu
            .child(
                Button::new("file-menu")
                    .label("File")
                    .ghost()
                    .small()
                    .popup_menu(|this, window, cx| {
                        this
                            .menu_with_icon("New File", IconName::PagePlus, Box::new(NewFile))
                            .menu_with_icon("New Project", IconName::FolderPlus, Box::new(NewProject))
                            .separator()
                            .menu_with_icon("Open File", IconName::FolderOpen, Box::new(OpenFile))
                            .menu_with_icon("Open Folder", IconName::FolderOpen, Box::new(OpenFolder))
                            .submenu("Open Recent", window, cx, |menu, _, _| {
                                menu
                                    .menu("project1.rs", Box::new(OpenRecent))
                                    .menu("project2.rs", Box::new(OpenRecent))
                                    .separator()
                                    .menu("Clear Recent", Box::new(OpenRecent))
                            })
                            .separator()
                            .menu_with_icon("Save", IconName::FloppyDisk, Box::new(SaveFile))
                            .menu("Save As...", Box::new(SaveAs))
                            .menu("Save All", Box::new(SaveAll))
                            .separator()
                            .menu("Close File", Box::new(CloseFile))
                            .menu("Close Folder", Box::new(CloseFolder))
                            .menu("Close All", Box::new(CloseAll))
                    })
            )
            // Edit Menu
            .child(
                Button::new("edit-menu")
                    .label("Edit")
                    .ghost()
                    .small()
                    .popup_menu(|this, window, cx| {
                        this
                            .menu("Undo", Box::new(Undo))
                            .menu("Redo", Box::new(Redo))
                            .separator()
                            .menu_with_icon("Cut", IconName::Scissor, Box::new(Cut))
                            .menu_with_icon("Copy", IconName::Copy, Box::new(Copy))
                            .menu_with_icon("Paste", IconName::PasteClipboard, Box::new(Paste))
                            .separator()
                            .menu("Select All", Box::new(SelectAll))
                            .separator()
                            .menu_with_icon("Find", IconName::Search, Box::new(Find))
                            .menu("Find & Replace", Box::new(FindReplace))
                            .menu("Find in Files", Box::new(FindInFiles))
                    })
            )
            // Selection Menu
            .child(
                Button::new("selection-menu")
                    .label("Selection")
                    .ghost()
                    .small()
                    .popup_menu(|this, window, cx| {
                        this
                            .menu("Select Line", Box::new(SelectLine))
                            .menu("Select Word", Box::new(SelectWord))
                            .separator()
                            .menu("Expand Selection", Box::new(ExpandSelection))
                            .menu("Shrink Selection", Box::new(ShrinkSelection))
                            .separator()
                            .menu("Add Cursor Above", Box::new(AddCursorAbove))
                            .menu("Add Cursor Below", Box::new(AddCursorBelow))
                    })
            )
            // Build Menu
            .child(
                Button::new("build-menu")
                    .label("Build")
                    .ghost()
                    .small()
                    .popup_menu(|this, window, cx| {
                        this
                            .menu_with_icon("Build", IconName::Tools, Box::new(Build))
                            .menu("Rebuild", Box::new(Rebuild))
                            .menu("Clean", Box::new(Clean))
                            .separator()
                            .menu("Build & Run", Box::new(BuildAndRun))
                            .separator()
                            .menu("Run Tests", Box::new(RunTests))
                    })
            )
            // View Menu
            .child(
                Button::new("view-menu")
                    .label("View")
                    .ghost()
                    .small()
                    .popup_menu(|this, window, cx| {
                        this
                            .menu_with_check("Explorer", true, Box::new(ToggleExplorer))
                            .menu_with_check("Terminal", true, Box::new(ToggleTerminal))
                            .menu_with_check("Output", false, Box::new(ToggleOutput))
                            .menu_with_check("Problems", false, Box::new(ToggleProblems))
                            .separator()
                            .menu("Zoom In", Box::new(ZoomIn))
                            .menu("Zoom Out", Box::new(ZoomOut))
                            .menu("Reset Zoom", Box::new(ResetZoom))
                            .separator()
                            .menu("Toggle Fullscreen", Box::new(ToggleFullscreen))
                    })
            )
            // Go Menu
            .child(
                Button::new("go-menu")
                    .label("Go")
                    .ghost()
                    .small()
                    .popup_menu(|this, window, cx| {
                        this
                            .menu("Go to File", Box::new(GoToFile))
                            .menu("Go to Line", Box::new(GoToLine))
                            .menu("Go to Symbol", Box::new(GoToSymbol))
                            .separator()
                            .menu("Go to Definition", Box::new(GoToDefinition))
                            .menu("Go to References", Box::new(GoToReferences))
                            .separator()
                            .menu("Go Back", Box::new(GoBack))
                            .menu("Go Forward", Box::new(GoForward))
                    })
            )
            // Run Menu
            .child(
                Button::new("run-menu")
                    .label("Run")
                    .ghost()
                    .small()
                    .popup_menu(|this, window, cx| {
                        this
                            .menu_with_icon("Run Project", IconName::Play, Box::new(RunProject))
                            .menu_with_icon("Debug Project", IconName::Bug, Box::new(DebugProject))
                            .menu("Run without Debugging", Box::new(RunWithoutDebugging))
                            .separator()
                            .menu("Stop Debugging", Box::new(StopDebugging))
                            .menu("Restart Debugging", Box::new(RestartDebugging))
                    })
            )
            // Terminal Menu
            .child(
                Button::new("terminal-menu")
                    .label("Terminal")
                    .ghost()
                    .small()
                    .popup_menu(|this, window, cx| {
                        this
                            .menu_with_icon("New Terminal", IconName::SquareTerminal, Box::new(NewTerminal))
                            .menu("Split Terminal", Box::new(SplitTerminal))
                            .separator()
                            .menu("Clear Terminal", Box::new(ClearTerminal))
                    })
            )
            // Help Menu
            .child(
                Button::new("help-menu")
                    .label("Help")
                    .ghost()
                    .small()
                    .popup_menu(|this, window, cx| {
                        this
                            .menu("Show Commands", Box::new(ShowCommands))
                            .separator()
                            .link_with_icon("Documentation", IconName::BookOpen, "https://docs.rs")
                            .link_with_icon("Report Issue", IconName::GitHub, "https://github.com/issues")
                            .separator()
                            .menu_with_icon("About", IconName::InfoCircle, Box::new(AboutApp))
                    })
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
        let font_size = cx.theme().font_size.0 as i32;
        let radius = cx.theme().radius.0 as i32;
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
                            .menu_with_check("Large", font_size == 18, Box::new(SelectFont(18)))
                            .menu_with_check(
                                "Medium (default)",
                                font_size == 16,
                                Box::new(SelectFont(16)),
                            )
                            .menu_with_check("Small", font_size == 14, Box::new(SelectFont(14)))
                            .separator()
                            .label("Border Radius")
                            .menu_with_check("8px", radius == 8, Box::new(SelectRadius(8)))
                            .menu_with_check(
                                "6px (default)",
                                radius == 6,
                                Box::new(SelectRadius(6)),
                            )
                            .menu_with_check("4px", radius == 4, Box::new(SelectRadius(4)))
                            .menu_with_check("0px", radius == 0, Box::new(SelectRadius(0)))
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
