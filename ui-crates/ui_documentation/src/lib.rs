mod state;
mod components;
mod views;

pub use views::MainView;

use gpui::*;

pub fn open_documentation_window(app: &mut App) {
    let window_options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: Point { x: px(100.0), y: px(100.0) },
            size: Size { width: px(1400.0), height: px(900.0) },
        })),
        titlebar: Some(TitlebarOptions {
            title: Some("Pulsar Engine Documentation".into()),
            appears_transparent: false,
            traffic_light_position: None,
        }),
        window_background: WindowBackgroundAppearance::Opaque,
        focus: true,
        show: true,
        kind: WindowKind::Normal,
        is_movable: true,
        is_resizable: true,
        is_minimizable: true,
        display_id: None,
        app_id: Some("pulsar-documentation".into()),
        window_min_size: Some(Size {
            width: px(800.0),
            height: px(600.0),
        }),
    };

    app.open_window(window_options, |window, cx| {
        cx.new(|cx| MainView::new(cx))
    })
    .ok();
}
