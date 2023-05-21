use crate::{FrameEncoder, GraphicsDevice};
use bytemuck::{Pod, Zeroable};
use glam::{vec4, Mat4, Vec3, Vec4};
use wgpu::util::DeviceExt;

struct Buffers {
    vertex_uniform: wgpu::Buffer,
    round_strip_geometry: wgpu::Buffer,
    round_strip_geometry_len: usize,
    round_strip_instances: wgpu::Buffer,
}

struct BindGroups {
    vertex_uniform: wgpu::BindGroup,
}

pub struct LineDrawer {
    round_line_strip_pipeline: wgpu::RenderPipeline,
    buffers: Buffers,
    bind_groups: BindGroups,
    round_line_strips: Vec<LineVertex3>,
    round_line_strip_indices: Vec<usize>,
}

impl LineDrawer {
    pub fn new(graphics_device: &GraphicsDevice) -> Self {
        let round_line_strip_pipeline = Self::build_round_line_strip_pipeline(graphics_device);

        let buffers = Self::build_buffers(graphics_device);
        let bind_groups =
            Self::build_bind_groups(graphics_device, &round_line_strip_pipeline, &buffers);

        Self {
            round_line_strip_pipeline,
            buffers,
            bind_groups,
            round_line_strips: Vec::new(),
            round_line_strip_indices: Vec::new(),
        }
    }

    pub fn begin(&mut self) -> LineRecorder {
        self.round_line_strips.clear();
        self.round_line_strip_indices.clear();

        LineRecorder { line_drawer: self }
    }

