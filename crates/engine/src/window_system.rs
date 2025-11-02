// Window system - manages multiple windows each in their own thread
use std::sync::{Arc, RwLock};
use std::thread;
use std::collections::HashMap;
use gpui::*;
use crate::Assets;

/// Global engine state shared across all windows
#[derive(Clone)]
pub struct EngineState {
    inner: Arc<RwLock<EngineStateInner>>,
}

struct EngineStateInner {
    pub windows: HashMap<usize, WindowInfo>,
    pub next_window_id: usize,
    pub active_project: Option<String>,
    // Add more global state as needed
}

#[derive(Clone)]
pub struct WindowInfo {
    pub id: usize,
    pub title: String,
    pub window_type: WindowType,
}

#[derive(Clone, Debug)]
pub enum WindowType {
    Main,
    Editor,
    Settings,
    Terminal,
    FileManager,
    Problems,
}

impl EngineState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(EngineStateInner {
                windows: HashMap::new(),
                next_window_id: 0,
                active_project: None,
            })),
        }
    }
    
    pub fn register_window(&self, title: String, window_type: WindowType) -> usize {
        let mut state = self.inner.write().unwrap();
        let id = state.next_window_id;
        state.next_window_id += 1;
        
        state.windows.insert(id, WindowInfo {
            id,
            title,
            window_type,
        });
        
        println!("📋 Registered window #{}: {:?}", id, state.windows.get(&id).unwrap().window_type);
        id
    }
    
    pub fn unregister_window(&self, id: usize) {
        let mut state = self.inner.write().unwrap();
        state.windows.remove(&id);
        println!("📋 Unregistered window #{}", id);
    }
    
    pub fn set_active_project(&self, path: Option<String>) {
        let mut state = self.inner.write().unwrap();
        state.active_project = path;
    }
    
    pub fn get_active_project(&self) -> Option<String> {
        let state = self.inner.read().unwrap();
        state.active_project.clone()
    }
    
    pub fn window_count(&self) -> usize {
        let state = self.inner.read().unwrap();
        state.windows.len()
    }
}

