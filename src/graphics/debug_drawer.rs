use crate::GraphicsDevice;
use bytemuck::{Pod, Zeroable};
use glam::{vec3, Mat4, Vec3};
use wgpu::util::DeviceExt;

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
    line_pipeline: wgpu::RenderPipeline,
    instanced_shape_pipeline: wgpu::RenderPipeline,
    buffers: Buffers,
    bind_groups: BindGroups,
    projection: Mat4,

    lines: Vec<LineVertex>,
    circles: Vec<CircleInstance>,
}

impl DebugDrawer {
    pub fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        screen_width: u32,
        screen_height: u32,
    ) -> Self {
        let line_pipeline = Self::build_line_pipeline(device, target_format);
        let instanced_shape_pipeline = Self::build_intanced_shape_pipeline(device, target_format);
        let buffers = Self::build_buffers(device);
        let bind_groups = Self::build_bind_groups(device, &line_pipeline, &buffers);
        let projection = Self::build_camera_matrix(screen_width, screen_height);

        Self {
            line_pipeline,
            instanced_shape_pipeline,
            buffers,
            bind_groups,
            projection,
            lines: Vec::new(),
            circles: Vec::new(),
        }
    }

    pub fn resize(&mut self, screen_width: u32, screen_height: u32) {
        self.projection = Self::build_camera_matrix(screen_width, screen_height);
    }

    pub fn begin(&mut self) -> ShapeRecorder {
        self.lines.clear();
        self.circles.clear();

        ShapeRecorder { debug_drawer: self }
    }

    fn build_line_pipeline(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let draw_shader =
            GraphicsDevice::load_wgsl_shader(device, include_str!("shaders/wgsl/debug_lines.wgsl"));

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
                label: Some("line renderer"),
                bind_group_layouts: &[&vertex_bind_group_layout],
                push_constant_ranges: &[],
            });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: Some("main_vs"),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<LineVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![0 => Float32x3],
                }],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: Some("main_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                // PolygonMode::Line needed?
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }

    fn build_intanced_shape_pipeline(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
    ) -> wgpu::RenderPipeline {
        let draw_shader = GraphicsDevice::load_wgsl_shader(
            device,
            include_str!("shaders/wgsl/instanced_shape.wgsl"),
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
                label: Some("instanced shape renderer"),
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
                        array_stride: std::mem::size_of::<CircleInstance>() as u64,
                        step_mode: wgpu::VertexStepMode::Instance,
                        attributes: &wgpu::vertex_attr_array![0 => Float32x4],
                    },
                    wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<LineVertex>() as u64,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &wgpu::vertex_attr_array![1 => Float32x3],
                    },
                ],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
                entry_point: Some("main_fs"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: target_format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                // PolygonMode::Line needed?
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        })
    }

    fn build_buffers(device: &wgpu::Device) -> Buffers {
        let (circle_geometry, circle_geometry_vertex_count) =
            Self::build_circle_geometry_buffer(device);

        Buffers {
            lines: Self::build_line_buffer(device),
            vertex_uniform: Self::build_vertex_uniform_buffer(device),
            circle_positions: Self::build_circle_positions_buffer(device),
            circle_geometry,
            circle_geometry_vertex_count,
        }
    }

    fn build_line_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        const MAX_LINES: u64 = 40_000;

        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Debug drawer line buffer"),
            size: MAX_LINES * std::mem::size_of::<LineVertex>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn build_vertex_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Particle system vertex shader uniform buffer"),
            size: std::mem::size_of::<Mat4>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn build_camera_matrix(screen_width: u32, screen_height: u32) -> Mat4 {
        let aspect_ratio = screen_width as f32 / screen_height as f32;
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

    fn build_circle_positions_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        const MAX_CIRCLES: usize = 40_000;

        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Circle positions buffer"),
            size: MAX_CIRCLES as u64 * std::mem::size_of::<CircleInstance>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn build_circle_geometry_buffer(device: &wgpu::Device) -> (wgpu::Buffer, usize) {
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
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        (buffer, circle_vertices.len())
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

    pub fn end(
        self,
        encoder: &mut wgpu::CommandEncoder,
        render_target: &wgpu::TextureView,
        queue: &wgpu::Queue,
    ) {
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

        queue.write_buffer(
            &self.debug_drawer.buffers.vertex_uniform,
            0,
            bytemuck::cast_slice(self.debug_drawer.projection.as_ref()),
        );

        encoder.push_debug_group("Debug drawer");
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: render_target,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Load, store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
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
