#![cfg(target_os = "windows")]
#![allow(non_snake_case)]
use core::ffi::c_void;
use windows_sys::Win32::Graphics::Direct3D12::*;

pub struct DxrState { pub rtso: ID3D12StateObject, pub sbt: ID3D12Resource }

pub unsafe fn create_minimal_dxr(device: ID3D12Device10) -> DxrState {
    // Fill subobjects for a tiny RT pipeline (empty placeholders).
    let desc = D3D12_STATE_OBJECT_DESC { Type: D3D12_STATE_OBJECT_TYPE_RAYTRACING_PIPELINE, NumSubobjects: 0, pSubobjects: core::ptr::null() };
    let mut rtso: ID3D12StateObject = 0; (*device).CreateStateObject(&desc, &ID3D12StateObject::IID, &mut rtso as *mut _ as *mut _);
    let mut sbt: ID3D12Resource = 0; // allocate later (upload/default resource with shader identifiers)
    DxrState { rtso, sbt }
}

pub unsafe fn dispatch(list: ID3D12GraphicsCommandList7, rtso: ID3D12StateObject, w: u32, h: u32) {
    let mut pso: ID3D12StateObjectProperties = 0; (*rtso).QueryInterface(&ID3D12StateObjectProperties::IID, &mut pso as *mut _ as *mut _);
    let desc = D3D12_DISPATCH_RAYS_DESC { Width: w, Height: h, Depth: 1, ..core::mem::zeroed() };
    (*list).SetPipelineState1(rtso);
    (*list).DispatchRays(&desc);
}