/// Spawn a new window in its own thread
pub fn spawn_window<V, F>(
    engine_state: EngineState,
    title: String,
    width: u32,
    height: u32,
    window_type: WindowType,
    build_view: F,
) -> thread::JoinHandle<()>
where
    V: 'static + Render,
    F: FnOnce(&mut Window, &mut App, EngineState) -> Entity<V> + Send + 'static,
{
    thread::Builder::new()
        .name(format!("Window-{}", title))
        .spawn(move || {
            use winit::application::ApplicationHandler;
            use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
            use winit::window::Window as WinitWindow;
            use std::sync::Arc;
            use raw_window_handle::HasWindowHandle;
            
            // Register this window with engine state
            let window_id = engine_state.register_window(title.clone(), window_type);
            
            // Create event loop for this window
            // On Windows, we need to use any_thread() to create event loops outside main thread
            use winit::platform::windows::EventLoopBuilderExtWindows;
            let event_loop = EventLoop::builder()
                .with_any_thread(true)
                .build()
                .expect("Failed to create event loop");
            event_loop.set_control_flow(ControlFlow::Poll);
            
            // Window handler
            struct WindowHandler {
                engine_state: EngineState,
                window_id: usize,
                title: String,
                width: u32,
                height: u32,
                winit_window: Option<Arc<WinitWindow>>,
                gpui_app: Option<Application>,
                gpui_window: Option<AnyWindowHandle>,
                build_view: Option<Box<dyn FnOnce(&mut Window, &mut App, EngineState) -> AnyView + Send>>,
            }
            
            impl ApplicationHandler for WindowHandler {
                fn resumed(&mut self, event_loop: &ActiveEventLoop) {
                    if self.winit_window.is_some() {
                        return;
                    }
                    
                    println!("🪟 Creating window '{}'...", self.title);
                    
                    // Create winit window
                    let window_attrs = WinitWindow::default_attributes()
                        .with_title(&self.title)
                        .with_inner_size(winit::dpi::LogicalSize::new(self.width, self.height));
                        
                    let winit_window = Arc::new(
                        event_loop.create_window(window_attrs)
                            .expect("Failed to create window")
                    );
                    
                    // Initialize GPUI app
                    let mut app = Application::new().with_assets(Assets);
                    
                    app.update(|app| {
                        // Load fonts
                        if let Some(font_data) = Assets::get("fonts/JetBrainsMono-Regular.ttf") {
                            let _ = app.text_system().add_fonts(vec![font_data.data]);
                        }
                        
                        // Initialize GPUI components
                        gpui_component::init(app);
                        crate::themes::init(app);
                        crate::ui::terminal::init(app);
                        
                        app.activate(true);
                    });
                    
                    // Create external window handle
                    let scale_factor = winit_window.scale_factor() as f32;
                    let size = winit_window.inner_size();
                    let logical_width = size.width as f32 / scale_factor;
                    let logical_height = size.height as f32 / scale_factor;
                    
                    let bounds = Bounds {
                        origin: point(px(0.0), px(0.0)),
                        size: gpui::size(px(logical_width), px(logical_height)),
                    };
                    
                    let gpui_raw_handle = winit_window.window_handle()
                        .expect("Failed to get window handle")
                        .as_raw();
                    
                    let external_handle = ExternalWindowHandle {
                        raw_handle: gpui_raw_handle,
                        bounds,
                        scale_factor,
                        surface_handle: None,
                    };
                    
                    // Build view with engine state
                    let engine_state = self.engine_state.clone();
                    let build_view = self.build_view.take().expect("build_view already taken");
                    
                    let gpui_window = app.open_window_external(external_handle, move |window, cx| {
                        let view = build_view(window, cx, engine_state);
                        cx.new(|cx| gpui_component::Root::new(view, window, cx))
                    }).expect("Failed to open GPUI window");
                    
                    self.winit_window = Some(winit_window);
                    self.gpui_app = Some(app);
                    self.gpui_window = Some(gpui_window.into());
                    
                    println!("✅ Window '{}' created!", self.title);
                }
                
                fn window_event(
                    &mut self,
                    event_loop: &ActiveEventLoop,
                    window_id: winit::window::WindowId,
                    event: winit::event::WindowEvent,
                ) {
                    use winit::event::WindowEvent;
                    
                    if let Some(winit_window) = &self.winit_window {
                        if winit_window.id() != window_id {
                            return;
                        }
                        
                        match event {
                            WindowEvent::CloseRequested => {
                                println!("🚪 Window '{}' closing...", self.title);
                                self.engine_state.unregister_window(self.window_id);
                                event_loop.exit();
                            }
                            WindowEvent::RedrawRequested => {
                                if let Some(ref mut gpui_app) = self.gpui_app {
                                    if let Some(ref gpui_window) = self.gpui_window {
                                        let _ = gpui_app.update(|cx| {
                                            let _ = gpui_window.update(cx, |_view, window, _cx| {
                                                window.refresh();
                                            });
                                        });
                                    }
                                }
                                winit_window.request_redraw();
                            }
                            _ => {}
                        }
                    }
                }
                
                fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
                    if let Some(window) = &self.winit_window {
                        window.request_redraw();
                    }
                }
            }
            
            // Convert the build_view closure to be type-erased
            let build_view_boxed: Box<dyn FnOnce(&mut Window, &mut App, EngineState) -> AnyView + Send> = 
                Box::new(move |window, cx, engine_state| {
                    let view = build_view(window, cx, engine_state);
                    view.into()
                });
            
            let mut handler = WindowHandler {
                engine_state: engine_state.clone(),
                window_id,
                title: title.clone(),
                width,
                height,
                winit_window: None,
                gpui_app: None,
                gpui_window: None,
                build_view: Some(build_view_boxed),
            };
            
            // Run the event loop for this window
            let _ = event_loop.run_app(&mut handler);
            
            // Cleanup when window closes
            engine_state.unregister_window(window_id);
            println!("🚪 Window '{}' thread exiting", title);
        })
        .expect("Failed to spawn window thread")
}
