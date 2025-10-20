// DEPRECATED: This file is stale - use GpuRenderer with WgpuViewport instead
use crate::ui::rainbow_engine_final::Framebuffer;
use std::time::Instant;

/// Simple 3D renderer using software rendering
/// This is a placeholder for a full WGPU/Bevy integration
pub struct Wgpu3DRenderer {
    frame_count: u64,
    start_time: Instant,
    camera_pos: [f32; 3],
    camera_rot: [f32; 3],
    time: f32,
}

impl Wgpu3DRenderer {
    pub fn new() -> Self {
        Self {
            frame_count: 0,
            start_time: Instant::now(),
            camera_pos: [0.0, 2.0, -5.0],
            camera_rot: [0.0, 0.0, 0.0],
            time: 0.0,
        }
    }

    pub fn set_camera_position(&mut self, pos: [f32; 3]) {
        self.camera_pos = pos;
    }

    pub fn set_camera_rotation(&mut self, rot: [f32; 3]) {
        self.camera_rot = rot;
    }

    pub fn render(&mut self, framebuffer: &mut Framebuffer) {
        self.frame_count += 1;
        self.time = self.start_time.elapsed().as_secs_f32();

        // Clear to sky blue
        let sky_color = self.get_sky_color();
        framebuffer.clear(sky_color);

        let width = framebuffer.width as i32;
        let height = framebuffer.height as i32;

        // Render a simple 3D scene using raytracing-like technique
        self.render_3d_scene(framebuffer, width, height);

        framebuffer.mark_dirty_all();
    }

    fn get_sky_color(&self) -> [u8; 4] {
        // Gradient sky from light blue at horizon to darker blue at top
        [135, 206, 235, 255]
    }

    fn render_3d_scene(&self, framebuffer: &mut Framebuffer, width: i32, height: i32) {
        let aspect = width as f32 / height as f32;
        let fov = 60.0_f32.to_radians();

        // Render grid floor
        self.render_grid_floor(framebuffer, width, height, aspect, fov);

        // Render a simple rotating cube
        self.render_cube(framebuffer, width, height, aspect, fov);

        // Render coordinate axes
        self.render_axes(framebuffer, width, height, aspect, fov);
    }

    fn render_grid_floor(&self, framebuffer: &mut Framebuffer, width: i32, height: i32, aspect: f32, fov: f32) {
        let grid_size = 10.0;
        let grid_spacing = 1.0;
        
        for y in 0..height {
            for x in 0..width {
                // Convert screen coordinates to normalized device coordinates
                let ndc_x = (x as f32 / width as f32) * 2.0 - 1.0;
                let ndc_y = 1.0 - (y as f32 / height as f32) * 2.0;

                // Create ray direction
                let ray_dir = self.get_ray_direction(ndc_x, ndc_y, aspect, fov);

                // Check if ray hits the ground plane (y = 0)
                if ray_dir[1] < -0.001 {
                    let t = -self.camera_pos[1] / ray_dir[1];
                    if t > 0.0 && t < 100.0 {
                        let hit_x = self.camera_pos[0] + ray_dir[0] * t;
                        let hit_z = self.camera_pos[2] + ray_dir[2] * t;

                        // Check if we're near a grid line
                        let grid_x = (hit_x / grid_spacing).fract().abs();
                        let grid_z = (hit_z / grid_spacing).fract().abs();
                        
                        let line_width = 0.05;
                        if grid_x < line_width || grid_z < line_width {
                            // Grid line color
                            let distance = t;
                            let fade = (1.0 - (distance / 50.0).min(1.0)) * 0.5;
                            let intensity = (128.0 * fade) as u8;
                            let color = [intensity, intensity, intensity, 255];
                            framebuffer.set_pixel(x as u32, y as u32, color);
                        }
                    }
                }
            }
        }
    }

    fn render_cube(&self, framebuffer: &mut Framebuffer, width: i32, height: i32, aspect: f32, fov: f32) {
        let cube_pos = [0.0, 1.0, 0.0];
        let cube_size = 1.0;
        
        // Rotate the cube over time
        let rotation = self.time * 0.5;

        // Define cube vertices
        let vertices = self.get_cube_vertices(cube_pos, cube_size, rotation);

        // Project and render edges
        for edge in self.get_cube_edges().iter() {
            let v1 = vertices[edge.0];
            let v2 = vertices[edge.1];

            // Project both vertices
            if let (Some(p1), Some(p2)) = (
                self.project_point(v1, width, height, aspect, fov),
                self.project_point(v2, width, height, aspect, fov),
            ) {
                self.draw_line(framebuffer, p1, p2, [255, 200, 0, 255]);
            }
        }

        // Draw filled faces (back to front)
        self.render_cube_faces(framebuffer, &vertices, width, height, aspect, fov);
    }

