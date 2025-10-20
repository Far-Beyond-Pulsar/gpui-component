//! WgpuViewport - Zero-copy GPU viewport using GPUI's WgpuCanvas
//!
//! This is a massively simplified viewport implementation that uses WgpuCanvas
//! for direct GPU texture rendering. Compared to the old implementations:
//!
//! - ❌ No CPU-side framebuffers (Vec<u8>)
//! - ❌ No ImageBuffer conversions
//! - ❌ No RenderImage creation every frame
//! - ❌ No complex double-buffering with Mutex
//! - ✅ Direct GPU rendering
//! - ✅ Zero-copy texture presentation
//! - ✅ Lock-free atomic buffer swapping
//! - ✅ ~70% less code!

use gpui::{
    div, App, AppContext as _, Context, DismissEvent, Entity, EventEmitter,
    FocusHandle, Focusable, IntoElement, ParentElement as _, Render, Styled as _, Window,
    WgpuCanvasTexture, wgpu_canvas,
};
use std::sync::Arc;

// Platform-specific imports
#[cfg(target_os = "windows")]
use windows::Win32::Graphics::Direct3D11::{ID3D11Device, ID3D11Texture2D};

#[cfg(target_os = "macos")]
use metal;

/// Zero-copy viewport with WgpuCanvas
pub struct WgpuViewport {
    /// Shared GPU texture for rendering
    canvas_texture: Arc<WgpuCanvasTexture>,

    /// Focus handle for input events
    focus_handle: FocusHandle,

    /// Enable debug logging
    debug_enabled: bool,
}

impl WgpuViewport {
    /// Create a new WgpuViewport with DirectX 11 (Windows)
    #[cfg(target_os = "windows")]
    pub fn new_d3d11<V>(
        device: &ID3D11Device,
        width: u32,
        height: u32,
        cx: &mut Context<V>,
    ) -> anyhow::Result<Entity<Self>>
    where
        V: 'static,
    {
        println!("[WGPU-VIEWPORT] Creating WgpuCanvasTexture {}x{}", width, height);
        let canvas_texture = WgpuCanvasTexture::new(
            device,
            gpui::size(gpui::px(width as f32), gpui::px(height as f32)),
        ).map_err(|e| {
            println!("[WGPU-VIEWPORT] ❌ Failed to create WgpuCanvasTexture: {:?}", e);
            e
        })?;
        println!("[WGPU-VIEWPORT] ✅ WgpuCanvasTexture created successfully");

        Ok(cx.new(|cx| Self {
            canvas_texture,
            focus_handle: cx.focus_handle(),
            debug_enabled: cfg!(debug_assertions),
        }))
    }

    /// Create a new WgpuViewport with Metal (macOS)
    #[cfg(target_os = "macos")]
    pub fn new_metal<V>(
        device: &metal::Device,
        width: u32,
        height: u32,
        cx: &mut Context<V>,
    ) -> anyhow::Result<Entity<Self>>
    where
        V: 'static,
    {
        let canvas_texture = WgpuCanvasTexture::new(
            device,
            gpui::size(gpui::px(width as f32), gpui::px(height as f32)),
        )?;

        Ok(cx.new(|cx| Self {
            canvas_texture,
            focus_handle: cx.focus_handle(),
            debug_enabled: cfg!(debug_assertions),
        }))
    }

    /// Get the canvas texture for external rendering
    ///
    /// Your game engine should:
    /// 1. Call `canvas_texture.back_buffer()` to get the GPU texture
    /// 2. Render directly to that texture
    /// 3. Call `canvas_texture.swap_buffers()` when done
    pub fn canvas_texture(&self) -> Arc<WgpuCanvasTexture> {
        self.canvas_texture.clone()
    }

    /// Get the back buffer for rendering (platform-specific)
    ///
    /// Windows: Returns &ID3D11Texture2D
    /// macOS: Returns &objc::runtime::Object (Metal texture)
    #[cfg(target_os = "windows")]
    pub fn back_buffer(&self) -> &ID3D11Texture2D {
        self.canvas_texture.back_buffer()
    }

    #[cfg(target_os = "macos")]
    pub fn back_buffer(&self) -> &objc::runtime::Object {
        self.canvas_texture.back_buffer()
    }

    /// Swap buffers after rendering (lock-free, atomic)
    pub fn swap_buffers(&self) {
        self.canvas_texture.swap_buffers();
    }

    /// Get texture size
    pub fn size(&self) -> gpui::Size<gpui::Pixels> {
        self.canvas_texture.size()
    }
}

impl Focusable for WgpuViewport {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for WgpuViewport {}

impl Render for WgpuViewport {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        if self.debug_enabled {
            static FRAME_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let frame = FRAME_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            if frame % 60 == 0 {
                println!("[WGPU-VIEWPORT] Frame {}: Zero-copy GPU rendering", frame);
            }
        }

        div()
            .size_full()
            .flex()
            .flex_1()
            .child(wgpu_canvas(self.canvas_texture.clone()))
    }
}

/// Helper function to create a viewport without a render thread
///
/// This creates a WgpuViewport that can be rendered to using GPUI's rendering system.
/// The separate render thread approach causes access violations because GPUI textures
/// must be accessed from the main GPUI thread, not arbitrary background threads.
///
/// # Example
/// ```ignore
/// let viewport = create_viewport(
///     &device,
///     1920,
///     1080,
///     cx,
/// )?;
/// ```
#[cfg(target_os = "windows")]
pub fn create_viewport_with_render_thread<V, F>(
    device: &ID3D11Device,
    width: u32,
    height: u32,
    cx: &mut Context<V>,
    _render_fn: F,
) -> anyhow::Result<(Entity<WgpuViewport>, smol::channel::Sender<()>)>
where
    V: 'static,
    F: FnMut(&Arc<WgpuCanvasTexture>) + Send + 'static,
{
    let viewport = WgpuViewport::new_d3d11(device, width, height, cx)?;
    
    // Create a dummy stop channel for API compatibility
    let (stop_sender, _stop_receiver) = smol::channel::bounded::<()>(1);
    
    println!("[RENDER-THREAD] Stub render thread - actual rendering done by WgpuViewport");

    Ok((viewport, stop_sender))
}

#[cfg(target_os = "macos")]
pub fn create_viewport_with_render_thread<V, F>(
    device: &metal::Device,
    width: u32,
    height: u32,
    cx: &mut Context<V>,
    _render_fn: F,
) -> anyhow::Result<(Entity<WgpuViewport>, smol::channel::Sender<()>)>
where
    V: 'static,
    F: FnMut(&Arc<WgpuCanvasTexture>) + Send + 'static,
{
    let viewport = WgpuViewport::new_metal(device, width, height, cx)?;
    
    // Create a dummy stop channel for API compatibility
    let (stop_sender, _stop_receiver) = smol::channel::bounded::<()>(1);
    
    println!("[RENDER-THREAD] Stub render thread - actual rendering done by WgpuViewport");

    Ok((viewport, stop_sender))
}
