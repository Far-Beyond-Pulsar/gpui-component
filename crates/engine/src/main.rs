use crate::settings::EngineSettings;
use gpui::*;
use gpui_component::Root;
use directories::ProjectDirs;
use std::path::PathBuf;
use ui::app::PulsarApp;
use ui::project_selector::ProjectSelected;
use ui::entry_window::EntryWindow;
use ui::settings_window::SettingsWindow;
use gpui::Action;
use gpui::SharedString;
use gpui_component::scroll::ScrollbarShow;
use serde::Deserialize;
use std::fs;
use std::path::Path;

mod assets;
mod compiler;
mod graph;
mod ui;
mod recent_projects;
pub mod settings;
pub mod themes;
pub use assets::Assets;

// +--------------------------------------------+
// |  Compile-time engine info from Cargo.toml  |
// +--------------------------------------------+

pub const ENGINE_NAME:         &str = env!("CARGO_PKG_NAME");
pub const ENGINE_LICENSE:      &str = env!("CARGO_PKG_LICENSE");
pub const ENGINE_AUTHORS:      &str = env!("CARGO_PKG_AUTHORS");
pub const ENGINE_VERSION:      &str = env!("CARGO_PKG_VERSION");
pub const ENGINE_HOMEPAGE:     &str = env!("CARGO_PKG_HOMEPAGE");
pub const ENGINE_REPOSITORY:   &str = env!("CARGO_PKG_REPOSITORY");
pub const ENGINE_DESCRIPTION:  &str = env!("CARGO_PKG_DESCRIPTION");
pub const ENGINE_LICENSE_FILE: &str = env!("CARGO_PKG_LICENSE_FILE");

// +----------------------------------+
// |   Actions for settings changes   |
// +----------------------------------+

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

#[derive(Action, Clone, PartialEq, Eq)]
#[action(namespace = pulsar, no_json)]
pub struct OpenSettings;

fn main() {
    println!("{}", ENGINE_NAME);
    println!("Version: {}", ENGINE_VERSION);
    println!("Authors: {}", ENGINE_AUTHORS);
    println!("Description: {}", ENGINE_DESCRIPTION);

    // Determine app data directory
    let proj_dirs = ProjectDirs::from("com", "Pulsar", "Pulsar_Engine")
        .expect("Could not determine app data directory");
    let appdata_dir = proj_dirs.data_dir();
    let themes_dir  = appdata_dir.join("themes");
    let config_dir  = appdata_dir.join("configs");
    let config_file = config_dir.join("engine.toml");

    println!("App data directory: {:?}", appdata_dir);
    println!("Themes directory: {:?}", themes_dir);
    println!("Config directory: {:?}", config_dir);
    println!("Config file: {:?}", config_file);

    // Extract bundled themes if not present
    if !themes_dir.exists() {
        if let Err(e) = fs::create_dir_all(&themes_dir) {
            eprintln!("Failed to create themes directory: {e}");
        } else {
            // Copy all themes from project themes/ to appdata_dir/themes/
            let project_themes_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .unwrap()
                .join("themes");
            if let Ok(entries) = fs::read_dir(&project_themes_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Some(name) = path.file_name() {
                            let dest = themes_dir.join(name);
                            let _ = fs::copy(&path, &dest);
                        }
                    }
                }
            }
        }
    }

    // Create default config if not present
    if !config_file.exists() {
        if let Err(e) = fs::create_dir_all(&config_dir) {
            eprintln!("Failed to create config directory: {e}");
        }
        let default_settings = EngineSettings::default();
        default_settings.save(&config_file);
    }

    // Load settings
    println!("Loading engine settings from {:?}", config_file);
    let mut engine_settings = EngineSettings::load(&config_file);

    let app = Application::new()
        .with_assets(Assets);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(8)
        .thread_name("PulsarEngineRuntime")
        .enable_all()
        .build()
        .unwrap();

    // Init the Game engine backend (subsystems, etc)
    rt.block_on(engine_backend::EngineBackend::init());

    app.run(move |cx| {
        // Load custom fonts first
        if let Some(font_data) = Assets::get("fonts/JetBrainsMono-Regular.ttf") {
            match cx.text_system().add_fonts(vec![font_data.data]) {
                Ok(_) => println!("Successfully loaded JetBrains Mono font"),
                Err(e) => println!("Failed to load JetBrains Mono font: {:?}", e),
            }
        } else {
            println!("Could not find JetBrains Mono font file");
        }

        gpui_component::init(cx);
        crate::themes::init(cx);
        crate::ui::terminal::init(cx);  // Initialize terminal keybindings (Tab handling)

        cx.bind_keys([KeyBinding::new("ctrl-,", OpenSettings, None)]);
        cx.on_action(|_: &OpenSettings, cx| {
            open_settings_window(cx);
        });

        cx.activate(true);

        // Open the entry/launcher window first (smaller size)
        let entry_window_size = size(px(1000.), px(600.));
        let entry_window_bounds = Bounds::centered(None, entry_window_size, cx);

        let entry_options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(entry_window_bounds)),
            titlebar: Some(gpui::TitlebarOptions {
                title: None,
                appears_transparent: true,
                traffic_light_position: None, // No traffic lights
            }),
            window_min_size: Some(gpui::Size {
                width: px(800.),
                height: px(500.),
            }),
            kind: WindowKind::Normal,
            is_resizable: true,
            window_decorations: Some(gpui::WindowDecorations::Client),
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            ..Default::default()
        };

        let entry_window = cx
            .open_window(entry_options, |window, cx| {
                let entry_view = cx.new(|cx| EntryWindow::new(window, cx));

                // Subscribe to project selection events inside the window
                if let Some(entry_screen) = entry_view.read(cx).entry_screen().cloned() {
                    cx.subscribe(&entry_screen, move |_entry, event: &ProjectSelected, cx| {
                        let project_path = event.path.clone();

                        eprintln!("DEBUG: Subscription called with path: {:?}", project_path);

                        // Close the entry window
                        if let Some(active_window) = cx.active_window() {
                            let _ = active_window.update(cx, |_, window, _cx| {
                                window.remove_window();
                            });
                        }

                        // Open the main engine window with the selected project
                        open_engine_window(project_path, cx);
                    })
                    .detach();
                }

                cx.new(|cx| Root::new(entry_view.clone().into(), window, cx))
            })
            .expect("failed to open entry window");
    });
}

