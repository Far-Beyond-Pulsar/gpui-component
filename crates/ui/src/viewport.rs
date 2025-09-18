use gpui::{
    canvas, div, App, Bounds, ContentMask, DismissEvent, EventEmitter,
    FocusHandle, Focusable, InteractiveElement, IntoElement,
    ParentElement as _, Pixels, Render, RenderImage, Size, Styled as _, Window, Corners, px,
    Context, PaintQuad, Point, BorderStyle,
};
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};

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

/// Double-buffered framebuffer system for smooth updates with zero-copy access
pub struct DoubleBuffer {
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

    /// Get mutable access to the back buffer for rendering (zero-copy)
    pub fn get_back_buffer(&mut self) -> &mut Framebuffer {
        if self.swapped.load(Ordering::Acquire) {
            &mut self.front
        } else {
            &mut self.back
        }
    }

    /// Get read-only access to the front buffer for display (zero-copy)
    pub fn get_front_buffer(&self) -> &Framebuffer {
        if self.swapped.load(Ordering::Acquire) {
            &self.back
        } else {
            &self.front
        }
    }

    /// Swap front and back buffers atomically
    pub fn swap(&mut self) {
        let current = self.swapped.load(Ordering::Acquire);
        self.swapped.store(!current, Ordering::Release);
    }
}

/// A handle that provides zero-copy access to viewport buffers
pub struct ViewportBuffers {
    double_buffer: Arc<Mutex<DoubleBuffer>>,
}

impl ViewportBuffers {
    /// Get mutable access to the back buffer for rendering
    /// Returns None if the mutex is poisoned
    pub fn with_back_buffer<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&mut Framebuffer) -> R,
    {
        self.double_buffer.lock().ok().map(|mut buffer| {
            let back = buffer.get_back_buffer();
            f(back)
        })
    }

    /// Get read-only access to the front buffer for reading
    /// Returns None if the mutex is poisoned
    pub fn with_front_buffer<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Framebuffer) -> R,
    {
        self.double_buffer.lock().ok().map(|buffer| {
            let front = buffer.get_front_buffer();
            f(front)
        })
    }

    /// Swap the front and back buffers
    /// Call this after rendering to the back buffer is complete
    pub fn swap_buffers(&self) {
        if let Ok(mut buffer) = self.double_buffer.lock() {
            buffer.swap();
        }
    }

    /// Resize both buffers
    pub fn resize(&self, width: u32, height: u32) {
        if let Ok(mut buffer) = self.double_buffer.lock() {
            buffer.resize(width, height);
        }
    }
}

/// A hook function that external render engines can call to trigger GPUI refresh
pub type RefreshHook = Arc<dyn Fn() + Send + Sync>;

/// Create viewport buffers and refresh hook
pub fn create_viewport_system(width: u32, height: u32, format: FramebufferFormat) -> (ViewportBuffers, RefreshHook) {
    let double_buffer = Arc::new(Mutex::new(DoubleBuffer::new(width, height, format)));
    let buffers = ViewportBuffers {
        double_buffer: double_buffer.clone(),
    };

    // This will be set up by the viewport when it's created
    let refresh_trigger = Arc::new(AtomicBool::new(false));
    let refresh_flag = refresh_trigger.clone();
    
    let refresh_hook: RefreshHook = Arc::new(move || {
        refresh_flag.store(true, Ordering::Relaxed);
    });

    (buffers, refresh_hook)
}




/// High-performance viewport component with zero-copy buffer access
pub struct Viewport {
    focus_handle: FocusHandle,
    double_buffer: Arc<Mutex<DoubleBuffer>>,
    visible: bool,
    bounds: Bounds<Pixels>,

    // Performance tracking
    metrics: Arc<Mutex<ViewportMetrics>>,
    last_texture_generation: u64,

    // Texture management
    current_texture: Option<Arc<RenderImage>>,
    texture_dirty: bool,
    rgba_conversion_buffer: Vec<u8>,
    last_width: u32,
    last_height: u32,

    // Debug flags
    debug_enabled: bool,

