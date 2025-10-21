// Rendering subsystem for Pulsar Engine Backend

pub mod bevy_renderer;
pub mod wgpu_renderer;
pub mod zero_copy_buffer;
pub mod shared_gpu_texture;
pub mod native_texture;

pub use wgpu_renderer::WgpuRenderer;
pub use bevy_renderer::{BevyRenderer, RenderMetrics, CameraInput};
pub use zero_copy_buffer::{ZeroCopyFrameBuffer, PersistentMappedBuffer};
pub use shared_gpu_texture::{SharedGpuTexture, SharedTextureFormat, GpuSyncPrimitive};
pub use native_texture::{NativeTextureHandle, SharedTextureInfo, TextureFormat};

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
