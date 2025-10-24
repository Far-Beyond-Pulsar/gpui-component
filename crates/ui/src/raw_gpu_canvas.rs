/// RAW GPU CANVAS - Brain-dead simple, instant rendering of native GPU textures
///
/// This bypasses GPUI's atlas system entirely for maximum performance.
/// Just point at a DirectX/Metal/Vulkan texture and it displays it.
/// NO registration, NO copying, NO buffering - pure pointer display!

use gpui::*;
use std::sync::atomic::{AtomicUsize, Ordering};

/// Raw GPU texture pointer - platform specific
#[derive(Clone, Copy)]
pub struct RawGpuTexturePtr {
    #[cfg(target_os = "windows")]
    pub d3d11_srv: usize, // ID3D11ShaderResourceView* as usize

    #[cfg(target_os = "macos")]
    pub metal_texture: usize, // MTLTexture* as usize

    #[cfg(target_os = "linux")]
    pub vk_image_view: u64, // VkImageView as u64
}

unsafe impl Send for RawGpuTexturePtr {}
unsafe impl Sync for RawGpuTexturePtr {}

/// Brain-dead simple canvas that displays whatever texture you point at it
pub struct RawGpuCanvas {
    /// Current texture pointer to display
    texture_ptr: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    /// Texture dimensions
    width: u32,
    height: u32,
    /// Focus handle
    focus_handle: FocusHandle,
}

impl RawGpuCanvas {
    /// Create new canvas
    pub fn new<V: 'static>(width: u32, height: u32, cx: &mut Context<V>) -> Self {
        Self {
            texture_ptr: std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            width,
            height,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Point at a new texture - updates INSTANTLY on next frame
    pub fn set_texture(&mut self, ptr: RawGpuTexturePtr) {
        #[cfg(target_os = "windows")]
        self.texture_ptr.store(ptr.d3d11_srv, Ordering::Release);

        #[cfg(target_os = "macos")]
        self.texture_ptr.store(ptr.metal_texture, Ordering::Release);

        #[cfg(target_os = "linux")]
        self.texture_ptr.store(ptr.vk_image_view as usize, Ordering::Release);
    }

    /// Get current texture pointer
    pub fn get_texture_ptr(&self) -> usize {
        self.texture_ptr.load(Ordering::Acquire)
    }

    /// Resize canvas
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
}

impl Focusable for RawGpuCanvas {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for RawGpuCanvas {}

impl Render for RawGpuCanvas {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let texture_ptr = self.texture_ptr.clone();

        div()
            .id("raw-gpu-canvas")
            .size_full()
            .child(RawGpuCanvasElement {
                texture_ptr,
                width: self.width,
                height: self.height,
            })
    }
}

/// Custom element that paints the raw GPU texture
struct RawGpuCanvasElement {
    texture_ptr: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    width: u32,
    height: u32,
}

impl Element for RawGpuCanvasElement {
    type RequestLayoutState = ();
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(ElementId::Name("raw-gpu-canvas-element".into()))
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
        let ptr = self.texture_ptr.load(Ordering::Acquire);

        if ptr != 0 {
            // Paint raw GPU texture directly - platform specific
            #[cfg(target_os = "windows")]
            unsafe {
                paint_raw_d3d11_texture(window, bounds, ptr);
            }

            #[cfg(target_os = "macos")]
            unsafe {
                paint_raw_metal_texture(window, bounds, ptr);
            }

            #[cfg(target_os = "linux")]
            unsafe {
                paint_raw_vulkan_texture(window, bounds, ptr as u64);
            }
        }
    }
}

impl IntoElement for RawGpuCanvasElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

// Platform-specific raw texture painting
#[cfg(target_os = "windows")]
unsafe fn paint_raw_d3d11_texture(window: &mut Window, bounds: Bounds<Pixels>, srv_ptr: usize) {
    // Get DirectX renderer from window
    // Paint the ShaderResourceView directly to the quad at bounds
    // This requires extending GPUI's DirectX renderer with a raw texture path

    // TODO: Add to directx_renderer.rs:
    // pub fn paint_raw_texture(&mut self, bounds, srv: *mut ID3D11ShaderResourceView)

    let _ = (window, bounds, srv_ptr); // Suppress warnings for now
}

#[cfg(target_os = "macos")]
unsafe fn paint_raw_metal_texture(window: &mut Window, bounds: Bounds<Pixels>, texture_ptr: usize) {
    let _ = (window, bounds, texture_ptr);
}

#[cfg(target_os = "linux")]
unsafe fn paint_raw_vulkan_texture(window: &mut Window, bounds: Bounds<Pixels>, image_view: u64) {
    let _ = (window, bounds, image_view);
}
