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
        // Render at 65% resolution for performance
        let render_width = ((display_width as f32 * 0.65) as u32 / 2) * 2; // Round to nearest even
        let render_height = ((display_height as f32 * 0.65) as u32 / 2) * 2;
        
        println!("[GPU-RENDERER] Initializing Bevy renderer at {}x{} (65% of {}x{})", 
            render_width, render_height, display_width, display_height);
        
        // Create renderer asynchronously with timeout
        let runtime = get_runtime();
        let bevy_renderer = runtime.block_on(async {
            println!("[GPU-RENDERER] Creating Bevy renderer asynchronously...");
            match tokio::time::timeout(
                tokio::time::Duration::from_secs(10),
                BevyRenderer::new(render_width, render_height)
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
            temp_framebuffer: RenderFramebuffer::new(render_width, render_height),
            render_width,
            render_height,
            display_width,
            display_height,
            frame_count: 0,
            start_time: Instant::now(),
        }
    }

    pub fn render(&mut self, framebuffer: &mut ViewportFramebuffer) {
        self.frame_count += 1;

        if let Some(ref mut renderer) = self.bevy_renderer {
            // Use Bevy renderer
            if self.frame_count % 60 == 0 {
                println!("[GPU-RENDERER] Frame {} - Using Bevy renderer ({}x{} → {}x{})", 
                    self.frame_count, self.render_width, self.render_height,
                    self.display_width, self.display_height);
            }

            // Render to our temp framebuffer at lower resolution
            renderer.render(&mut self.temp_framebuffer);

            // Scale up to display resolution
            self.scale_to_framebuffer(framebuffer);
            
            framebuffer.mark_dirty_all();
        } else {
            // Fallback: render a test pattern
            if self.frame_count % 60 == 0 {
                println!("[GPU-RENDERER] Frame {} - Using fallback renderer", self.frame_count);
            }
            
            self.render_fallback(framebuffer);
        }
    }

    fn scale_to_framebuffer(&self, framebuffer: &mut ViewportFramebuffer) {
        // Simple nearest-neighbor upscaling
        let x_scale = self.display_width as f32 / self.render_width as f32;
        let y_scale = self.display_height as f32 / self.render_height as f32;

        for dy in 0..self.display_height {
            for dx in 0..self.display_width {
                let sx = (dx as f32 / x_scale) as u32;
                let sy = (dy as f32 / y_scale) as u32;

                let src_idx = ((sy * self.render_width + sx) * 4) as usize;
                let dst_idx = ((dy * self.display_width + dx) * 4) as usize;

                if src_idx + 3 < self.temp_framebuffer.buffer.len() && dst_idx + 3 < framebuffer.buffer.len() {
                    framebuffer.buffer[dst_idx..dst_idx + 4]
                        .copy_from_slice(&self.temp_framebuffer.buffer[src_idx..src_idx + 4]);
                }
            }
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
        let render_width = ((display_width as f32 * 0.65) as u32 / 2) * 2;
        let render_height = ((display_height as f32 * 0.65) as u32 / 2) * 2;
        
        if self.render_width != render_width || self.render_height != render_height {
            self.render_width = render_width;
            self.render_height = render_height;
            self.display_width = display_width;
            self.display_height = display_height;
            self.temp_framebuffer.resize(render_width, render_height);
            
            println!("[GPU-RENDERER] Resizing to {}x{} (render) → {}x{} (display)", 
                render_width, render_height, display_width, display_height);
            
            // Recreate Bevy renderer at new resolution
            let runtime = get_runtime();
            self.bevy_renderer = Some(runtime.block_on(async {
                BevyRenderer::new(render_width, render_height).await
            }));
        }
    }
}

unsafe impl Send for GpuRenderer {}
unsafe impl Sync for GpuRenderer {}
