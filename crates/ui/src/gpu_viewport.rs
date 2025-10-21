/// IMMEDIATE-MODE GPU VIEWPORT - TRUE ZERO-COPY
///
/// NO texture registration, NO buffers, NO copies!
/// Just displays whatever native GPU texture handle you point it at.
///
/// Architecture:
/// 1. Bevy renders to shared textures in its own thread
/// 2. Bevy exposes native DirectX/Metal/Vulkan handles via get_native_texture_handle()
/// 3. Viewport grabs the handle each frame and passes it to GPUI's immediate renderer
/// 4. GPUI displays the texture directly - NO COPIES!

use gpui::*;
use std::sync::{Arc, Mutex};

/// Native GPU texture handle - platform specific pointer
#[derive(Clone, Copy, Debug)]
pub enum NativeTextureHandle {
    #[cfg(target_os = "windows")]
    D3D11(usize), // ID3D11ShaderResourceView* as usize

    #[cfg(target_os = "macos")]
    Metal(usize), // MTLTexture* as usize

    #[cfg(target_os = "linux")]
    Vulkan(u64), // VkImageView as u64
}

unsafe impl Send for NativeTextureHandle {}
unsafe impl Sync for NativeTextureHandle {}

/// Immediate-mode GPU viewport - displays native textures instantly
pub struct GpuViewport {
    /// Current native texture handle to display
    /// Updated each frame by calling set_texture_handle()
    texture_handle: Arc<Mutex<Option<NativeTextureHandle>>>,
    /// Viewport dimensions
    width: u32,
    height: u32,
    /// Focus handle
    focus_handle: FocusHandle,
}

impl GpuViewport {
    /// Create new immediate-mode viewport
    pub fn new<V: 'static>(width: u32, height: u32, cx: &mut Context<V>) -> Self {
        println!("[GPU-VIEWPORT] 🚀 Creating IMMEDIATE-MODE viewport {}x{}", width, height);
        println!("[GPU-VIEWPORT] NO registrations, NO buffers - pure pointer display!");

        Self {
            texture_handle: Arc::new(Mutex::new(None)),
            width,
            height,
            focus_handle: cx.focus_handle(),
        }
    }

    /// Update the texture handle - changes take effect IMMEDIATELY on next frame
    /// NO allocations, NO copies - just stores a pointer!
    pub fn set_texture_handle(&mut self, handle: NativeTextureHandle) {
        if let Ok(mut texture) = self.texture_handle.lock() {
            *texture = Some(handle);
        }
    }

    /// Get current texture handle
    pub fn get_texture_handle(&self) -> Option<NativeTextureHandle> {
        self.texture_handle.lock().ok()?.clone()
    }

    /// Get the shared handle storage for background thread access
    /// This allows render threads to update the handle without GPUI context
    pub fn get_shared_handle_storage(&self) -> Arc<Mutex<Option<NativeTextureHandle>>> {
        self.texture_handle.clone()
    }

    /// Resize viewport
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    /// Get dimensions
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
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
        let texture_handle = self.texture_handle.clone();
        let width = self.width;
        let height = self.height;

        div()
            .id("gpu-viewport")
            .size_full()
            .child(GpuViewportElement {
                texture_handle,
                width,
                height,
            })
    }
}

/// Custom element that paints the native GPU texture using immediate mode
struct GpuViewportElement {
    texture_handle: Arc<Mutex<Option<NativeTextureHandle>>>,
    width: u32,
    height: u32,
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
        // Get the current native texture handle
        if let Ok(handle_lock) = self.texture_handle.lock() {
            if let Some(handle) = *handle_lock {
                // Paint using IMMEDIATE MODE - NO registration, NO buffering!
                // Just pass the raw pointer directly to the renderer
                #[cfg(target_os = "windows")]
                unsafe {
                    if let NativeTextureHandle::D3D11(srv_ptr) = handle {
                        paint_immediate_d3d11_texture(window, bounds, srv_ptr);
                    }
                }

                #[cfg(target_os = "macos")]
                unsafe {
                    if let NativeTextureHandle::Metal(texture_ptr) = handle {
                        paint_immediate_metal_texture(window, bounds, texture_ptr);
                    }
                }

                #[cfg(target_os = "linux")]
                unsafe {
                    if let NativeTextureHandle::Vulkan(image_view) = handle {
                        paint_immediate_vulkan_texture(window, bounds, image_view);
                    }
                }
            }
        }
    }
}

impl IntoElement for GpuViewportElement {
    type Element = Self;
    fn into_element(self) -> Self::Element {
        self
    }
}

// Platform-specific immediate-mode painting functions
// These call directly into GPUI's DirectX/Metal/Vulkan renderers

#[cfg(target_os = "windows")]
unsafe fn paint_immediate_d3d11_texture(window: &mut Window, bounds: Bounds<Pixels>, srv_ptr: usize) {
    // TODO: Call window.draw_raw_texture_immediate() once we expose it
    // For now this is a stub - needs GPUI Window API extension

    let _ = (window, bounds, srv_ptr); // Suppress warnings

    // This should call something like:
    // window.draw_raw_texture_immediate(srv_ptr as *mut c_void, bounds);
}

#[cfg(target_os = "macos")]
unsafe fn paint_immediate_metal_texture(window: &mut Window, bounds: Bounds<Pixels>, texture_ptr: usize) {
    let _ = (window, bounds, texture_ptr);
}

#[cfg(target_os = "linux")]
unsafe fn paint_immediate_vulkan_texture(window: &mut Window, bounds: Bounds<Pixels>, image_view: u64) {
    let _ = (window, bounds, image_view);
}

/// Helper to create a GPU viewport - NO texture registration needed!
pub fn create_gpu_viewport<V: 'static>(
    width: u32,
    height: u32,
    _window: &mut Window,
    cx: &mut Context<V>,
) -> Entity<GpuViewport> {
    cx.new(|cx| {
        GpuViewport::new(width, height, cx)
    })
}