fn open_engine_window(project_path: PathBuf, cx: &mut App) {
    eprintln!("DEBUG: open_engine_window called with path: {:?}", project_path);
    let mut window_size = size(px(1200.), px(800.));
    if let Some(display) = cx.primary_display() {
        let display_size = display.bounds().size;
        window_size.width = window_size.width.min(display_size.width * 0.85);
        window_size.height = window_size.height.min(display_size.height * 0.85);
    }

    let window_bounds = Bounds::centered(None, window_size, cx);

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(window_bounds)),
        titlebar: Some(gpui::TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: None, // No traffic lights
        }),
        window_min_size: Some(gpui::Size {
            width: px(1200.),
            height: px(800.),
        }),
        kind: WindowKind::Normal,
        is_resizable: true,
        window_decorations: Some(gpui::WindowDecorations::Client),
        #[cfg(target_os = "linux")]
        window_background: gpui::WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };

    let window = cx
        .open_window(options, |window, cx| {
            eprintln!("DEBUG: Creating PulsarApp");
            let view = cx.new(|cx| PulsarApp::new_with_project(project_path.clone(), window, cx));
            cx.new(|cx| Root::new(view.into(), window, cx))
        })
        .expect("failed to open engine window");

    eprintln!("DEBUG: Engine window opened");

    window
        .update(cx, |_, window, _| {
            window.activate_window();
            window.set_window_title("Pulsar Engine");
        })
        .expect("failed to update engine window");

    eprintln!("DEBUG: Engine window activated");
}

fn open_settings_window(cx: &mut App) {
    let proj_dirs = ProjectDirs::from("com", "Pulsar", "Pulsar_Engine")
        .expect("Could not determine app data directory");
    let appdata_dir = proj_dirs.data_dir();
    let config_dir = appdata_dir.join("configs");
    let config_file = config_dir.join("engine.toml");

    let window_bounds = Bounds::centered(None, size(px(800.), px(600.)), cx);

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(window_bounds)),
        titlebar: Some(gpui::TitlebarOptions {
            title: None,
            appears_transparent: true,
            traffic_light_position: None, // No traffic lights
        }),
        window_min_size: Some(Size {
            width: px(600.),
            height: px(400.),
        }),
        kind: WindowKind::Normal,
        is_resizable: true,
        window_decorations: Some(gpui::WindowDecorations::Client),
        #[cfg(target_os = "linux")]
        window_background: gpui::WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };

    let window = cx
        .open_window(options, |window, cx| {
            cx.new(|cx| SettingsWindow::new(window, cx))
        })
        .expect("failed to open settings window");

    window
        .update(cx, |_, window, _| {
            window.activate_window();
            window.set_window_title("Pulsar - Settings");
        })
        .expect("failed to update settings window");
}
