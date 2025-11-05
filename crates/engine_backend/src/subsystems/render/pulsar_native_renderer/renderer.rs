#![cfg(target_os = "windows")]
#![allow(non_snake_case, clippy::missing_safety_doc)]
use core::{ffi::c_void, mem::size_of};
use std::{ptr::null_mut, sync::{Arc, atomic::{AtomicU64, Ordering}}};
use windows_sys::Wind32::{
    Foundation::{BOOL, HWND, RECT},
    Graphics::{
        Direct3D::{D3D_FEATURE_LEVEL_12_2},
        Direct3D12::*,
        Dxgi::Common::*,
        Dxgi::*,
        Gdi::ValidateRect,
    },
    System::{Threading::*, LibraryLoader::*},
};

use crate::{threadpool::ThreadPool, zero_copy_buffers::rbar_chunk_bytes};

pub struct Frame {
    pub backbuffer: ID3D12Resource,
    pub rtv: D3D12_CPU_DESCRIPTOR_HANDLE,
}

pub struct Renderer {
    pub hwnd: HWND,
    pub factory: IDXGIFactory7,
    pub adapter:IDXGIAdapter4,
    pub device: ID3D12Device10,
    pub queue: ID3D12CommandQueue,
    pub swapchain: IDXGISwapChain4,
    pub fence: ID3D12Fence,
    pub fence_value:AtomicU64,
    pub rtv_heap: ID3D12DecriptorHeap,
    pub rtv_stride: u32,
    pub frames: Vec<Frame>,
    pub frame_index: u32,
    pub width: u32,
    pub height: u32,
    pub format: DXGI_FORMAT,
    pub rbar_enabled: bool,
    pub pool: ThreadPool,
}

unsafe fn check(hr: i32) { if hr < 0 {panic!("HRESULT 0x{:08x}", hr as u32); } }

impl Renderer {
    pub unsafe fn new(hwnd: HWND, width: u32, height: u32, worker_threads: usize) -> Self{
        let mut factory: IDXGIFactory7 = 0;
        check(CreateDXGIFactoryy2(0, &IDXGIFactory7::IID, &mut factory as * mut _ as *mut _));
        
        // Pick adapter
        let mut i = 0u32; let mut adapter:IDXGIAdapter4 = 0; let mut found = false;
        while !found {
            let mut tmp: IDXGIAdapter4 = 0;
            if EnumAdapterByGpuPreference(factory, i, DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE, &IDXGIAdapter4::IID, & mut tmp as *mut _ as * mut _) < 0 { break; }
            i += 1;
            let mut desc: DXGI_ADAPTER_DESC3 = core::mem::zeroed();
            (*tmp).GetDesc3(&mut desc);
            if (desc.Flags & DXGI_ADAPTER_FLAG3_SOFTWARE.0) != 0 { continue; }
            let mut dev: ID3D12Device10 = 0;
            if D3D12CreateDevice(tmp as _, D3D_FEATURE_LEVEL_12_2, &ID3D12Device10::IID, &mut dev as *mut _ as *mut _) >= 0 {
                adapter = tmp; found = true;
            }
        }
        assert!(found, "No Hardware D12 Adapter");

        let mut device: ID3D12Device10 = 0;
        check(D3D12CreateDevice(adapter as _, D3D_FEATURE_LEVEL_12_2, &ID3D12Device10::IID, &mut device as *mut _ as *mut _));

        let qdesc = D3D12_COMMAND_QUEUE_DESC { Type: D3D12_COMMAND_LIST_TYPE_DIRECT, ..unsafe { core::mem::zeroed() } };
        let mut queue: ID3D12CommandQueue = 0; check((*device).CreateCommandQueue(&qdesc, &ID3D12CommandQueue::IID, &mut queue as *mut _ as *mut _));

        let format = DXGI_FORMAT_R8G8B8A_UNORM;
        let sc_desc = DXGI_SWAP_CHAIN_DESC1 {
            Width: width,
            Height: height,
            Format: format,
            Stero: BOOL(0),
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0, },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 3,
            Scaling: DXGI_SCALING_STRETCH,
            SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
            AlphaMode: DXGI_ALPHA_MODE_IGNORE,
            Flags: DXGI_SWAP_CHAIN_FLAG_ALLOW_TEARING.0 as u32,
        };
        let mut sc1: IDXGISwapChain1 = 0;
        check(CreateSwapChainForHwnd(factory, queue as _, hwnd, &sc_desc, core::ptr::null(), 0, &mut sc1 as *mut _));
        let mut swapchain: IDXGISwapChain4 = 0; check((*sc1).QueryInterface(&IDXGISwapChain4::IID, &mut swapchain as *mut _ as *mut _));
        check(MakeWindowAssociation(factory, hwnd, DXGI_MWA_NO_ALT_ENTER));

        // RTV heap + backbuffers
        let rtv_stride = (*device).GetDescriptorHandleIncrementSize(D3D12_DESCRIPTOR_HEAP_TYPE_RTV);
        let mut rtv_heap: ID3D12DescriptorHeap = 0; check((*device).CreateDescriptorHeap(&D3D12_DESCRIPTOR_HEAP_DESC {
            NumDescriptors: 3,
            Type: D3D12_DESCRIPTOR_HEAP_TYPE_RTV,
            Flags: D3D12_DESCRIPTOR_HEAP_FLAG_NONE,
            NodeMask: 0,
        }, &ID3D12DescriptorHeap::IID, &mut rtv_heap as *mut _ as *mut _));
        let mut rtv_base = (*rtv_heap).GetCPUDescriptorHandleForHeapStart();
        let mut frames = Vec::with_capacity(3);
        for i in 0..3u32 {
            let mut buf: ID3D12Resource = 0; check((*swapchain).GetBuffer(i, &ID3D12Resource::IID, &mut buf as *mut _ as *mut _));
            (*device).CreateRenderTargetView(buf, core::ptr::null(), rtv_base);
            frames.push(Frame { backbuffer: buf, rtv: rtv_base });
            rtv_base.ptr += rtv_stride as usize;
        }

