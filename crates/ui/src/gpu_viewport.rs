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
    div, App, Bounds, Context, DismissEvent, Element, ElementId,
    Entity, EventEmitter, FocusHandle, Focusable, GlobalElementId, InspectorElementId,
    InteractiveElement, IntoElement, LayoutId, ParentElement as _, Pixels, Render,
    Size, StatefulInteractiveElement, Styled as _, Window, DevicePixels, ScaledPixels,
    ExternalTextureId, ContentMask, Corners,
};
use std::sync::{Arc, atomic::{AtomicU64, Ordering}};

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

    /// Get texture ID for rendering
    pub fn texture_id(&self) -> Option<ExternalTextureId> {
        self.texture_id
    }

    /// Set texture ID (called from external initialization)
    pub fn set_texture_id(&mut self, texture_id: ExternalTextureId) {
        self.texture_id = Some(texture_id);
        println!("[GPU-VIEWPORT] ✅ Set external texture: {:?}, size: {}x{}",
            texture_id, self.width, self.height);
    }

    /// Get dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Resize the viewport
    pub fn resize(&mut self, width: u32, height: u32) {
        if width != self.width || height != self.height {
            self.width = width;
            self.height = height;
            // Note: Caller must re-register the texture with new dimensions
            self.texture_id = None;
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
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
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
        _cx: &mut App,
    ) {
        if let Some(texture_id) = self.texture_id {
            // Convert bounds from Pixels to ScaledPixels
            let scaled_bounds: Bounds<ScaledPixels> = bounds.map(|p| p.into());

            // Paint the external texture into the scene
            window.paint_external_texture(
                scaled_bounds,
                Corners::all(ScaledPixels(0.0)),
                texture_id,
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
    cx.new_entity(|cx| GpuViewport::new(width, height, cx))
}
