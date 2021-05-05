use crate::{FrameEncoder, GraphicsDevice};
use bytemuck::{Pod, Zeroable};
use wgpu::{util::DeviceExt, BindGroup, Buffer, RenderPipeline};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FullscreenQuadVertex {
    pos: [f32; 2],
    uv: [f32; 2],
}

pub struct FullscreenQuad {
    vertex_buf: Buffer,
    index_buf: Buffer,
    bind_group: BindGroup,
    pipeline: RenderPipeline,
}

impl FullscreenQuad {
    pub fn new(graphics_device: &GraphicsDevice) -> Self {
        let vertex_data = vec![
            FullscreenQuadVertex { pos: [-1.0, -1.0], uv: [0.0, 1.0] },
            FullscreenQuadVertex { pos: [-1.0, 1.0], uv: [0.0, 0.0] },
            FullscreenQuadVertex { pos: [1.0, 1.0], uv: [1.0, 0.0] },
            FullscreenQuadVertex { pos: [1.0, -1.0], uv: [1.0, 1.0] },
        ];

        let index_data = vec![0u16, 1, 3, 2];

        let device = graphics_device.device();

        let vertex_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsage::VERTEX,
        });

        let index_buf = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsage::INDEX,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("TexturedQuad bind group layout"),
            entries: &[],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("TexturedQuad pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("TexturedQuad bind group"),
            layout: &bind_group_layout,
            entries: &[],
        });

        let vertex_buffers = &[wgpu::VertexBufferLayout {
            array_stride: (std::mem::size_of::<FullscreenQuadVertex>()) as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x2, // pos
                1 => Float32x2, // uv
            ],
        }];

        let draw_shader = device
            .create_shader_module(&wgpu::include_spirv!("shaders/compiled/fullscreen_quad.spv"));

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("TexturedQuad render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: "vs_main",
                buffers: vertex_buffers,
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: Some(wgpu::IndexFormat::Uint16),
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                clamp_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: graphics_device.swap_chain_descriptor().format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrite::ALL,
                }],
            }),
        });

        Self { vertex_buf, index_buf, pipeline, bind_group }
    }

    pub fn render(&self, frame_encoder: &mut FrameEncoder) {
        let frame = &frame_encoder.frame;
        let encoder = &mut frame_encoder.encoder;

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("TexturedQuad render pass"),
            color_attachments: &[wgpu::RenderPassColorAttachment {
                view: &frame.view,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: true },
            }],
            depth_stencil_attachment: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_index_buffer(self.index_buf.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.set_vertex_buffer(0, self.vertex_buf.slice(..));
        render_pass.draw_indexed(0..4u32, 0, 0..1);
    }
}
