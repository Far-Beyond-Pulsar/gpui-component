use gpui::{
    canvas, div, App, Bounds, ContentMask, DismissEvent, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement as _, Pixels, Render, RenderImage, Size, Styled as _, Window, Corners, px,
    Context, PaintQuad, Point, BorderStyle, Entity,
};
use std::sync::{Arc, Mutex, mpsc, atomic::{AtomicBool, Ordering}};
use std::collections::VecDeque;
use std::time::{Duration, Instant};

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

/// A trait for render engines that can render to a GPU texture
pub trait RenderEngine: Send + Sync + 'static {
    /// Render to the given framebuffer
    /// This should be as fast as possible and not block the UI thread
    fn render(&mut self, framebuffer: &mut Framebuffer) -> Result<(), RenderError>;

    /// Get the preferred format for the framebuffer
    fn preferred_format(&self) -> FramebufferFormat {
        FramebufferFormat::Rgba8
    }

    /// Called when the viewport is resized
    fn on_resize(&mut self, _width: u32, _height: u32) {}

    /// Called when the viewport needs to be initialized
    fn initialize(&mut self) -> Result<(), RenderError> { Ok(()) }

    /// Called when the viewport is being destroyed
    fn cleanup(&mut self) {}

    /// Set a callback that the render engine can use to trigger GPUI redraws
    fn set_notify_callback(&mut self, _callback: Box<dyn Fn() + Send + Sync>) {}
}

/// Render engine errors
#[derive(Debug, Clone)]
pub enum RenderError {
    InitializationFailed(String),
    RenderFailed(String),
    TextureError(String),
    OutOfMemory,
}

impl std::fmt::Display for RenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RenderError::InitializationFailed(msg) => write!(f, "Initialization failed: {}", msg),
            RenderError::RenderFailed(msg) => write!(f, "Render failed: {}", msg),
            RenderError::TextureError(msg) => write!(f, "Texture error: {}", msg),
            RenderError::OutOfMemory => write!(f, "Out of memory"),
        }
    }
}

impl std::error::Error for RenderError {}

/// Supported framebuffer formats
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FramebufferFormat {
    Rgba8,
    Rgb8,
    Bgra8,
    Bgr8,
}

impl FramebufferFormat {
    pub fn bytes_per_pixel(&self) -> u32 {
        match self {
            FramebufferFormat::Rgba8 | FramebufferFormat::Bgra8 => 4,
            FramebufferFormat::Rgb8 | FramebufferFormat::Bgr8 => 3,
        }
    }
}

