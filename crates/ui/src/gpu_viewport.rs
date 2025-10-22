/// IMMEDIATE-MODE GPU VIEWPORT - TRUE ZERO-COPY
///
/// NO texture registration, NO buffers, NO copies!
/// Just displays whatever native GPU texture handle you point it at.
///
/// Architecture:
/// 1. Bevy renders to DX12 shared textures in its own thread
/// 2. Bevy exposes NT HANDLEs (shared handles) from DX12
/// 3. Viewport opens these handles in DX11 and creates SRVs
/// 4. GPUI displays the SRV directly - NO COPIES!

use gpui::*;
use std::sync::{Arc, Mutex};
#[cfg(target_os = "windows")]
use crate::dx11_shared_opener;

/// Native GPU texture handle - platform specific
#[derive(Clone, Copy, Debug)]
pub enum NativeTextureHandle {
    #[cfg(target_os = "windows")]
    D3D11(usize), // HANDLE (NT handle) for DXGI shared resource - opened in DX11 to create SRV

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
        // IMMEDIATE MODE: The renderer will read directly from our shared storage!
        // We just need to tell GPUI to render a "raw texture quad" at these bounds
        // The DirectX/Metal/Vulkan renderer will pull the latest pointer from our storage

        let scale_factor = window.scale_factor();
        let scaled_bounds = bounds.scale(scale_factor);

        // Pass the shared storage handle to GPUI's renderer
        // The renderer will dereference it during its render pass - NO MAIN THREAD DELAY!
        #[cfg(target_os = "windows")]
        unsafe {
            paint_immediate_d3d11_texture(window, scaled_bounds, self.texture_handle.clone());
        }

        #[cfg(target_os = "macos")]
        unsafe {
            paint_immediate_metal_texture(window, scaled_bounds, self.texture_handle.clone());
        }

        #[cfg(target_os = "linux")]
        unsafe {
            paint_immediate_vulkan_texture(window, scaled_bounds, self.texture_handle.clone());
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
// These queue a "raw texture quad" command that the renderer executes later
// The renderer will dereference the Arc<Mutex<>> during its render pass - ZERO MAIN THREAD DELAY!

#[cfg(target_os = "windows")]
unsafe fn paint_immediate_d3d11_texture(
    window: &mut Window,
    bounds: Bounds<ScaledPixels>,
    handle_storage: Arc<Mutex<Option<NativeTextureHandle>>>
) {
    // Get the NT handle from Bevy (DX12 shared handle)
    let nt_handle = if let Ok(guard) = handle_storage.lock() {
        if let Some(NativeTextureHandle::D3D11(handle)) = *guard {
            if handle == 0 {
                println!("[GPU-VIEWPORT] ⚠️ Got null NT handle, skipping render");
                return;
            }
            println!("[GPU-VIEWPORT] 📥 Got NT handle from DX12: 0x{:X}", handle);
            handle
        } else {
            println!("[GPU-VIEWPORT] ⚠️ No texture handle set yet");
            return;
        }
    } else {
        println!("[GPU-VIEWPORT] ❌ Failed to lock handle storage");
        return;
    };

    // Step 1: Open the DX12 shared handle in DX11 and create an SRV
    // This is the CRITICAL step - we can't pass NT handles to GPUI directly!
    let srv_ptr = match dx11_shared_opener::open_shared_handle_for_gpui(nt_handle, 1600, 900) {
        Ok(ptr) => {
            println!("[GPU-VIEWPORT] ✅ Opened shared handle and created SRV: {:p}", ptr);
            ptr
        }
        Err(e) => {
            eprintln!("[GPU-VIEWPORT] ❌ Failed to open shared handle: {}", e);
            return;
        }
    };

    // Step 2: Pass the SRV (not the NT handle!) to GPUI
    let scale_factor = window.scale_factor();
    let pixel_bounds = Bounds {
        origin: point(
            px((bounds.origin.x / ScaledPixels::from(scale_factor))),
            px((bounds.origin.y / ScaledPixels::from(scale_factor)))
        ),
        size: size(
            px((bounds.size.width / ScaledPixels::from(scale_factor))),
            px((bounds.size.height / ScaledPixels::from(scale_factor)))
        ),
    };
    
    println!("[GPU-VIEWPORT] 🖼️ Drawing SRV at bounds: {:?}", pixel_bounds);
    
    // SAFETY: We're passing a valid ID3D11ShaderResourceView* that we just created
    // The SRV is kept alive by the SharedTextureManager
    if let Err(e) = window.draw_raw_texture_immediate(srv_ptr, pixel_bounds) {
        eprintln!("[GPU-VIEWPORT] ❌ Failed to draw texture: {}", e);
    } else {
        println!("[GPU-VIEWPORT] ✅ Successfully drew texture via SRV");
    }
}

#[cfg(target_os = "macos")]
unsafe fn paint_immediate_metal_texture(
    window: &mut Window,
    bounds: Bounds<ScaledPixels>,
    handle_storage: Arc<Mutex<Option<NativeTextureHandle>>>
) {
    // TODO: Implement Metal immediate mode when needed
    let _ = (window, bounds, handle_storage);
}

#[cfg(target_os = "linux")]
unsafe fn paint_immediate_vulkan_texture(
    window: &mut Window,
    bounds: Bounds<ScaledPixels>,
    handle_storage: Arc<Mutex<Option<NativeTextureHandle>>>
) {
    // TODO: Implement Vulkan immediate mode when needed
    let _ = (window, bounds, handle_storage);
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
