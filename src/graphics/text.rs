use crate::{graphics::screen_projection_matrix, GraphicsDevice};
use bytemuck::{Pod, Zeroable};
use cosmic_text::{
    Align, Attrs, Buffer, CacheKey, Family, FontSystem, Metrics, Shaping, SwashCache, SwashContent,
    Weight,
};
use etagere::{AllocId, AtlasAllocator};
use glam::Mat4;
use lru::LruCache;
use std::collections::HashSet;

const GLYPH_TEXTURE_WIDTH: u32 = 4096;
const GLYPH_TEXTURE_HEIGHT: u32 = 4096;
const BORDER_PADDING: u32 = 2;

pub const WHITE: Color = Color::new(255, 255, 255, 255);

/// Vertex attributes for instanced glyph data.
#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
struct TextVertex {
    pos: [f32; 2],
    tex_coords: [f32; 2],
    color: [f32; 4], // Premultiplied alpha
}

impl Default for TextVertex {
    fn default() -> Self {
        TextVertex { pos: [0.0; 2], tex_coords: [0.0; 2], color: [1.0; 4] }
    }
}

pub trait Font: std::fmt::Debug + Clone + Copy + PartialEq + Eq + std::hash::Hash {
    fn size(&self) -> u32;
    fn font_bytes(&self) -> &'static [u8];
    fn default() -> Self;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DefaultFont {
    SpaceMono400(u32),
}

impl Font for DefaultFont {
    fn size(&self) -> u32 {
        use DefaultFont::*;

        match self {
            SpaceMono400(size) => *size,
        }
    }

    fn font_bytes(&self) -> &'static [u8] {
        use DefaultFont::*;

        match self {
            SpaceMono400(_) => include_bytes!("./resources/fonts/space_mono_400.ttf"),
        }
    }

    fn default() -> Self {
        DefaultFont::SpaceMono400(40)
    }
}

#[derive(Debug)]
pub enum RasterizationError {
    NoTextureSpace,
}

/// Where to align on a particular axis.
/// Y: Start = top of the text box aligned to the Y coord
///    End   = bottom of the text box aligned to the Y coord
/// X: Start = left side of the text box aligned to the X coord
///    End   = right side of the text box aligned to the X coord
/// Units are in pixels.
#[derive(Debug, Copy, Clone)]
pub enum AxisAlign {
    Start(f32),
    End(f32),
    CenteredAt(f32),
    CanvasCenter,
}

impl Default for AxisAlign {
    fn default() -> Self {
        AxisAlign::Start(0.0)
    }
}

#[derive(Debug, Default, Copy, Clone)]
pub struct TextAlignment {
    pub x: AxisAlign,
    pub y: AxisAlign,
}

#[derive(Debug, Default, Copy, Clone)]
pub enum TextJustify {
    #[default]
    Left,
    Right,
    End,
    Center,
    Justified,
}

#[derive(Debug)]
pub struct Text {
    pub text: String,
    pub font_size: f32,
    pub font_weight: Option<u16>,
    pub color: Color,
    /// If present, will override the font, if it exists,
    /// specified on `TextBlock`.
    pub font: Option<&'static str>,
}

