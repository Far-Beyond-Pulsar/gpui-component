#![cfg(target_os = "windows")]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum GfxApi { Dx12, Dx11, Vulkan, OpenGL }

pub struct PresentParams { pub width: u32, pub height: u32 }

pub trait Backend {
    unsafe fn resize(&mut self, w: u32, h: u32);
    unsafe fn render(&mut self);
}

// A simple factory to pick best API available on this machine (DX12 -> Vulkan -> DX11 -> OpenGL)
pub unsafe fn pick_api(dx12_ok: bool, vk_ok: bool, dx11_ok: bool, gl_ok: bool) -> GfxApi {
    if dx12_ok { return GfxApi::Dx12; }
    if vk_ok { return GfxApi::Vulkan; }
    if dx11_ok { return GfxApi::Dx11; }
    if gl_ok { return GfxApi::OpenGL; }
    GfxApi::Dx11 // last-ditch default
}