/// A high-performance zero-copy framebuffer that can be rendered to
pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pub format: FramebufferFormat,
    pub buffer: Vec<u8>,
    pub pitch: u32, // bytes per row
    dirty_rect: Option<Bounds<Pixels>>,
    generation: u64,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32, format: FramebufferFormat) -> Self {
        let bytes_per_pixel = format.bytes_per_pixel();
        let pitch = width * bytes_per_pixel;
        let buffer_size = (pitch * height) as usize;

        Self {
            width,
            height,
            format,
            buffer: vec![0; buffer_size],
            pitch,
            dirty_rect: Some(Bounds {
                origin: Point { x: px(0.0), y: px(0.0) },
                size: Size { width: px(width as f32), height: px(height as f32) }
            }),
            generation: 0,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if self.width == width && self.height == height {
            return;
        }

        self.width = width;
        self.height = height;
        self.pitch = width * self.format.bytes_per_pixel();
        let buffer_size = (self.pitch * height) as usize;

        self.buffer.resize(buffer_size, 0);
        self.dirty_rect = Some(Bounds {
            origin: Point { x: px(0.0), y: px(0.0) },
            size: Size { width: px(width as f32), height: px(height as f32) }
        });
        self.generation += 1;
    }

    pub fn mark_dirty(&mut self, rect: Option<Bounds<Pixels>>) {
        self.dirty_rect = rect.or(self.dirty_rect);
        self.generation += 1;
    }

    pub fn clear_dirty(&mut self) {
        self.dirty_rect = None;
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty_rect.is_some()
    }

    pub fn dirty_rect(&self) -> Option<Bounds<Pixels>> {
        self.dirty_rect
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn clear(&mut self, color: [u8; 4]) {
        match self.format {
            FramebufferFormat::Rgba8 => {
                for chunk in self.buffer.chunks_exact_mut(4) {
                    chunk.copy_from_slice(&color);
                }
            }
            FramebufferFormat::Bgra8 => {
                let bgra = [color[2], color[1], color[0], color[3]];
                for chunk in self.buffer.chunks_exact_mut(4) {
                    chunk.copy_from_slice(&bgra);
                }
            }
            FramebufferFormat::Rgb8 => {
                for chunk in self.buffer.chunks_exact_mut(3) {
                    chunk.copy_from_slice(&color[0..3]);
                }
            }
            FramebufferFormat::Bgr8 => {
                let bgr = [color[2], color[1], color[0]];
                for chunk in self.buffer.chunks_exact_mut(3) {
                    chunk.copy_from_slice(&bgr);
                }
            }
        }
        self.mark_dirty(Some(Bounds {
            origin: Point { x: px(0.0), y: px(0.0) },
            size: Size { width: px(self.width as f32), height: px(self.height as f32) }
        }));
    }
}

/// Double-buffered framebuffer system for smooth updates
struct DoubleBuffer {
    front: Framebuffer,
    back: Framebuffer,
    swapped: AtomicBool,
}

impl DoubleBuffer {
    fn new(width: u32, height: u32, format: FramebufferFormat) -> Self {
        Self {
            front: Framebuffer::new(width, height, format),
            back: Framebuffer::new(width, height, format),
            swapped: AtomicBool::new(false),
        }
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.front.resize(width, height);
        self.back.resize(width, height);
    }

    fn get_back_buffer(&mut self) -> &mut Framebuffer {
        if self.swapped.load(Ordering::Acquire) {
            &mut self.front
        } else {
            &mut self.back
        }
    }

    fn get_front_buffer(&self) -> &Framebuffer {
        if self.swapped.load(Ordering::Acquire) {
            &self.back
        } else {
            &self.front
        }
    }

    fn swap(&mut self) {
        let current = self.swapped.load(Ordering::Acquire);
        self.swapped.store(!current, Ordering::Release);
    }
}

/// Commands sent to the render thread
#[derive(Debug)]
enum RenderCommand {
    Render,
    Resize(u32, u32),
    Shutdown,
}


/// High-performance viewport component with async rendering
pub struct Viewport<E: RenderEngine> {
    focus_handle: FocusHandle,
    render_engine: Arc<Mutex<E>>,
    double_buffer: Arc<Mutex<DoubleBuffer>>,
    visible: bool,
    bounds: Bounds<Pixels>,

    // Async rendering
    render_tx: mpsc::Sender<RenderCommand>,
    _render_thread: std::thread::JoinHandle<()>,

    // Performance tracking
    metrics: Arc<Mutex<ViewportMetrics>>,
    frame_times: Arc<Mutex<VecDeque<Instant>>>,
    last_texture_generation: u64,

    // Texture management
    current_texture: Option<Arc<RenderImage>>,
    texture_dirty: bool,

    // Debug flags
    debug_enabled: bool,

    // GPUI integration
    entity: Option<Entity<Self>>,
}

impl<E: RenderEngine> Drop for Viewport<E> {
    fn drop(&mut self) {
        let _ = self.render_tx.send(RenderCommand::Shutdown);
        self.hide();
    }
}

impl<E: RenderEngine> Viewport<E> {
    pub fn new(render_engine: E, initial_width: u32, initial_height: u32, cx: &mut App) -> Self {
        let format = render_engine.preferred_format();
        let double_buffer = Arc::new(Mutex::new(DoubleBuffer::new(initial_width, initial_height, format)));
        let render_engine = Arc::new(Mutex::new(render_engine));
        let metrics = Arc::new(Mutex::new(ViewportMetrics::default()));
        let frame_times = Arc::new(Mutex::new(VecDeque::with_capacity(60)));

        // Create render thread
        let (render_tx, render_rx) = mpsc::channel();

        // Initialize render engine
        if let Ok(mut engine) = render_engine.lock() {
            if let Err(e) = engine.initialize() {
                eprintln!("[VIEWPORT] Failed to initialize render engine: {}", e);
            }
        }
        let engine_clone = render_engine.clone();
        let buffer_clone = double_buffer.clone();
        let metrics_clone = metrics.clone();
        let frame_times_clone = frame_times.clone();

        let render_thread = std::thread::spawn(move || {
            Self::render_thread_main(
                engine_clone,
                buffer_clone,
                metrics_clone,
                frame_times_clone,
                render_rx
            );
        });

        Self {
            focus_handle: cx.focus_handle(),
            render_engine,
            double_buffer,
            visible: true,
            bounds: Bounds::default(),
            render_tx,
            _render_thread: render_thread,
            metrics,
            frame_times,
            last_texture_generation: 0,
            current_texture: None,
            texture_dirty: true,
            debug_enabled: cfg!(debug_assertions),
            entity: None,
        }
    }

    /// Set the entity reference for this viewport and provide it to the render engine
    pub fn set_entity(&mut self, entity: Entity<Self>) {
        self.entity = Some(entity.clone());

        // Create a callback that can trigger GPUI notifications from the render thread
        let (notify_tx, notify_rx) = std::sync::mpsc::channel();
        let entity_for_callback = entity.clone();

        // Spawn a thread to handle notifications from render thread
        std::thread::spawn(move || {
            while let Ok(()) = notify_rx.recv() {
                // We still need to find the right way to get a context here
                // For now, let's just mark this as needing implementation
                println!("[VIEWPORT] Render notification received - need to implement GPUI update");
            }
        });

        let callback = Box::new(move || {
            // This will be called from the render thread
            let _ = notify_tx.send(());
        });

        // Provide the callback to the render engine
        if let Ok(mut engine) = self.render_engine.lock() {
            engine.set_notify_callback(callback);
        }
    }

    /// Main render thread loop
    fn render_thread_main(
        render_engine: Arc<Mutex<E>>,
        double_buffer: Arc<Mutex<DoubleBuffer>>,
        metrics: Arc<Mutex<ViewportMetrics>>,
        frame_times: Arc<Mutex<VecDeque<Instant>>>,
        render_rx: mpsc::Receiver<RenderCommand>,
    ) {
        let mut should_continue = true;

        while should_continue {
            match render_rx.recv_timeout(Duration::from_millis(16)) { // ~60 FPS max
                Ok(command) => match command {
                    RenderCommand::Render => {
                        Self::perform_render(&render_engine, &double_buffer, &metrics, &frame_times);
                    }
                    RenderCommand::Resize(width, height) => {
                        if let Ok(mut buffer) = double_buffer.lock() {
                            buffer.resize(width, height);
                        }
                        if let Ok(mut engine) = render_engine.lock() {
                            engine.on_resize(width, height);
                        }
                    }
                    RenderCommand::Shutdown => {
                        should_continue = false;
                        if let Ok(mut engine) = render_engine.lock() {
                            engine.cleanup();
                        }
                    }
                },
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Continue rendering at target framerate
                    Self::perform_render(&render_engine, &double_buffer, &metrics, &frame_times);
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    should_continue = false;
                }
            }
        }
    }

    fn perform_render(
        render_engine: &Arc<Mutex<E>>,
        double_buffer: &Arc<Mutex<DoubleBuffer>>,
        metrics: &Arc<Mutex<ViewportMetrics>>,
        frame_times: &Arc<Mutex<VecDeque<Instant>>>,
    ) {
        let start_time = Instant::now();

        // Render to back buffer
        let render_result = {
            let mut buffer_guard = match double_buffer.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };

            let mut engine_guard = match render_engine.lock() {
                Ok(guard) => guard,
                Err(_) => return,
            };

            let back_buffer = buffer_guard.get_back_buffer();
            engine_guard.render(back_buffer)
        };

        if let Err(e) = render_result {
            eprintln!("[VIEWPORT] Render error: {}", e);
            return;
        }

        // Swap buffers
        if let Ok(mut buffer_guard) = double_buffer.lock() {
            buffer_guard.swap();
        }

        // Update metrics
        let frame_time = start_time.elapsed();
        Self::update_metrics(metrics, frame_times, frame_time);
    }

    fn update_metrics(
        metrics: &Arc<Mutex<ViewportMetrics>>,
        frame_times: &Arc<Mutex<VecDeque<Instant>>>,
        frame_time: Duration,
    ) {
        let frame_time_ms = frame_time.as_secs_f64() * 1000.0;

        if let (Ok(mut metrics_guard), Ok(mut times_guard)) = (metrics.lock(), frame_times.lock()) {
            metrics_guard.frame_count += 1;
            metrics_guard.buffer_swaps += 1;

            // Update frame time stats
            if metrics_guard.frame_count == 1 {
                metrics_guard.min_frame_time_ms = frame_time_ms;
                metrics_guard.max_frame_time_ms = frame_time_ms;
                metrics_guard.avg_frame_time_ms = frame_time_ms;
            } else {
                metrics_guard.min_frame_time_ms = metrics_guard.min_frame_time_ms.min(frame_time_ms);
                metrics_guard.max_frame_time_ms = metrics_guard.max_frame_time_ms.max(frame_time_ms);

                // Rolling average
                let alpha = 0.1;
                metrics_guard.avg_frame_time_ms =
                    alpha * frame_time_ms + (1.0 - alpha) * metrics_guard.avg_frame_time_ms;
            }

            // Calculate FPS from recent frames
            let now = Instant::now();
            times_guard.push_back(now);

            // Keep only last 60 frames
            while times_guard.len() > 60 {
                times_guard.pop_front();
            }

            if times_guard.len() >= 2 {
                let time_span = now.duration_since(times_guard[0]).as_secs_f64();
                if time_span > 0.0 {
                    metrics_guard.fps = (times_guard.len() - 1) as f64 / time_span;
                }
            }
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    /// Trigger a render (non-blocking)
    pub fn request_render(&self) {
        let _ = self.render_tx.send(RenderCommand::Render);
    }

    /// Get current performance metrics
    pub fn metrics(&self) -> ViewportMetrics {
        self.metrics.lock().map(|m| m.clone()).unwrap_or_default()
    }

    /// Enable or disable debug output
    pub fn set_debug_enabled(&mut self, enabled: bool) {
        self.debug_enabled = enabled;
    }

    /// Access the render engine (use with caution - prefer async rendering)
    pub fn with_render_engine<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut E) -> R,
    {
        self.render_engine.lock().ok().map(|mut engine| f(&mut *engine))
    }

    /// Get the current framebuffer for reading (front buffer)
    pub fn current_framebuffer(&self) -> Option<Framebuffer> {
        self.double_buffer.lock().ok().map(|buffer| {
            let front = buffer.get_front_buffer();
            Framebuffer {
                width: front.width,
                height: front.height,
                format: front.format,
                buffer: front.buffer.clone(),
                pitch: front.pitch,
                dirty_rect: front.dirty_rect,
                generation: front.generation,
            }
        })
    }

    fn update_texture_if_needed(&mut self, _window: &mut Window) {
        let buffer_guard = match self.double_buffer.lock() {
            Ok(guard) => guard,
            Err(_) => return,
        };

        let front_buffer = buffer_guard.get_front_buffer();

        // Check if texture needs updating
        let needs_update = self.current_texture.is_none()
            || self.texture_dirty
            || front_buffer.generation() != self.last_texture_generation;

        if !needs_update {
            return;
        }

        if front_buffer.width == 0 || front_buffer.height == 0 {
            return;
        }

        // Convert to RGBA8 format for GPUI
        let rgba_buffer = match front_buffer.format {
            FramebufferFormat::Rgba8 => front_buffer.buffer.clone(),
            FramebufferFormat::Bgra8 => {
                // Convert BGRA to RGBA
                let mut rgba = Vec::with_capacity(front_buffer.buffer.len());
                for chunk in front_buffer.buffer.chunks_exact(4) {
                    rgba.extend_from_slice(&[chunk[2], chunk[1], chunk[0], chunk[3]]);
                }
                rgba
            }
            FramebufferFormat::Rgb8 => {
                // Convert RGB to RGBA
                let mut rgba = Vec::with_capacity(front_buffer.buffer.len() * 4 / 3);
                for chunk in front_buffer.buffer.chunks_exact(3) {
                    rgba.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
                }
                rgba
            }
            FramebufferFormat::Bgr8 => {
                // Convert BGR to RGBA
                let mut rgba = Vec::with_capacity(front_buffer.buffer.len() * 4 / 3);
                for chunk in front_buffer.buffer.chunks_exact(3) {
                    rgba.extend_from_slice(&[chunk[2], chunk[1], chunk[0], 255]);
                }
                rgba
            }
        };

        // Create image buffer
        if let Some(image_buffer) = image::ImageBuffer::from_vec(
            front_buffer.width,
            front_buffer.height,
            rgba_buffer,
        ) {
            let render_image = Arc::new(RenderImage::new([image::Frame::new(image_buffer)]));
            self.current_texture = Some(render_image);
            self.last_texture_generation = front_buffer.generation();
            self.texture_dirty = false;

            // Update metrics
            if let Ok(mut metrics) = self.metrics.lock() {
                metrics.texture_updates += 1;
            }

            if self.debug_enabled {
                println!("[VIEWPORT] Texture updated: {}x{} gen:{}",
                    front_buffer.width, front_buffer.height, front_buffer.generation());
            }
        }
    }
}

