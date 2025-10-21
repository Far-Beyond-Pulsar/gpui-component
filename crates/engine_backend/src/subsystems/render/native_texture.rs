// Cross-platform native texture handle extraction from wgpu
// This allows GPUI to directly display Bevy's GPU textures with ZERO COPIES
//
// Architecture:
// - Windows: wgpu Texture → ID3D11Texture2D handle
// - macOS: wgpu Texture → MTLTexture handle
// - Linux: wgpu Texture → VkImage handle
//
// NO copying, NO staging buffers - pure pointer casting!

use bevy::render::{
    render_resource::Texture,
    renderer::RenderDevice,
};

/// Native GPU texture handle - platform-specific
#[derive(Clone, Copy, Debug)]
pub enum NativeTextureHandle {
    #[cfg(target_os = "windows")]
    D3D11(usize), // ID3D11Texture2D* as usize

    #[cfg(target_os = "macos")]
    Metal(usize), // MTLTexture* as usize

    #[cfg(target_os = "linux")]
    Vulkan(u64), // VkImage as u64

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
    Unsupported,
}

impl NativeTextureHandle {
    /// Extract native handle from Bevy's wgpu texture
    /// This is UNSAFE but required for zero-copy integration with GPUI
    pub unsafe fn from_wgpu_texture(
        _texture: &Texture,
        _device: &RenderDevice,
    ) -> Option<Self> {
        // TODO: Use wgpu's HAL to extract native handles
        // For now, return None until we implement HAL access

        #[cfg(target_os = "windows")]
        {
            // Need to access wgpu's HAL layer:
            // let hal_texture = texture.as_hal::<wgpu::hal::dx11::Api>();
            // let dx11_ptr = hal_texture.unwrap().raw_resource();
            // Some(NativeTextureHandle::D3D11(dx11_ptr as usize))
            None
        }

        #[cfg(target_os = "macos")]
        {
            // let hal_texture = texture.as_hal::<wgpu::hal::metal::Api>();
            // let metal_ptr = hal_texture.unwrap().raw_texture();
            // Some(NativeTextureHandle::Metal(metal_ptr as usize))
            None
        }

        #[cfg(target_os = "linux")]
        {
            // let hal_texture = texture.as_hal::<wgpu::hal::vulkan::Api>();
            // let vk_image = hal_texture.unwrap().raw_image();
            // Some(NativeTextureHandle::Vulkan(vk_image))
            None
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    /// Convert to raw pointer for GPUI integration
    #[cfg(target_os = "windows")]
    pub unsafe fn as_d3d11_ptr(self) -> Option<*mut std::ffi::c_void> {
        match self {
            NativeTextureHandle::D3D11(ptr) => Some(ptr as *mut std::ffi::c_void),
            _ => None,
        }
    }

    #[cfg(target_os = "macos")]
    pub unsafe fn as_metal_ptr(self) -> Option<*mut std::ffi::c_void> {
        match self {
            NativeTextureHandle::Metal(ptr) => Some(ptr as *mut std::ffi::c_void),
            _ => None,
        }
    }

    #[cfg(target_os = "linux")]
    pub unsafe fn as_vulkan_image(self) -> Option<u64> {
        match self {
            NativeTextureHandle::Vulkan(img) => Some(img),
            _ => None,
        }
    }
}

/// Information about a shared texture for GPUI integration
#[derive(Clone, Debug)]
pub struct SharedTextureInfo {
    /// Native GPU handle
    pub handle: NativeTextureHandle,
    /// Texture dimensions
    pub width: u32,
    pub height: u32,
    /// Format (BGRA8UnormSrgb)
    pub format: TextureFormat,
}

#[derive(Clone, Copy, Debug)]
pub enum TextureFormat {
    Bgra8UnormSrgb,
    Rgba8UnormSrgb,
}

impl SharedTextureInfo {
    /// Create from Bevy texture
    pub unsafe fn from_bevy_texture(
        texture: &Texture,
        device: &RenderDevice,
        width: u32,
        height: u32,
    ) -> Option<Self> {
        let handle = NativeTextureHandle::from_wgpu_texture(texture, device)?;

        Some(Self {
            handle,
            width,
            height,
            format: TextureFormat::Bgra8UnormSrgb,
        })
    }
}
