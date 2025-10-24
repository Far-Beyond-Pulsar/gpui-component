use wgpu::util::DeviceExt;
use super::Framebuffer;

/// High-performance WGPU-based 3D renderer
/// Renders directly to a texture that can be copied to the framebuffer
pub struct WgpuRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    render_texture: wgpu::Texture,
    render_texture_view: wgpu::TextureView,
    depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,
    render_pipeline: wgpu::RenderPipeline,
    
    // Scene data
    cube_vertex_buffer: wgpu::Buffer,
    cube_index_buffer: wgpu::Buffer,
    cube_num_indices: u32,
    
    grid_vertex_buffer: wgpu::Buffer,
    grid_index_buffer: wgpu::Buffer,
    grid_num_indices: u32,
    
    axes_vertex_buffer: wgpu::Buffer,
    axes_index_buffer: wgpu::Buffer,
    axes_num_indices: u32,
    
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    
    width: u32,
    height: u32,
    frame_count: u64,
    time: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
    normal: [f32; 3],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [[f32; 4]; 4],
    model: [[f32; 4]; 4],
    time: f32,
    _padding: [f32; 3],
}

impl WgpuRenderer {
    pub async fn new(width: u32, height: u32) -> Self {
        // Create WGPU instance
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        // Request adapter
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find an appropriate adapter");

        // Create device and queue
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Pulsar 3D Renderer"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        // Create render texture
        let render_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Render Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let render_texture_view = render_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create depth texture
        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let depth_texture_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/basic_3d.wgsl").into()),
        });

        // Create uniform buffer
        let uniforms = Uniforms {
            view_proj: [[0.0; 4]; 4],
            model: [[0.0; 4]; 4],
            time: 0.0,
            _padding: [0.0; 3],
        };

        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::cast_slice(&[uniforms]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Uniform Bind Group Layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        // Create bind group
        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Uniform Bind Group"),
            layout: &uniform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Create pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&uniform_bind_group_layout],
            push_constant_ranges: &[],
        });

        // Create render pipeline
        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: wgpu::TextureFormat::Rgba8UnormSrgb,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        // Create geometry
        let (cube_vertex_buffer, cube_index_buffer, cube_num_indices) =
            Self::create_cube_geometry(&device);
        let (grid_vertex_buffer, grid_index_buffer, grid_num_indices) =
            Self::create_grid_geometry(&device);
        let (axes_vertex_buffer, axes_index_buffer, axes_num_indices) =
            Self::create_axes_geometry(&device);

        Self {
            device,
            queue,
            render_texture,
            render_texture_view,
            depth_texture,
            depth_texture_view,
            render_pipeline,
            cube_vertex_buffer,
            cube_index_buffer,
            cube_num_indices,
            grid_vertex_buffer,
            grid_index_buffer,
            grid_num_indices,
            axes_vertex_buffer,
            axes_index_buffer,
            axes_num_indices,
            uniform_buffer,
            uniform_bind_group,
            width,
            height,
            frame_count: 0,
            time: 0.0,
        }
    }

    pub fn render(&mut self, framebuffer: &mut Framebuffer) {
        self.frame_count += 1;
        self.time += 0.016; // ~60 FPS time step

        // Update uniforms
        let aspect = self.width as f32 / self.height as f32;
        let view_proj = self.create_view_projection_matrix(aspect);
        let model = self.create_model_matrix();

        let uniforms = Uniforms {
            view_proj,
            model,
            time: self.time,
            _padding: [0.0; 3],
        };

        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Render pass
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.render_texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.53,
                            g: 0.81,
                            b: 0.92,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture_view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);

            // Draw grid
            render_pass.set_vertex_buffer(0, self.grid_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.grid_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.grid_num_indices, 0, 0..1);

            // Draw axes
            render_pass.set_vertex_buffer(0, self.axes_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.axes_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.axes_num_indices, 0, 0..1);

            // Draw cube
            render_pass.set_vertex_buffer(0, self.cube_vertex_buffer.slice(..));
            render_pass.set_index_buffer(self.cube_index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..self.cube_num_indices, 0, 0..1);
        }

        // Submit commands
        self.queue.submit(std::iter::once(encoder.finish()));

        // Copy texture to framebuffer
        self.copy_texture_to_framebuffer(framebuffer);
    }

    fn copy_texture_to_framebuffer(&self, framebuffer: &mut Framebuffer) {        
        // WGPU requires bytes_per_row to be aligned to COPY_BYTES_PER_ROW_ALIGNMENT (256)
        let unpadded_bytes_per_row = 4 * self.width;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = ((unpadded_bytes_per_row + align - 1) / align) * align;
                
        // Create a buffer to copy the texture data to
        let buffer_size = (padded_bytes_per_row * self.height) as wgpu::BufferAddress;
        let output_buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output Buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Copy Encoder"),
            });

        encoder.copy_texture_to_buffer(
            wgpu::TextureCopyView {
                texture: &self.render_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::BufferCopyView {
                buffer: &output_buffer,
                layout: wgpu::TextureDataLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(self.height),
                },
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        self.queue.submit(std::iter::once(encoder.finish()));

        // Map the buffer and copy to framebuffer
        let buffer_slice = output_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });

        self.device.poll(wgpu::MaintainBase::Wait);
        
        match rx.recv() {
            Ok(Ok(())) => {
                {
                    let data = buffer_slice.get_mapped_range();
                    // Copy row by row, skipping padding
                    for y in 0..self.height as usize {
                        let src_offset = y * padded_bytes_per_row as usize;
                        let dst_offset = y * unpadded_bytes_per_row as usize;
                        let row_bytes = unpadded_bytes_per_row as usize;
                        
                        framebuffer.buffer[dst_offset..dst_offset + row_bytes]
                            .copy_from_slice(&data[src_offset..src_offset + row_bytes]);
                    }
                    
                }
                output_buffer.unmap();
            }
            Ok(Err(_e)) => {
                eprintln!("[WGPU-RENDERER] Buffer mapping failed");
            }
            Err(_e) => {
                eprintln!("[WGPU-RENDERER] Failed to receive map_async message");
            }
        }
    }

    fn create_view_projection_matrix(&self, _aspect: f32) -> [[f32; 4]; 4] {
        // Simple perspective projection
        let _fov = 60.0_f32.to_radians();
        let _near = 0.1;
        let _far = 100.0;

        // View matrix (camera at (0, 2, 5) looking at origin)
        let _eye = [0.0, 2.0, 5.0];
        let _target = [0.0, 0.0, 0.0];
        let _up = [0.0, 1.0, 0.0];

        // For now, return identity - we'll implement proper matrix math
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ]
    }

    fn create_model_matrix(&self) -> [[f32; 4]; 4] {
        // Rotation matrix for the cube
        let angle = self.time * 0.5;
        let cos = angle.cos();
        let sin = angle.sin();

        [
            [cos, 0.0, sin, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [-sin, 0.0, cos, 0.0],
            [0.0, 1.0, 0.0, 1.0],
        ]
    }

    fn create_cube_geometry(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        // Cube vertices with colors
        let vertices = &[
            // Front face (red)
            Vertex { position: [-0.5, -0.5, 0.5], color: [1.0, 0.0, 0.0], normal: [0.0, 0.0, 1.0] },
            Vertex { position: [0.5, -0.5, 0.5], color: [1.0, 0.0, 0.0], normal: [0.0, 0.0, 1.0] },
            Vertex { position: [0.5, 0.5, 0.5], color: [1.0, 0.0, 0.0], normal: [0.0, 0.0, 1.0] },
            Vertex { position: [-0.5, 0.5, 0.5], color: [1.0, 0.0, 0.0], normal: [0.0, 0.0, 1.0] },
            // Back face (green)
            Vertex { position: [-0.5, -0.5, -0.5], color: [0.0, 1.0, 0.0], normal: [0.0, 0.0, -1.0] },
            Vertex { position: [0.5, -0.5, -0.5], color: [0.0, 1.0, 0.0], normal: [0.0, 0.0, -1.0] },
            Vertex { position: [0.5, 0.5, -0.5], color: [0.0, 1.0, 0.0], normal: [0.0, 0.0, -1.0] },
            Vertex { position: [-0.5, 0.5, -0.5], color: [0.0, 1.0, 0.0], normal: [0.0, 0.0, -1.0] },
        ];

        let indices: &[u16] = &[
            0, 1, 2, 2, 3, 0, // Front
            5, 4, 7, 7, 6, 5, // Back
            4, 0, 3, 3, 7, 4, // Left
            1, 5, 6, 6, 2, 1, // Right
            3, 2, 6, 6, 7, 3, // Top
            4, 5, 1, 1, 0, 4, // Bottom
        ];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Cube Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (vertex_buffer, index_buffer, indices.len() as u32)
    }

    fn create_grid_geometry(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let grid_size = 10.0;
        let grid_divisions = 10;
        let step = grid_size / grid_divisions as f32;

        let mut index = 0;
        for i in 0..=grid_divisions {
            let pos = -grid_size / 2.0 + i as f32 * step;
            
            // X-axis line
            vertices.push(Vertex {
                position: [-grid_size / 2.0, 0.0, pos],
                color: [0.5, 0.5, 0.5],
                normal: [0.0, 1.0, 0.0],
            });
            vertices.push(Vertex {
                position: [grid_size / 2.0, 0.0, pos],
                color: [0.5, 0.5, 0.5],
                normal: [0.0, 1.0, 0.0],
            });
            indices.push(index);
            indices.push(index + 1);
            index += 2;

            // Z-axis line
            vertices.push(Vertex {
                position: [pos, 0.0, -grid_size / 2.0],
                color: [0.5, 0.5, 0.5],
                normal: [0.0, 1.0, 0.0],
            });
            vertices.push(Vertex {
                position: [pos, 0.0, grid_size / 2.0],
                color: [0.5, 0.5, 0.5],
                normal: [0.0, 1.0, 0.0],
            });
            indices.push(index);
            indices.push(index + 1);
            index += 2;
        }

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid Index Buffer"),
            contents: bytemuck::cast_slice(&indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (vertex_buffer, index_buffer, indices.len() as u32)
    }

    fn create_axes_geometry(device: &wgpu::Device) -> (wgpu::Buffer, wgpu::Buffer, u32) {
        let vertices = &[
            // X axis (red)
            Vertex { position: [0.0, 0.0, 0.0], color: [1.0, 0.0, 0.0], normal: [0.0, 1.0, 0.0] },
            Vertex { position: [2.0, 0.0, 0.0], color: [1.0, 0.0, 0.0], normal: [0.0, 1.0, 0.0] },
            // Y axis (green)
            Vertex { position: [0.0, 0.0, 0.0], color: [0.0, 1.0, 0.0], normal: [0.0, 1.0, 0.0] },
            Vertex { position: [0.0, 2.0, 0.0], color: [0.0, 1.0, 0.0], normal: [0.0, 1.0, 0.0] },
            // Z axis (blue)
            Vertex { position: [0.0, 0.0, 0.0], color: [0.0, 0.0, 1.0], normal: [0.0, 1.0, 0.0] },
            Vertex { position: [0.0, 0.0, 2.0], color: [0.0, 0.0, 1.0], normal: [0.0, 1.0, 0.0] },
        ];

        let indices: &[u16] = &[0, 1, 2, 3, 4, 5];

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Axes Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Axes Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        (vertex_buffer, index_buffer, indices.len() as u32)
    }

    pub fn get_frame_count(&self) -> u64 {
        self.frame_count
    }

    pub fn get_fps(&self) -> f32 {
        // Calculate FPS based on frame count and time
        if self.time > 0.0 {
            self.frame_count as f32 / self.time
        } else {
            0.0
        }
    }
}

