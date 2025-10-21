//! Helper to create wgpu textures from DXGI shared resources
//!
//! This allows Bevy to render to our pre-created shared textures

use anyhow::{Context, Result};

#[cfg(target_os = "windows")]
pub mod windows_impl {
    use super::*;
    use crate::subsystems::render::dxgi_shared_texture::DxgiSharedTexture;
    use wgpu_hal::api::Dx12;

    /// Create a wgpu::Texture from a DXGI shared D3D12 resource
    /// 
    /// # Safety
    /// The resource must be valid and remain alive for the lifetime of the texture
    pub unsafe fn create_wgpu_texture_from_shared_resource(
        device: &wgpu::Device,
        shared_tex: &DxgiSharedTexture,
    ) -> Result<wgpu::Texture> {
        // Create wgpu texture descriptor
        let desc = wgpu::TextureDescriptor {
            label: Some("DXGI Shared Render Target"),
            size: wgpu::Extent3d {
                width: shared_tex.width,
                height: shared_tex.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Bgra8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };

        // Clone the resource so wgpu can own it
        let resource_clone = shared_tex.dx12_resource.clone();

        // Get the HAL device to use texture_from_raw
        let hal_texture = device.as_hal::<Dx12, _, _>(|hal_device| {
            hal_device.map(|dev| {
                // Use the device's texture_from_raw method
                dev.texture_from_raw(
                    resource_clone,
                    wgpu_types::TextureFormat::Bgra8UnormSrgb,
                    wgpu_types::TextureDimension::D2,
                    wgpu_types::Extent3d {
                        width: shared_tex.width,
                        height: shared_tex.height,
                        depth_or_array_layers: 1,
                    },
                    1, // mip levels
                )
            })
        }).ok_or_else(|| anyhow::anyhow!("Failed to get HAL device"))?;

        // Wrap in wgpu::Texture
        let texture = device.create_texture_from_hal::<Dx12>(
            hal_texture,
            &desc,
        );

        println!("[WGPU-WRAPPER] ✅ Created wgpu texture from DXGI shared resource");

        Ok(texture)
    }
}

#[cfg(target_os = "windows")]
pub use windows_impl::*;
