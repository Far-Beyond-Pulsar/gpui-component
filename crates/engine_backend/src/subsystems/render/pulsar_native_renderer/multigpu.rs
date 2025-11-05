#![cfg(target_os = "windows")]
#![allow(non_snake_case)]
use core::mem::size_of;
use std::sync::atomic::{AtomicU64, Ordering};
use windows_sys::Win32::Graphics::{
    Direct3D::D3D_FEATURE_LEVEL_12_1,
    Direct3D12::*,
    Dxgi::Common::*,
    Dxgi::*,
};
use windows_sys::Win32::Foundation::BOOL;

use crate::threadpool::ThreadPool;

pub struct GpuNode {
    pub adapter: IDXGIAdapter4,
    pub device: ID3D12Device10,
    pub queue: ID3D12CommandQueue,
    pub fence: ID3D12Fence,
    pub fence_value: AtomicU64,
    pub rbar_enabled: bool,
}

pub struct MultiGpuRenderer {
    pub factory: IDXGIFactory7,
    pub nodes: Vec<GpuNode>,
    pub primary_index: usize,
    pub pool: ThreadPool,
}

unsafe fn check(hr: i32) { if hr < 0 { panic!("HRESULT 0x{:08x}", hr as u32); } }

impl MultiGpuRenderer {
    /// Enumerate all hardware adapters and make one device/queue per adapter.
    pub unsafe fn enumerate(worker_threads: usize) -> Self {
        let mut factory: IDXGIFactory7 = 0;
        check(CreateDXGIFactory2(0, &IDXGIFactory7::IID, &mut factory as *mut _ as *mut _));

        let mut nodes = Vec::new();
        let mut i = 0u32;
        loop {
            let mut ad: IDXGIAdapter4 = 0;
            if EnumAdapterByGpuPreference(factory, i, DXGI_GPU_PREFERENCE_HIGH_PERFORMANCE, &IDXGIAdapter4::IID, &mut ad as *mut _ as *mut _) < 0 { break; }
            i += 1;
            let mut desc: DXGI_ADAPTER_DESC3 = core::mem::zeroed();
            (*ad).GetDesc3(&mut desc);
            if (desc.Flags & DXGI_ADAPTER_FLAG3_SOFTWARE.0) != 0 { continue; }

            let mut dev: ID3D12Device10 = 0;
            if D3D12CreateDevice(ad as _, D3D_FEATURE_LEVEL_12_1, &ID3D12Device10::IID, &mut dev as *mut _ as *mut _) < 0 { continue; }

            let qdesc = D3D12_COMMAND_QUEUE_DESC { Type: D3D12_COMMAND_LIST_TYPE_DIRECT, ..core::mem::zeroed() };
            let mut queue: ID3D12CommandQueue = 0; check((*dev).CreateCommandQueue(&qdesc, &ID3D12CommandQueue::IID, &mut queue as *mut _ as *mut _));

            let mut fence: ID3D12Fence = 0; check((*dev).CreateFence(0, D3D12_FENCE_FLAG_NONE, &ID3D12Fence::IID, &mut fence as *mut _ as *mut _));

            let rbar_enabled = crate::zero_copy_buffers::detect_resizable_bar_dxgi(ad);

            nodes.push(GpuNode { adapter: ad, device: dev, queue, fence, fence_value: AtomicU64::new(1), rbar_enabled });
        }

        assert!(!nodes.is_empty(), "No hardware adapters found");
        let pool = ThreadPool::new(worker_threads.max(1));
        Self { factory, nodes, primary_index: 0, pool }
    }

    /// Alternate-Frame Rendering: pick a node by frame index and record/execute there.
    /// Results must be visible to the primary for presentation (copy via cross-adapter shared resource).
    pub unsafe fn render_frame_afr<F: FnOnce(ID3D12GraphicsCommandList7) + Send + 'static>(&self, frame_index: u64, record: F) {
        let node_idx = (frame_index as usize) % self.nodes.len();
        let node = &self.nodes[node_idx];
        let device = node.device; let queue = node.queue; let fence = node.fence;

