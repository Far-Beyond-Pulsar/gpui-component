use gpui_component::viewport::{Framebuffer, FramebufferFormat};
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

    /// High-performance render function optimized for smooth double-buffered rendering
    pub fn render(&mut self, framebuffer: &mut Framebuffer) {
        let now = Instant::now();
        self.frame_time_ms = now.duration_since(self.last_frame_time).as_secs_f32() * 1000.0;
        self.last_frame_time = now;

        self.frame_count += 1;
        self.time_offset = now.duration_since(self.start_time).as_secs_f32() * self.rainbow_speed;

        // Always render every frame for smooth animation (no frame skipping)
        match framebuffer.format {
            FramebufferFormat::Rgba8 => self.render_rgba_optimized(framebuffer),
            FramebufferFormat::Bgra8 => self.render_bgra_optimized(framebuffer),
            FramebufferFormat::Rgb8 => self.render_rgb_optimized(framebuffer),
            FramebufferFormat::Bgr8 => self.render_bgr_optimized(framebuffer),
        }
        
        framebuffer.mark_dirty(None);
    }

    #[inline(always)]
    fn render_rgba_optimized(&self, framebuffer: &mut Framebuffer) {
        let width = framebuffer.width as usize;
        let height = framebuffer.height as usize;
        let buffer = &mut framebuffer.buffer;

        // Pre-calculate constants for performance
        let time = self.time_offset;
        
        // Use smaller blocks for smoother visuals while maintaining performance
        let step = 1; // Full resolution for smooth animation
        
        for y in (0..height).step_by(step) {
            let y_norm = y as f32 / height as f32;
            let row_offset = y * width * 4;

            for x in (0..width).step_by(step) {
                let x_norm = x as f32 / width as f32;
                let (r, g, b) = self.calculate_rainbow_color_fast(x_norm, y_norm, time);
                
                let pixel_offset = row_offset + x * 4;
                if pixel_offset + 3 < buffer.len() {
                    buffer[pixel_offset] = r;
                    buffer[pixel_offset + 1] = g;
                    buffer[pixel_offset + 2] = b;
                    buffer[pixel_offset + 3] = 255;
                }
            }
        }
    }

    #[inline(always)]
    fn render_bgra_fast(&self, framebuffer: &mut Framebuffer) {
        let width = framebuffer.width as usize;
        let height = framebuffer.height as usize;
        let buffer = &mut framebuffer.buffer;

        let time = self.time_offset;
        let width_inv = 1.0 / width as f32;
        let height_inv = 1.0 / height as f32;

        for y in 0..height {
            let y_norm = y as f32 * height_inv;
            let row_offset = y * width * 4;

            for x in 0..width {
                let x_norm = x as f32 * width_inv;
                let pixel_offset = row_offset + x * 4;

                if pixel_offset + 3 < buffer.len() {
                    let (r, g, b) = self.calculate_rainbow_color(x_norm, y_norm, time);
                    
                    buffer[pixel_offset] = b;     // B
                    buffer[pixel_offset + 1] = g; // G
                    buffer[pixel_offset + 2] = r; // R
                    buffer[pixel_offset + 3] = 255; // A
                }
            }
        }
    }

    #[inline(always)]
    fn render_rgb_fast(&self, framebuffer: &mut Framebuffer) {
        let width = framebuffer.width as usize;
        let height = framebuffer.height as usize;
        let buffer = &mut framebuffer.buffer;

        let time = self.time_offset;
        let width_inv = 1.0 / width as f32;
        let height_inv = 1.0 / height as f32;

        for y in 0..height {
            let y_norm = y as f32 * height_inv;
            let row_offset = y * width * 3;

            for x in 0..width {
                let x_norm = x as f32 * width_inv;
                let pixel_offset = row_offset + x * 3;

                if pixel_offset + 2 < buffer.len() {
                    let (r, g, b) = self.calculate_rainbow_color(x_norm, y_norm, time);
                    
                    buffer[pixel_offset] = r;
                    buffer[pixel_offset + 1] = g;
                    buffer[pixel_offset + 2] = b;
                }
            }
        }
    }

    #[inline(always)]
    fn render_bgr_fast(&self, framebuffer: &mut Framebuffer) {
        let width = framebuffer.width as usize;
        let height = framebuffer.height as usize;
        let buffer = &mut framebuffer.buffer;

        let time = self.time_offset;
        let width_inv = 1.0 / width as f32;
        let height_inv = 1.0 / height as f32;

        for y in 0..height {
            let y_norm = y as f32 * height_inv;
            let row_offset = y * width * 3;

            for x in 0..width {
                let x_norm = x as f32 * width_inv;
                let pixel_offset = row_offset + x * 3;

                if pixel_offset + 2 < buffer.len() {
                    let (r, g, b) = self.calculate_rainbow_color(x_norm, y_norm, time);
                    
                    buffer[pixel_offset] = b;     // B
                    buffer[pixel_offset + 1] = g; // G
                    buffer[pixel_offset + 2] = r; // R
                }
            }
        }
    }

    #[inline(always)]
    fn calculate_rainbow_color_fast(&self, x: f32, y: f32, time: f32) -> (u8, u8, u8) {
        // Simplified calculation for better performance
        let hue = (x + y + time * 0.1) % (2.0 * std::f32::consts::PI);
        self.hue_to_rgb(hue)
    }

    #[inline(always)]
    fn calculate_rainbow_color(&self, x: f32, y: f32, time: f32) -> (u8, u8, u8) {
        match self.pattern_type {
            RainbowPattern::Waves => self.waves_pattern(x, y, time),
            RainbowPattern::Spiral => self.spiral_pattern(x, y, time),
            RainbowPattern::Plasma => self.plasma_pattern(x, y, time),
            RainbowPattern::Ripples => self.ripples_pattern(x, y, time),
            RainbowPattern::Matrix => self.matrix_pattern(x, y, time),
        }
    }

    #[inline(always)]
    fn waves_pattern(&self, x: f32, y: f32, time: f32) -> (u8, u8, u8) {
        let wave1 = (x * 10.0 + time * 2.0).sin();
        let wave2 = (y * 8.0 + time * 1.5).sin();
        let wave3 = ((x + y) * 6.0 + time * 3.0).sin();
        
        let hue = (wave1 + wave2 + wave3) * 0.5 + time * 0.1;
        self.hue_to_rgb(hue)
    }

    #[inline(always)]
    fn spiral_pattern(&self, x: f32, y: f32, time: f32) -> (u8, u8, u8) {
        let center_x = x - 0.5;
        let center_y = y - 0.5;
        let distance = (center_x * center_x + center_y * center_y).sqrt();
        let angle = center_y.atan2(center_x);
        
        let hue = angle + distance * 10.0 + time;
        self.hue_to_rgb(hue)
    }

    #[inline(always)]
    fn plasma_pattern(&self, x: f32, y: f32, time: f32) -> (u8, u8, u8) {
        let plasma = (x * 16.0 + time).sin() +
                    (y * 16.0 + time * 1.2).sin() +
                    ((x + y) * 16.0 + time * 1.5).sin() +
                    ((x - y) * 8.0 + time * 0.8).sin();
        
        let hue = plasma * 0.5 + time * 0.2;
        self.hue_to_rgb(hue)
    }

    #[inline(always)]
    fn ripples_pattern(&self, x: f32, y: f32, time: f32) -> (u8, u8, u8) {
        let center1_x = 0.3 + (time * 0.5).sin() * 0.2;
        let center1_y = 0.3 + (time * 0.7).cos() * 0.2;
        let center2_x = 0.7 + (time * 0.3).cos() * 0.2;
        let center2_y = 0.7 + (time * 0.4).sin() * 0.2;
        
        let dist1 = ((x - center1_x).powi(2) + (y - center1_y).powi(2)).sqrt();
        let dist2 = ((x - center2_x).powi(2) + (y - center2_y).powi(2)).sqrt();
        
        let ripple1 = (dist1 * 20.0 - time * 3.0).sin();
        let ripple2 = (dist2 * 25.0 - time * 4.0).sin();
        
        let hue = (ripple1 + ripple2) * 0.5 + time * 0.1;
        self.hue_to_rgb(hue)
    }

    #[inline(always)]
    fn matrix_pattern(&self, x: f32, y: f32, time: f32) -> (u8, u8, u8) {
        let grid_x = (x * 20.0).floor();
        let grid_y = (y * 20.0).floor();
        let cell_time = time + (grid_x + grid_y) * 0.1;
        
        let intensity = (cell_time * 5.0).sin() * 0.5 + 0.5;
        let hue = (grid_x * 0.1 + grid_y * 0.15 + time * 0.5) % (2.0 * std::f32::consts::PI);
        
        let (r, g, b) = self.hue_to_rgb(hue);
        (
            (r as f32 * intensity) as u8,
            (g as f32 * intensity) as u8,
            (b as f32 * intensity) as u8,
        )
    }

    #[inline(always)]
    fn hue_to_rgb(&self, hue: f32) -> (u8, u8, u8) {
        let h = (hue % (2.0 * std::f32::consts::PI)) / (2.0 * std::f32::consts::PI) * 6.0;
        let c = 1.0;
        let x = c * (1.0 - ((h % 2.0) - 1.0).abs());
        let m = 0.0;

        let (r, g, b) = if h < 1.0 {
            (c, x, 0.0)
        } else if h < 2.0 {
            (x, c, 0.0)
        } else if h < 3.0 {
            (0.0, c, x)
        } else if h < 4.0 {
            (0.0, x, c)
        } else if h < 5.0 {
            (x, 0.0, c)
        } else {
            (c, 0.0, x)
        };

        (
            ((r + m) * 255.0) as u8,
            ((g + m) * 255.0) as u8,
            ((b + m) * 255.0) as u8,
        )
    }

    #[inline(always)]
    fn render_bgra_optimized(&self, framebuffer: &mut Framebuffer) {
        let width = framebuffer.width as usize;
        let height = framebuffer.height as usize;
        let buffer = &mut framebuffer.buffer;

        let time = self.time_offset;

        for y in 0..height {
            let y_norm = y as f32 / height as f32;
            let row_offset = y * width * 4;

            for x in 0..width {
                let x_norm = x as f32 / width as f32;
                let (r, g, b) = self.calculate_rainbow_color_fast(x_norm, y_norm, time);
                
                let pixel_offset = row_offset + x * 4;
                if pixel_offset + 3 < buffer.len() {
                    buffer[pixel_offset] = b;     // B
                    buffer[pixel_offset + 1] = g; // G
                    buffer[pixel_offset + 2] = r; // R
                    buffer[pixel_offset + 3] = 255; // A
                }
            }
        }
    }

    #[inline(always)]
    fn render_rgb_optimized(&self, framebuffer: &mut Framebuffer) {
        let width = framebuffer.width as usize;
        let height = framebuffer.height as usize;
        let buffer = &mut framebuffer.buffer;

        let time = self.time_offset;

        for y in 0..height {
            let y_norm = y as f32 / height as f32;
            let row_offset = y * width * 3;

            for x in 0..width {
                let x_norm = x as f32 / width as f32;
                let (r, g, b) = self.calculate_rainbow_color_fast(x_norm, y_norm, time);
                
                let pixel_offset = row_offset + x * 3;
                if pixel_offset + 2 < buffer.len() {
                    buffer[pixel_offset] = r;
                    buffer[pixel_offset + 1] = g;
                    buffer[pixel_offset + 2] = b;
                }
            }
        }
    }

    #[inline(always)]
    fn render_bgr_optimized(&self, framebuffer: &mut Framebuffer) {
        let width = framebuffer.width as usize;
        let height = framebuffer.height as usize;
        let buffer = &mut framebuffer.buffer;

        let time = self.time_offset;

        for y in 0..height {
            let y_norm = y as f32 / height as f32;
            let row_offset = y * width * 3;

            for x in 0..width {
                let x_norm = x as f32 / width as f32;
                let (r, g, b) = self.calculate_rainbow_color_fast(x_norm, y_norm, time);
                
                let pixel_offset = row_offset + x * 3;
                if pixel_offset + 2 < buffer.len() {
                    buffer[pixel_offset] = b;     // B
                    buffer[pixel_offset + 1] = g; // G
                    buffer[pixel_offset + 2] = r; // R
                }
            }
        }
    }
}

impl Default for RainbowRenderEngine {
    fn default() -> Self {
        Self::new()
    }
}