    // GPUI refresh mechanism
    refresh_trigger: Arc<AtomicBool>,
}

impl Drop for Viewport {
    fn drop(&mut self) {
        self.hide();

        // Clean up memory allocations
        self.current_texture = None;
        self.rgba_conversion_buffer.clear();
        self.rgba_conversion_buffer.shrink_to_fit();
    }
}

impl Viewport {
    /// Create a new viewport and return it along with buffer access and refresh hook
    /// Uses proper GPUI background task pattern for reactive refreshes
    pub fn new(initial_width: u32, initial_height: u32, format: FramebufferFormat, cx: &mut Context<impl Focusable>) -> (Self, ViewportBuffers, RefreshHook) {
        let double_buffer = Arc::new(Mutex::new(DoubleBuffer::new(initial_width, initial_height, format)));
        let metrics = Arc::new(Mutex::new(ViewportMetrics::default()));

        // Create buffer handle for external access
        let buffers = ViewportBuffers {
            double_buffer: double_buffer.clone(),
        };

        // Create async channel for GPUI background task communication
        let (refresh_sender, refresh_receiver) = smol::channel::unbounded::<()>();
        let refresh_hook: RefreshHook = Arc::new(move || {
            let _ = refresh_sender.try_send(()); // Send signal to background task
        });

        let viewport = Self {
            focus_handle: cx.focus_handle(),
            double_buffer: double_buffer.clone(),
            visible: true,
            bounds: Bounds::default(),
            metrics,
            last_texture_generation: 0,
            current_texture: None,
            texture_dirty: true,
            rgba_conversion_buffer: Vec::new(),
            last_width: initial_width,
            last_height: initial_height,
            debug_enabled: cfg!(debug_assertions),
            refresh_trigger: Arc::new(AtomicBool::new(false)),
        };

        // Start GPUI background task for reactive refreshes (GPML pattern)
        cx.spawn(async move |this, mut cx| {
            while let Ok(()) = refresh_receiver.recv().await {
                // Update and trigger reactive refresh - this is the key GPUI pattern
                let _ = this.update(cx, |_viewport, cx| {
                    cx.notify(); // Triggers GPUI's reactive refresh system
                });
            }
        }).detach();

        (viewport, buffers, refresh_hook)
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

    /// Get current performance metrics
    pub fn metrics(&self) -> ViewportMetrics {
        self.metrics.lock().map(|m| m.clone()).unwrap_or_default()
    }

    /// Enable or disable debug output
    pub fn set_debug_enabled(&mut self, enabled: bool) {
        self.debug_enabled = enabled;
    }

    /// Get a reference to the current framebuffer for reading (front buffer)
    /// Returns width, height, format, and generation without cloning the buffer
    pub fn current_framebuffer_info(&self) -> Option<(u32, u32, FramebufferFormat, u64)> {
        self.double_buffer.lock().ok().map(|buffer| {
            let front = buffer.get_front_buffer();
            (front.width, front.height, front.format, front.generation)
        })
    }

    /// Access the current framebuffer with a closure to avoid cloning
    pub fn with_current_framebuffer<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&Framebuffer) -> R,
    {
        self.double_buffer.lock().ok().map(|buffer| {
            let front = buffer.get_front_buffer();
            f(front)
        })
    }

    /// Check if the GPUI should refresh (called from render loop)
    pub fn should_refresh(&self) -> bool {
        self.refresh_trigger.load(Ordering::Relaxed)
    }