    fn render_cube_faces(&self, framebuffer: &mut Framebuffer, vertices: &[[f32; 3]], width: i32, height: i32, aspect: f32, fov: f32) {
        // Define face colors
        let face_colors = [
            [200, 100, 100, 255], // Red
            [100, 200, 100, 255], // Green
            [100, 100, 200, 255], // Blue
            [200, 200, 100, 255], // Yellow
            [200, 100, 200, 255], // Magenta
            [100, 200, 200, 255], // Cyan
        ];

        // Project all vertices first
        let mut projected: Vec<Option<[i32; 2]>> = vertices.iter()
            .map(|&v| self.project_point(v, width, height, aspect, fov))
            .collect();

        // Render each face
        for (face_idx, face) in self.get_cube_faces().iter().enumerate() {
            // Check if all vertices of this face are visible
            let points: Vec<[i32; 2]> = face.iter()
                .filter_map(|&idx| projected[idx])
                .collect();

            if points.len() == 4 {
                // Fill the face with semi-transparent color
                self.fill_quad(framebuffer, &points, face_colors[face_idx]);
            }
        }
    }

    fn render_axes(&self, framebuffer: &mut Framebuffer, width: i32, height: i32, aspect: f32, fov: f32) {
        let origin = [0.0, 0.0, 0.0];
        let axis_length = 2.0;

        // X axis (red)
        let x_end = [axis_length, 0.0, 0.0];
        if let (Some(p1), Some(p2)) = (
            self.project_point(origin, width, height, aspect, fov),
            self.project_point(x_end, width, height, aspect, fov),
        ) {
            self.draw_line(framebuffer, p1, p2, [255, 0, 0, 255]);
        }

        // Y axis (green)
        let y_end = [0.0, axis_length, 0.0];
        if let (Some(p1), Some(p2)) = (
            self.project_point(origin, width, height, aspect, fov),
            self.project_point(y_end, width, height, aspect, fov),
        ) {
            self.draw_line(framebuffer, p1, p2, [0, 255, 0, 255]);
        }

        // Z axis (blue)
        let z_end = [0.0, 0.0, axis_length];
        if let (Some(p1), Some(p2)) = (
            self.project_point(origin, width, height, aspect, fov),
            self.project_point(z_end, width, height, aspect, fov),
        ) {
            self.draw_line(framebuffer, p1, p2, [0, 0, 255, 255]);
        }
    }

    fn get_ray_direction(&self, ndc_x: f32, ndc_y: f32, aspect: f32, fov: f32) -> [f32; 3] {
        let tan_fov = (fov * 0.5).tan();
        let ray_x = ndc_x * aspect * tan_fov;
        let ray_y = ndc_y * tan_fov;
        let ray_z = 1.0;

        // Normalize
        let len = (ray_x * ray_x + ray_y * ray_y + ray_z * ray_z).sqrt();
        [ray_x / len, ray_y / len, ray_z / len]
    }

    fn get_cube_vertices(&self, pos: [f32; 3], size: f32, rotation: f32) -> Vec<[f32; 3]> {
        let half_size = size / 2.0;
        let cos_r = rotation.cos();
        let sin_r = rotation.sin();

        // Define vertices in local space
        let local_vertices = [
            [-half_size, -half_size, -half_size],
            [half_size, -half_size, -half_size],
            [half_size, half_size, -half_size],
            [-half_size, half_size, -half_size],
            [-half_size, -half_size, half_size],
            [half_size, -half_size, half_size],
            [half_size, half_size, half_size],
            [-half_size, half_size, half_size],
        ];

        // Rotate around Y axis and translate
        local_vertices.iter().map(|&v| {
            let rotated_x = v[0] * cos_r + v[2] * sin_r;
            let rotated_z = -v[0] * sin_r + v[2] * cos_r;
            [pos[0] + rotated_x, pos[1] + v[1], pos[2] + rotated_z]
        }).collect()
    }

    fn get_cube_edges(&self) -> Vec<(usize, usize)> {
        vec![
            // Bottom face
            (0, 1), (1, 2), (2, 3), (3, 0),
            // Top face
            (4, 5), (5, 6), (6, 7), (7, 4),
            // Vertical edges
            (0, 4), (1, 5), (2, 6), (3, 7),
        ]
    }

