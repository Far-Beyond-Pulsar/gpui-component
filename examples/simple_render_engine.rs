use ui::viewport::{Framebuffer, FramebufferFormat};

/// Example render engine that demonstrates the new viewport integration pattern
pub struct SimpleRenderEngine {
    frame_count: u64,
    clear_color: [u8; 4],
}

impl SimpleRenderEngine {
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            clear_color: [0, 0, 0, 255], // Black background
        }
    }

    pub fn set_clear_color(&mut self, color: [u8; 4]) {
        self.clear_color = color;
    }

    /// Render to the provided framebuffer
    /// This is the key method that external render engines must implement
    pub fn render(&mut self, framebuffer: &mut Framebuffer) {
        self.frame_count += 1;

        // Clear the framebuffer
        framebuffer.clear(self.clear_color);

        // Draw some simple content based on frame count
        self.draw_animated_pattern(framebuffer);

        // Mark the framebuffer as dirty to trigger texture updates
        framebuffer.mark_dirty(None);
    }

    fn draw_animated_pattern(&self, framebuffer: &mut Framebuffer) {
        let time = self.frame_count as f32 * 0.02;
        
        match framebuffer.format {
            FramebufferFormat::Rgba8 | FramebufferFormat::Bgra8 => {
                self.draw_rgba_pattern(framebuffer, time);
            }
            FramebufferFormat::Rgb8 | FramebufferFormat::Bgr8 => {
                self.draw_rgb_pattern(framebuffer, time);
            }
        }
    }

    fn draw_rgba_pattern(&self, framebuffer: &mut Framebuffer, time: f32) {
        let bytes_per_pixel = 4;
        
        for y in 0..framebuffer.height {
            for x in 0..framebuffer.width {
                let offset = (y * framebuffer.pitch + x * bytes_per_pixel) as usize;
                
                if offset + 3 < framebuffer.buffer.len() {
                    // Create a simple animated wave pattern
                    let wave_x = ((x as f32 / 30.0 + time).sin() * 127.0 + 128.0) as u8;
                    let wave_y = ((y as f32 / 30.0 + time).cos() * 127.0 + 128.0) as u8;
                    let wave_mix = (((x + y) as f32 / 20.0 + time).sin() * 127.0 + 128.0) as u8;

                    match framebuffer.format {
                        FramebufferFormat::Rgba8 => {
                            framebuffer.buffer[offset] = wave_x;     // R
                            framebuffer.buffer[offset + 1] = wave_y; // G  
                            framebuffer.buffer[offset + 2] = wave_mix; // B
                            framebuffer.buffer[offset + 3] = 255;    // A
                        }
                        FramebufferFormat::Bgra8 => {
                            framebuffer.buffer[offset] = wave_mix;   // B
                            framebuffer.buffer[offset + 1] = wave_y; // G
                            framebuffer.buffer[offset + 2] = wave_x; // R
                            framebuffer.buffer[offset + 3] = 255;    // A
                        }
                        _ => unreachable!()
                    }
                }
            }
        }
    }

    fn draw_rgb_pattern(&self, framebuffer: &mut Framebuffer, time: f32) {
        let bytes_per_pixel = 3;
        
        for y in 0..framebuffer.height {
            for x in 0..framebuffer.width {
                let offset = (y * framebuffer.pitch + x * bytes_per_pixel) as usize;
                
                if offset + 2 < framebuffer.buffer.len() {
                    let wave_x = ((x as f32 / 30.0 + time).sin() * 127.0 + 128.0) as u8;
                    let wave_y = ((y as f32 / 30.0 + time).cos() * 127.0 + 128.0) as u8;
                    let wave_mix = (((x + y) as f32 / 20.0 + time).sin() * 127.0 + 128.0) as u8;

                    match framebuffer.format {
                        FramebufferFormat::Rgb8 => {
                            framebuffer.buffer[offset] = wave_x;     // R
                            framebuffer.buffer[offset + 1] = wave_y; // G
                            framebuffer.buffer[offset + 2] = wave_mix; // B
                        }
                        FramebufferFormat::Bgr8 => {
                            framebuffer.buffer[offset] = wave_mix;   // B
                            framebuffer.buffer[offset + 1] = wave_y; // G
                            framebuffer.buffer[offset + 2] = wave_x; // R
                        }
                        _ => unreachable!()
                    }
                }
            }
        }
    }

    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }
}

impl Default for SimpleRenderEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_render_engine() {
        let mut engine = SimpleRenderEngine::new();
        let mut framebuffer = Framebuffer::new(100, 100, FramebufferFormat::Rgba8);

        // Test initial state
        assert_eq!(engine.frame_count(), 0);

        // Test rendering
        engine.render(&mut framebuffer);
        assert_eq!(engine.frame_count(), 1);

        // Test that buffer is marked dirty
        assert!(framebuffer.is_dirty());

        // Test multiple renders
        engine.render(&mut framebuffer);
        assert_eq!(engine.frame_count(), 2);
    }

    #[test]
    fn test_color_setting() {
        let mut engine = SimpleRenderEngine::new();
        engine.set_clear_color([255, 128, 64, 255]);
        
        let mut framebuffer = Framebuffer::new(10, 10, FramebufferFormat::Rgba8);
        engine.render(&mut framebuffer);
        
        // The buffer should have been cleared with the set color
        // (though it will be overwritten by the pattern)
        assert!(framebuffer.is_dirty());
    }
}