    fn build_round_line_strip_pipeline(graphics_device: &GraphicsDevice) -> wgpu::RenderPipeline {
        let device = graphics_device.device();

        let draw_shader =
            graphics_device.load_wgsl_shader(include_str!("shaders/wgsl/round_line_strip.wgsl"));

        let vertex_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<LineUniforms>() as u64,
                        ),
                    },
                    count: None,
                }],
                label: None,
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Round line strip renderer"),
                bind_group_layouts: &[&vertex_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: "main_vs",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<RoundLineStripVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Float32x3, // XY position of this particular vertex, with Z indicating sides.
                        ],
                    },
                    wgpu::VertexBufferLayout {
                        // The stride is one LineVertex3 here intentionally.
                        array_stride: std::mem::size_of::<LineVertex3>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            1 => Float32x4, // Point A
                            2 => Float32x4, // Point B
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "main_fs",
                targets: &[Some(graphics_device.surface_config().format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Front), // TODO - figure out culling
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        render_pipeline
    }

    fn build_bind_groups(
        graphics_device: &GraphicsDevice,
        render_pipeline: &wgpu::RenderPipeline,
        buffers: &Buffers,
    ) -> BindGroups {
        let device = graphics_device.device();

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

    fn build_buffers(graphics_device: &GraphicsDevice) -> Buffers {
        const MAX_LINES: u64 = 40_000;
        const CIRCLE_RESOLUTION: usize = 30;

        let device = graphics_device.device();

        // Uniform buffer
        let line_uniforms = LineUniforms::default();

        let vertex_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Line drawer vertex shader uniform buffer"),
            contents: bytemuck::bytes_of(&line_uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Round strip geometry
        let mut round_strip_vertices = vec![
            RoundLineStripVertex { pos: [0.0, -0.5, 0.0] },
            RoundLineStripVertex { pos: [0.0, 0.5, 0.0] },
            RoundLineStripVertex { pos: [0.0, 0.5, 1.0] },
            RoundLineStripVertex { pos: [0.0, -0.5, 0.0] },
            RoundLineStripVertex { pos: [0.0, 0.5, 1.0] },
            RoundLineStripVertex { pos: [0.0, -0.5, 1.0] },
        ];

        // Left circle cap
        for i in 0..CIRCLE_RESOLUTION {
            let frac_1 = (std::f32::consts::PI / 2.0)
                + (i as f32 / CIRCLE_RESOLUTION as f32) * std::f32::consts::PI;
            let frac_2 = (std::f32::consts::PI / 2.0)
                + ((i + 1) as f32 / CIRCLE_RESOLUTION as f32) * std::f32::consts::PI;

            round_strip_vertices.push(RoundLineStripVertex { pos: [0.0, 0.0, 0.0] });
            round_strip_vertices
                .push(RoundLineStripVertex { pos: [0.5 * frac_2.cos(), 0.5 * frac_2.sin(), 0.0] });
            round_strip_vertices
                .push(RoundLineStripVertex { pos: [0.5 * frac_1.cos(), 0.5 * frac_1.sin(), 0.0] });
        }

        // Right circle cap
        for i in 0..CIRCLE_RESOLUTION {
            let frac_1 = (3.0 * std::f32::consts::PI / 2.0)
                + (i as f32 / CIRCLE_RESOLUTION as f32) * std::f32::consts::PI;
            let frac_2 = (3.0 * std::f32::consts::PI / 2.0)
                + ((i + 1) as f32 / CIRCLE_RESOLUTION as f32) * std::f32::consts::PI;

            round_strip_vertices.push(RoundLineStripVertex { pos: [0.0, 0.0, 1.0] });
            round_strip_vertices
                .push(RoundLineStripVertex { pos: [0.5 * frac_2.cos(), 0.5 * frac_2.sin(), 1.0] });
            round_strip_vertices
                .push(RoundLineStripVertex { pos: [0.5 * frac_1.cos(), 0.5 * frac_1.sin(), 1.0] });
        }

        let round_strip_geometry = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Round line segment geometry buffer"),
            contents: bytemuck::cast_slice(&round_strip_vertices),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        // Round strip instances
        let round_strip_instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Line strip instance buffer"),
            size: MAX_LINES * std::mem::size_of::<LineVertex3>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Buffers {
            vertex_uniform,
            round_strip_geometry,
            round_strip_geometry_len: round_strip_vertices.len(),
            round_strip_instances,
        }
    }
}

pub struct LineRecorder<'a> {
    line_drawer: &'a mut LineDrawer,
}

impl LineRecorder<'_> {
    /// A special-case where round line joins and caps are desired. This can be achieved
    /// with a single draw call.
    pub fn draw_round_line_strip(&mut self, positions: &[LineVertex3]) {
        self.line_drawer.round_line_strips.extend_from_slice(positions);
        self.line_drawer.round_line_strip_indices.push(positions.len());
    }

    pub fn end(self, frame_encoder: &mut FrameEncoder, camera_matrix: Mat4) {
        let (width, height) = frame_encoder.surface_dimensions();

        let queue = frame_encoder.queue();

        queue.write_buffer(
            &self.line_drawer.buffers.round_strip_instances,
            0,
            bytemuck::cast_slice(&self.line_drawer.round_line_strips),
        );

        let uniforms = LineUniforms {
            proj: camera_matrix,
            resolution: vec4(width as f32, height as f32, 0.0, 0.0),
        };

        queue.write_buffer(
            &self.line_drawer.buffers.vertex_uniform,
            0,
            bytemuck::bytes_of(&uniforms),
        );

        let encoder = &mut frame_encoder.encoder;

        encoder.push_debug_group("Line drawer");
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &frame_encoder.backbuffer_view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: true },
                })],
                depth_stencil_attachment: None,
            });

            // Render round line strips
            render_pass.set_pipeline(&self.line_drawer.round_line_strip_pipeline);
            render_pass
                .set_vertex_buffer(0, self.line_drawer.buffers.round_strip_geometry.slice(..));
            render_pass
                .set_vertex_buffer(1, self.line_drawer.buffers.round_strip_instances.slice(..));
            render_pass.set_bind_group(0, &self.line_drawer.bind_groups.vertex_uniform, &[]);

            let mut offset = 0usize;
            let vertex_count = self.line_drawer.buffers.round_strip_geometry_len as u32;

            for line_strip_size in &self.line_drawer.round_line_strip_indices {
                let range = (offset as u32)..(offset + line_strip_size - 1) as u32;
                offset += line_strip_size;
                render_pass.draw(0..vertex_count, range);
            }
        }
        encoder.pop_debug_group();
    }
}

#[repr(C)]
#[derive(Default, Debug, Copy, Clone, Pod, Zeroable)]
struct LineUniforms {
    proj: Mat4,
    resolution: Vec4,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct LineVertex3 {
    /// XYZ position of the line vertex, W = line thickness
    pos: Vec4,
}

impl LineVertex3 {
    pub fn new(pos: Vec3, thickness: f32) -> Self {
        Self { pos: vec4(pos.x, pos.y, pos.z, thickness) }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct RoundLineStripVertex {
    /// XY position of the line vertex, with Z indicating:
    /// 0: The left part of the line segment.
    /// 1: The right part of the line segment.
    pos: [f32; 3],
}
