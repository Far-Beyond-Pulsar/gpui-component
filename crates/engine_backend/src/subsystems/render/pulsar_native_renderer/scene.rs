#![cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Direct3D12::*;
use windows_sys::Win32::Graphics::Dxgi::Common::*;

pub unsafe fn record_raster_triangle(list: ID3D12GraphicsCommandList7, rtv: D3D12_CPU_DESCRIPTOR_HANDLE, format: DXGI_FORMAT) {
    // In a real build, attach VS/PS DXIL and draw a triangle.
    (*list).OMSetRenderTargets(1, &rtv, 0, core::ptr::null());
}