use gpui::*;
use gpui_component::{
    Root,
};

mod compiler;
mod assets;
mod ui;
mod graph;

pub use assets::Assets;
use serde::Deserialize;

// Compile-time engine version from Cargo.toml
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");

// pub mod renderer;
pub mod themes;

use gpui::Action;
use gpui::SharedString;
use gpui_component::scroll::ScrollbarShow;


#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectScrollbarShow(ScrollbarShow);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectLocale(SharedString);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectFont(usize);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = story, no_json)]
pub struct SelectRadius(usize);

use ui::app::PulsarApp;
use ui::entry_window::EntryWindow;
use ui::entry_screen::ProjectSelected;
use std::path::PathBuf;

fn main() {
    // Note: Node metadata is now loaded lazily from pulsar_std when needed
    println!("Pulsar Engine - Visual Programming Environment");
    println!("Using macro-based node system from pulsar_std");

    let app = Application::new().with_assets(Assets);

    app.run(move |cx| {
        gpui_component::init(cx);

        cx.activate(true);

        // Open the entry/launcher window first (smaller size)
        let entry_window_size = size(px(1000.), px(600.));
        let entry_window_bounds = Bounds::centered(None, entry_window_size, cx);

        let entry_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(entry_window_bounds)),
            titlebar: None,
            window_min_size: Some(gpui::Size {
                width: px(800.),
                height: px(500.),
            }),
            kind: WindowKind::Normal,
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            ..Default::default()
        };

        // Create the entry view before opening the window so we can subscribe to it
        let entry_view = cx.new(|cx| EntryWindow::new_placeholder(cx));

        let entry_window = cx
            .open_window(entry_options, |window, cx| {
                // Initialize the entry window now that we have the window
                entry_view.update(cx, |view, cx| {
                    *view = EntryWindow::new(window, cx);
                });

                cx.new(|cx| Root::new(entry_view.clone().into(), window, cx))
            })
            .expect("failed to open entry window");

        entry_window
            .update(cx, |_, window, _| {
                window.activate_window();
                window.set_window_title("Pulsar - Select Project");
            })
            .expect("failed to update entry window");

        // Subscribe to project selection events at the app level
        let window_handle = entry_window;
        cx.subscribe(&entry_view, move |_entry, event: &ProjectSelected, cx| {
            let project_path = event.path.clone();

            // Close the entry window
            window_handle.update(cx, |_, window, _| {
                window.remove_window();
            }).ok();

            // Open the main engine window with the selected project
            open_engine_window(project_path, cx);
        }).detach();
    });
}

fn open_engine_window(project_path: PathBuf, cx: &mut App) {
    let mut window_size = size(px(1200.), px(800.));
    if let Some(display) = cx.primary_display() {
        let display_size = display.bounds().size;
        window_size.width = window_size.width.min(display_size.width * 0.85);
        window_size.height = window_size.height.min(display_size.height * 0.85);
    }

    let window_bounds = Bounds::centered(None, window_size, cx);

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(window_bounds)),
        titlebar: None,
        window_min_size: Some(gpui::Size {
            width: px(1200.),
            height: px(800.),
        }),
        kind: WindowKind::Normal,
        #[cfg(target_os = "linux")]
        window_background: gpui::WindowBackgroundAppearance::Transparent,
        #[cfg(target_os = "linux")]
        window_decorations: Some(gpui::WindowDecorations::Client),
        ..Default::default()
    };

    let window = cx
        .open_window(options, |window, cx| {
            let view = cx.new(|cx| PulsarApp::new_with_project(project_path.clone(), window, cx));
            cx.new(|cx| Root::new(view.into(), window, cx))
        })
        .expect("failed to open engine window");

    window
        .update(cx, |_, window, _| {
            window.activate_window();
            window.set_window_title("Pulsar Engine");
        })
        .expect("failed to update engine window");
}
