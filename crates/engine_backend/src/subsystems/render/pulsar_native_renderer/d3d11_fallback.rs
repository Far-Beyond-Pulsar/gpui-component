// =============================
// file: d3d11_fallback.rs
// =============================
#![cfg(target_os = "windows")]
#![allow(non_snake_case)]
use windows_sys::Win32::{
    Foundation::HWND,
    Graphics::{
        Direct3D::*,
        Direct3D11::*,
        Dxgi::Common::*,
        Dxgi::*,
    },
};

pub struct Dx11Renderer {
    pub device: ID3D11Device, pub ctx: ID3D11DeviceContext,
    pub swapchain: IDXGISwapChain, pub rtv: ID3D11RenderTargetView,
    pub width: u32, pub height: u32,
}

unsafe fn check(hr: i32) { if hr < 0 { panic!("HRESULT 0x{:08x}", hr as u32); } }

impl Dx11Renderer {
    pub unsafe fn new(hwnd: HWND, width: u32, height: u32) -> Self {
        let mut sc_desc = DXGI_SWAP_CHAIN_DESC {
            BufferDesc: DXGI_MODE_DESC { Width: width, Height: height, RefreshRate: DXGI_RATIONAL { Numerator: 0, Denominator: 1 }, Format: DXGI_FORMAT_R8G8B8A8_UNORM, ScanlineOrdering: 0, Scaling: 0 },
            SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            OutputWindow: hwnd,
            Windowed: 1,
            SwapEffect: DXGI_SWAP_EFFECT_DISCARD,
            Flags: 0,
        };
        let mut dev: ID3D11Device = 0; let mut ctx: ID3D11DeviceContext = 0; let mut sc: IDXGISwapChain = 0;
        check(D3D11CreateDeviceAndSwapChain(
            0, D3D_DRIVER_TYPE_HARDWARE, 0, D3D11_CREATE_DEVICE_BGRA_SUPPORT,
            [D3D_FEATURE_LEVEL_11_1, D3D_FEATURE_LEVEL_11_0].as_ptr(), 2,
            D3D11_SDK_VERSION, &mut sc_desc, &mut sc, &mut dev, core::ptr::null_mut(), &mut ctx
        ));
        let mut back: ID3D11Texture2D = 0; check((*sc).GetBuffer(0, &ID3D11Texture2D::IID, &mut back as *mut _ as *mut _));
        let mut rtv: ID3D11RenderTargetView = 0; check((*dev).CreateRenderTargetView(back as _, core::ptr::null(), &mut rtv as *mut _));
        Self { device: dev, ctx: ctx, swapchain: sc, rtv, width, height }
    }

    pub unsafe fn resize(&mut self, w: u32, h: u32) {
        self.width = w; self.height = h;
        (*self.swapchain).ResizeBuffers(0, w, h, DXGI_FORMAT_UNKNOWN, 0);
        let mut back: ID3D11Texture2D = 0; (*self.swapchain).GetBuffer(0, &ID3D11Texture2D::IID, &mut back as *mut _ as *mut _);
        let mut rtv: ID3D11RenderTargetView = 0; (*self.device).CreateRenderTargetView(back as _, core::ptr::null(), &mut rtv as *mut _);
        self.rtv = rtv;
    }

    pub unsafe fn render(&mut self) {
        let clear = [0.1, 0.1, 0.12, 1.0];
        (*self.ctx).OMSetRenderTargets(1, &self.rtv, 0);
        (*self.ctx).ClearRenderTargetView(self.rtv, clear.as_ptr());
        (*self.swapchain).Present(0, 0);
    }
}
