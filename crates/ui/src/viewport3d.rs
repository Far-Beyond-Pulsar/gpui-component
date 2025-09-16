use std::time::Instant;
use wgpu::util::DeviceExt;

use bytemuck::{Pod, Zeroable};

use gpui::{
    canvas, div, App, Bounds, ContentMask, DismissEvent, Element, ElementId, Entity, EventEmitter,
    FocusHandle, Focusable, GlobalElementId, Hitbox, InteractiveElement, IntoElement, LayoutId,
    MouseDownEvent, ParentElement as _, Pixels, Render, Size, Style, Styled as _, Window,
};

/// Represents a 3D viewport UI component.
pub struct Viewport3D {
    focus_handle: FocusHandle,
    visible: bool,
    bounds: Bounds<Pixels>,
    renderer: Option<Renderer3D>,
    last_frame: Instant,
}


impl Viewport3D {
    pub fn new<T>(cx: &mut gpui::Context<T>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            visible: true,
            bounds: Bounds::default(),
            renderer: None,
            last_frame: Instant::now(),
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn visible(&self) -> bool {
        self.visible
    }

    pub fn bounds(&self) -> Bounds<Pixels> {
        self.bounds
    }

    // Add 3D-specific methods here, e.g., camera controls, scene loading, etc.
}

/// Minimal 3D renderer using wgpu for a rotating triangle demo
pub struct Renderer3D {
    device: wgpu::Device,
    queue: wgpu::Queue,
    texture: wgpu::Texture,
    texture_view: wgpu::TextureView,
    size: (u32, u32),
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
    start_time: Instant,
    format: wgpu::TextureFormat,
}

impl Renderer3D {
    pub async fn new(
        width: u32,
        height: u32,
    ) -> Self {
        let instance = wgpu::Instance::default();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .unwrap();
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default(), None)
            .await
            .unwrap();
        let format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Offscreen Render Target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Simple colored triangle

        #[repr(C)]
        #[derive(Copy, Clone, Debug, Pod, Zeroable)]
        struct Vertex {
            position: [f32; 3],
            color: [f32; 3],
        }
        let vertices = [
            Vertex { position: [0.0, 0.5, 0.0], color: [1.0, 0.0, 0.0] },
            Vertex { position: [-0.5, -0.5, 0.0], color: [0.0, 1.0, 0.0] },
            Vertex { position: [0.5, -0.5, 0.0], color: [0.0, 0.0, 1.0] },
        ];
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Triangle Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader_triangle.wgsl").into()),
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: 6 * 4,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                        wgpu::VertexAttribute {
                            offset: 3 * 4,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x3,
                        },
                    ],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        Self {
            device,
            queue,
            texture,
            texture_view,
            size: (width, height),
            render_pipeline,
            vertex_buffer,
            num_vertices: 3,
            start_time: Instant::now(),
            format,
        }
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.size = (width, height);
        self.texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Offscreen Render Target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        self.texture_view = self.texture.create_view(&wgpu::TextureViewDescriptor::default());
    }

    pub fn render(&mut self) {
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        let _elapsed = self.start_time.elapsed().as_secs_f32();
        // TODO: Pass rotation to shader for animation
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.texture_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.1,
                            b: 0.2,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            rpass.set_pipeline(&self.render_pipeline);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.draw(0..self.num_vertices, 0..1);
        }
        self.queue.submit(Some(encoder.finish()));
    }
}


impl Focusable for Viewport3D {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for Viewport3D {}

impl Render for Viewport3D {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let view = cx.entity().clone();
        div()
            .track_focus(&self.focus_handle)
            .size_full()
            .child({
                let view1 = view.clone();
                let view2 = view.clone();
                canvas(
                    move |bounds, _window_handle, cx| {
                        view1.update(cx, |r, _| r.bounds = bounds);
                        // Initialize renderer if needed
                        let (width, height) = (bounds.size.width.0 as u32, bounds.size.height.0 as u32);
                        let needs_init = {
                            let r = view1.read(cx);
                            r.renderer.is_none()
                        };
                        if needs_init {
                            let fut = async move {
                                Renderer3D::new(width, height).await
                            };
                            let renderer = futures::executor::block_on(fut);
                            view1.update(cx, |r, _| r.renderer = Some(renderer));
                        } else {
                            view1.update(cx, |r, _| {
                                if let Some(renderer) = &mut r.renderer {
                                    renderer.resize(width, height);
                                }
                            });
                        }
                    },
                    move |_, _, _, app| {
                        // Draw the 3D scene
                        view2.update(app, |r, _| {
                            if let Some(renderer) = &mut r.renderer {
                                renderer.render();
                            }
                        });
                    },
                )
                .absolute()
                .size_full()
            })
            .child(Viewport3DElement::new(view, window, cx))
    }
}

/// A UI element for the 3D viewport.
pub struct Viewport3DElement {
    parent: Entity<Viewport3D>,
}

impl Viewport3DElement {
    pub fn new(
        parent: Entity<Viewport3D>,
        _window: &mut Window,
        _cx: &mut App,
    ) -> Self {
        Self { parent }
    }
}

impl IntoElement for Viewport3DElement {
    type Element = Viewport3DElement;
    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for Viewport3DElement {
    type RequestLayoutState = ();
    type PrepaintState = Option<Hitbox>;

    fn id(&self) -> Option<ElementId> { None }
    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> { None }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.flex_grow = 0.0;
        style.flex_shrink = 1.;
        style.size = Size::full();
        let id = window.request_layout(style, [], cx);
        (id, ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        if !self.parent.read(cx).visible() {
            return None;
        }
        // Set up 3D viewport bounds here if needed
        Some(window.insert_hitbox(bounds, gpui::HitboxBehavior::Normal))
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        hitbox: &mut Self::PrepaintState,
        window: &mut Window,
        _: &mut App,
    ) {
        let bounds = hitbox.clone().map(|h| h.bounds).unwrap_or(bounds);
        window.with_content_mask(Some(ContentMask { bounds }), |window| {
            // Handle mouse/interaction for 3D viewport here
            window.on_mouse_event(move |event: &MouseDownEvent, _, _, _| {
                if !bounds.contains(&event.position) {
                    // Click outside to blur focus, if needed
                }
            });
        });
    }
}
