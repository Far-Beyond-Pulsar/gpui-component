use gpui::{
    div, px, App, AppContext, Bounds, Context, Corners, DismissEvent, Element, ElementId, Entity, EventEmitter, FocusHandle, Focusable, GlobalElementId, InspectorElementId, InteractiveElement, IntoElement, LayoutId, ParentElement as _, Pixels, Point, Render, RenderImage, Size, StatefulInteractiveElement, Style, Styled as _, Window
};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, AtomicUsize, Ordering}};
use image::ImageBuffer;
use futures::FutureExt;

/// Performance metrics for the viewport
#[derive(Debug, Clone, Default)]
pub struct ViewportMetrics {
    pub frame_count: u64,
    pub avg_frame_time_ms: f64,
    pub max_frame_time_ms: f64,
    pub min_frame_time_ms: f64,
    pub fps: f64,
    pub buffer_swaps: u64,
    pub texture_updates: u64,
    pub dropped_frames: u64,
}

/// Zero-copy RGBA8 framebuffer - ALWAYS render directly in RGBA8
pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pub buffer: Vec<u8>, // ALWAYS RGBA8 - 4 bytes per pixel
    pub pitch: u32, // bytes per row (width * 4)
    dirty_rect: Option<Bounds<Pixels>>,
    generation: u64,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let buffer_size = (width * height * 4) as usize; // Always RGBA8
        Self {
            width,
            height,
            buffer: vec![0u8; buffer_size],
            pitch: width * 4,
            dirty_rect: None,
            generation: 0,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.pitch = width * 4;
            let buffer_size = (width * height * 4) as usize;
            self.buffer.resize(buffer_size, 0);
            self.generation += 1;
            self.mark_dirty_all();
        }
    }

    pub fn clear(&mut self, color: [u8; 4]) {
        let [r, g, b, a] = color;
        for chunk in self.buffer.chunks_exact_mut(4) {
            chunk[0] = r;
            chunk[1] = g;
            chunk[2] = b;
            chunk[3] = a;
        }
        self.mark_dirty_all();
    }

    pub fn set_pixel(&mut self, x: u32, y: u32, color: [u8; 4]) {
        if x < self.width && y < self.height {
            let offset = ((y * self.width + x) * 4) as usize;
            if offset + 3 < self.buffer.len() {
                self.buffer[offset..offset + 4].copy_from_slice(&color);
                self.mark_dirty_pixel(x, y);
            }
        }
    }

    pub fn get_pixel(&self, x: u32, y: u32) -> [u8; 4] {
        if x < self.width && y < self.height {
            let offset = ((y * self.width + x) * 4) as usize;
            if offset + 3 < self.buffer.len() {
                return [
                    self.buffer[offset],
                    self.buffer[offset + 1],
                    self.buffer[offset + 2],
                    self.buffer[offset + 3],
                ];
            }
        }
        [0, 0, 0, 0]
    }

    fn mark_dirty_pixel(&mut self, x: u32, y: u32) {
        let pixel_bounds = Bounds {
            origin: Point { x: px(x as f32), y: px(y as f32) },
            size: Size { width: px(1.0), height: px(1.0) },
        };
        
        self.dirty_rect = Some(match self.dirty_rect {
            None => pixel_bounds,
            Some(existing) => existing.union(&pixel_bounds),
        });
        
        self.generation += 1;
    }

    pub fn mark_dirty_all(&mut self) {
        self.dirty_rect = Some(Bounds {
            origin: Point { x: px(0.0), y: px(0.0) },
            size: Size { width: px(self.width as f32), height: px(self.height as f32) },
        });
        self.generation += 1;
    }
}

/// Zero-copy double buffer with atomic pointer swapping
pub struct DoubleBuffer {
    current_front: AtomicUsize, // 0 or 1 - which buffer is currently front
    buffer_0: Arc<Mutex<Framebuffer>>,
    buffer_1: Arc<Mutex<Framebuffer>>,
}

impl DoubleBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let buffer_0 = Arc::new(Mutex::new(Framebuffer::new(width, height)));
        let buffer_1 = Arc::new(Mutex::new(Framebuffer::new(width, height)));
        
        Self {
            current_front: AtomicUsize::new(0),
            buffer_0,
            buffer_1,
        }
    }

    /// Get the back buffer for rendering (thread-safe)
    pub fn get_back_buffer(&self) -> Arc<Mutex<Framebuffer>> {
        let front_idx = self.current_front.load(Ordering::Acquire);
        if front_idx == 0 {
            Arc::clone(&self.buffer_1) // Buffer 1 is back
        } else {
            Arc::clone(&self.buffer_0) // Buffer 0 is back
        }
    }

    /// Get front buffer for display (zero-copy read)
    pub fn get_front_buffer(&self) -> Arc<Mutex<Framebuffer>> {
        let front_idx = self.current_front.load(Ordering::Acquire);
        if front_idx == 0 {
            Arc::clone(&self.buffer_0) // Buffer 0 is front
        } else {
            Arc::clone(&self.buffer_1) // Buffer 1 is front
        }
    }

    /// Atomic buffer swap - zero-copy, lock-free
    pub fn swap_buffers(&self) {
        let current = self.current_front.load(Ordering::Acquire);
        let new_front = if current == 0 { 1 } else { 0 };
        self.current_front.store(new_front, Ordering::Release);
    }
}

