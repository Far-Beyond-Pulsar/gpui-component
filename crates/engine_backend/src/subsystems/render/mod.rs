// Rendering subsystem for Pulsar Engine Backend

pub mod bevy_renderer;
pub mod zero_copy_buffer;
pub mod native_texture;
pub mod dxgi_shared_texture;

pub use bevy_renderer::{BevyRenderer, CameraInput, RenderMetrics, GpuProfilerData};
pub use zero_copy_buffer::{ZeroCopyFrameBuffer, PersistentMappedBuffer};
pub use native_texture::{NativeTextureHandle, SharedTextureInfo, TextureFormat};
pub use dxgi_shared_texture::*;

// Optional Pulsar_Native renderer (only compiled when feature is present)
#[cfg(feature = "pulsar_native_renderer")]
pub mod pulsar_native_renderer {
    // Reuse the native DX12/Vulkan backend from this crate \
    pub use crate::renderer as native;
    pub use crate::win_host;

    pub fn init_for_testing() {
        unsafe {
            let hwnd = win_host::create_window(1280, 720, "Pulsar_Native Test");
            let mut r = native::Renderer::new(hwnd, 1280, 720, 4);
            while win_host::pump_messages() {
                r.render_frame(|_, _| {});
            }
        }
    }
}

// Stub for compatibility
pub struct WgpuRenderer;

// Re-export common types
pub struct Framebuffer {
    pub width: u32,
    pub height: u32,
    pub buffer: Vec<u8>,
}

impl Framebuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let buffer_size = (width * height * 4) as usize;
        Self {
            width,
            height,
            buffer: vec![0; buffer_size],
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
        let buffer_size = (width * height * 4) as usize;
        self.buffer.resize(buffer_size, 0);
    }

    pub fn clear(&mut self, color: [u8; 4]) {
        for chunk in self.buffer.chunks_exact_mut(4) {
            chunk.copy_from_slice(&color);
        }
    }
}

fn should_use_pulsar_from_exec() -> bool {
    // Env override first
    if std::env::var("PULSAR_RENDERER").map(|v| v == "1" || v.eq_ignore_ascii_case("true") || v.eq_ignore_ascii_case("pulsar")).unwrap_or(false) {
        return true;
    }

    // CLI Flags: --renderer=pulsar | --pulsar-native
    for arg in std::env::args() {
        let a = arg.to_ascii_lowercase();
        if a == "--pulsar-native" || a == "--renderer=pulsar" || a == "--renderer=pulsar_native" { return true; }
    }
    false
}