impl Text {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            font_size: 32.0, // TODO(bschwind) - Is this the best way to handle a default size?
            font_weight: None,
            color: Color::white(),
            font: None,
        }
    }

    pub fn with_font_size(mut self, font_size: f32) -> Self {
        self.font_size = font_size;
        self
    }

    pub fn with_font_weight(mut self, font_weight: u16) -> Self {
        self.font_weight = Some(font_weight);
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

#[derive(Debug)]
pub struct TextBlock {
    pub alignment: TextAlignment,
    pub max_width: Option<f32>,
    pub max_height: Option<f32>,
    pub justify: TextJustify,
    pub text_spans: Vec<Text>,
    /// The default font to use for all text spans.
    /// Can be overrided with `Text.font`.
    pub font: Option<&'static str>,
    pub font_weight: Option<u16>,
}

impl Default for TextBlock {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBlock {
    pub fn new() -> Self {
        Self {
            alignment: TextAlignment::default(),
            max_width: None,
            max_height: None,
            justify: Default::default(),
            text_spans: vec![],
            font: None,
            font_weight: None,
        }
    }

    pub fn text_blocks(text_spans: impl IntoIterator<Item = Text>) -> Self {
        Self {
            alignment: TextAlignment::default(),
            max_width: None,
            max_height: None,
            justify: Default::default(),
            text_spans: text_spans.into_iter().collect(),
            font: None,
            font_weight: None,
        }
    }

    pub fn string(text: impl Into<String>) -> Self {
        Self {
            alignment: TextAlignment::default(),
            max_width: None,
            max_height: None,
            justify: Default::default(),
            text_spans: vec![Text::new(text)],
            font: None,
            font_weight: None,
        }
    }

    pub fn with_alignment(mut self, alignment: TextAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    pub fn with_max_width(mut self, max_width: f32) -> Self {
        self.max_width = Some(max_width);
        self
    }

    pub fn with_max_height(mut self, max_height: f32) -> Self {
        self.max_height = Some(max_height);
        self
    }

    pub fn with_text_blocks(mut self, text_spans: impl Iterator<Item = Text>) -> Self {
        self.text_spans = text_spans.collect();
        self
    }

    pub fn with_justify(mut self, text_justify: TextJustify) -> Self {
        self.justify = text_justify;
        self
    }

    pub fn with_font(mut self, font: &'static str) -> Self {
        self.font = Some(font);
        self
    }

    pub fn with_font_weight(mut self, font_weight: u16) -> Self {
        self.font_weight = Some(font_weight);
        self
    }
}

pub struct TextPainter {
    text_vertices: Vec<TextVertex>,
    glyph_vertex_buffer: wgpu::Buffer,
    glyph_texture: wgpu::Texture,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
    // The projection used to map pixel coordinates to normalized device coordinates.
    projection: Mat4,

    screen_width: u32,
    screen_height: u32,

    glyph_space_allocator: AtlasAllocator,
    lru_cache: LruCache<CacheKey, GlyphTextureAllocation>,
    font_system: FontSystem,
    swash_cache: SwashCache,
    glyphs_for_this_frame: HashSet<CacheKey>,
}

impl TextPainter {
    pub fn new(
        font_database: fontdb::Database,
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        screen_width: u32,
        screen_height: u32,
    ) -> Self {
        let glyph_texture = Self::build_glyph_texture(device);
        let glyph_vertex_buffer = Self::build_vertex_buffer(device);
        let uniform_buffer = Self::build_uniform_buffer(device);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GlyphPainter bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: core::num::NonZeroU64::new(
                            std::mem::size_of::<Mat4>() as u64
                        ), // Size of a 4x4 f32 matrix
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GlyphPainter pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let texture_view = glyph_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest, // TODO - nearest
            min_filter: wgpu::FilterMode::Nearest, // TODO - nearest
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GlyphPainter bind group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: uniform_buffer.as_entire_binding() },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let draw_shader =
            GraphicsDevice::load_wgsl_shader(device, include_str!("shaders/wgsl/glyph.wgsl"));

        let vertex_buffers = &[wgpu::VertexBufferLayout {
            array_stride: (std::mem::size_of::<TextVertex>()) as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 => Float32x2, // pos
                1 => Float32x2, // tex_coords
                2 => Float32x4, // color
            ],
        }];

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("GlyphPainter render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &draw_shader,
                entry_point: "main_vs",
                buffers: vertex_buffers,
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
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
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            multiview: None,
            cache: None,
        });

        let projection = screen_projection_matrix(screen_width, screen_height);

        let glyph_space_allocator = AtlasAllocator::new(etagere::size2(
            GLYPH_TEXTURE_WIDTH as i32,
            GLYPH_TEXTURE_HEIGHT as i32,
        ));

        let lru_cache = LruCache::unbounded();

        let locale = sys_locale::get_locale().unwrap_or_else(|| "en-US".to_string());
        let font_system = FontSystem::new_with_locale_and_db(locale, font_database);

        let swash_cache = SwashCache::new();

        dbg!(font_system.db().len());

        for face in font_system.db().faces() {
            dbg!(&face.post_script_name);
            dbg!(&face.families);
        }

        Self {
            text_vertices: vec![],
            glyph_vertex_buffer,
            uniform_buffer,
            bind_group,
            glyph_texture,
            pipeline,

            projection,
            screen_width,
            screen_height,

            glyph_space_allocator,
            lru_cache,
            font_system,
            swash_cache,
            glyphs_for_this_frame: HashSet::new(),
        }
    }

    fn build_glyph_texture(device: &wgpu::Device) -> wgpu::Texture {
        let glyph_texture_extent = wgpu::Extent3d {
            width: GLYPH_TEXTURE_WIDTH,
            height: GLYPH_TEXTURE_HEIGHT,
            depth_or_array_layers: 1,
        };

        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Glyph texture"),
            size: glyph_texture_extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            view_formats: &[],
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        })
    }

    fn build_vertex_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Glyph Vertex Buffer"),
            size: 10000,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    fn build_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Glyph Uniform Buffer"),
            size: std::mem::size_of::<Mat4>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    pub fn resize(&mut self, screen_width: u32, screen_height: u32) {
        self.projection = screen_projection_matrix(screen_width, screen_height);
        self.screen_width = screen_width;
        self.screen_height = screen_height;
    }

    fn font_system(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    pub fn shape_text_block(&mut self, text_block: &TextBlock) -> ShapedTextBlock {
        const DEFAULT_FONT_SIZE: f32 = 32.0;

        // TODO(bschwind) - Why do we need to construct Metrics here if each text span
        //                  has its own size?
        let block_font_size =
            text_block.text_spans.first().map(|s| s.font_size).unwrap_or(DEFAULT_FONT_SIZE);
        let mut buffer = Buffer::new_empty(Metrics::relative(block_font_size, 1.0));

        let mut default_attrs = Attrs::new();
        if let Some(font) = text_block.font {
            default_attrs = default_attrs.family(Family::Name(font));
        }

        if let Some(weight) = text_block.font_weight {
            default_attrs = default_attrs.weight(Weight(weight));
        }

        let spans = text_block.text_spans.iter().map(|span| {
            let text: &str = &span.text;
            let mut metrics = default_attrs.metrics(Metrics::relative(span.font_size, 1.0));

            if let Some(font) = span.font {
                metrics = metrics.family(Family::Name(font));
            }

            if let Some(weight) = span.font_weight {
                metrics = metrics.weight(Weight(weight));
            }

            metrics = metrics.color(cosmic_text::Color::rgba(
                span.color.red,
                span.color.green,
                span.color.blue,
                span.color.alpha,
            ));

            (text, metrics)
        });

        buffer.set_rich_text(&mut self.font_system, spans, default_attrs, Shaping::Advanced);

        buffer.set_size(&mut self.font_system, text_block.max_width, text_block.max_height);

        for line in &mut buffer.lines {
            let align = match text_block.justify {
                TextJustify::Left => Align::Left,
                TextJustify::Right => Align::Right,
                TextJustify::End => Align::End,
                TextJustify::Center => Align::Center,
                TextJustify::Justified => Align::Justified,
            };

            line.set_align(Some(align));
        }

        let prune = true;
        buffer.shape_until_scroll(&mut self.font_system, prune);

        ShapedTextBlock { buffer }
    }

    pub fn measure_width(&mut self, text_block: &TextBlock) -> f32 {
        let shaped_text_block = self.shape_text_block(text_block);

        self.measure_text_block_width(&shaped_text_block)
    }

    pub fn measure_height(&mut self, text_block: &TextBlock) -> f32 {
        let shaped_text_block = self.shape_text_block(text_block);

        self.measure_text_block_height(&shaped_text_block)
    }

    pub fn measure(&mut self, text_block: &TextBlock) -> (f32, f32) {
        let shaped_text_block = self.shape_text_block(text_block);

        let width = self.measure_text_block_width(&shaped_text_block);
        let height = self.measure_text_block_height(&shaped_text_block);

        (width, height)
    }

    fn measure_text_block_width(&self, shaped_text_block: &ShapedTextBlock) -> f32 {
        let width =
            shaped_text_block.buffer.layout_runs().fold(0.0, |width, run| run.line_w.max(width));

        width
    }

    fn measure_text_block_height(&self, shaped_text_block: &ShapedTextBlock) -> f32 {
        let total_lines = shaped_text_block.buffer.layout_runs().count();

        total_lines as f32 * shaped_text_block.buffer.metrics().line_height
    }

    pub fn add_text_block(&mut self, queue: &wgpu::Queue, text_block: TextBlock) {
        // We need to shape once, measure the actual width, and then shape again
        // with that width defined as the max_width. This second shaping is only
        // needed if the justify is center, right, or justified, because cosmic-text
        // Align doesn't work without a max width defined.
        let mut shaped_text_block = self.shape_text_block(&text_block);

        let measured_width = self.measure_text_block_width(&shaped_text_block);
        shaped_text_block.set_size(self.font_system(), Some(measured_width), text_block.max_height);

        let x = match text_block.alignment.x {
            AxisAlign::Start(start_x) => start_x,
            AxisAlign::End(end_x) => {
                // Measure width, subtract from end_x
                let width = self.measure_text_block_width(&shaped_text_block);
                end_x - width
            },
            AxisAlign::CenteredAt(center_x) => {
                // Measure width, subtract half of width
                let width = self.measure_text_block_width(&shaped_text_block);
                center_x - (width / 2.0)
            },
            AxisAlign::CanvasCenter => {
                // Measure width, subtract half of width
                // from canvas center. Use `surface_dimensions`
                // for now until we have a concept of layers.
                let width = self.measure_text_block_width(&shaped_text_block);
                (self.screen_width as f32 / 2.0) - (width / 2.0)
            },
        };

        let y = match text_block.alignment.y {
            AxisAlign::Start(start_y) => start_y,
            AxisAlign::End(end_y) => {
                // Measure height, subtract from end_y
                let height = self.measure_text_block_height(&shaped_text_block);
                end_y - height
            },
            AxisAlign::CenteredAt(center_y) => {
                // Measure height, subtract half of height
                let height = self.measure_text_block_height(&shaped_text_block);
                center_y - (height / 2.0)
            },
            AxisAlign::CanvasCenter => {
                // Measure height, subtract half of height
                // from canvas center. Use `surface_dimensions`
                // for now until we have a concept of layers.
                let height = self.measure_text_block_height(&shaped_text_block);
                (self.screen_height as f32 / 2.0) - (height / 2.0)
            },
        };

        self.add_text(queue, x, y, shaped_text_block);
    }

    fn add_text(
        &mut self,
        queue: &wgpu::Queue,
        x: f32,
        y: f32,
        shaped_text_block: ShapedTextBlock,
    ) {
        let buffer = shaped_text_block.buffer;

        // Inspect the output runs
        for run in buffer.layout_runs() {
            'glyph_loop: for glyph in run.glyphs.iter() {
                // TODO(bschwind) - Provide actual offset and scale here.
                let physical_glyph = glyph.physical((x, y), 1.0);

                let glyph_allocation = if let Some(allocation) =
                    self.lru_cache.get(&physical_glyph.cache_key)
                {
                    // Good to go!
                    self.glyphs_for_this_frame.insert(physical_glyph.cache_key);
                    *allocation
                } else {
                    // TODO(bschwind) - Cache the image returned here.
                    // Add the glyph image to the glyph texture
                    let Some(mut image) = self
                        .swash_cache
                        .get_image_uncached(&mut self.font_system, physical_glyph.cache_key)
                    else {
                        // No rasterization needed?
                        continue;
                    };

                    let image_width = image.placement.width as i32;
                    let image_height = image.placement.height as i32;

                    if image_width <= 0 || image_height <= 0 {
                        continue;
                    }

                    let allocation = loop {
                        let allocation_opt = self.glyph_space_allocator.allocate(etagere::size2(
                            image_width + (2 * BORDER_PADDING as i32),
                            image_height + (2 * BORDER_PADDING as i32),
                        ));

                        if let Some(allocation) = allocation_opt {
                            break allocation;
                        }

                        // We weren't able to allocate, let's deallocate glyphs until we get more free
                        // space or run out of glyphs in the cache.
                        let free_space_before_dealloc = self.glyph_space_allocator.free_space();

                        loop {
                            let least_recently_used = self.lru_cache.peek_lru();

                            let Some((least_recently_used_glyph_key, least_recently_used_glyph)) =
                                least_recently_used
                            else {
                                // No more glyphs in the cache, we can't deallocate any more so no point in trying.
                                continue 'glyph_loop;
                            };

                            if self.glyphs_for_this_frame.contains(least_recently_used_glyph_key) {
                                // We want to deallocate this glyph but it is already set to be used
                                // for this frame. The atlas isn't big enough to hold glyphs for this
                                // frame. We can either resize the texture, or render what we have so far.
                                continue 'glyph_loop;
                            }

                            self.glyph_space_allocator
                                .deallocate(least_recently_used_glyph.allocation_id);

                            let _ = self.lru_cache.pop_lru();

                            if self.glyph_space_allocator.free_space() != free_space_before_dealloc
                            {
                                break;
                            }
                        }

                        // We deallocated enough to free up some space, go to the top of the
                        // loop and try allocating again.
                    };

                    self.glyphs_for_this_frame.insert(physical_glyph.cache_key);

                    let x = allocation.rectangle.min.x as u32;
                    let y = allocation.rectangle.min.y as u32;

                    let (data, glyph_type) = match image.content {
                        SwashContent::Mask | SwashContent::SubpixelMask => {
                            // Convert the single channel alpha mask to pre-multiplied
                            // RGBA white. This allows us to store alpha glyphs and
                            // color glyphs in the same texture at the expense of extra
                            // texture memory used for alpha glyphs.
                            let image_data: Vec<u8> = image
                                .data
                                .iter()
                                .flat_map(|byte| {
                                    // Pre-multiplied alpha white
                                    [*byte, *byte, *byte, *byte]
                                })
                                .collect();

                            (image_data, GlyphType::Grayscale)
                        },
                        SwashContent::Color => {
                            // Convert the color glyph to pre-multiplied alpha.
                            for chunk in image.data.chunks_exact_mut(4) {
                                let alpha = chunk[3] as u16;

                                chunk[0] = ((chunk[0] as u16 * alpha) >> 8) as u8;
                                chunk[1] = ((chunk[1] as u16 * alpha) >> 8) as u8;
                                chunk[2] = ((chunk[2] as u16 * alpha) >> 8) as u8;
                            }

                            (image.data, GlyphType::Color)
                        },
                    };

                    self.write_to_texture(
                        queue,
                        GlyphType::Color, // Glyph image data is always RGBA
                        &data,
                        x + BORDER_PADDING,
                        y + BORDER_PADDING,
                        image_width as u32,
                        image_height as u32,
                    );

                    let uv_rect = allocation.rectangle;
                    let uv_x =
                        (uv_rect.min.x + BORDER_PADDING as i32) as f32 / GLYPH_TEXTURE_WIDTH as f32;
                    let uv_y = (uv_rect.min.y + BORDER_PADDING as i32) as f32
                        / GLYPH_TEXTURE_HEIGHT as f32;
                    let uv_width = image_width as f32 / GLYPH_TEXTURE_WIDTH as f32;
                    let uv_height = image_height as f32 / GLYPH_TEXTURE_HEIGHT as f32;

                    let glyph_allocation = GlyphTextureAllocation {
                        glyph_type,
                        allocation_id: allocation.id,
                        uv_x,
                        uv_y,
                        uv_width,
                        uv_height,
                        glyph_top: image.placement.top,
                        glyph_left: image.placement.left,
                        glyph_width: image_width as u32,
                        glyph_height: image_height as u32,
                    };

                    self.lru_cache.push(physical_glyph.cache_key, glyph_allocation);

                    glyph_allocation
                };

                let color = match glyph_allocation.glyph_type {
                    GlyphType::Grayscale => {
                        let (r, g, b, a) = glyph
                            .color_opt
                            .map(|c| c.as_rgba_tuple())
                            .unwrap_or((255, 255, 255, 255));
                        (r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, a as f32 / 255.0)
                    },
                    GlyphType::Color => {
                        // Don't tint emojis
                        (1.0, 1.0, 1.0, 1.0)
                    },
                };

                let glyph_x = physical_glyph.x as f32 + glyph_allocation.glyph_left as f32;
                let glyph_y =
                    run.line_y + physical_glyph.y as f32 - glyph_allocation.glyph_top as f32;

                self.add_glyph(
                    glyph_x,
                    glyph_y,
                    glyph_allocation.glyph_width as f32,
                    glyph_allocation.glyph_height as f32,
                    glyph_allocation.uv_x,
                    glyph_allocation.uv_y,
                    glyph_allocation.uv_width,
                    glyph_allocation.uv_height,
                    color,
                );
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_glyph(
        &mut self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        uv_x: f32,
        uv_y: f32,
        uv_width: f32,
        uv_height: f32,
        color: (f32, f32, f32, f32),
    ) {
        let left = x;
        let top = y;
        let right = x + width;
        let bottom = y + height;

        let uv_left = uv_x;
        let uv_top = uv_y;
        let uv_right = uv_x + uv_width;
        let uv_bottom = uv_y + uv_height;

        for [pos, uv] in [
            [[left, top], [uv_left, uv_top]],
            [[left, bottom], [uv_left, uv_bottom]],
            [[right, bottom], [uv_right, uv_bottom]],
            [[right, bottom], [uv_right, uv_bottom]],
            [[right, top], [uv_right, uv_top]],
            [[left, top], [uv_left, uv_top]],
        ] {
            self.text_vertices.push(TextVertex {
                pos,
                tex_coords: uv,
                color: [color.0, color.1, color.2, color.3],
            });
        }
    }

    pub fn paint(&mut self, render_pass: &mut wgpu::RenderPass<'_>, queue: &wgpu::Queue) {
        // TODO(bschwind) - Resize vertex buffer if it's not big enough.
        let num_vertices = self.text_vertices.len();

        {
            // Write CPU data to GPU
            queue.write_buffer(
                &self.glyph_vertex_buffer,
                0,
                bytemuck::cast_slice(self.text_vertices.as_slice()),
            );
        }

        let proj = screen_projection_matrix(self.screen_width, self.screen_height);
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(proj.as_ref()));

        if num_vertices > 0 {
            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.glyph_vertex_buffer.slice(..));

            render_pass.draw(0..num_vertices as u32, 0..1);
        }

        self.text_vertices.clear();
        self.glyphs_for_this_frame.clear();

        // // Draw the glyph texture itself
        // self.add_glyph(0.0, 0.0, 1024.0, 1024.0, 0.0, 0.0, 1.0, 1.0, (1.0, 1.0, 1.0, 1.0));
        // let num_vertices = self.text_vertices.len();

        // {
        //     // TODO(bschwind) - Use map_write() for more efficiency?
        //     let mut mapping = self.rectangle_vertex_buffer.map();

        //     for (cpu_rect, gpu_vertex) in self.text_vertices.drain(..).zip(mapping.iter_mut()) {
        //         *gpu_vertex = cpu_rect;
        //     }
        // }

        // if num_vertices > 0 {
        //     surface.draw(
        //         self.rectangle_vertex_buffer.slice(0..num_vertices).unwrap(),
        //         NoIndices(PrimitiveType::TrianglesList),
        //         &self.shader,
        //         &glium::uniform! { proj: proj, glyph_texture: self.glyph_texture.sampled()/*.sampled().minify_filter(MinifySamplerFilter::Nearest).magnify_filter(MagnifySamplerFilter::Nearest)*/ },
        //         &draw_params,
        //     )?;
        // }
    }

    #[allow(clippy::too_many_arguments)]
    fn write_to_texture(
        &self,
        queue: &wgpu::Queue,
        glyph_type: GlyphType,
        bitmap: &[u8],
        x: u32,
        y: u32,
        width: u32,
        height: u32,
    ) {
        let bitmap_texture_extent = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };

        let bytes_per_row = match glyph_type {
            GlyphType::Grayscale => width,
            GlyphType::Color => width * 4,
        };

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.glyph_texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            bitmap,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: None,
            },
            bitmap_texture_extent,
        );
    }
}

