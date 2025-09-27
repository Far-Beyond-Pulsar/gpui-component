mod assets;

use std::time::Duration;

use assets::Assets;
use futures_util::StreamExt;
use gpui::{
    div, px, size, App, AppContext, Application, Bounds, Context,
    Entity, InteractiveElement, IntoElement, ParentElement, Render, Styled, Timer, Window,
    WindowBounds, WindowOptions,
};
use gpui::prelude::FluentBuilder;
use gpui_component::{
    button::{Button, ButtonVariants},
    input::{InputEvent, InputState, TextInput},
    scroll::ScrollbarAxis,
    ActiveTheme, IconName, Root, Selectable, Sizable, StyledExt, TitleBar, h_flex, v_flex,
};
use gpui_webview::{
    events::TitleChangedEvent,
    wef::{self, Frame, FuncRegistry, Settings},
    WebView,
};
use serde::Serialize;

#[derive(Clone)]
struct BrowserTab {
    id: usize,
    title: String,
    url: String,
    webview: Entity<WebView>,
    address_state: Entity<InputState>,
}

struct Main {
    tabs: Vec<BrowserTab>,
    active_tab_index: usize,
    next_tab_id: usize,
    func_registry: FuncRegistry,
}

impl Main {
    fn new(window: &mut Window, cx: &mut App) -> Entity<Self> {
        let background_executor = cx.background_executor().clone();

        let func_registry = FuncRegistry::builder()
            .with_spawner(move |fut| {
                background_executor.spawn(fut).detach();
            })
            .register("toUppercase", |value: String| value.to_uppercase())
            .register("addInt", |a: i32, b: i32| a + b)
            .register("parseInt", |value: String| value.parse::<i32>())
            .register_async("sleep", |millis: u64| async move {
                Timer::after(Duration::from_millis(millis)).await;
                "ok"
            })
            .register("emit", |frame: Frame| {
                #[derive(Debug, Serialize)]
                struct Message {
                    event: String,
                    data: String,
                }

                frame.emit(Message {
                    event: "custom".to_string(),
                    data: "ok".to_string(),
                });
            })
            .build();

        cx.new(|cx| {
            let mut main = Self {
                tabs: Vec::new(),
                active_tab_index: 0,
                next_tab_id: 0,
                func_registry,
            };

            // Create initial tab
            main.create_new_tab("https://www.google.com", window, cx);
            main
        })
    }

    fn create_new_tab(&mut self, url: &str, window: &mut Window, cx: &mut Context<Self>) {
        let tab_id = self.next_tab_id;
        self.next_tab_id += 1;

        // Create webview for the tab
        let webview = WebView::with_func_registry(url, self.func_registry.clone(), window, cx);

        // Create address input for the tab
        let url_string = url.to_string();
        let address_state = cx.new(|cx| InputState::new(window, cx).default_value(&url_string));

        // Subscribe to address input events
        window
            .subscribe(&address_state, cx, {
                let webview = webview.clone();
                move |state, event: &InputEvent, _, cx| {
                    if let InputEvent::PressEnter { .. } = event {
                        let url = state.read(cx).value();
                        webview.read(cx).browser().load_url(&url);
                    }
                }
            })
            .detach();

        // Subscribe to title changes
        let webview_clone = webview.clone();
        let main_entity = cx.entity().clone();
        window
            .subscribe(&webview, cx, move |_, event: &TitleChangedEvent, window, cx| {
                main_entity.update(cx, |main, cx| {
                    let tab_id = {
                        if let Some(tab) = main.tabs.iter_mut().find(|t| t.webview == webview_clone) {
                            tab.title = event.title.clone();
                            Some(tab.id)
                        } else {
                            None
                        }
                    };
                    // Update window title if this is the active tab
                    if let Some(id) = tab_id {
                        if main.tabs[main.active_tab_index].id == id {
                            window.set_window_title(&event.title);
                        }
                    }
                    cx.notify();
                });
            })
            .detach();

        let tab = BrowserTab {
            id: tab_id,
            title: "New Tab".to_string(),
            url: url.to_string(),
            webview,
            address_state,
        };

        self.tabs.push(tab);
        self.active_tab_index = self.tabs.len() - 1;
        cx.notify();
    }

