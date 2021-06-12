use crate::{FrameEncoder, GraphicsDevice};
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Vec2};
use wgpu::util::DeviceExt;

struct Buffers {
    vertex_uniform: wgpu::Buffer,
    segment_geometry: wgpu::Buffer,
    segment_instances: wgpu::Buffer,
    strip_instances: wgpu::Buffer,

    round_strip_geometry: wgpu::Buffer,
    round_strip_geometry_len: usize,
    round_strip_instances: wgpu::Buffer,
}

struct BindGroups {
    vertex_uniform: wgpu::BindGroup,
}

pub struct LineDrawer {
    line_pipeline: wgpu::RenderPipeline,
    line_strip_pipeline: wgpu::RenderPipeline,
    round_line_strip_pipeline: wgpu::RenderPipeline,

    buffers: Buffers,
    bind_groups: BindGroups,

    lines: Vec<LineVertex>,
    line_strips: Vec<LineVertex>,
    strip_indices: Vec<usize>,

    round_line_strips: Vec<LineVertex>,
    round_line_strip_indices: Vec<usize>,
}

impl LineDrawer {
    pub fn new(graphics_device: &GraphicsDevice) -> Self {
        let line_pipeline = Self::build_line_pipeline(graphics_device);
        let line_strip_pipeline = Self::build_line_strip_pipeline(graphics_device);
        let round_line_strip_pipeline = Self::build_round_line_strip_pipeline(graphics_device);

        let buffers = Self::build_buffers(graphics_device);
        let bind_groups = Self::build_bind_groups(graphics_device, &line_pipeline, &buffers);

        Self {
            line_pipeline,
            line_strip_pipeline,
            round_line_strip_pipeline,
            buffers,
            bind_groups,
            lines: Vec::new(),
            line_strips: Vec::new(),
            strip_indices: Vec::new(),

            round_line_strips: Vec::new(),
            round_line_strip_indices: Vec::new(),
        }
    }

    pub fn begin(&mut self) -> LineRecorder {
        self.lines.clear();
        self.line_strips.clear();
        self.strip_indices.clear();

        self.round_line_strips.clear();
        self.round_line_strip_indices.clear();

        LineRecorder { line_drawer: self }
    }