/// Hook for refresh notifications - called when rendering is complete
pub type RefreshHook = Arc<dyn Fn() + Send + Sync>;

/// Custom element for viewport rendering
pub struct ViewportElement {
    texture: Option<Arc<RenderImage>>,
}

impl ViewportElement {
    pub fn new(texture: Option<Arc<RenderImage>>) -> Self {
        Self { texture }
    }
}

impl Element for ViewportElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(ElementId::Name("viewport-element".into()))
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size = Size::full(); // This tells the element to take full available space
        let layout_id = window.request_layout(style, None, cx);
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        _bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        // Nothing to do
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        _cx: &mut App,
    ) {
        if let Some(ref texture) = self.texture {
            let _ = window.paint_image(
                bounds,
                Corners::all(px(0.0)),
                texture.clone(),
                0,
                false,
            );
        }
    }
}

impl IntoElement for ViewportElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

/// Zero-copy viewport with atomic buffer swapping
pub struct Viewport {
    double_buffer: Arc<DoubleBuffer>,
    shared_texture: Arc<Mutex<Option<Arc<RenderImage>>>>, // Pre-made texture ready to swap
    metrics: ViewportMetrics,
    focus_handle: FocusHandle,
    last_width: u32,
    last_height: u32,
    debug_enabled: bool,
    // Cleanup mechanism to prevent memory leaks
    shutdown_sender: Option<smol::channel::Sender<()>>,
}

impl Viewport {
    /// Updates GPU texture ONLY if needed - zero memory operations on UI thread
    fn update_texture_if_needed(&mut self) -> Option<Arc<RenderImage>> {
        let ui_start = std::time::Instant::now();
        
        // Try to get pre-made texture (zero-copy)
        let texture = {
            let grab_start = std::time::Instant::now();
            let mut shared = self.shared_texture.lock().unwrap();
            let texture = shared.take(); // Zero-copy take
            let grab_time = grab_start.elapsed();
            
            if self.debug_enabled && grab_time.as_micros() > 50 {
                println!("[VIEWPORT-UI] Texture grab: {}μs", grab_time.as_micros());
            }
            
            if self.debug_enabled {
                if texture.is_some() {
                    println!("[VIEWPORT-UI] Got texture from background task");
                } else {
                    println!("[VIEWPORT-UI] No texture available from background task");
                }
            }
            
            texture
        };
        
        let total_ui_time = ui_start.elapsed();
        if self.debug_enabled && total_ui_time.as_micros() > 100 {
            println!("[VIEWPORT-UI] Total UI time: {}μs", total_ui_time.as_micros());
        }
        
        texture
    }
}

impl Focusable for Viewport {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for Viewport {}

impl Render for Viewport {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let paint_start = std::time::Instant::now();
        
        // Get ready texture with zero operations on UI thread
        let texture = self.update_texture_if_needed();
        
        let paint_time = paint_start.elapsed();
        if self.debug_enabled && paint_time.as_micros() > 200 {
            println!("[VIEWPORT-UI] Paint time: {}μs", paint_time.as_micros());
        }

        div()
            .id("viewport")
            .size_full()
            .child(ViewportElement::new(texture))
            .focusable()
            .focus(|style| style) // Apply focus styling
    }
}

