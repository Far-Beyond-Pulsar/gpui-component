use gpui_component::viewport_final::Framebuffer as ViewportFramebuffer;
use engine_backend::{WgpuRenderer, RenderFramebuffer};
use std::sync::{Arc, Mutex, Once};
use std::time::Instant;

static INIT: Once = Once::new();
static mut RUNTIME: Option<tokio::runtime::Runtime> = None;

fn get_runtime() -> &'static tokio::runtime::Runtime {
    unsafe {
        INIT.call_once(|| {
            RUNTIME = Some(
                tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime")
            );
        });
        RUNTIME.as_ref().unwrap()
    }
}

/// Wrapper around the backend WGPU renderer that works with our viewport framebuffer
pub struct GpuRenderer {
    wgpu_renderer: Option<WgpuRenderer>,
    temp_framebuffer: RenderFramebuffer,
    width: u32,
    height: u32,
    frame_count: u64,
    start_time: Instant,
}

impl GpuRenderer {
    pub fn new(width: u32, height: u32) -> Self {
        println!("[GPU-RENDERER] Initializing WGPU renderer {}x{}", width, height);
        
        // Create renderer asynchronously with timeout
        let runtime = get_runtime();
        let wgpu_renderer = runtime.block_on(async {
            println!("[GPU-RENDERER] Creating WGPU renderer asynchronously...");
            match tokio::time::timeout(
                tokio::time::Duration::from_secs(5),
                WgpuRenderer::new(width, height)
            ).await {
                Ok(renderer) => {
                    println!("[GPU-RENDERER] WGPU renderer created successfully!");
                    Some(renderer)
                }
                Err(_) => {
                    println!("[GPU-RENDERER] WARNING: WGPU renderer creation timed out! Will use fallback rendering.");
                    None
                }
            }
        });

        if wgpu_renderer.is_none() {
            println!("[GPU-RENDERER] Using fallback CPU rendering");
        }

        Self {
            wgpu_renderer,
            temp_framebuffer: RenderFramebuffer::new(width, height),
            width,
            height,
            frame_count: 0,
            start_time: Instant::now(),
        }
    }

    pub fn render(&mut self, framebuffer: &mut ViewportFramebuffer) {
        self.frame_count += 1;

        if let Some(ref mut renderer) = self.wgpu_renderer {
            // Use GPU renderer
            if self.frame_count % 60 == 0 {
                println!("[GPU-RENDERER] Frame {} - Using GPU renderer", self.frame_count);
            }

            // Render to our temp framebuffer
            renderer.render(&mut self.temp_framebuffer);

            // Copy to viewport framebuffer
            framebuffer.buffer.copy_from_slice(&self.temp_framebuffer.buffer);
            framebuffer.mark_dirty_all();
        } else {
            // Fallback: render a test pattern
            if self.frame_count % 60 == 0 {
                println!("[GPU-RENDERER] Frame {} - Using fallback renderer", self.frame_count);
            }
            
            self.render_fallback(framebuffer);
        }
    }

    fn render_fallback(&self, framebuffer: &mut ViewportFramebuffer) {
        // Render a simple animated pattern to show the system works
        let time = self.frame_count as f32 * 0.016;
        
        for y in 0..framebuffer.height {
            for x in 0..framebuffer.width {
                let idx = ((y * framebuffer.width + x) * 4) as usize;
                
                let u = x as f32 / framebuffer.width as f32;
                let v = y as f32 / framebuffer.height as f32;
                
                // Create a moving gradient pattern
                let r = ((u + time.sin() * 0.5).sin() * 128.0 + 127.0) as u8;
                let g = ((v + time.cos() * 0.5).cos() * 128.0 + 127.0) as u8;
                let b = (((u + v) * 2.0 + time).sin() * 128.0 + 127.0) as u8;
                
                framebuffer.buffer[idx] = r;
                framebuffer.buffer[idx + 1] = g;
                framebuffer.buffer[idx + 2] = b;
                framebuffer.buffer[idx + 3] = 255;
            }
        }
        
        framebuffer.mark_dirty_all();
    }

    pub fn get_frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn get_fps(&self) -> f32 {
        let elapsed = self.start_time.elapsed().as_secs_f32();
        if elapsed > 0.0 {
            self.frame_count as f32 / elapsed
        } else {
            0.0
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if self.width != width || self.height != height {
            self.width = width;
            self.height = height;
            self.temp_framebuffer.resize(width, height);
            
            // TODO: Resize the WGPU renderer
            // For now, recreate it
            println!("[GPU-RENDERER] Resizing to {}x{}", width, height);
            let runtime = get_runtime();
            self.wgpu_renderer = Some(runtime.block_on(async {
                WgpuRenderer::new(width, height).await
            }));
        }
    }
}

unsafe impl Send for GpuRenderer {}
unsafe impl Sync for GpuRenderer {}
