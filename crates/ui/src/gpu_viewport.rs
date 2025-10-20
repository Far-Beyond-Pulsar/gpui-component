/// Zero-copy GPU viewport using GPUI's new ExternalTexture system
///
/// This eliminates ALL CPU copies by allowing Bevy to render directly
/// to a GPU texture that GPUI can composite without any transfers.
///
/// Flow:
/// 1. Register external GPU texture with GPUI
/// 2. Map texture for CPU writes (returns GPU-visible memory pointer)
/// 3. Bevy renders directly to this mapped memory
/// 4. Unmap to finalize GPU writes
/// 5. Swap buffers for tear-free display
/// 6. GPUI composites the texture directly - ZERO COPIES!

use gpui::{
    div, px, App, Bounds, Context, Corners, DismissEvent, Element, ElementId,
    Entity, EventEmitter, FocusHandle, Focusable, GlobalElementId, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, ParentElement as _, Pixels, Render,
    Size, StatefulInteractiveElement, Styled as _, Window, DevicePixels,
    ExternalTextureId, ExternalTexture as ExternalTexturePrimitive, DrawOrder,
};
use std::sync::{Arc, atomic::{AtomicBool, AtomicU64, Ordering}};

/// GPU viewport with zero-copy rendering
pub struct GpuViewport {
    /// External texture ID for direct GPU rendering
    texture_id: Option<ExternalTextureId>,
    /// Viewport dimensions
    width: u32,
    height: u32,
    /// Focus handle
    focus_handle: FocusHandle,
    /// Frame counter for diagnostics
    frame_count: Arc<AtomicU64>,
    /// Debug mode
    debug: bool,
}

impl GpuViewport {
    /// Create new GPU viewport
    pub fn new<V: 'static>(
        width: u32,
        height: u32,
        cx: &mut Context<V>,
    ) -> Self {
        Self {
            texture_id: None,
            width,
            height,
            focus_handle: cx.focus_handle(),
            frame_count: Arc::new(AtomicU64::new(0)),
            debug: cfg!(debug_assertions),
        }
    }

    /// Initialize the GPU texture
    pub fn initialize(&mut self, window: &mut Window) {
        if self.texture_id.is_some() {
            return; // Already initialized
        }

        let size = Size {
            width: DevicePixels(self.width as i32),
            height: DevicePixels(self.height as i32),
        };

        // Register external texture with GPUI's atlas
        match window.sprite_atlas().register_external_texture(size, gpu::TextureFormat::Bgra8Unorm) {
            Ok(texture_id) => {
                println!("[GPU-VIEWPORT] ✅ Registered external texture: {:?}, size: {}x{}",
                    texture_id, self.width, self.height);
                self.texture_id = Some(texture_id);
            }
            Err(e) => {
                eprintln!("[GPU-VIEWPORT] ❌ Failed to register external texture: {}", e);
            }
        }
    }

    /// Resize the viewport
    pub fn resize(&mut self, window: &mut Window, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }

        // Unregister old texture
        if let Some(texture_id) = self.texture_id.take() {
            window.sprite_atlas().unregister_external_texture(texture_id);
        }

        self.width = width;
        self.height = height;

        // Re-initialize with new size
        self.initialize(window);
    }

    /// Get texture ID for rendering
    pub fn texture_id(&self) -> Option<ExternalTextureId> {
        self.texture_id
    }

    /// Get dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Map texture for CPU writes (returns GPU-visible memory)
    ///
    /// SAFETY: The returned slice is valid until unmap() is called.
    /// Do not use after calling unmap()!
    pub unsafe fn map(&self, window: &Window) -> Option<&mut [u8]> {
        let texture_id = self.texture_id?;

        match window.sprite_atlas().map_external_texture(texture_id) {
            Ok(buffer) => {
                if self.debug {
                    println!("[GPU-VIEWPORT] 🗺️  Mapped texture buffer: {} bytes", buffer.len());
                }
                Some(buffer)
            }
            Err(e) => {
                eprintln!("[GPU-VIEWPORT] ❌ Failed to map texture: {}", e);
                None
            }
        }
    }

    /// Unmap texture after CPU writes
    pub fn unmap(&self, window: &Window) {
        if let Some(texture_id) = self.texture_id {
            if let Err(e) = window.sprite_atlas().unmap_external_texture(texture_id) {
                eprintln!("[GPU-VIEWPORT] ❌ Failed to unmap texture: {}", e);
            }
        }
    }

    /// Swap front/back buffers for tear-free display
    pub fn swap_buffers(&self, window: &Window) {
        if let Some(texture_id) = self.texture_id {
            if let Err(e) = window.sprite_atlas().swap_external_texture_buffers(texture_id) {
                eprintln!("[GPU-VIEWPORT] ❌ Failed to swap buffers: {}", e);
            }
        }
    }
}

impl Drop for GpuViewport {
    fn drop(&mut self) {
        // Note: We can't unregister here because we don't have access to Window
        // The texture will be cleaned up when the window closes
        println!("[GPU-VIEWPORT] 🗑️  Viewport dropped");
    }
}

impl Focusable for GpuViewport {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for GpuViewport {}

impl Render for GpuViewport {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Initialize on first render
        if self.texture_id.is_none() {
            self.initialize(window);
        }

        let texture_id = self.texture_id;
        let frame = self.frame_count.fetch_add(1, Ordering::Relaxed);

        if self.debug && frame % 60 == 0 {
            println!("[GPU-VIEWPORT] 📊 Frame {}, texture: {:?}", frame, texture_id);
        }

        div()
            .id("gpu-viewport")
            .size_full()
            .flex()
            .flex_1()
            .child(GpuViewportElement { texture_id })
            .focusable()
    }
}

/// Custom element for rendering the GPU texture
struct GpuViewportElement {
    texture_id: Option<ExternalTextureId>,
}

impl Element for GpuViewportElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(ElementId::Name("gpu-viewport-element".into()))
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
        let mut style = gpui::Style::default();
        style.size = Size::full();
        style.flex_grow = 1.0;
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
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        _prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        if let Some(texture_id) = self.texture_id {
            // Paint the external texture directly into the scene
            let content_mask = gpui::ContentMask {
                bounds: gpui::Bounds {
                    origin: bounds.origin.map(|p| p.into()),
                    size: bounds.size.map(|p| p.into()),
                },
            };

            window.paint_external_texture(
                ExternalTexturePrimitive {
                    order: DrawOrder::new(0), // TODO: Get proper draw order
                    bounds: gpui::Bounds {
                        origin: bounds.origin.map(|p| p.into()),
                        size: bounds.size.map(|p| p.into()),
                    },
                    corner_radii: Corners::all(gpui::ScaledPixels(0.0)),
                    content_mask,
                    opacity: 1.0,
                    grayscale: false,
                    texture_id,
                },
            );
        }
    }
}

impl IntoElement for GpuViewportElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

/// Helper function to create GPU viewport
pub fn create_gpu_viewport<V: 'static>(
    width: u32,
    height: u32,
    cx: &mut Context<V>,
) -> Entity<GpuViewport> {
    cx.new(|cx| GpuViewport::new(width, height, cx))
}
