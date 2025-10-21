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
    D3D11(usize), // ID3D11ShaderResourceView* as usize

    #[cfg(target_os = "macos")]
    Metal(usize), // MTLTexture* as usize

    #[cfg(target_os = "linux")]
    Vulkan(u64), // VkImageView as u64
}

impl NativeTextureHandle {
    /// Extract native handle from Bevy's wgpu texture
    /// This is UNSAFE but required for zero-copy integration with GPUI
    pub unsafe fn from_wgpu_texture(
        texture: &Texture,
        _device: &RenderDevice,
    ) -> Option<Self> {
        // Use wgpu's HAL to extract native handles
        // This gives us direct access to the underlying GPU API objects

        #[cfg(target_os = "windows")]
        {
            use wgpu_core::hal_api::HalApi;
            use wgpu_hal::api::Dx12;
            
            // Get the HAL texture for DirectX 12
            let hal_texture = texture.as_hal::<Dx12>();
            if let Some(hal_tex) = hal_texture {
                // Get the raw D3D12 resource
                let raw_tex = hal_tex.as_ref();
                // For GPUI integration, we pass the texture pointer
                let ptr = raw_tex as *const _ as *mut std::ffi::c_void;
                println!("[NATIVE-TEXTURE] ✅ Extracted D3D12 texture: {:p}", ptr);
                Some(NativeTextureHandle::D3D11(ptr as usize))
            } else {
                println!("[NATIVE-TEXTURE] ❌ Failed to get HAL texture for DirectX");
                None
            }
        }

        #[cfg(target_os = "macos")]
        {
            use wgpu_core::hal_api::HalApi;
            use wgpu_hal::api::Metal;
            
            // Get the HAL texture for Metal
            let hal_texture = texture.as_hal::<Metal>();
            if let Some(hal_tex) = hal_texture {
                // Get the raw Metal texture
                let raw_tex = hal_tex.as_ref();
                let ptr = raw_tex as *const _ as *mut std::ffi::c_void;
                println!("[NATIVE-TEXTURE] ✅ Extracted Metal texture: {:p}", ptr);
                Some(NativeTextureHandle::Metal(ptr as usize))
            } else {
                println!("[NATIVE-TEXTURE] ❌ Failed to get HAL texture for Metal");
                None
            }
        }

        #[cfg(target_os = "linux")]
        {
            use wgpu_core::hal_api::HalApi;
            use wgpu_hal::api::Vulkan;
            
            // Get the HAL texture for Vulkan
            let hal_texture = texture.as_hal::<Vulkan>();
            if let Some(hal_tex) = hal_texture {
                // Get the raw Vulkan image
                let raw_tex = hal_tex.as_ref();
                let ptr = raw_tex as *const _ as u64;
                println!("[NATIVE-TEXTURE] ✅ Extracted Vulkan image: 0x{:X}", ptr);
                Some(NativeTextureHandle::Vulkan(ptr))
            } else {
                println!("[NATIVE-TEXTURE] ❌ Failed to get HAL texture for Vulkan");
                None
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            println!("[NATIVE-TEXTURE] ❌ Unsupported platform");
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
