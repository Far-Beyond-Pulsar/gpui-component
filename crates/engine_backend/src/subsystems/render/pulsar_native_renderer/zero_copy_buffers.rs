#![cfg(target_os = "windows")]
use windows_sys::Win32::Graphics::Direct3D12::*;
use windows_sys::Win32::Graphics::Dxgi::*;

pub fn detect_resizable_bar_dxgi(_adapter: IDXGIAdapter4) -> bool {
    // No generic DXGI flag; allow env var override for now.
    matches!(std::env::var("PULSAR_RESIZABLE_BAR").ok().as_deref(), Some("1"))
}

pub fn rbar_chunk_bytes(rbar_enabled: bool) -> usize { if rbar_enabled { 64<<20 } else { 8<<20 } }

pub unsafe fn create_upload_buffer(device: ID3D12Device10, bytes: usize) -> ID3D12Resource {
    let heap = D3D12_HEAP_PROPERTIES { Type: D3D12_HEAP_TYPE_UPLOAD, ..core::mem::zeroed() };
    let desc = D3D12_RESOURCE_DESC { Dimension: D3D12_RESOURCE_DIMENSION_BUFFER, Width: bytes as u64, Height: 1, DepthOrArraySize: 1, MipLevels: 1, Format: DXGI_FORMAT_UNKNOWN, SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 }, Layout: D3D12_TEXTURE_LAYOUT_ROW_MAJOR, Flags: D3D12_RESOURCE_FLAG_NONE, Alignment: 0 };
    let mut res: ID3D12Resource = 0; (*device).CreateCommittedResource(&heap, D3D12_HEAP_FLAG_NONE, &desc, D3D12_RESOURCE_STATE_GENERIC_READ, core::ptr::null(), &ID3D12Resource::IID, &mut res as *mut _ as *mut _);
    res
}