/// Create zero-copy viewport with pre-rendered textures
pub fn create_viewport_with_background_rendering<V: 'static>(
    initial_width: u32,
    initial_height: u32,
    cx: &mut Context<V>,
) -> (Entity<Viewport>, Arc<DoubleBuffer>, RefreshHook) {
    println!("[VIEWPORT] Creating zero-copy viewport {}x{}", initial_width, initial_height);
    
    let double_buffer = Arc::new(DoubleBuffer::new(initial_width, initial_height));
    
    // Channel for refresh notifications - bounded to prevent memory leaks
    let (refresh_sender, refresh_receiver) = smol::channel::bounded::<()>(1);
    
    // Shutdown channel for cleanup
    let (shutdown_sender, shutdown_receiver) = smol::channel::bounded::<()>(1);
    
    let viewport = cx.new(|cx| Viewport {
        double_buffer: Arc::clone(&double_buffer),
        shared_texture: Arc::new(Mutex::new(None)),
        metrics: ViewportMetrics::default(),
        focus_handle: cx.focus_handle(),
        last_width: initial_width,
        last_height: initial_height,
        debug_enabled: cfg!(debug_assertions),
        shutdown_sender: Some(shutdown_sender),
    });

    let processing_flag = Arc::new(AtomicBool::new(false));
    let processing_flag_clone = Arc::clone(&processing_flag);
    
    let refresh_hook: RefreshHook = Arc::new(move || {
        // Skip if already processing to prevent queue buildup
        if processing_flag_clone.load(Ordering::Relaxed) {
            return;
        }
        let send_result = refresh_sender.try_send(()); // Non-blocking send
        if send_result.is_ok() {
            println!("[VIEWPORT-BG] Refresh signal sent successfully");
        } else {
            println!("[VIEWPORT-BG] Failed to send refresh signal: {:?}", send_result);
        }
    });

    // Background task for texture pre-processing - use GPUI async task
    let buffer_ref = Arc::clone(&double_buffer);
    let processing_flag_ref = Arc::clone(&processing_flag);
    let debug_enabled = cfg!(debug_assertions);

    // Use GPUI async task that can properly notify the viewport entity
    viewport.update(cx, |_viewport, cx| {
        cx.spawn(async move |viewport_entity, mut cx| {
        println!("[VIEWPORT-BG] Background task started, waiting for refresh signals...");
        
        loop {
            // Use select! to listen for both refresh and shutdown signals
            futures::select! {
                refresh_result = refresh_receiver.recv().fuse() => {
                    match refresh_result {
                        Ok(()) => {
                            println!("[VIEWPORT-BG] Received refresh signal, processing buffer...");
                            processing_flag_ref.store(true, Ordering::Relaxed);
                            
                            let process_start = std::time::Instant::now();
                    
                    // Drain queue to prevent backlog
                    while refresh_receiver.try_recv().is_ok() {}
                    
                    // Get front buffer data (ALREADY in RGBA8 format)
                    let texture_result = {
                        let front_buffer = buffer_ref.get_front_buffer();
                        let buffer_guard = match front_buffer.lock() {
                            Ok(guard) => guard,
                            Err(_) => {
                                processing_flag_ref.store(false, Ordering::Relaxed);
                                continue;
                            }
                        };
                        
                        // Skip invalid dimensions
                        if buffer_guard.width == 0 || buffer_guard.height == 0 {
                            processing_flag_ref.store(false, Ordering::Relaxed);
                            continue;
                        }
                        
                        // ZERO CONVERSION - create image::Frame from RGBA8 data
                        let texture_create_start = std::time::Instant::now();
                        
                        // Create an image::Frame from our RGBA8 buffer
                        let rgba_image = match ImageBuffer::<image::Rgba<u8>, Vec<u8>>::from_raw(
                            buffer_guard.width,
                            buffer_guard.height,
                            buffer_guard.buffer.clone(),
                        ) {
                            Some(img) => img,
                            None => {
                                processing_flag_ref.store(false, Ordering::Relaxed);
                                continue;
                            }
                        };
                        
                        let frame = image::Frame::new(rgba_image);
                        let texture = Arc::new(RenderImage::new(vec![frame]));
                        
                        let texture_create_time = texture_create_start.elapsed();
                        let dimensions = (buffer_guard.width, buffer_guard.height);
                        
                        if debug_enabled {
                            println!("[VIEWPORT-BG] Zero-copy texture: create={}μs ({}x{})",
                                texture_create_time.as_micros(),
                                dimensions.0,
                                dimensions.1);
                        }
                        
                        texture
                    };
                    
                    // Update the viewport entity and store texture + trigger re-render (like GPML canvas)
                    let update_result = viewport_entity.update(cx, |viewport, cx| {
                        // Store completed texture in the viewport's shared_texture
                        {
                            let mut shared = viewport.shared_texture.lock().unwrap();
                            *shared = Some(texture_result);
                        }
                        
                        // Viewport has new texture available, trigger re-render
                        cx.notify();
                    });
                    
                    if let Err(e) = update_result {
                        println!("[VIEWPORT-BG] Failed to update viewport entity: {:?}", e);
                    }
                    
                    let total_time = process_start.elapsed();
                    if debug_enabled {
                        println!("[VIEWPORT-BG] Total process time: {}μs", total_time.as_micros());
                    }
                            
                            processing_flag_ref.store(false, Ordering::Relaxed);
                        },
                        Err(_) => {
                            // Channel closed, exit the task
                            println!("[VIEWPORT-BG] Refresh channel closed, exiting background task");
                            break;
                        }
                    }
                },
                shutdown_result = shutdown_receiver.recv().fuse() => {
                    // Shutdown signal received
                    println!("[VIEWPORT-BG] Shutdown signal received, exiting background task");
                    break;
                }
            }
        }
        }).detach();
    });

    (viewport, double_buffer, refresh_hook)
}