    /// Clear the refresh flag after handling the refresh
    pub fn clear_refresh_flag(&self) {
        self.refresh_trigger.store(false, Ordering::Relaxed);
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
            || front_buffer.generation() != self.last_texture_generation
            || self.should_refresh();

        if !needs_update {
            return;
        }

        // Clear refresh flag since we're handling the update
        self.clear_refresh_flag();

        // Throttle updates during rapid resizing to prevent memory pressure
        if front_buffer.width != self.last_width || front_buffer.height != self.last_height {
            self.last_width = front_buffer.width;
            self.last_height = front_buffer.height;

            // Skip update if dimensions are invalid
            if front_buffer.width == 0 || front_buffer.height == 0 {
                return;
            }
        }

        // Reuse conversion buffer to avoid allocations
        let required_size = match front_buffer.format {
            FramebufferFormat::Rgba8 | FramebufferFormat::Bgra8 => front_buffer.buffer.len(),
            FramebufferFormat::Rgb8 | FramebufferFormat::Bgr8 => front_buffer.buffer.len() * 4 / 3,
        };

        if self.rgba_conversion_buffer.len() != required_size {
            self.rgba_conversion_buffer.resize(required_size, 0);
        }

        // Convert to RGBA8 format for GPUI using pre-allocated buffer
        let rgba_buffer = match front_buffer.format {
            FramebufferFormat::Rgba8 => {
                self.rgba_conversion_buffer.copy_from_slice(&front_buffer.buffer);
                &self.rgba_conversion_buffer
            }
            FramebufferFormat::Bgra8 => {
                // Convert BGRA to RGBA in-place
                for (i, chunk) in front_buffer.buffer.chunks_exact(4).enumerate() {
                    let offset = i * 4;
                    self.rgba_conversion_buffer[offset] = chunk[2];     // R
                    self.rgba_conversion_buffer[offset + 1] = chunk[1]; // G
                    self.rgba_conversion_buffer[offset + 2] = chunk[0]; // B
                    self.rgba_conversion_buffer[offset + 3] = chunk[3]; // A
                }
                &self.rgba_conversion_buffer
            }
            FramebufferFormat::Rgb8 => {
                // Convert RGB to RGBA
                for (i, chunk) in front_buffer.buffer.chunks_exact(3).enumerate() {
                    let offset = i * 4;
                    self.rgba_conversion_buffer[offset] = chunk[0];     // R
                    self.rgba_conversion_buffer[offset + 1] = chunk[1]; // G
                    self.rgba_conversion_buffer[offset + 2] = chunk[2]; // B
                    self.rgba_conversion_buffer[offset + 3] = 255;     // A
                }
                &self.rgba_conversion_buffer
            }
            FramebufferFormat::Bgr8 => {
                // Convert BGR to RGBA
                for (i, chunk) in front_buffer.buffer.chunks_exact(3).enumerate() {
                    let offset = i * 4;
                    self.rgba_conversion_buffer[offset] = chunk[2];     // R
                    self.rgba_conversion_buffer[offset + 1] = chunk[1]; // G
                    self.rgba_conversion_buffer[offset + 2] = chunk[0]; // B
                    self.rgba_conversion_buffer[offset + 3] = 255;     // A
                }
                &self.rgba_conversion_buffer
            }
        };

        // Create image buffer from converted data
        if let Some(image_buffer) = image::ImageBuffer::from_vec(
            front_buffer.width,
            front_buffer.height,
            rgba_buffer.to_vec(), // Only clone when creating the texture
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

impl Focusable for Viewport {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for Viewport {}

impl Render for Viewport {
    fn render(
        &mut self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
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
                                    // Clean up current texture before resize to free memory
                                    viewport.current_texture = None;
                                    viewport.rgba_conversion_buffer.clear();
                                    viewport.rgba_conversion_buffer.shrink_to_fit();

                                    // Resize the buffers
                                    drop(buffer);
                                    if let Ok(mut buffer) = viewport.double_buffer.lock() {
                                        buffer.resize(width, height);
                                    }
                                    viewport.texture_dirty = true;
                                    viewport.last_width = width;
                                    viewport.last_height = height;
                                }
                            }
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
}

impl std::fmt::Debug for TestRenderEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TestRenderEngine")
            .field("frame_count", &self.frame_count)
            .field("color_cycle", &self.color_cycle)
            .finish()
    }
}

impl TestRenderEngine {
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            color_cycle: 0.0,
        }
    }
    
    pub fn render(&mut self, framebuffer: &mut Framebuffer) {
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
    }
}

impl Default for TestRenderEngine {
    fn default() -> Self {
        Self::new()
    }
}