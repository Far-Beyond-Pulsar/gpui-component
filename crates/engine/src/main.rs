use gpui::*;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::window::{Window as WinitWindow, WindowId};

#[cfg(target_os = "windows")]
use windows::{
    core::*,
    Win32::{
        Foundation::*,
        Graphics::{
            Gdi::*,
            Direct3D::*,
            Direct3D11::*,
            Direct3D::Fxc::*,
            Dxgi::{Common::*, *},
        },
        UI::WindowsAndMessaging::*,
    },
};


fn main() {
    println!("🚀 Starting Winit + GPUI Zero-Copy Composition Demo");
    println!("This demonstrates GPUI rendering to shared texture + winit GPU composition\n");

    let event_loop = EventLoop::new().expect("Failed to create event loop");
    event_loop.set_control_flow(ControlFlow::Poll);

    let mut app = WinitGpuiApp::new();
    event_loop.run_app(&mut app).expect("Failed to run event loop");
}

struct WinitGpuiApp {
    winit_window: Option<Arc<WinitWindow>>,
    gpui_app: Option<Application>,
    gpui_window: Option<WindowHandle<DemoView>>,
    gpui_window_initialized: bool,
    #[cfg(target_os = "windows")]
    d3d_device: Option<ID3D11Device>,
    #[cfg(target_os = "windows")]
    d3d_context: Option<ID3D11DeviceContext>,
    #[cfg(target_os = "windows")]
    shared_texture: Option<ID3D11Texture2D>,
    #[cfg(target_os = "windows")]
    shared_texture_initialized: bool,
    #[cfg(target_os = "windows")]
    swap_chain: Option<IDXGISwapChain1>,
    #[cfg(target_os = "windows")]
    blend_state: Option<ID3D11BlendState>,
    #[cfg(target_os = "windows")]
    render_target_view: Option<ID3D11RenderTargetView>,
    #[cfg(target_os = "windows")]
    vertex_shader: Option<ID3D11VertexShader>,
    #[cfg(target_os = "windows")]
    pixel_shader: Option<ID3D11PixelShader>,
    #[cfg(target_os = "windows")]
    vertex_buffer: Option<ID3D11Buffer>,
    #[cfg(target_os = "windows")]
    input_layout: Option<ID3D11InputLayout>,
    #[cfg(target_os = "windows")]
    sampler_state: Option<ID3D11SamplerState>,
    #[cfg(target_os = "windows")]
    persistent_gpui_texture: Option<ID3D11Texture2D>, // Our copy of GPUI's texture that persists
}

impl WinitGpuiApp {
    fn new() -> Self {
        Self {
            winit_window: None,
            gpui_app: None,
            gpui_window: None,
            gpui_window_initialized: false,
            #[cfg(target_os = "windows")]
            d3d_device: None,
            #[cfg(target_os = "windows")]
            d3d_context: None,
            #[cfg(target_os = "windows")]
            shared_texture: None,
            #[cfg(target_os = "windows")]
            shared_texture_initialized: false,
            #[cfg(target_os = "windows")]
            swap_chain: None,
            #[cfg(target_os = "windows")]
            blend_state: None,
            #[cfg(target_os = "windows")]
            render_target_view: None,
            #[cfg(target_os = "windows")]
            vertex_shader: None,
            #[cfg(target_os = "windows")]
            pixel_shader: None,
            #[cfg(target_os = "windows")]
            vertex_buffer: None,
            #[cfg(target_os = "windows")]
            input_layout: None,
            #[cfg(target_os = "windows")]
            sampler_state: None,
            #[cfg(target_os = "windows")]
            persistent_gpui_texture: None,
        }
    }
}

