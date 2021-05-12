use crate::{FrameEncoder, GraphicsDevice};
use bytemuck::{Pod, Zeroable};
use glam::{vec3, Mat4, Vec3};
use wgpu::{util::DeviceExt, RenderPipeline};

struct Buffers {
    lines: wgpu::Buffer,
    vertex_uniform: wgpu::Buffer,
    circle_positions: wgpu::Buffer,
    circle_geometry: wgpu::Buffer,
    circle_geometry_vertex_count: usize,
}

struct BindGroups {
    vertex_uniform: wgpu::BindGroup,
}

pub struct DebugDrawer {
    line_pipeline: RenderPipeline,
    instanced_shape_pipeline: RenderPipeline,
    buffers: Buffers,
    bind_groups: BindGroups,

    lines: Vec<LineVertex>,
    circles: Vec<CircleInstance>,
}

impl DebugDrawer {
    pub fn new(graphics_device: &GraphicsDevice) -> Self {
        let line_pipeline = Self::build_line_pipeline(graphics_device);
        let instanced_shape_pipeline = Self::build_intanced_shape_pipeline(graphics_device);
        let buffers = Self::build_buffers(graphics_device);
        let bind_groups = Self::build_bind_groups(graphics_device, &line_pipeline, &buffers);

        Self {
            line_pipeline,
            instanced_shape_pipeline,
            buffers,
            bind_groups,
            lines: Vec::new(),
            circles: Vec::new(),
        }
    }

    pub fn begin(&mut self) -> ShapeRecorder {
        self.lines.clear();
        self.circles.clear();

        ShapeRecorder { debug_drawer: self }
    }

    fn build_line_pipeline(graphics_device: &GraphicsDevice) -> RenderPipeline {
        let device = graphics_device.device();

        let draw_shader =
            graphics_device.load_wgsl_shader(include_str!("shaders/wgsl/debug_lines.wgsl"));

        let vertex_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<[[f32; 4]; 4]>() as u64,
                        ),
                    },
                    count: None,
                }],
                label: None,
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("line renderer"),
                bind_group_layouts: &[&vertex_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: "main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<LineVertex>() as u64,
                    step_mode: wgpu::InputStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "main",
                targets: &[graphics_device.swap_chain_descriptor().format.into()],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                // PolygonMode::Line needed?
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        render_pipeline
    }

    fn build_intanced_shape_pipeline(graphics_device: &GraphicsDevice) -> RenderPipeline {
        let device = graphics_device.device();

        let draw_shader =
            graphics_device.load_wgsl_shader(include_str!("shaders/wgsl/instanced_shape.wgsl"));

        let vertex_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            std::mem::size_of::<[[f32; 4]; 4]>() as u64,
                        ),
                    },
                    count: None,
                }],
                label: None,
            });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("instanced shape renderer"),
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
                        array_stride: std::mem::size_of::<CircleInstance>() as u64,
                        step_mode: wgpu::InputStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x4],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<LineVertex>() as u64,
                        step_mode: wgpu::InputStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![1 => Float32x3],
                    },
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: "main",
                targets: &[graphics_device.swap_chain_descriptor().format.into()],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                // PolygonMode::Line needed?
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
        });

        render_pipeline
    }

    fn build_buffers(graphics_device: &GraphicsDevice) -> Buffers {
        let (circle_geometry, circle_geometry_vertex_count) =
            Self::build_circle_geometry_buffer(graphics_device);

        Buffers {
            lines: Self::build_line_buffer(graphics_device),
            vertex_uniform: Self::build_vertex_uniform_buffer(graphics_device),
            circle_positions: Self::build_circle_positions_buffer(graphics_device),
            circle_geometry,
            circle_geometry_vertex_count,
        }
    }

    fn build_line_buffer(graphics_device: &GraphicsDevice) -> wgpu::Buffer {
        let device = graphics_device.device();

        const MAX_LINES: usize = 40_000;

        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Debug drawer line buffer"),
            size: MAX_LINES as u64, // TODO - multiply by instance size?
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn build_vertex_uniform_buffer(graphics_device: &GraphicsDevice) -> wgpu::Buffer {
        let (width, height) = graphics_device.surface_dimensions();
        let device = graphics_device.device();
        let camera_matrix = Self::build_camera_matrix(width as f32 / height as f32);

        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Particle system vertex shader uniform buffer"),
            contents: bytemuck::cast_slice(camera_matrix.as_ref()),
            usage: wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
        })
    }

    fn build_camera_matrix(aspect_ratio: f32) -> Mat4 {
        let height = 20.0;
        let half_height = height / 2.0;
        let half_width = (aspect_ratio * height) / 2.0;

        let proj =
            Mat4::orthographic_rh(-half_width, half_width, -half_height, half_height, -1.0, 1.0);

        let view = Mat4::look_at_rh(
            vec3(0.0, 0.0, 1.0), // Eye position
            vec3(0.0, 0.0, 0.0), // Look-at target
            vec3(0.0, 1.0, 0.0), // Up vector of the camera
        );

        proj * view
    }

    fn build_circle_positions_buffer(graphics_device: &GraphicsDevice) -> wgpu::Buffer {
        let device = graphics_device.device();

        const MAX_CIRCLES: usize = 40_000;

        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Circle positions buffer"),
            size: MAX_CIRCLES as u64 * std::mem::size_of::<CircleInstance>() as u64,
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn build_circle_geometry_buffer(graphics_device: &GraphicsDevice) -> (wgpu::Buffer, usize) {
        let device = graphics_device.device();

        let mut circle_vertices = vec![
            LineVertex { pos: [0.0, -1.0, 0.0] },
            LineVertex { pos: [0.0, 1.0, 0.0] },
            LineVertex { pos: [-1.0, 0.0, 0.0] },
            LineVertex { pos: [1.0, 0.0, 0.0] },
        ];

        const CIRCLE_SEGMENTS: usize = 50;

        for i in 0..CIRCLE_SEGMENTS {
            let frac_1 = (i as f32 / CIRCLE_SEGMENTS as f32) * 2.0 * std::f32::consts::PI;
            let frac_2 = ((i + 1) as f32 / CIRCLE_SEGMENTS as f32) * 2.0 * std::f32::consts::PI;

            circle_vertices.push(LineVertex { pos: [frac_1.cos(), frac_1.sin(), 0.0] });

            circle_vertices.push(LineVertex { pos: [frac_2.cos(), frac_2.sin(), 0.0] });
        }

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Circle geometry buffer"),
            contents: bytemuck::cast_slice(&circle_vertices),
            usage: wgpu::BufferUsage::VERTEX | wgpu::BufferUsage::COPY_DST,
        });

        (buffer, circle_vertices.len())
    }

    fn build_bind_groups(
        graphics_device: &GraphicsDevice,
        render_pipeline: &RenderPipeline,
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
}