    fn build_line_pipeline(graphics_device: &GraphicsDevice) -> wgpu::RenderPipeline {
        let device = graphics_device.device();

        let draw_shader =
            graphics_device.load_wgsl_shader(include_str!("shaders/wgsl/instanced_lines.wgsl"));

        let vertex_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
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
                label: Some("instanced line renderer"),
                bind_group_layouts: &[&vertex_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: "main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<LineVertex>() as u64,
                        step_mode: wgpu::InputStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Float32x2, // XY position of this particular vertex.
                        ],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<LineSegmentInstance>() as u64,
                        step_mode: wgpu::InputStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            1 => Float32x2, // Point A
                            2 => Float32x2, // Point B
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "main",
                targets: &[graphics_device.swap_chain_descriptor().format.into()],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back), // TODO - figure out culling
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        render_pipeline
    }

    fn build_line_strip_pipeline(graphics_device: &GraphicsDevice) -> wgpu::RenderPipeline {
        let device = graphics_device.device();

        let draw_shader =
            graphics_device.load_wgsl_shader(include_str!("shaders/wgsl/instanced_lines.wgsl"));

        let vertex_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
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
                label: Some("instanced line segment renderer"),
                bind_group_layouts: &[&vertex_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: "main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<LineVertex>() as u64,
                        step_mode: wgpu::InputStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Float32x2, // XY position of this particular vertex.
                        ],
                    },
                    wgpu::VertexBufferLayout {
                        // The stride is one LineVertex here intentionally.
                        array_stride: std::mem::size_of::<LineVertex>() as u64,
                        step_mode: wgpu::InputStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            1 => Float32x2, // Point A
                            2 => Float32x2, // Point B
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "main",
                targets: &[graphics_device.swap_chain_descriptor().format.into()],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back), // TODO - figure out culling
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        render_pipeline
    }

    fn build_round_line_strip_pipeline(graphics_device: &GraphicsDevice) -> wgpu::RenderPipeline {
        let device = graphics_device.device();

        let draw_shader =
            graphics_device.load_wgsl_shader(include_str!("shaders/wgsl/round_line_strip.wgsl"));

        let vertex_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
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

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: "main",
                buffers: &[
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<RoundLineStripVertex>() as u64,
                        step_mode: wgpu::InputStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![
                            0 => Float32x3, // XY position of this particular vertex, with Z indicating sides.
                        ],
                    },
                    wgpu::VertexBufferLayout {
                        // The stride is one LineVertex here intentionally.
                        array_stride: std::mem::size_of::<LineVertex>() as u64,
                        step_mode: wgpu::InputStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![
                            1 => Float32x2, // Point A
                            2 => Float32x2, // Point B
                        ],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "main",
                targets: &[graphics_device.swap_chain_descriptor().format.into()],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back), // TODO - figure out culling
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
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

        let (width, height) = graphics_device.surface_dimensions();
        let device = graphics_device.device();

        // Uniform buffer
        let camera_matrix = screen_projection_matrix(width as f32, height as f32);
        let vertex_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particle system vertex shader uniform buffer"),
            contents: bytemuck::cast_slice(camera_matrix.as_ref()),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        });

        // Segment geometry
        let segment_vertices = [
            LineVertex { pos: [0.0, -0.5] },
            LineVertex { pos: [0.0, 0.5] },
            LineVertex { pos: [1.0, 0.5] },
            LineVertex { pos: [0.0, -0.5] },
            LineVertex { pos: [1.0, 0.5] },
            LineVertex { pos: [1.0, -0.5] },
        ];

        let segment_geometry = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Line segment geometry buffer"),
            contents: bytemuck::cast_slice(&segment_vertices),
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });

        // Segment instances
        let segment_instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Line segment instance buffer"),
            size: MAX_LINES * std::mem::size_of::<LineVertex>() as u64,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        // Strip instances
        let strip_instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Line strip instance buffer"),
            size: MAX_LINES * std::mem::size_of::<LineVertex>() as u64,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
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
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });

        // Round strip instances
        let round_strip_instances = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Line strip instance buffer"),
            size: MAX_LINES * std::mem::size_of::<RoundLineStripVertex>() as u64,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        });

        Buffers {
            vertex_uniform,
            segment_geometry,
            segment_instances,
            strip_instances,

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
    pub fn draw_line(&mut self, start: Vec2, end: Vec2) {
        self.line_drawer.lines.push(LineVertex { pos: [start.x, start.y] });
        self.line_drawer.lines.push(LineVertex { pos: [end.x, end.y] });
    }

    pub fn draw_line_strip(&mut self, positions: &[Vec2]) {
        for pos in positions {
            self.line_drawer.line_strips.push(LineVertex { pos: [pos.x, pos.y] });
        }

        self.line_drawer.strip_indices.push(positions.len());
    }

    /// A special-case where round line joins and caps are desired. This can be achieved
    /// with a single draw call.
    pub fn draw_round_line_strip(&mut self, positions: &[Vec2]) {
        for pos in positions {
            self.line_drawer.round_line_strips.push(LineVertex { pos: [pos.x, pos.y] });
        }

        self.line_drawer.round_line_strip_indices.push(positions.len());
    }

    pub fn end(self, frame_encoder: &mut FrameEncoder) {
        let (width, height) = frame_encoder.surface_dimensions();

        let queue = frame_encoder.queue();
        queue.write_buffer(
            &self.line_drawer.buffers.segment_instances,
            0,
            bytemuck::cast_slice(&self.line_drawer.lines),
        );

        queue.write_buffer(
            &self.line_drawer.buffers.strip_instances,
            0,
            bytemuck::cast_slice(&self.line_drawer.line_strips),
        );

        queue.write_buffer(
            &self.line_drawer.buffers.round_strip_instances,
            0,
            bytemuck::cast_slice(&self.line_drawer.round_line_strips),
        );

        let proj = screen_projection_matrix(width as f32, height as f32);
        queue.write_buffer(
            &self.line_drawer.buffers.vertex_uniform,
            0,
            bytemuck::cast_slice(proj.as_ref()),
        );

        let frame = &frame_encoder.frame;
        let encoder = &mut frame_encoder.encoder;

        encoder.push_debug_group("Line drawer");
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &frame.view,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: true },
                }],
                depth_stencil_attachment: None,
            });

            // Render lines
            render_pass.set_pipeline(&self.line_drawer.line_pipeline);
            render_pass.set_vertex_buffer(0, self.line_drawer.buffers.segment_geometry.slice(..));
            render_pass.set_vertex_buffer(1, self.line_drawer.buffers.segment_instances.slice(..));
            render_pass.set_bind_group(0, &self.line_drawer.bind_groups.vertex_uniform, &[]);
            render_pass.draw(0..6, 0..(self.line_drawer.lines.len() / 2) as u32);

            // Render line strips
            render_pass.set_pipeline(&self.line_drawer.line_strip_pipeline);
            render_pass.set_vertex_buffer(0, self.line_drawer.buffers.segment_geometry.slice(..));
            render_pass.set_vertex_buffer(1, self.line_drawer.buffers.strip_instances.slice(..));
            render_pass.set_bind_group(0, &self.line_drawer.bind_groups.vertex_uniform, &[]);

            let mut offset = 0usize;
            for line_strip_size in &self.line_drawer.strip_indices {
                let range = (offset as u32)..(offset + line_strip_size - 1) as u32;
                offset += line_strip_size;
                render_pass.draw(0..6, range);
            }

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

// Creates a matrix that projects screen coordinates defined by width and
// height orthographically onto the OpenGL vertex coordinates.
fn screen_projection_matrix(width: f32, height: f32) -> Mat4 {
    Mat4::orthographic_rh(0.0, width, height, 0.0, -1.0, 1.0)
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct LineVertex {
    /// XY position of the line vertex
    pos: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct RoundLineStripVertex {
    /// XY position of the line vertex, with Z indicating:
    /// 0: The left part of the line segment.
    /// 1: The right part of the line segment.
    pos: [f32; 3],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct LineSegmentInstance {
    /// XY position of the start of the line segment.
    point_a: [f32; 2],

    /// XY position of the end of the line segment.
    point_b: [f32; 2],
}