#[derive(Debug, Copy, Clone)]
pub struct Color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl Color {
    pub const fn new(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self { red, green, blue, alpha }
    }

    pub const fn white() -> Self {
        Self { red: 255, green: 255, blue: 255, alpha: 255 }
    }
}

#[derive(Debug, Copy, Clone)]
enum GlyphType {
    Grayscale,
    Color,
}

pub struct ShapedTextBlock {
    buffer: Buffer,
}

impl ShapedTextBlock {
    pub fn set_size(
        &mut self,
        font_system: &mut FontSystem,
        width: Option<f32>,
        height: Option<f32>,
    ) {
        self.buffer.set_size(font_system, width, height);

        let prune = true;
        self.buffer.shape_until_scroll(font_system, prune);
    }

    pub fn set_alignment(&mut self, font_system: &mut FontSystem, align: Align) {
        for line in &mut self.buffer.lines {
            line.set_align(Some(align));
        }

        let prune = true;
        self.buffer.shape_until_scroll(font_system, prune);
    }
}

#[derive(Copy, Clone)]
struct GlyphTextureAllocation {
    glyph_type: GlyphType,
    allocation_id: AllocId,
    uv_x: f32,
    uv_y: f32,
    uv_width: f32,
    uv_height: f32,
    glyph_top: i32,
    glyph_left: i32,
    glyph_width: u32,
    glyph_height: u32,
}

pub fn system_font_db() -> fontdb::Database {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    db
}
