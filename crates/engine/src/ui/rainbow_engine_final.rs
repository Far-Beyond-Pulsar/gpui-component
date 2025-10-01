use gpui_component::viewport_final::Framebuffer;
use std::time::Instant;

/// High-performance rainbow render engine targeting 240 FPS
pub struct RainbowRenderEngine {
    frame_count: u64,
    start_time: Instant,
    last_frame_time: Instant,
    time_offset: f32,
    rainbow_speed: f32,
    pattern_type: RainbowPattern,
    frame_time_ms: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum RainbowPattern {
    Waves,
    Spiral,
    Plasma,
    Ripples,
    Matrix,
}

impl RainbowRenderEngine {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            frame_count: 0,
            start_time: now,
            last_frame_time: now,
            time_offset: 0.0,
            rainbow_speed: 2.0,
            pattern_type: RainbowPattern::Waves,
            frame_time_ms: 0.0,
        }
    }

    pub fn set_pattern(&mut self, pattern: RainbowPattern) {
        self.pattern_type = pattern;
    }

    pub fn set_speed(&mut self, speed: f32) {
        self.rainbow_speed = speed.clamp(0.1, 10.0);
    }

    pub fn get_fps(&self) -> f32 {
        if self.frame_time_ms > 0.0 {
            1000.0 / self.frame_time_ms
        } else {
            0.0
        }
    }

    pub fn get_frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Zero-copy RGBA8 render method - NO format conversion needed
    pub fn render_rgba8(&mut self, framebuffer: &mut Framebuffer) {
        let now = Instant::now();
        self.frame_time_ms = now.duration_since(self.last_frame_time).as_secs_f32() * 1000.0;
        self.last_frame_time = now;

        self.frame_count += 1;
        self.time_offset = now.duration_since(self.start_time).as_secs_f32() * self.rainbow_speed;

        // Render directly in RGBA8 format - zero conversion overhead
        self.render_rgba_optimized(framebuffer);
        
        framebuffer.mark_dirty_all();
    }

    #[inline(always)]
    fn render_rgba_optimized(&self, framebuffer: &mut Framebuffer) {
        let width = framebuffer.width as usize;
        let height = framebuffer.height as usize;
        let buffer = &mut framebuffer.buffer;

        // Pre-calculate constants for performance
        let time = self.time_offset;
        
        // Use full resolution for smooth animation
        for y in 0..height {
            let y_norm = y as f32 / height as f32;
            let row_offset = y * width * 4;

            for x in 0..width {
                let x_norm = x as f32 / width as f32;
                let pixel_offset = row_offset + x * 4;

                // Calculate color based on pattern type
                let color = match self.pattern_type {
                    RainbowPattern::Waves => self.waves_pattern(x_norm, y_norm, time),
                    RainbowPattern::Spiral => self.spiral_pattern(x_norm, y_norm, time),
                    RainbowPattern::Plasma => self.plasma_pattern(x_norm, y_norm, time),
                    RainbowPattern::Ripples => self.ripples_pattern(x_norm, y_norm, time),
                    RainbowPattern::Matrix => self.matrix_pattern(x_norm, y_norm, time),
                };

                // Store RGBA values directly
                if pixel_offset + 3 < buffer.len() {
                    buffer[pixel_offset] = color[0];     // R
                    buffer[pixel_offset + 1] = color[1]; // G
                    buffer[pixel_offset + 2] = color[2]; // B
                    buffer[pixel_offset + 3] = color[3]; // A
                }
            }
        }
    }

    #[inline(always)]
    fn waves_pattern(&self, x: f32, y: f32, time: f32) -> [u8; 4] {
        let wave1 = ((x * 8.0 + time * 2.0).sin() * 0.5 + 0.5);
        let wave2 = ((y * 6.0 + time * 1.5).sin() * 0.5 + 0.5);
        let combined = (wave1 + wave2) * 0.5;
        
        self.hsv_to_rgba(combined * 360.0, 0.8, 0.9)
    }

    #[inline(always)]
    fn spiral_pattern(&self, x: f32, y: f32, time: f32) -> [u8; 4] {
        let center_x = x - 0.5;
        let center_y = y - 0.5;
        let angle = center_y.atan2(center_x);
        let distance = (center_x * center_x + center_y * center_y).sqrt();
        
        let spiral_value = (angle * 3.0 + distance * 10.0 + time * 3.0).sin() * 0.5 + 0.5;
        
        self.hsv_to_rgba(spiral_value * 360.0, 0.9, 0.8)
    }

    #[inline(always)]
    fn plasma_pattern(&self, x: f32, y: f32, time: f32) -> [u8; 4] {
        let plasma = (
            (x * 16.0 + time).sin() +
            (y * 16.0 + time * 1.3).sin() +
            ((x * 16.0 + y * 16.0 + time * 0.7).sin() * 2.0) +
            ((x * x + y * y).sqrt() * 8.0 + time * 2.0).sin()
        ) * 0.125 + 0.5;
        
        self.hsv_to_rgba(plasma * 360.0, 0.8, 0.9)
    }

    #[inline(always)]
    fn ripples_pattern(&self, x: f32, y: f32, time: f32) -> [u8; 4] {
        let center_x = x - 0.5;
        let center_y = y - 0.5;
        let distance = (center_x * center_x + center_y * center_y).sqrt();
        
        let ripple = (distance * 20.0 - time * 5.0).sin() * 0.5 + 0.5;
        
        self.hsv_to_rgba(ripple * 360.0, 0.7, 0.9)
    }

    #[inline(always)]
    fn matrix_pattern(&self, x: f32, y: f32, time: f32) -> [u8; 4] {
        let grid_x = (x * 20.0).floor();
        let grid_y = (y * 20.0).floor();
        
        // Pseudo-random based on grid position and time
        let seed = grid_x * 37.0 + grid_y * 41.0 + (time * 2.0).floor() * 13.0;
        let random = ((seed * 9.871).sin() * 10000.0).fract();
        
        if random > 0.95 {
            // Bright green character
            [0, 255, 100, 255]
        } else if random > 0.8 {
            // Dim green
            [0, 150, 50, 255]
        } else {
            // Black background
            [0, 0, 0, 255]
        }
    }

    #[inline(always)]
    fn hsv_to_rgba(&self, h: f32, s: f32, v: f32) -> [u8; 4] {
        let h = h % 360.0;
        let c = v * s;
        let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
        let m = v - c;

        let (r, g, b) = if h < 60.0 {
            (c, x, 0.0)
        } else if h < 120.0 {
            (x, c, 0.0)
        } else if h < 180.0 {
            (0.0, c, x)
        } else if h < 240.0 {
            (0.0, x, c)
        } else if h < 300.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        [
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
            255,
        ]
    }
}

impl Default for RainbowRenderEngine {
    fn default() -> Self {
        Self::new()
    }
}