impl ApplicationHandler for WinitGpuiApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.winit_window.is_some() {
            return;
        }

        println!("✅ Creating Winit window...");

        let window_attributes = WinitWindow::default_attributes()
            .with_title("Winit + GPUI Zero-Copy Demo")
            .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
            .with_transparent(false);

        let winit_window = Arc::new(
            event_loop
                .create_window(window_attributes)
                .expect("Failed to create window"),
        );

        println!("✅ Winit window created");
        println!("✅ Initializing GPUI...");

        let app = Application::new();

        self.winit_window = Some(winit_window);
        self.gpui_app = Some(app);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // Only handle events for our winit window
        if let Some(winit_window) = &self.winit_window {
            if winit_window.id() != window_id {
                return;
            }

            match event {
                WindowEvent::CloseRequested => {
                    println!("\n👋 Closing application...");
                    event_loop.exit();
                }
                WindowEvent::RedrawRequested => {
                    #[cfg(target_os = "windows")]
                    unsafe {
                        // Manually trigger GPUI rendering each frame
                        if self.gpui_app.is_some() {
                            let gpui_app = self.gpui_app.as_mut().unwrap();

                            // First refresh windows (marks windows as dirty)
                            let _ = gpui_app.update(|app| {
                                app.refresh_windows();
                            });
                            // After update finishes, effects are flushed
                            // Now manually trigger drawing
                            let _ = gpui_app.update(|app| {
                                app.draw_windows();
                            });
                        }

                        // Lazy initialization of shared texture on first render
                        if !self.shared_texture_initialized && self.gpui_window.is_some() && self.gpui_app.is_some() && self.d3d_device.is_some() {
                            let gpui_window = self.gpui_window.as_ref().unwrap();
                            let gpui_app = self.gpui_app.as_mut().unwrap();
                            let device = self.d3d_device.as_ref().unwrap();

                            // Get the shared texture handle from GPUI using the new update method
                            let handle_result = gpui_app.update(|app| {
                                gpui_window.update(app, |_view, window, _cx| {
                                    window.get_shared_texture_handle()
                                })
                            });

                            if let Ok(handle_ptr) = handle_result {
                                if let Some(handle_ptr) = handle_ptr {
                                    println!("✅ Got shared texture handle from GPUI: {:p}", handle_ptr);

                                    // Open the shared texture using OpenSharedResource (legacy API)
                                    // GPUI uses GetSharedHandle() which requires the legacy API
                                    let mut texture: Option<ID3D11Texture2D> = None;
                                    let result = device.OpenSharedResource(
                                        HANDLE(handle_ptr as _),
                                        &mut texture
                                    );

                                    match result {
                                        Ok(_) => {
                                            if let Some(shared_texture) = texture {
                                                // Get texture description to create our persistent copy
                                                let mut desc = D3D11_TEXTURE2D_DESC::default();
                                                shared_texture.GetDesc(&mut desc);

                                                // Create persistent texture (not shared, just ours)
                                                desc.MiscFlags = D3D11_RESOURCE_MISC_FLAG(0).0 as u32; // Remove shared flag
                                                desc.Usage = D3D11_USAGE_DEFAULT;
                                                desc.BindFlags = D3D11_BIND_SHADER_RESOURCE.0 as u32;

                                                let mut persistent_texture: Option<ID3D11Texture2D> = None;
                                                let create_result = device.CreateTexture2D(&desc, None, Some(&mut persistent_texture));

                                                if create_result.is_ok() && persistent_texture.is_some() {
                                                    self.persistent_gpui_texture = persistent_texture;
                                                    println!("✅ Created persistent GPUI texture buffer!");
                                                } else {
                                                    eprintln!("❌ Failed to create persistent texture: {:?}", create_result);
                                                }

                                                self.shared_texture = Some(shared_texture);
                                                self.shared_texture_initialized = true;
                                                println!("✅ Opened shared texture in winit D3D11 device!");
                                            }
                                        }
                                        Err(e) => {
                                            println!("❌ Failed to open shared texture: {:?}", e);
                                            self.shared_texture_initialized = true;
                                        }
                                    }
                                } else {
                                    println!("⚠️  GPUI hasn't created shared texture yet, will retry next frame");
                                }
                            }
                        }

                        // Note: We don't present here - we'll present once after compositing GPUI on top

                        // GPU-side zero-copy composition: Winit renders green, then GPUI texture on top
                        if let (Some(device), Some(context), Some(shared_texture), Some(persistent_texture), Some(swap_chain), Some(render_target_view), Some(blend_state), Some(vertex_shader), Some(pixel_shader), Some(vertex_buffer), Some(input_layout), Some(sampler_state)) =
                            (&self.d3d_device, &self.d3d_context, &self.shared_texture, &self.persistent_gpui_texture, &self.swap_chain, &self.render_target_view, &self.blend_state, &self.vertex_shader, &self.pixel_shader, &self.vertex_buffer, &self.input_layout, &self.sampler_state) {

                            // Copy from GPUI's shared texture to our persistent buffer
                            // This preserves the last rendered frame even if GPUI doesn't re-render
                            context.CopyResource(persistent_texture, shared_texture);

                            // Clear to green (bottom layer) - immediate mode background
                            let green = [0.0f32, 1.0, 0.0, 1.0];
                            context.ClearRenderTargetView(render_target_view, &green);

                            // Set render target
                            context.OMSetRenderTargets(Some(&[Some(render_target_view.clone())]), None);

                            // Set blend state for alpha blending (top layer)
                            let blend_factor = [0.0f32, 0.0, 0.0, 0.0];
                            context.OMSetBlendState(Some(blend_state), Some(&blend_factor), 0xffffffff);

                            // Create SRV for our persistent GPUI texture (retained mode UI)
                            let mut srv: Option<ID3D11ShaderResourceView> = None;
                            let srv_result = device.CreateShaderResourceView(persistent_texture, None, Some(&mut srv));
                            if srv_result.is_err() {
                                eprintln!("❌ Failed to create SRV for GPUI texture: {:?}", srv_result);
                            }

                            if let Some(srv) = srv {
                                static mut FRAME_COUNT: u32 = 0;
                                FRAME_COUNT += 1;
                                if FRAME_COUNT % 60 == 1 {
                                    eprintln!("🎨 Compositing GPUI texture (frame {})", FRAME_COUNT);
                                }

                                // Set shaders
                                context.VSSetShader(vertex_shader, None);
                                context.PSSetShader(pixel_shader, None);

                                // Set input layout
                                context.IASetInputLayout(input_layout);

                                // Set vertex buffer (fullscreen quad)
                                let stride = 16u32; // 2 floats pos + 2 floats tex = 16 bytes
                                let offset = 0u32;
                                context.IASetVertexBuffers(0, 1, Some(&Some(vertex_buffer.clone())), Some(&stride), Some(&offset));

                                // Set topology
                                context.IASetPrimitiveTopology(D3D11_PRIMITIVE_TOPOLOGY_TRIANGLESTRIP);

                                // Set GPUI texture and sampler
                                context.PSSetShaderResources(0, Some(&[Some(srv)]));
                                context.PSSetSamplers(0, Some(&[Some(sampler_state.clone())]));

                                // Set viewport
                                let size = winit_window.inner_size();
                                let viewport = D3D11_VIEWPORT {
                                    TopLeftX: 0.0,
                                    TopLeftY: 0.0,
                                    Width: size.width as f32,
                                    Height: size.height as f32,
                                    MinDepth: 0.0,
                                    MaxDepth: 1.0,
                                };
                                context.RSSetViewports(Some(&[viewport]));

                                // Draw fullscreen quad with GPUI texture on top of yellow
                                context.Draw(4, 0);
                            } else {
                                eprintln!("⚠️  SRV is None - cannot draw GPUI texture!");
                            }

                            // Present to screen
                            let _ = swap_chain.Present(1, DXGI_PRESENT(0));
                        }
                    }

                    winit_window.request_redraw();
                }
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        // Initialize GPUI window ONCE using external window API
        if !self.gpui_window_initialized && self.winit_window.is_some() && self.gpui_app.is_some() {
            let winit_window = self.winit_window.as_ref().unwrap().clone();
            let scale_factor = winit_window.scale_factor() as f32;
            let size = winit_window.inner_size();

            let bounds = Bounds {
                origin: point(px(0.0), px(0.0)),
                size: gpui::size(px(size.width as f32), px(size.height as f32)),
            };

            let gpui_raw_handle = winit_window
                .window_handle()
                .expect("Failed to get window handle")
                .as_raw();

            let external_handle = ExternalWindowHandle {
                raw_handle: gpui_raw_handle,
                bounds,
                scale_factor,
                surface_handle: None,
            };

            println!("✅ Opening GPUI window on external winit window...");

            // Open GPUI window using external window API
            let app = self.gpui_app.as_mut().unwrap();
            let gpui_window = app.open_window_external(external_handle.clone(), |window, cx| {
                cx.new(|cx| DemoView::new(window, cx))
            }).expect("Failed to open GPUI window");

            self.gpui_window = Some(gpui_window);

            // Initialize D3D11 for blitting on Windows
            #[cfg(target_os = "windows")]
            unsafe {
                println!("✅ Initializing D3D11 for GPU blitting...");

                let mut device = None;
                let mut context = None;
                let mut feature_level = Default::default();

                let result = D3D11CreateDevice(
                    None,
                    D3D_DRIVER_TYPE_HARDWARE,
                    HMODULE(std::ptr::null_mut()),
                    D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                    None,
                    D3D11_SDK_VERSION,
                    Some(&mut device),
                    Some(&mut feature_level),
                    Some(&mut context),
                );

                if result.is_ok() && device.is_some() {
                    self.d3d_device = device.clone();
                    self.d3d_context = context;
                    println!("✅ D3D11 device created successfully!");

                    // Create swap chain for the winit window
                    let parent_raw = winit_window.window_handle().unwrap().as_raw();
                    let hwnd = match parent_raw {
                        RawWindowHandle::Win32(h) => HWND(h.hwnd.get() as isize as *mut _),
                        _ => {
                            println!("❌ Failed to get HWND");
                            return;
                        }
                    };

                    let dxgi_device: IDXGIDevice = device.as_ref().unwrap().cast().unwrap();
                    let adapter = dxgi_device.GetAdapter().unwrap();
                    let dxgi_factory: IDXGIFactory2 = adapter.GetParent().unwrap();

                    let swap_chain_desc = DXGI_SWAP_CHAIN_DESC1 {
                        Width: size.width,
                        Height: size.height,
                        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                        Stereo: FALSE,
                        SampleDesc: DXGI_SAMPLE_DESC {
                            Count: 1,
                            Quality: 0,
                        },
                        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
                        BufferCount: 2,
                        Scaling: DXGI_SCALING_NONE,
                        SwapEffect: DXGI_SWAP_EFFECT_FLIP_DISCARD,
                        AlphaMode: DXGI_ALPHA_MODE_IGNORE,  // Ignore alpha for solid window
                        Flags: 0,
                    };

                    let swap_chain = dxgi_factory.CreateSwapChainForHwnd(
                        device.as_ref().unwrap(),
                        hwnd,
                        &swap_chain_desc,
                        None,
                        None,
                    );
                    if let Ok(swap_chain) = swap_chain {
                        self.swap_chain = Some(swap_chain.clone());
                        println!("✅ Swap chain created for winit window!");

                        // Create render target view from swap chain back buffer
                        if let Ok(back_buffer) = swap_chain.GetBuffer::<ID3D11Texture2D>(0) {
                            let mut rtv: Option<ID3D11RenderTargetView> = None;
                            if device.as_ref().unwrap().CreateRenderTargetView(&back_buffer, None, Some(&mut rtv as *mut _)).is_ok() {
                                self.render_target_view = rtv;
                                println!("✅ Render target view created!");
                            }
                        }

                        // Create blend state for alpha blending
                        let blend_desc = D3D11_BLEND_DESC {
                            AlphaToCoverageEnable: FALSE,
                            IndependentBlendEnable: FALSE,
                            RenderTarget: [
                                D3D11_RENDER_TARGET_BLEND_DESC {
                                    BlendEnable: TRUE,
                                    SrcBlend: D3D11_BLEND_SRC_ALPHA,
                                    DestBlend: D3D11_BLEND_INV_SRC_ALPHA,
                                    BlendOp: D3D11_BLEND_OP_ADD,
                                    SrcBlendAlpha: D3D11_BLEND_ONE,
                                    DestBlendAlpha: D3D11_BLEND_ZERO,
                                    BlendOpAlpha: D3D11_BLEND_OP_ADD,
                                    RenderTargetWriteMask: D3D11_COLOR_WRITE_ENABLE_ALL.0 as u8,
                                },
                                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                                D3D11_RENDER_TARGET_BLEND_DESC::default(),
                            ],
                        };

                        let mut blend_state = None;
                        if device.as_ref().unwrap().CreateBlendState(&blend_desc, Some(&mut blend_state as *mut _)).is_ok() {
                            self.blend_state = blend_state;
                            println!("✅ Blend state created for alpha composition!");
                        }

                        // Create shaders for GPU alpha blending by compiling HLSL at runtime
                        println!("🔨 Compiling shaders at runtime...");

                        // Vertex shader source: passthrough with position and texcoord
                        let vs_source = r#"
struct VS_INPUT {
    float2 pos : POSITION;
    float2 tex : TEXCOORD0;
};

struct PS_INPUT {
    float4 pos : SV_POSITION;
    float2 tex : TEXCOORD0;
};

PS_INPUT main(VS_INPUT input) {
    PS_INPUT output;
    output.pos = float4(input.pos, 0.0f, 1.0f);
    output.tex = input.tex;
    return output;
}
"#;

                        // Pixel shader source: sample texture with alpha
                        let ps_source = r#"
Texture2D gpuiTexture : register(t0);
SamplerState samplerState : register(s0);

struct PS_INPUT {
    float4 pos : SV_POSITION;
    float2 tex : TEXCOORD0;
};

float4 main(PS_INPUT input) : SV_TARGET {
    return gpuiTexture.Sample(samplerState, input.tex);
}
"#;

                        // Compile vertex shader
                        let vs_bytecode_blob = {
                            let mut blob: Option<ID3DBlob> = None;
                            let mut error_blob: Option<ID3DBlob> = None;
                            let result = D3DCompile(
                                vs_source.as_ptr() as *const _,
                                vs_source.len(),
                                None,
                                None,
                                None,
                                s!("main"),
                                s!("vs_5_0"),
                                0,
                                0,
                                &mut blob,
                                Some(&mut error_blob),
                            );

                            if result.is_err() {
                                if let Some(err) = error_blob {
                                    let err_msg = std::slice::from_raw_parts(
                                        err.GetBufferPointer() as *const u8,
                                        err.GetBufferSize(),
                                    );
                                    println!("❌ VS compile error: {}", String::from_utf8_lossy(err_msg));
                                }
                            }
                            blob
                        };

                        // Compile pixel shader
                        let ps_bytecode_blob = {
                            let mut blob: Option<ID3DBlob> = None;
                            let mut error_blob: Option<ID3DBlob> = None;
                            let result = D3DCompile(
                                ps_source.as_ptr() as *const _,
                                ps_source.len(),
                                None,
                                None,
                                None,
                                s!("main"),
                                s!("ps_5_0"),
                                0,
                                0,
                                &mut blob,
                                Some(&mut error_blob),
                            );

                            if result.is_err() {
                                if let Some(err) = error_blob {
                                    let err_msg = std::slice::from_raw_parts(
                                        err.GetBufferPointer() as *const u8,
                                        err.GetBufferSize(),
                                    );
                                    println!("❌ PS compile error: {}", String::from_utf8_lossy(err_msg));
                                }
                            }
                            blob
                        };

                        let vs_bytecode = if let Some(blob) = &vs_bytecode_blob {
                            std::slice::from_raw_parts(
                                blob.GetBufferPointer() as *const u8,
                                blob.GetBufferSize(),
                            )
                        } else {
                            &[] as &[u8]
                        };

                        let ps_bytecode = if let Some(blob) = &ps_bytecode_blob {
                            std::slice::from_raw_parts(
                                blob.GetBufferPointer() as *const u8,
                                blob.GetBufferSize(),
                            )
                        } else {
                            &[] as &[u8]
                        };

                        if vs_bytecode.is_empty() || ps_bytecode.is_empty() {
                            println!("❌ Shader compilation failed!");
                        }

                        // Create D3D11 shader objects from compiled bytecode
                        let mut vertex_shader = None;
                        let mut pixel_shader = None;

                        let vs_result = if !vs_bytecode.is_empty() {
                            device.as_ref().unwrap().CreateVertexShader(vs_bytecode, None, Some(&mut vertex_shader as *mut _))
                        } else {
                            Err(Error::from(E_FAIL))
                        };

                        let ps_result = if !ps_bytecode.is_empty() {
                            device.as_ref().unwrap().CreatePixelShader(ps_bytecode, None, Some(&mut pixel_shader as *mut _))
                        } else {
                            Err(Error::from(E_FAIL))
                        };

                        if vs_result.is_ok() && ps_result.is_ok() {
                            self.vertex_shader = vertex_shader;
                            self.pixel_shader = pixel_shader;
                            println!("✅ Shaders created from bytecode!");
                        } else {
                            println!("❌ Failed to create shaders - VS: {:?}, PS: {:?}", vs_result, ps_result);
                        }

                        if self.vertex_shader.is_some() && self.pixel_shader.is_some() {

                            // Create input layout that matches the vertex shader
                            // VS_INPUT has: float2 pos : POSITION; float2 tex : TEXCOORD0;
                            let layout = [
                                D3D11_INPUT_ELEMENT_DESC {
                                    SemanticName: s!("POSITION"),
                                    SemanticIndex: 0,
                                    Format: DXGI_FORMAT_R32G32_FLOAT,
                                    InputSlot: 0,
                                    AlignedByteOffset: 0,
                                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                                    InstanceDataStepRate: 0,
                                },
                                D3D11_INPUT_ELEMENT_DESC {
                                    SemanticName: s!("TEXCOORD"),
                                    SemanticIndex: 0,
                                    Format: DXGI_FORMAT_R32G32_FLOAT,
                                    InputSlot: 0,
                                    AlignedByteOffset: 8,
                                    InputSlotClass: D3D11_INPUT_PER_VERTEX_DATA,
                                    InstanceDataStepRate: 0,
                                },
                            ];

                            let mut input_layout = None;
                            if device.as_ref().unwrap().CreateInputLayout(&layout, vs_bytecode, Some(&mut input_layout as *mut _)).is_ok() {
                                self.input_layout = input_layout;
                                println!("✅ Input layout created!");
                            } else {
                                println!("❌ Failed to create input layout");
                            }
                        }

                        // Create vertex buffer for fullscreen quad
                        #[repr(C)]
                        struct Vertex {
                            pos: [f32; 2],
                            tex: [f32; 2],
                        }

                        let vertices = [
                            Vertex { pos: [-1.0, -1.0], tex: [0.0, 1.0] },
                            Vertex { pos: [-1.0,  1.0], tex: [0.0, 0.0] },
                            Vertex { pos: [ 1.0, -1.0], tex: [1.0, 1.0] },
                            Vertex { pos: [ 1.0,  1.0], tex: [1.0, 0.0] },
                        ];

                        let vb_desc = D3D11_BUFFER_DESC {
                            ByteWidth: std::mem::size_of_val(&vertices) as u32,
                            Usage: D3D11_USAGE_DEFAULT,
                            BindFlags: D3D11_BIND_VERTEX_BUFFER.0 as u32,
                            CPUAccessFlags: 0,
                            MiscFlags: 0,
                            StructureByteStride: 0,
                        };

                        let vb_data = D3D11_SUBRESOURCE_DATA {
                            pSysMem: vertices.as_ptr() as *const _,
                            SysMemPitch: 0,
                            SysMemSlicePitch: 0,
                        };

                        let mut vertex_buffer = None;
                        if device.as_ref().unwrap().CreateBuffer(&vb_desc, Some(&vb_data), Some(&mut vertex_buffer as *mut _)).is_ok() {
                            self.vertex_buffer = vertex_buffer;
                            println!("✅ Vertex buffer created!");
                        }

                        // Create sampler state
                        let sampler_desc = D3D11_SAMPLER_DESC {
                            Filter: D3D11_FILTER_MIN_MAG_MIP_LINEAR,
                            AddressU: D3D11_TEXTURE_ADDRESS_CLAMP,
                            AddressV: D3D11_TEXTURE_ADDRESS_CLAMP,
                            AddressW: D3D11_TEXTURE_ADDRESS_CLAMP,
                            MipLODBias: 0.0,
                            MaxAnisotropy: 1,
                            ComparisonFunc: D3D11_COMPARISON_NEVER,
                            BorderColor: [0.0, 0.0, 0.0, 0.0],
                            MinLOD: 0.0,
                            MaxLOD: f32::MAX,
                        };

                        let mut sampler_state = None;
                        if device.as_ref().unwrap().CreateSamplerState(&sampler_desc, Some(&mut sampler_state as *mut _)).is_ok() {
                            self.sampler_state = sampler_state;
                            println!("✅ Sampler state created!");
                        }
                    } else {
                        println!("❌ Failed to create swap chain");
                    }

                    // Note: We'll get the shared texture handle lazily on first render
                    // GPUI creates the shared texture during its first draw call
                    println!("💡 Shared texture will be retrieved on first render");
                } else {
                    println!("❌ Failed to create D3D11 device: {:?}", result);
                }
            }

            self.gpui_window_initialized = true;
            println!("✅ GPUI window opened! Ready for GPU composition!\n");
        }
    }
}

struct DemoView {
    counter: usize,
}

impl DemoView {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { counter: 0 }
    }
}

impl Render for DemoView {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.counter += 1;

        // Transparent background - let Winit's green show through
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .size_full()
            .gap_4()
            .child(
                // Small blue square to show GPUI is rendering
                div()
                    .size(px(200.0))
                    .bg(rgb(0x4A90E2))
                    .rounded_lg()
                    .shadow_lg()
                    .border_2()
                    .border_color(rgb(0xFFFFFF))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        div()
                            .text_2xl()
                            .font_weight(gpui::FontWeight::BOLD)
                            .text_color(rgb(0xFFFFFF))
                            .child("GPUI"),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_xl()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(0x333333))
                            .child(format!("Frame: {}", self.counter)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x666666))
                            .child("✅ GPUI rendering on Winit window!"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(0x666666))
                            .child("🎨 Zero-copy GPU composition"),
                    ),
            )
    }
}