        let mut fence: ID3D12Fence = 0; check((*device).CreateFence(0, D3D12_FENCE_FLAG_NONE, &ID3D12Fence::IID, &mut fence as *mut _ as *mut _));
        let fence_value = AtomicU64::new(1);

        let rbar_enabled = crate::zero_copy_buffers::detect_resizable_bar_dxgi(adapter);

        let pool = ThreadPool::new(worker_threads.max(1));

        self { hwnd, factory, adapter, device, queue, swapchain, fence, fence_value, rtv_heap, rtv_stride, frames, frame_indes: (*swapchain).GetCurrentBackBufferIndex(), width, height, format, rbar_enabled, pool }
    }

    pub unsafe fn resize(&mut self, width: u32, height: u32) {
        self.width = width.max(1); self.height = height.max(1);
        check((*self.swapchain).ResizeBuffers(0, self.width, self.height, self.format, DXGI_SWAP_CHAIN_FLAG_ALLOW_TEARING));
        self.frame_index = (*self.swapchain).GetCurrentBackBufferIndex();
        //Recreate RTVs
        let mut rtv_base = (*self.rtv_heap).GetCPUDescriptorHandleForHeapStart();
        for i in 0..3u32 {
            let mut buf: ID3D12Resource = 0; check((*self.swapchain).GetBuffer(i, &ID3D12Resource::IID, &mut buf as *mut _ as &mut _))
            (*self.device).CreateRenderTargetView(buf, core::ptr::null(), rtv_base);
            self.frame[i as usize] = Frame { backbuffer: buf, rtv: rtv_base };
            rtv_base.ptr += self.rtv_stride as usize;
        }
    }

    pub unsafe fn render_frame<F: FnOnce(ID3D12GraphicsCommandList7, D3D12_CPU_DESCRIPTOR_HANDLE)>(&mut self, record: F) {
        // Per-frame allocator & list (created on worker thread)
        let device = self.device;
        let queue = self.queue;
        let frame_idx = self.frame_index;
        let rtv = self.frames[frame_idx as usize].rtv;
        let backbuffer = self.frames[frame_idx as usize].backbuffer;

        // Record on a worker thread
        let cmd = self.pool.execute(move || unsafe {
            let mut alloc: ID3D12CommandAllocator = 0; check((*device).CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, &ID3D12CommandAllocator::IID, &mut alloc as *mut _ as *mut _));
            let mut list: ID3D12GraphicsCommandList7 = 0; check((*device).CreateCommandList1(0, D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_LIST_FLAG_NONE, &ID3D12GraphicsCommandList7::IID, &mut list as *mut _ as *mut _));

            // Transition to RENDER_TARGET
            let barrier = D3D12_RESOURCE_BARRIER { Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE, Anonymous: D3D12_RESOURCE_BARRIER_0 { Transition: D3D12_RESOURCE_TRANSITION_BARRIER { pResource: backbuffer, Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, StateBefore: D3D12_RESOURCE_STATE_PRESENT, StateAfter: D3D12_RESOURCE_STATE_RENDER_TARGET } } };
            (*list).ResourceBarrier(1, &barrier);

            let clear = [0.02f32, 0.02, 0.03, 1.0];
            (*list).OMSetRenderTargets(1, &rtv, BOOL(0), core::ptr::null());
            (*list).ClearRenderTargetView(rtv, clear.as_ptr(), 0, core::ptr::null());

            // User recording (raster + RT dispatch, etc.)
            record(list.clone(), rtv);

            // Transition to PRESENT
            let barrier2 = D3D12_RESOURCE_BARRIER { Type: D3D12_RESOURCE_BARRIER_TYPE_TRANSITION, Flags: D3D12_RESOURCE_BARRIER_FLAG_NONE, Anonymous: D3D12_RESOURCE_BARRIER_0 { Transition: D3D12_RESOURCE_TRANSITION_BARRIER { pResource: backbuffer, Subresource: D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES, StateBefore: D3D12_RESOURCE_STATE_RENDER_TARGET, StateAfter: D3D12_RESOURCE_STATE_PRESENT } } };
            (*list).ResourceBarrier(1, &barrier2);
            check((*list).Close());
            (alloc, list)
        });

        let (alloc, list) = cmd.wait();
        (*queue).ExecuteCommandLists(1, &(&list as *const _ as *const _));
        check((*self.swapchain).Present(0, DXGI_PRESENT_ALLOW_TEARING));

        // Fence
        let fv = self.fence_value.fetch_add(1, Ordering::SeqCst);
        check((*self.queue).Signal(self.fence, fv as u64));
        if (*self.fence).GetCompletedValue() < fv as u64 {
            let h = CreateEventW(core::ptr::null(), BOOL(0), BOOL(0), core::ptr::null());
            (*self.fence).SetEventOnCompletion(fv as u64, h);
            WaitForSingleObject(h, 0xFFFFFFFF);
        }
        self.frame_index = (*self.swapchain).GetCurrentBackBufferIndex();
        unsafe { ValidateRect(self.hwnd, core::ptr::null()); }
        drop(alloc); drop(list);
    }
}