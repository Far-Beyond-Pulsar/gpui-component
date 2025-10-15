use gpui_component::viewport_final::Framebuffer as ViewportFramebuffer;
use engine_backend::{BevyRenderer, RenderFramebuffer};
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

/// Wrapper around the backend Bevy renderer that works with our viewport framebuffer
pub struct GpuRenderer {
    bevy_renderer: Option<BevyRenderer>,
    temp_framebuffer: RenderFramebuffer,
    render_width: u32,
    render_height: u32,
    display_width: u32,
    display_height: u32,
    frame_count: u64,
    start_time: Instant,
}

impl GpuRenderer {
    pub fn new(display_width: u32, display_height: u32) -> Self {
        // Render directly at viewport size - no downscaling
        let width = display_width;
        let height = display_height;
        
        println!("[GPU-RENDERER] Initializing Bevy renderer at {}x{}", width, height);
        
        // Create renderer asynchronously with timeout
        let runtime = get_runtime();
        let bevy_renderer = runtime.block_on(async {
            println!("[GPU-RENDERER] Creating Bevy renderer asynchronously...");
            match tokio::time::timeout(
                tokio::time::Duration::from_secs(10),
                BevyRenderer::new(width, height)
            ).await {
                Ok(renderer) => {
                    println!("[GPU-RENDERER] Bevy renderer created successfully!");
                    Some(renderer)
                }
                Err(_) => {
                    println!("[GPU-RENDERER] WARNING: Bevy renderer creation timed out! Will use fallback rendering.");
                    None
                }
            }
        });

        if bevy_renderer.is_none() {
            println!("[GPU-RENDERER] Using fallback CPU rendering");
        }

        Self {
            bevy_renderer,
            temp_framebuffer: RenderFramebuffer::new(width, height),
            render_width: width,
            render_height: height,
            display_width,
            display_height,
            frame_count: 0,
            start_time: Instant::now(),
        }
    }

    pub fn render(&mut self, framebuffer: &mut ViewportFramebuffer) {
        self.frame_count += 1;

        if let Some(ref mut renderer) = self.bevy_renderer {
            // Use Bevy renderer - render directly to temp buffer at full viewport size
            if self.frame_count % 60 == 0 {
                println!("[GPU-RENDERER] Frame {} - Using Bevy renderer ({}x{})", 
                    self.frame_count, self.render_width, self.render_height);
            }

            // Render directly at viewport resolution
            renderer.render(&mut self.temp_framebuffer);

            // Copy directly to framebuffer (no scaling needed)
            self.copy_to_framebuffer(framebuffer);
            
            framebuffer.mark_dirty_all();
        } else {
            // Fallback: render a test pattern
            if self.frame_count % 60 == 0 {
                println!("[GPU-RENDERER] Frame {} - Using fallback renderer", self.frame_count);
            }
            
            self.render_fallback(framebuffer);
        }
    }

    fn copy_to_framebuffer(&self, framebuffer: &mut ViewportFramebuffer) {
        // Direct copy - both buffers should be the same size now
        let copy_len = self.temp_framebuffer.buffer.len().min(framebuffer.buffer.len());
        framebuffer.buffer[..copy_len].copy_from_slice(&self.temp_framebuffer.buffer[..copy_len]);
    }

    fn render_fallback(&self, framebuffer: &mut ViewportFramebuffer) {
        // Render a simple animated pattern to show the system works
        let time = self.frame_count as f32 * 0.016;
        
        for y in 0..framebuffer.height {
            for x in 0..framebuffer.width {
                let idx = ((y * framebuffer.width + x) * 4) as usize;
                
                let u = x as f32 / framebuffer.width as f32;
                let v = y as f32 / framebuffer.height as f32;
                
                // Create a moving gradient pattern with "FALLBACK" indicator
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

    pub fn resize(&mut self, display_width: u32, display_height: u32) {
        if self.display_width != display_width || self.display_height != display_height {
            self.render_width = display_width;
            self.render_height = display_height;
            self.display_width = display_width;
            self.display_height = display_height;
            self.temp_framebuffer.resize(display_width, display_height);
            
            println!("[GPU-RENDERER] Resizing to {}x{}", display_width, display_height);
            
            // Recreate Bevy renderer at new resolution
            let runtime = get_runtime();
            self.bevy_renderer = Some(runtime.block_on(async {
                BevyRenderer::new(display_width, display_height).await
            }));
        }
    }
}

unsafe impl Send for GpuRenderer {}
unsafe impl Sync for GpuRenderer {}