    fn get_cube_faces(&self) -> Vec<[usize; 4]> {
        vec![
            [0, 1, 2, 3], // Front
            [5, 4, 7, 6], // Back
            [4, 0, 3, 7], // Left
            [1, 5, 6, 2], // Right
            [3, 2, 6, 7], // Top
            [4, 5, 1, 0], // Bottom
        ]
    }

    fn project_point(&self, point: [f32; 3], width: i32, height: i32, aspect: f32, fov: f32) -> Option<[i32; 2]> {
        // Transform to camera space
        let rel_x = point[0] - self.camera_pos[0];
        let rel_y = point[1] - self.camera_pos[1];
        let rel_z = point[2] - self.camera_pos[2];

        // Simple perspective projection (no rotation applied for now)
        let z = rel_z;
        
        if z <= 0.1 {
            return None; // Behind camera
        }

        let tan_fov = (fov * 0.5).tan();
        let x_ndc = (rel_x / z) / (aspect * tan_fov);
        let y_ndc = (rel_y / z) / tan_fov;

        // Convert to screen coordinates
        let screen_x = ((x_ndc + 1.0) * 0.5 * width as f32) as i32;
        let screen_y = ((1.0 - y_ndc) * 0.5 * height as f32) as i32;

        if screen_x >= 0 && screen_x < width && screen_y >= 0 && screen_y < height {
            Some([screen_x, screen_y])
        } else {
            None
        }
    }

    fn draw_line(&self, framebuffer: &mut Framebuffer, p1: [i32; 2], p2: [i32; 2], color: [u8; 4]) {
        // Bresenham's line algorithm
        let mut x0 = p1[0];
        let mut y0 = p1[1];
        let x1 = p2[0];
        let y1 = p2[1];

        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx - dy;

        loop {
            if x0 >= 0 && x0 < framebuffer.width as i32 && y0 >= 0 && y0 < framebuffer.height as i32 {
                framebuffer.set_pixel(x0 as u32, y0 as u32, color);
            }

            if x0 == x1 && y0 == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x0 += sx;
            }
            if e2 < dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    fn fill_quad(&self, framebuffer: &mut Framebuffer, points: &[[i32; 2]], color: [u8; 4]) {
        if points.len() != 4 {
            return;
        }

        // Find bounding box
        let min_x = points.iter().map(|p| p[0]).min().unwrap_or(0).max(0);
        let max_x = points.iter().map(|p| p[0]).max().unwrap_or(0).min(framebuffer.width as i32 - 1);
        let min_y = points.iter().map(|p| p[1]).min().unwrap_or(0).max(0);
        let max_y = points.iter().map(|p| p[1]).max().unwrap_or(0).min(framebuffer.height as i32 - 1);

        // Simple scanline fill
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                // Check if point is inside quad using cross product
                if self.point_in_quad([x, y], points) {
                    // Blend with existing color for semi-transparency
                    let existing = framebuffer.get_pixel(x as u32, y as u32);
                    let alpha = 0.7;
                    let blended = [
                        ((color[0] as f32 * alpha + existing[0] as f32 * (1.0 - alpha)) as u8),
                        ((color[1] as f32 * alpha + existing[1] as f32 * (1.0 - alpha)) as u8),
                        ((color[2] as f32 * alpha + existing[2] as f32 * (1.0 - alpha)) as u8),
                        255,
                    ];
                    framebuffer.set_pixel(x as u32, y as u32, blended);
                }
            }
        }
    }

    fn point_in_quad(&self, point: [i32; 2], quad: &[[i32; 2]]) -> bool {
        // Use winding number algorithm
        let mut winding = 0;
        let n = quad.len();

        for i in 0..n {
            let p1 = quad[i];
            let p2 = quad[(i + 1) % n];

            if p1[1] <= point[1] {
                if p2[1] > point[1] {
                    if self.is_left(p1, p2, point) > 0 {
                        winding += 1;
                    }
                }
            } else {
                if p2[1] <= point[1] {
                    if self.is_left(p1, p2, point) < 0 {
                        winding -= 1;
                    }
                }
            }
        }

        winding != 0
    }

    fn is_left(&self, p0: [i32; 2], p1: [i32; 2], p2: [i32; 2]) -> i32 {
        (p1[0] - p0[0]) * (p2[1] - p0[1]) - (p2[0] - p0[0]) * (p1[1] - p0[1])
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
}
