use crate::{graphics::screen_projection_matrix, GraphicsDevice};
use bytemuck::{Pod, Zeroable};
use glam::{vec3, Mat4, Vec2, Vec3};
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

pub struct LineDrawer2d {
    round_line_strip_pipeline: wgpu::RenderPipeline,
    buffers: Buffers,
    bind_groups: BindGroups,
    round_line_strips: Vec<LineVertex>,
    round_line_strip_indices: Vec<usize>,
    projection: Mat4,
}

impl LineDrawer2d {
    pub fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        screen_width: u32,
        screen_height: u32,
    ) -> Self {
        let round_line_strip_pipeline =
            Self::build_round_line_strip_pipeline(device, target_format);

        let buffers = Self::build_buffers(device);
        let bind_groups = Self::build_bind_groups(device, &round_line_strip_pipeline, &buffers);
        let projection = screen_projection_matrix(screen_width, screen_height);

        Self {
            round_line_strip_pipeline,
            buffers,
            bind_groups,
            round_line_strips: Vec::new(),
            round_line_strip_indices: Vec::new(),
            projection,
        }
    }

    pub fn resize(&mut self, screen_width: u32, screen_height: u32) {
        self.projection = screen_projection_matrix(screen_width, screen_height);
    }

    pub fn begin(&mut self) -> Line2dRecorder {
        self.round_line_strips.clear();
        self.round_line_strip_indices.clear();

        Line2dRecorder { line_drawer: self }
    }

    fn build_round_line_strip_pipeline(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let draw_shader = GraphicsDevice::load_wgsl_shader(
            device,
            include_str!("shaders/wgsl/round_line_strip2d.wgsl"),
        );

        let vertex_bind_group_layout =
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

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Round line strip renderer"),
                bind_group_layouts: &[&vertex_bind_group_layout],
                push_constant_ranges: &[],
            });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: Some("main_vs"),
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<RoundLineStripVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Float32x3, // XY position of this particular vertex, with Z indicating sides.
                        ],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<LineVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            1 => Float32x3, // Point A
                        ],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<LineVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            2 => Float32x3, // Point B
                        ],
                    },
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: Some("main_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: Some(wgpu::BlendState {
                        color: wgpu::BlendComponent::REPLACE,
                        alpha: wgpu::BlendComponent::REPLACE,
                    }),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back), // TODO - figure out culling
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }

    fn build_bind_groups(
        device: &wgpu::Device,
        render_pipeline: &wgpu::RenderPipeline,
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

    fn build_buffers(device: &wgpu::Device) -> Buffers {
        const MAX_LINES: u64 = 40_000;
        const CIRCLE_RESOLUTION: usize = 30;

        // Uniform buffer
        let vertex_uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Line drawer vertex shader uniform buffer"),
            size: std::mem::size_of::<Mat4>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
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
            size: MAX_LINES * std::mem::size_of::<LineVertex>() as u64,
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

pub struct Line2dRecorder<'a> {
    line_drawer: &'a mut LineDrawer2d,
}

impl Line2dRecorder<'_> {
    /// A special-case where round line joins and caps are desired. This can be achieved
    /// with a single draw call.
    pub fn draw_round_line_strip(&mut self, positions: &[LineVertex]) {
        self.line_drawer.round_line_strips.extend_from_slice(positions);
        self.line_drawer.round_line_strip_indices.push(positions.len());
    }

    pub fn end(self, render_pass: &mut wgpu::RenderPass, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.line_drawer.buffers.round_strip_instances,
            0,
            bytemuck::cast_slice(&self.line_drawer.round_line_strips),
        );

        queue.write_buffer(
            &self.line_drawer.buffers.vertex_uniform,
            0,
            bytemuck::cast_slice(self.line_drawer.projection.as_ref()),
        );

        render_pass.push_debug_group("Line drawer");
        {
            // Render round line strips
            let instance_buffer_size = self.line_drawer.buffers.round_strip_instances.size();
            let one_instance_size = std::mem::size_of::<LineVertex>() as u64;

            render_pass.set_pipeline(&self.line_drawer.round_line_strip_pipeline);
            render_pass
                .set_vertex_buffer(0, self.line_drawer.buffers.round_strip_geometry.slice(..));
            render_pass.set_vertex_buffer(
                1,
                self.line_drawer
                    .buffers
                    .round_strip_instances
                    .slice(..(instance_buffer_size - one_instance_size)),
            );
            render_pass.set_vertex_buffer(
                2,
                self.line_drawer.buffers.round_strip_instances.slice(one_instance_size..),
            );
            render_pass.set_bind_group(0, &self.line_drawer.bind_groups.vertex_uniform, &[]);

            let mut offset = 0usize;
            let vertex_count = self.line_drawer.buffers.round_strip_geometry_len as u32;

            for line_strip_size in &self.line_drawer.round_line_strip_indices {
                let range = (offset as u32)..(offset + line_strip_size - 1) as u32;
                offset += line_strip_size;
                render_pass.draw(0..vertex_count, range);
            }
        }
        render_pass.pop_debug_group();
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct LineVertex {
    /// XY position of the line vertex, Z = line thickness
    pos: Vec3,
}

impl LineVertex {
    pub fn new(pos: Vec2, thickness: f32) -> Self {
        Self { pos: vec3(pos.x, pos.y, thickness) }
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
