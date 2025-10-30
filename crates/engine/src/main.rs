use gpui::*;
use raw_window_handle::{HasWindowHandle, HasDisplayHandle};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window as WinitWindow, WindowId};

fn main() {
    println!("🚀 Starting Winit + GPUI External Surface Demo");
    println!("This demonstrates GPUI rendering on top of a Winit window\n");

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = WinitGpuiApp::new();
    event_loop.run_app(&mut app).expect("Failed to run event loop");
}

struct WinitGpuiApp {
    winit_window: Option<Arc<WinitWindow>>,
    gpui_app: Option<Application>,
}

impl WinitGpuiApp {
    fn new() -> Self {
        Self {
            winit_window: None,
            gpui_app: None,
        }
    }
}

impl ApplicationHandler for WinitGpuiApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.winit_window.is_some() {
            return;
        }

        println!("✅ Creating Winit window...");

        let window_attributes = WinitWindow::default_attributes()
            .with_title("Winit + GPUI Demo - Yellow Background")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
            .with_transparent(false);

        let winit_window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("Failed to create window"),
        );

        println!("✅ Winit window created");

        // Get window handle for GPUI
        let raw_handle = winit_window
            .window_handle()
            .expect("Failed to get window handle")
            .as_raw();

        let scale_factor = winit_window.scale_factor() as f32;
        let size = winit_window.inner_size();

        let bounds = Bounds {
            origin: point(px(0.0), px(0.0)),
            size: size(px(size.width as f32), px(size.height as f32)),
        };

        let external_handle = ExternalWindowHandle {
            raw_handle,
            bounds,
            scale_factor,
        };

        println!("✅ Initializing GPUI...");

        let app = Application::new();

        app.run(move |cx| {
            println!("✅ Opening GPUI window on external surface...");

            // Try to open window with external handle
            match cx.open_window_external(external_handle.clone(), |window, cx| {
                println!("✅ GPUI window created successfully on external surface!");
                println!("\n🎨 RENDERING STARTED!");
                println!("You should see:");
                println!("  • Yellow Winit window background");
                println!("  • Small blue square with GPUI");
                println!("  • Text overlay from GPUI\n");

                cx.new(|cx| DemoView::new(window, cx))
            }) {
                Ok(window_handle) => {
                    println!("✅ Successfully opened GPUI window");
                }
                Err(e) => {
                    println!("❌ Failed to open GPUI window: {}", e);
                    println!("Falling back to regular GPUI window...");

                    let window_options = WindowOptions {
                        window_bounds: Some(WindowBounds::Windowed(bounds)),
                        titlebar: None,
                        window_min_size: Some(Size {
                            width: px(400.),
                            height: px(300.),
                        }),
                        kind: WindowKind::Normal,
                        is_resizable: true,
                        window_background: WindowBackgroundAppearance::Transparent,
                        ..Default::default()
                    };

                    cx.open_window(window_options, |window, cx| {
                        cx.new(|cx| DemoView::new(window, cx))
                    }).expect("Failed to open fallback window");
                }
            }
        });

        self.winit_window = Some(winit_window);
        self.gpui_app = Some(app);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("\n👋 Closing application...");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(window) = &self.winit_window {
                    // Here we would normally render the yellow background with wgpu/DirectX
                    // For now, winit will handle the window background
                    window.request_redraw();
                }
            }
            _ => {}
        }
    }
}

pub struct DemoView {
    counter: usize,
}

impl DemoView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { counter: 0 }
    }
}

impl Render for DemoView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.counter += 1;

        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap_4()
            .child(
                // Small blue square
                div()
                    .size(px(200.0))
                    .bg(rgb(0x4A90E2))
                    .rounded_lg()
                    .shadow_lg()
                    .border_2()
                    .border_color(rgb(0xFFFFFF))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_2xl()
                            .font_bold()
                            .text_color(rgb(0xFFFFFF))
                            .child("GPUI"),
                    ),
            )
            .child(
                // Text overlay
                div()
                    .p_4()
                    .bg(rgba(0x000000CC))
                    .rounded_lg()
                    .border_1()
                    .border_color(rgba(0xFFFFFF80))
                    .child(
                        div()
                            .text_lg()
                            .text_color(rgb(0xFFFFFF))
                            .child("✅ GPUI rendering on Winit window!"),
                    ),
            )
            .child(
                div()
                    .p_2()
                    .text_sm()
                    .text_color(rgba(0xFFFFFFDD))
                    .child(format!("Frame: {}", self.counter)),
            )
    }
}