impl<E: RenderEngine> Focusable for Viewport<E> {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl<E: RenderEngine> EventEmitter<DismissEvent> for Viewport<E> {}

impl<E: RenderEngine> Render for Viewport<E> {
    fn render(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity().clone();

        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .child({
                let view_layout = cx.entity().clone();
                let view_paint = cx.entity().clone();
                canvas(
                    move |bounds, _, cx| {
                        view_layout.update(cx, |viewport, _| {
                            let width = bounds.size.width.0 as u32;
                            let height = bounds.size.height.0 as u32;

                            viewport.bounds = bounds;

                            // Resize if needed
                            if let Ok(buffer) = viewport.double_buffer.lock() {
                                let front = buffer.get_front_buffer();
                                if front.width != width || front.height != height {
                                    let _ = viewport.render_tx.send(RenderCommand::Resize(width, height));
                                    viewport.texture_dirty = true;
                                }
                            }

                            // Request render
                            viewport.request_render();
                        });
                    },
                    move |bounds, _hitbox, window, cx| {
                        view_paint.update(cx, |viewport, _| {
                            if !viewport.visible {
                                return;
                            }

                            // Update texture if needed
                            viewport.update_texture_if_needed(window);

                            // Paint the texture
                            if let Some(ref texture) = viewport.current_texture {
                                window.with_content_mask(Some(ContentMask { bounds }), |window| {
                                    let _ = window.paint_image(
                                        bounds,
                                        Corners::all(px(0.0)),
                                        texture.clone(),
                                        0,
                                        false,
                                    );
                                });
                            } else if viewport.debug_enabled {
                                // Draw debug placeholder
                                window.with_content_mask(Some(ContentMask { bounds }), |window| {
                                    window.paint_quad(PaintQuad {
                                        bounds,
                                        corner_radii: Corners::all(px(0.0)),
                                        background: gpui::rgba(0x20202080).into(),
                                        border_widths: gpui::Edges::all(px(1.0)),
                                        border_color: gpui::rgba(0x808080ff).into(),
                                        border_style: BorderStyle::Solid,
                                    });
                                });
                            }
                        });
                    },
                )
                .absolute()
                .size_full()
            })
    }
}

/// A simple test render engine for debugging
pub struct TestRenderEngine {
    frame_count: u64,
    color_cycle: f32,
    notify_callback: Option<Box<dyn Fn() + Send + Sync>>,
}

impl std::fmt::Debug for TestRenderEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestRenderEngine")
            .field("frame_count", &self.frame_count)
            .field("color_cycle", &self.color_cycle)
            .field("notify_callback", &self.notify_callback.as_ref().map(|_| "<callback>"))
            .finish()
    }
}