    fn close_tab(&mut self, tab_index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.len() <= 1 {
            return; // Don't close the last tab
        }

        self.tabs.remove(tab_index);

        if self.active_tab_index >= self.tabs.len() {
            self.active_tab_index = self.tabs.len() - 1;
        } else if tab_index <= self.active_tab_index && self.active_tab_index > 0 {
            self.active_tab_index -= 1;
        }

        // Update window title
        if let Some(active_tab) = self.tabs.get(self.active_tab_index) {
            window.set_window_title(&active_tab.title);
        }

        cx.notify();
    }

    fn set_active_tab(&mut self, tab_index: usize, window: &mut Window, cx: &mut Context<Self>) {
        if tab_index < self.tabs.len() {
            self.active_tab_index = tab_index;
            if let Some(active_tab) = self.tabs.get(self.active_tab_index) {
                window.set_window_title(&active_tab.title);
            }
            cx.notify();
        }
    }

    fn navigate_back(&self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_tab) = self.tabs.get(self.active_tab_index) {
            active_tab.webview.read(cx).browser().back();
        }
    }

    fn navigate_forward(&self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_tab) = self.tabs.get(self.active_tab_index) {
            active_tab.webview.read(cx).browser().forward();
        }
    }

    fn reload_page(&self, _window: &mut Window, cx: &mut Context<Self>) {
        if let Some(active_tab) = self.tabs.get(self.active_tab_index) {
            active_tab.webview.read(cx).browser().reload();
        }
    }

    fn render_integrated_titlebar(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        TitleBar::new()
            // Left side: Navigation controls (fixed width, won't be pushed off)
            .child(
                h_flex()
                    .items_center()
                    .gap_1()
                    .px_2()
                    .py_1() // Add vertical padding for easier grabbing
                    .flex_shrink_0() // Prevent from being compressed
                    // Stop propagation to allow controls to work without triggering window drag
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        Button::new("back")
                            .icon(IconName::ChevronLeft)
                            .small()
                            .ghost()
                            .tooltip("Go Back")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.navigate_back(window, cx);
                            })),
                    )
                    .child(
                        Button::new("forward")
                            .icon(IconName::ChevronRight)
                            .small()
                            .ghost()
                            .tooltip("Go Forward")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.navigate_forward(window, cx);
                            })),
                    )
                    .child(
                        Button::new("reload")
                            .icon(IconName::Replace)
                            .small()
                            .ghost()
                            .tooltip("Reload")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.reload_page(window, cx);
                            })),
                    )
            )
            // Middle: Tabs section with constrained width that scrolls internally
            .child(
                div()
                    .flex_1() // Take remaining space between nav and address bar
                    .py_1() // Add vertical padding for easier grabbing (draggable area!)
                    .px_1()
                    .min_w_0() // Allow shrinking
                    .overflow_hidden() // Hide overflow to enable scrolling
                    .child(
                        div()
                            .w_full()
                            .h_full()
                            .scrollable(ScrollbarAxis::Horizontal)
                            .id("tabs-scroll")
                            .child(
                                h_flex()
                                    .gap_1()
                                    .items_center()
                                    .w_auto() // Auto width to fit content
                                    // Don't stop propagation on the container - only on individual buttons
                                    .children(self.tabs.iter().enumerate().map(|(index, tab)| {
                                        h_flex()
                                            .items_center()
                                            .min_w_24()
                                            .max_w_48()
                                            .flex_shrink_0() // Prevent tabs from shrinking
                                            // Stop propagation only on tab buttons
                                            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                            .child(
                                                Button::new(("tab", index))
                                                    .child(
                                                        div()
                                                            .max_w_40()
                                                            .overflow_hidden()
                                                            .text_ellipsis()
                                                            .whitespace_nowrap()
                                                            .child(tab.title.clone())
                                                    )
                                                    .when(index == self.active_tab_index, |this| {
                                                        this.selected(true)
                                                    })
                                                    .ghost()
                                                    .small()
                                                    .on_click(cx.listener(move |this, _, window, cx| {
                                                        this.set_active_tab(index, window, cx);
                                                    }))
                                            )
                                            .when(self.tabs.len() > 1, |this| {
                                                this.child(
                                                    Button::new(("close-tab", index))
                                                        .icon(IconName::Close)
                                                        .xsmall()
                                                        .ghost()
                                                        .on_click(cx.listener(move |this, _, window, cx| {
                                                            this.close_tab(index, window, cx);
                                                        }))
                                                )
                                            })
                                    }))
                                    .child(
                                        div()
                                            // Stop propagation only on the new tab button
                                            .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                                            .child(
                                                Button::new("new-tab")
                                                    .icon(IconName::Plus)
                                                    .xsmall()
                                                    .ghost()
                                                    .tooltip("New Tab")
                                                    .flex_shrink_0() // Prevent new tab button from shrinking
                                                    .on_click(cx.listener(|this, _, window, cx| {
                                                        this.create_new_tab("https://www.google.com", window, cx);
                                                    })),
                                            )
                                    )
                            )
                    )
            )
            // Right side: Address bar and menu (fixed width, won't be pushed off)
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .w_80() // Fixed width to ensure it's never pushed off
                    .py_1() // Add vertical padding for easier grabbing
                    .px_2()
                    .flex_shrink_0() // Prevent from being compressed
                    // Stop propagation for interactive elements
                    .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .child(
                        div()
                            .flex_1()
                            .when_some(self.tabs.get(self.active_tab_index), |this, active_tab| {
                                this.child(TextInput::new(&active_tab.address_state))
                            })
                    )
                    .child(
                        Button::new("menu")
                            .icon(IconName::Menu)
                            .small()
                            .ghost()
                            .tooltip("Menu")
                    )
            )
    }


}

