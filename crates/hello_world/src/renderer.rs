use gpui_component::viewport::{Framebuffer, RenderEngine};

pub struct ShaderRenderer {
    time: f32,
}

impl ShaderRenderer {
    pub fn new() -> Self {
        Self {
            time: 0.0,
        }
    }
}

impl RenderEngine for ShaderRenderer {
    fn render(&mut self, framebuffer: &mut Framebuffer) {
        self.time += 0.016; // Simulate ~60fps

        let width = framebuffer.width;
        let height = framebuffer.height;

        // Basic shader that creates a moving gradient pattern
        for y in 0..height {
            for x in 0..width {
                let u = x as f32 / width as f32;
                let v = y as f32 / height as f32;
                
                // Create a moving pattern based on time
                let r = ((u + self.time.sin() * 0.5) * 255.0) as u8;
                let g = ((v + self.time.cos() * 0.5) * 255.0) as u8;
                let b = (((u + v) * 0.5 + self.time * 0.5).sin() * 255.0) as u8;
                
                let index = ((y * width + x) * 4) as usize;
                framebuffer.buffer[index] = r;     // R
                framebuffer.buffer[index + 1] = g; // G
                framebuffer.buffer[index + 2] = b; // B
                framebuffer.buffer[index + 3] = 255; // A
            }
        }
    }
}