pub struct ShapeRecorder<'a> {
    debug_drawer: &'a mut DebugDrawer,
}

impl ShapeRecorder<'_> {
    pub fn draw_line(&mut self, start: Vec3, end: Vec3) {
        self.debug_drawer.lines.push(LineVertex { pos: [start.x, start.y, start.z] });
        self.debug_drawer.lines.push(LineVertex { pos: [end.x, end.y, end.z] });
    }

    pub fn draw_circle(&mut self, center: Vec3, radius: f32, rotation: f32) {
        self.debug_drawer.circles.push(CircleInstance {
            center: [center.x, center.y],
            radius,
            rotation,
        });
    }

    pub fn end(self, frame_encoder: &mut FrameEncoder) {
        let (width, height) = frame_encoder.surface_dimensions();

        let queue = frame_encoder.queue();
        queue.write_buffer(
            &self.debug_drawer.buffers.lines,
            0,
            bytemuck::cast_slice(&self.debug_drawer.lines),
        );

        queue.write_buffer(
            &self.debug_drawer.buffers.circle_positions,
            0,
            bytemuck::cast_slice(&self.debug_drawer.circles),
        );

        let proj = DebugDrawer::build_camera_matrix(width as f32 / height as f32);
        queue.write_buffer(
            &self.debug_drawer.buffers.vertex_uniform,
            0,
            bytemuck::cast_slice(proj.as_ref()),
        );

        let frame = &frame_encoder.frame;
        let encoder = &mut frame_encoder.encoder;

        encoder.push_debug_group("Debug drawer");
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
            render_pass.set_pipeline(&self.debug_drawer.line_pipeline);
            render_pass.set_vertex_buffer(0, self.debug_drawer.buffers.lines.slice(..));
            render_pass.set_bind_group(0, &self.debug_drawer.bind_groups.vertex_uniform, &[]);
            render_pass.draw(0..self.debug_drawer.lines.len() as u32, 0..1);

            // Render circles
            let vert_count = self.debug_drawer.buffers.circle_geometry_vertex_count as u32;

            render_pass.set_pipeline(&self.debug_drawer.instanced_shape_pipeline);
            render_pass.set_vertex_buffer(0, self.debug_drawer.buffers.circle_positions.slice(..));
            render_pass.set_vertex_buffer(1, self.debug_drawer.buffers.circle_geometry.slice(..));
            render_pass.set_bind_group(0, &self.debug_drawer.bind_groups.vertex_uniform, &[]);
            render_pass.draw(0..vert_count, 0..self.debug_drawer.circles.len() as u32);
        }
        encoder.pop_debug_group();
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct LineVertex {
    /// XYZ position of the line vertex
    pos: [f32; 3],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct CircleInstance {
    center: [f32; 2],
    radius: f32,
    rotation: f32,
}