impl TestRenderEngine {
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            color_cycle: 0.0,
            notify_callback: None,
        }
    }
}

impl RenderEngine for TestRenderEngine {
    fn render(&mut self, framebuffer: &mut Framebuffer) -> Result<(), RenderError> {
        self.frame_count += 1;
        self.color_cycle += 0.02;

        // Create a simple animated pattern
        let r = ((self.color_cycle.sin() * 0.5 + 0.5) * 255.0) as u8;
        let g = (((self.color_cycle + 2.0).sin() * 0.5 + 0.5) * 255.0) as u8;
        let b = (((self.color_cycle + 4.0).sin() * 0.5 + 0.5) * 255.0) as u8;

        framebuffer.clear([r, g, b, 255]);

        // Draw some animated content
        for y in 0..framebuffer.height {
            for x in 0..framebuffer.width {
                let offset = ((y * framebuffer.pitch + x * 4) as usize).min(framebuffer.buffer.len().saturating_sub(4));
                if offset + 3 < framebuffer.buffer.len() {
                    let wave = ((x as f32 / 50.0 + self.color_cycle).sin() * 127.0 + 128.0) as u8;
                    framebuffer.buffer[offset] = wave;
                    framebuffer.buffer[offset + 1] = ((y as f32 / 50.0 + self.color_cycle).cos() * 127.0 + 128.0) as u8;
                    framebuffer.buffer[offset + 2] = b;
                    framebuffer.buffer[offset + 3] = 255;
                }
            }
        }

        framebuffer.mark_dirty(None);

        // Notify GPUI that the viewport needs to be redrawn
        // This is called from the render thread after each frame is complete
        if let Some(callback) = &self.notify_callback {
            callback();
        }

        Ok(())
    }

    fn initialize(&mut self) -> Result<(), RenderError> {
        println!("[TEST_ENGINE] Initialized");
        Ok(())
    }

    fn cleanup(&mut self) {
        println!("[TEST_ENGINE] Cleaned up");
    }

    fn on_resize(&mut self, _width: u32, _height: u32) {
        // TestRenderEngine doesn't need special resize handling
    }

    fn set_notify_callback(&mut self, callback: Box<dyn Fn() + Send + Sync>) {
        self.notify_callback = Some(callback);
    }
}

impl Default for TestRenderEngine {
    fn default() -> Self {
        Self::new()
    }
}