        let task = self.pool.execute(move || unsafe {
            let mut alloc: ID3D12CommandAllocator = 0; check((*device).CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, &ID3D12CommandAllocator::IID, &mut alloc as *mut _ as *mut _));
            let mut list: ID3D12GraphicsCommandList7 = 0; check((*device).CreateCommandList1(0, D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_LIST_FLAG_NONE, &ID3D12GraphicsCommandList7::IID, &mut list as *mut _ as *mut _));
            // User work for this node
            record(list.clone());
            check((*list).Close());
            (*queue).ExecuteCommandLists(1, &(&list as *const _ as *const _));
            (alloc, list)
        });

        let (_alloc, _list) = task.wait();
        let fv = node.fence_value.fetch_add(1, Ordering::SeqCst);
        check((*queue).Signal(fence, fv as u64));
    }

    /// Split-Frame Rendering: split the frame into N tiles, each node renders a tile to its local texture.
    /// Caller must provide per-node color targets and a compose step on primary.
    pub unsafe fn render_frame_sfr<F: Fn(usize, ID3D12GraphicsCommandList7) + Send + Sync + 'static>(&self, tiles: usize, record_tile: F) {
        use std::sync::Arc; let record_arc = Arc::new(record_tile);
        let mut tasks = Vec::new();
        for (i, node) in self.nodes.iter().take(tiles.min(self.nodes.len())).enumerate() {
            let device = node.device; let queue = node.queue; let fence = node.fence; let rec = record_arc.clone();
            tasks.push(self.pool.execute(move || unsafe {
                let mut alloc: ID3D12CommandAllocator = 0; check((*device).CreateCommandAllocator(D3D12_COMMAND_LIST_TYPE_DIRECT, &ID3D12CommandAllocator::IID, &mut alloc as *mut _ as *mut _));
                let mut list: ID3D12GraphicsCommandList7 = 0; check((*device).CreateCommandList1(0, D3D12_COMMAND_LIST_TYPE_DIRECT, D3D12_COMMAND_LIST_FLAG_NONE, &ID3D12GraphicsCommandList7::IID, &mut list as *mut _ as *mut _));
                rec(i, list.clone());
                check((*list).Close());
                (*queue).ExecuteCommandLists(1, &(&list as *const _ as *const _));
                (alloc, list)
            }));
        }
        for (i, t) in tasks.into_iter().enumerate() {
            let (_a,_l) = t.wait();
            let node = &self.nodes[i];
            let fv = node.fence_value.fetch_add(1, Ordering::SeqCst);
            check((*node.queue).Signal(node.fence, fv as u64));
        }
    }

    /// Cross-adapter shared resource creation on the primary device.
    /// Use D3D12_HEAP_FLAG_SHARED | D3D12_HEAP_FLAG_SHARED_CROSS_ADAPTER and
    /// D3D12_RESOURCE_FLAG_ALLOW_CROSS_ADAPTER where applicable.
    pub unsafe fn create_cross_adapter_texture(
        &self,
        primary_device: ID3D12Device10,
        width: u32,
        height: u32,
        format: DXGI_FORMAT,
    ) -> (ID3D12Resource, HANDLE) {
        let heap_props = D3D12_HEAP_PROPERTIES { Type: D3D12_HEAP_TYPE_DEFAULT, ..core::mem::zeroed() };
        let desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Width: width as u64,
            Height: height,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: format,
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
            Flags: D3D12_RESOURCE_FLAG_ALLOW_UNORDERED_ACCESS | D3D12_RESOURCE_FLAG_ALLOW_CROSS_ADAPTER,
            Alignment: 0,
        };
        let mut tex: ID3D12Resource = 0;
        check((*primary_device).CreateCommittedResource(&heap_props, D3D12_HEAP_FLAG_SHARED | D3D12_HEAP_FLAG_SHARED_CROSS_ADAPTER, &desc, D3D12_RESOURCE_STATE_COMMON, core::ptr::null(), &ID3D12Resource::IID, &mut tex as *mut _ as *mut _));
        let mut shared: HANDLE = 0; check((*primary_device).CreateSharedHandle(tex, core::ptr::null(), 0, core::ptr::null(), &mut shared as *mut _));
        (tex, shared)
    }

    /// Open a cross-adapter shared resource on a secondary node.
    pub unsafe fn open_shared_resource(&self, device: ID3D12Device10, handle: HANDLE) -> ID3D12Resource {
        let mut r: ID3D12Resource = 0; check((*device).OpenSharedHandle(handle, &ID3D12Resource::IID, &mut r as *mut _ as *mut _));
        r
    }
}