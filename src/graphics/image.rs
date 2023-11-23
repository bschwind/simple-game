use crate::{graphics::screen_projection_matrix, GraphicsDevice};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2};
use wgpu::{util::DeviceExt, BindGroup, BindGroupLayout, RenderPipeline};

pub struct Image {
    _width: usize,
    _height: usize,
    _texture: wgpu::Texture,
    vertex_buffer: wgpu::Buffer,
    _bind_group_layout: BindGroupLayout,
    bind_group: BindGroup,
}

impl Image {
    pub fn from_png(png_bytes: &[u8], device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let (header, image_data) = png_decoder::decode(png_bytes).expect("Invalid PNG bytes");
        let width = header.width;
        let height = header.height;

        let glyph_texture_extent = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };

        let texture_descriptor = wgpu::TextureDescriptor {
            label: Some("Image::from_png"),
            size: glyph_texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            view_formats: &[],
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        };

        let texture = device.create_texture_with_data(queue, &texture_descriptor, &image_data);
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let vertex_data = vec![
            ImageQuadVertex { pos: [0.0, height as f32], uv: [0.0, 1.0] },
            ImageQuadVertex { pos: [0.0, 0.0], uv: [0.0, 0.0] },
            ImageQuadVertex { pos: [width as f32, 0.0], uv: [1.0, 0.0] },
            ImageQuadVertex { pos: [width as f32, height as f32], uv: [1.0, 1.0] },
        ];
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Image Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GlyphPainter bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Image::from_png bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            _width: header.width as usize,
            _height: header.height as usize,
            _texture: texture,
            vertex_buffer,
            bind_group,
            _bind_group_layout: bind_group_layout,
        }
    }

    pub fn bind_group(&self) -> &BindGroup {
        &self.bind_group
    }
}

struct Buffers {
    vertex_uniform: wgpu::Buffer,
    index: wgpu::Buffer,
}

struct BindGroups {
    vertex_uniform: wgpu::BindGroup,
}

pub struct ImageDrawer {
    image_pipeline: RenderPipeline,
    buffers: Buffers,
    bind_groups: BindGroups,
    projection: Mat4,
}

impl ImageDrawer {
    pub fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        screen_width: u32,
        screen_height: u32,
    ) -> Self {
        let image_pipeline = Self::build_pipeline(device, target_format);
        let buffers = Self::build_buffers(device);
        let bind_groups = Self::build_bind_groups(device, &image_pipeline, &buffers);
        let projection = screen_projection_matrix(screen_width, screen_height);

        Self { image_pipeline, buffers, bind_groups, projection }
    }

    pub fn resize(&mut self, screen_width: u32, screen_height: u32) {
        self.projection = screen_projection_matrix(screen_width, screen_height);
    }

    pub fn begin(&mut self) -> ImageRecorder {
        ImageRecorder { image_drawer: self, images: vec![] }
    }

    fn build_pipeline(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> RenderPipeline {
        let draw_shader =
            GraphicsDevice::load_wgsl_shader(device, include_str!("shaders/wgsl/image.wgsl"));

        let vertex_uniform_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Mat4>() as u64),
                    },
                    count: None,
                }],
                label: None,
            });

        let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: None,
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("image renderer"),
                bind_group_layouts: &[
                    &vertex_uniform_bind_group_layout,
                    &texture_bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: "main_vs",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ImageQuadVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2,
                        1 => Float32x2,
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "main_fs",
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                        alpha: wgpu::BlendComponent {
                            src_factor: wgpu::BlendFactor::SrcAlpha,
                            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
                            operation: wgpu::BlendOperation::Add,
                        },
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint16),
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        })
    }

    fn build_buffers(device: &wgpu::Device) -> Buffers {
        let index_data = [0u16, 1, 3, 2];
        let index = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Image Index Buffer"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX,
        });

        Buffers { vertex_uniform: Self::build_vertex_uniform_buffer(device), index }
    }

    fn build_vertex_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle system vertex shader uniform buffer"),
            size: std::mem::size_of::<Mat4>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn build_bind_groups(
        device: &wgpu::Device,
        render_pipeline: &RenderPipeline,
        buffers: &Buffers,
    ) -> BindGroups {
        let vertex_uniform = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &render_pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffers.vertex_uniform.as_entire_binding(),
            }],
            label: None,
        });

        BindGroups { vertex_uniform }
    }
}

struct PositionedImage<'a> {
    image: &'a Image,
    _pos: Vec2,
}

pub struct ImageRecorder<'a> {
    image_drawer: &'a mut ImageDrawer,
    images: Vec<PositionedImage<'a>>,
}

impl<'a> ImageRecorder<'a> {
    pub fn draw_image(&mut self, image: &'a Image, pos: Vec2) {
        self.images.push(PositionedImage { image, _pos: pos });
    }

    pub fn end(
        self,
        encoder: &mut wgpu::CommandEncoder,
        render_target: &wgpu::TextureView,
        queue: &wgpu::Queue,
    ) {
        queue.write_buffer(
            &self.image_drawer.buffers.vertex_uniform,
            0,
            bytemuck::cast_slice(self.image_drawer.projection.as_ref()),
        );

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ImageRecorder render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: render_target,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.image_drawer.image_pipeline);
        render_pass.set_bind_group(0, &self.image_drawer.bind_groups.vertex_uniform, &[]);
        render_pass
            .set_index_buffer(self.image_drawer.buffers.index.slice(..), wgpu::IndexFormat::Uint16);

        for image in self.images {
            render_pass.set_vertex_buffer(0, image.image.vertex_buffer.slice(..));
            render_pass.set_bind_group(1, image.image.bind_group(), &[]);
            render_pass.draw_indexed(0..4u32, 0, 0..1);
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct ImageQuadVertex {
    /// XY position of the top left of the image in pixels
    pos: [f32; 2],
    /// UV coordinates of the image.
    uv: [f32; 2],
}