impl Render for Main {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().background)
            // Single integrated titlebar with tabs, navigation, address bar, and window controls
            .child(self.render_integrated_titlebar(window, cx))
            .child(
                div()
                    .flex_1()
                    .when_some(self.tabs.get(self.active_tab_index), |this, active_tab| {
                        this.child(active_tab.webview.clone())
                    })
            )
            .children(Root::render_modal_layer(window, cx))
    }
}

fn run() {
    Application::new().with_assets(Assets).run(|cx: &mut App| {
        if cfg!(target_os = "linux") {
            cx.spawn(async move |cx| {
                let (tx, rx) = flume::unbounded();

                cx.background_spawn(async move {
                    let mut timer = Timer::interval(Duration::from_millis(1000 / 60));
                    while timer.next().await.is_some() {
                        _ = tx.send_async(()).await;
                    }
                })
                .detach();

                while rx.recv_async().await.is_ok() {
                    wef::do_message_work();
                }
            })
            .detach();
        }

        gpui_component::init(cx);

        let bounds = Bounds::centered(None, size(px(1200.), px(800.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitleBar::title_bar_options()), // Use GPUI component titlebar options
                ..Default::default()
            },
            |window, cx| {
                let main = Main::new(window, cx);
                cx.new(|cx| Root::new(main.into(), window, cx))
            },
        )
        .unwrap();
        cx.activate(true);
    });
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure CEF settings with proper cache paths to avoid singleton warnings
    let exe_path = std::env::current_exe().unwrap();
    let exe_dir = exe_path.parent().unwrap();
    let cache_dir = exe_dir.join("cef_cache");
    let root_cache_dir = exe_dir.join("cef_root_cache");

    // Create cache directories if they don't exist
    std::fs::create_dir_all(&cache_dir).ok();
    std::fs::create_dir_all(&root_cache_dir).ok();

    let settings = Settings::new()
        .cache_path(cache_dir.to_string_lossy().as_bytes())
        .root_cache_path(root_cache_dir.to_string_lossy().as_bytes())
        .browser_subprocess_path(exe_path.to_string_lossy().as_bytes());

    wef::launch(settings, run);
    Ok(())
}
