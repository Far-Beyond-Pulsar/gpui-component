#![cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Direct3D12::*;
use windows_sys::Win32::Graphics::Dxgi::Common::*;

pub unsafe fn create_texture2d(device: ID3D12Device10, width: u32, height: u32, format: DXGI_FORMAT, initial_state: D3D12_RESOURCE_STATES) -> ID3D12Resource {
    let heap = D3D12_HEAP_PROPERTIES { Type: D3D12_HEAP_TYPE_DEFAULT, ..core::mem::zeroed() };
    let desc = D3D12_RESOURCE_DESC { Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D, Width: width as u64, Height: height, DepthOrArraySize: 1, MipLevels: 1, Format: format, SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 }, Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN, Flags: D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS, Alignment: 0 };
    let mut res: ID3D12Resource = 0; (*device).CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &desc, initial_state, core::ptr::null(), &ID3D12Resource::IID, &mut res as *mut _ as *mut _);
    res
}
