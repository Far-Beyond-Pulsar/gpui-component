use gpui::{App, AppContext, Entity, Context, Global, Bounds, ParentElement, WindowOptions, Styled, Render, IntoElement, div};
use ui::viewport::{Viewport, ViewportBuffers, RefreshHook, TestRenderEngine, FramebufferFormat};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Global state for the example
struct ExampleState {
    render_engine: Arc<Mutex<TestRenderEngine>>,
    buffers: ViewportBuffers,
    refresh_hook: RefreshHook,
}

impl Global for ExampleState {}

/// Main window view that contains the viewport
struct MainView {
    viewport: Entity<Viewport>,
}

impl MainView {
    fn new(cx: &mut Context<Self>) -> Self {
        // Create the viewport with buffer access and refresh hook
        let (viewport, buffers, refresh_hook) = Viewport::new(800, 600, FramebufferFormat::Rgba8, cx.app());

        // Create a test render engine
        let render_engine = Arc::new(Mutex::new(TestRenderEngine::new()));

        // Store in global state for background rendering
        cx.app().set_global(ExampleState {
            render_engine: render_engine.clone(),
            buffers,
            refresh_hook: refresh_hook.clone(),
        });

        // Spawn background render thread
        spawn_render_thread(render_engine, refresh_hook);

        Self {
            viewport: cx.new_entity(viewport),
        }
    }
}

impl Render for MainView {
    fn render(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .bg(gpui::rgb(0x2a2a2a))
            .child(
                div()
                    .size_full()
                    .p_4()
                    .child(self.viewport.clone())
            )
    }
}

/// Spawn a background thread that renders continuously to the viewport buffers
fn spawn_render_thread(render_engine: Arc<Mutex<TestRenderEngine>>, refresh_hook: RefreshHook) {
    let state = unsafe { &*std::ptr::from_ref(&*render_engine) };
    
    thread::spawn(move || {
        loop {
            // Access the global state safely in this example
            // In a real application, you'd want proper lifetime management
            if let Ok(app_state) = App::try_global::<ExampleState>() {
                // Render to the back buffer
                app_state.buffers.with_back_buffer(|back_buffer| {
                    if let Ok(mut engine) = render_engine.lock() {
                        engine.render(back_buffer);
                    }
                });

                // Swap buffers to make the rendered frame visible
                app_state.buffers.swap_buffers();

                // Trigger GPUI refresh
                refresh_hook();
            }

            // Target 60 FPS
            thread::sleep(Duration::from_millis(16));
        }
    });
}

fn main() {
    App::new().run(|cx| {
        cx.open_window(
            WindowOptions {
                window_bounds: Some(Bounds::centered(None, gpui::size(gpui::px(900.0), gpui::px(700.0)), cx)),
                ..Default::default()
            },
            |cx| cx.new_entity(MainView::new(cx)),
        ).unwrap();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_viewport_creation() {
        App::test(|cx| {
            let (viewport, buffers, refresh_hook) = Viewport::new(100, 100, FramebufferFormat::Rgba8, cx);
            
            // Test that we can access buffer info
            let info = buffers.with_front_buffer(|buffer| {
                (buffer.width, buffer.height, buffer.format)
            });
            
            assert_eq!(info, Some((100, 100, FramebufferFormat::Rgba8)));
            
            // Test that we can render to back buffer
            buffers.with_back_buffer(|back_buffer| {
                back_buffer.clear([255, 0, 0, 255]); // Red background
            });
            
            // Test buffer swap
            buffers.swap_buffers();
            
            // Test refresh hook
            refresh_hook();
        });
    }

    #[test]
    fn test_zero_copy_access() {
        App::test(|cx| {
            let (_viewport, buffers, _refresh_hook) = Viewport::new(10, 10, FramebufferFormat::Rgba8, cx);
            
            // Test zero-copy write access
            buffers.with_back_buffer(|back_buffer| {
                // Directly modify buffer without copying
                back_buffer.buffer[0] = 128;
                back_buffer.buffer[1] = 64;
                back_buffer.buffer[2] = 32;
                back_buffer.buffer[3] = 255;
            });
            
            buffers.swap_buffers();
            
            // Test zero-copy read access
            let pixel_data = buffers.with_front_buffer(|front_buffer| {
                // Read without copying the entire buffer
                (front_buffer.buffer[0], front_buffer.buffer[1], front_buffer.buffer[2], front_buffer.buffer[3])
            });
            
            assert_eq!(pixel_data, Some((128, 64, 32, 255)));
        });
    }
}
