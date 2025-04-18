use crate::graphics::screen_projection_matrix;
use fontdue::{
    layout::{CoordinateSystem, HorizontalAlign, Layout, LayoutSettings, TextStyle, VerticalAlign},
    Font as FontdueFont, FontSettings, Metrics,
};
use glam::Mat4;
use gpu::GlyphPainter;
use rect_packer::Packer;
use std::{
    borrow::Borrow,
    collections::{hash_map::Entry, HashMap},
};

const BITMAP_WIDTH: u32 = 4096;
const BITMAP_HEIGHT: u32 = 4096;
const BORDER_PADDING: u32 = 2;
const RECTANGLE_PADDING: u32 = 2;

pub const WHITE: Color = Color::new(255, 255, 255, 255);

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

struct FontData<F: Font> {
    /// A map of Fonts to their indices in `rasterizers` and `fonts`.
    rasterizer_indices: HashMap<F, usize>,
    rasterizers: Vec<FontdueFont>,
    fonts: Vec<F>,
}

impl<F: Font> FontData<F> {
    fn new() -> Self {
        FontData { rasterizer_indices: HashMap::new(), rasterizers: Vec::new(), fonts: Vec::new() }
    }

    /// Creates and stores a rasterizer for this Font if one doesn't already exist.
    fn create_rasterizer(&mut self, font: F) {
        // Asserting this as it otherwise causes a sudden segfault.
        assert!(font.size() > 0, "expecting a positive font size");

        if let Entry::Vacant(entry) = self.rasterizer_indices.entry(font) {
            let font_index = self.rasterizers.len();

            let rasterizer = FontdueFont::from_bytes(
                font.font_bytes(),
                FontSettings { scale: font.size() as f32, ..FontSettings::default() },
            )
            .unwrap();

            self.rasterizers.push(rasterizer);
            self.fonts.push(font);
            entry.insert(font_index);
        }
    }

    fn rasterizer_for_font(&self, font: &F) -> Option<&FontdueFont> {
        self.rasterizer_indices.get(font).map(|font_index| &self.rasterizers[*font_index])
    }

    fn font_index(&self, font: &F) -> Option<usize> {
        self.rasterizer_indices.get(font).copied()
    }

    fn font(&self, font_index: usize) -> Option<&F> {
        self.fonts.get(font_index)
    }

    fn rasterizers(&self) -> &[FontdueFont] {
        &self.rasterizers
    }
}

#[derive(Debug)]
enum RasterizeResult {
    /// The glyph exists and was successfully packed into the
    /// the glyph texture.
    Packed,

    /// The glyph was a whitespace character which doesn't need
    /// to be packed into the glyph texture.
    WhitespaceChar,

    // Issue here: https://github.com/mooman219/fontdue/issues/43
    /// The glyph was missing, but a fallback character was still
    /// packed into the glyph texture.
    GlyphMissing,
}

#[derive(Debug)]
pub enum RasterizationError {
    NoTextureSpace,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StyledCharacter<F: Font> {
    pub character: char,
    pub font: F,
}

#[derive(Debug, Clone)]
pub struct CharacterMetadata {
    _metrics: Metrics,
    texture_x: f32,      // Texture space
    texture_y: f32,      // Texture space
    texture_width: f32,  // Texture space
    texture_height: f32, // Texture space
}

pub struct StyledText<'a, F: Font> {
    pub text: &'a str,
    pub font: F,
    pub color: Color,
}

impl<'a, F: Font> StyledText<'a, F> {
    pub fn default_styling(text: &'a str) -> Self {
        StyledText { text, font: F::default(), color: WHITE }
    }
}

/// Where to align on a particular axis.
/// Y: Start = top of the text box aligned to the Y coord
///    End   = bottom of the text box aligned to the Y coord
/// X: Start = left side of the text box aligned to the X coord
///    End   = right side of the text box aligned to the X coord
/// Units are in pixels.
#[derive(Debug)]
pub enum AxisAlign {
    Start(i32),
    End(i32),
    Center(i32),
    WindowCenter,
}

impl Default for AxisAlign {
    fn default() -> Self {
        AxisAlign::Start(0)
    }
}

/// Describes alignment for a block of text. Max width
/// and height are optional and default to the window width
/// and height.
#[derive(Debug, Default)]
pub struct TextAlignment {
    pub x: AxisAlign,
    pub y: AxisAlign,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
}

impl TextAlignment {
    pub fn new(x: AxisAlign, y: AxisAlign) -> Self {
        Self { x, y, max_width: None, max_height: None }
    }

    pub fn left_top(x: i32, y: i32) -> Self {
        Self { x: AxisAlign::Start(x), y: AxisAlign::Start(y), max_width: None, max_height: None }
    }

    fn into_layout_settings(self, (width, height): (u32, u32)) -> LayoutSettings {
        let window_width = width as i32;
        let window_height = height as i32;
        let max_width = self.max_width.unwrap_or(window_width as u32) as i32;
        let max_height = self.max_height.unwrap_or(window_height as u32) as i32;

        let (x, horizontal_align) = match self.x {
            AxisAlign::Start(x) => (x, HorizontalAlign::Left),
            AxisAlign::End(x) => (window_width - x - max_width, HorizontalAlign::Right),
            AxisAlign::Center(x) => (x - (max_width / 2), HorizontalAlign::Center),
            AxisAlign::WindowCenter => {
                ((window_width / 2) - (max_width / 2), HorizontalAlign::Center)
            },
        };

        let (y, vertical_align) = match self.y {
            AxisAlign::Start(y) => (y, VerticalAlign::Top),
            AxisAlign::End(y) => (window_height - y - max_height, VerticalAlign::Bottom),
            AxisAlign::Center(y) => (y - (max_height / 2), VerticalAlign::Middle),
            AxisAlign::WindowCenter => {
                ((window_height / 2) - (max_height / 2), VerticalAlign::Middle)
            },
        };

        LayoutSettings {
            x: x as f32,
            y: y as f32,
            max_width: Some(max_width as f32),
            max_height: Some(max_height as f32),
            horizontal_align,
            vertical_align,
            ..LayoutSettings::default()
        }
    }
}

// TODO - Make this public only to the module
#[derive(Debug)]
pub struct PositionedGlyph {
    x: f32,
    y: f32,
    width: usize,
    height: usize,
    color: Color,

    // Texture properties
    texture_x: f32,
    texture_y: f32,
    texture_width: f32,
    texture_height: f32,
}

pub struct TextSystem<F: Font = DefaultFont> {
    font_data: FontData<F>,

    /// A map of styled characters to their associated metadata
    /// (their location in the font bitmap, width, height, etc.)
    char_metadata: HashMap<StyledCharacter<F>, CharacterMetadata>,

    /// Data structure to pack glyph rectangles into a larger GPU bitmap.
    glyph_packer: Packer,

    /// Object to perform text layout on content blocks.
    layout: Layout<usize>,

    /// GPU glyph renderer.
    glpyh_painter: GlyphPainter,

    // The projection used to map pixel coordinates to normalized device coordinates.
    projection: Mat4,

    screen_width: u32,
    screen_height: u32,
}

impl<F: Font> TextSystem<F> {
    pub fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        depth_format: Option<wgpu::TextureFormat>,
        screen_width: u32,
        screen_height: u32,
    ) -> Self {
        let font_data = FontData::new();
        let char_metadata = HashMap::new();

        let packer_config = rect_packer::Config {
            width: BITMAP_WIDTH as i32,
            height: BITMAP_HEIGHT as i32,
            border_padding: BORDER_PADDING as i32,
            rectangle_padding: RECTANGLE_PADDING as i32,
        };

        let glyph_packer = Packer::new(packer_config);
        let layout = Layout::new(CoordinateSystem::PositiveYDown);

        let glpyh_painter = GlyphPainter::new(device, target_format, depth_format);

        let projection = screen_projection_matrix(screen_width, screen_height);

        Self {
            font_data,
            char_metadata,
            glyph_packer,
            layout,
            glpyh_painter,
            projection,
            screen_width,
            screen_height,
        }
    }

    pub fn resize(&mut self, screen_width: u32, screen_height: u32) {
        self.projection = screen_projection_matrix(screen_width, screen_height);
        self.screen_width = screen_width;
        self.screen_height = screen_height;
    }

    /// Rasterizes and caches this character in the glyph texture.
    /// Returns Some(RasterizeResult) if the character is packed into the texture,
    /// otherwise None.
    fn rasterize_and_cache(
        &mut self,
        c: StyledCharacter<F>,
        queue: &wgpu::Queue,
    ) -> Result<RasterizeResult, RasterizationError> {
        let metadata = self.char_metadata.entry(c);

        match metadata {
            Entry::Occupied(_) => {
                // Good to go, this character already exists
                Ok(RasterizeResult::Packed)
            },
            Entry::Vacant(entry) => {
                let styled_char = entry.key();

                let character = styled_char.character;
                let font_size = styled_char.font.size() as f32;

                let rasterizer =
                    self.font_data.rasterizer_for_font(&styled_char.font).unwrap_or_else(|| {
                        panic!("Rasterizer should exist for Font: {:?}", styled_char.font)
                    });

                let (metrics, bitmap) = rasterizer.rasterize(character, font_size);
                let can_rotate = false;

                if metrics.width == 0 || metrics.height == 0 {
                    // This was likely a whitespace character which isn't missing from the font
                    // but does not have an actual bitmap. The rectangle packer would fail on
                    // this case so we return here as everything will still work.
                    return Ok(RasterizeResult::WhitespaceChar);
                }

                if let Some(packed_rect) =
                    self.glyph_packer.pack(metrics.width as i32, metrics.height as i32, can_rotate)
                {
                    let float_width = BITMAP_WIDTH as f32;
                    let float_height = BITMAP_HEIGHT as f32;

                    let char_metadata = CharacterMetadata {
                        _metrics: metrics,
                        texture_x: packed_rect.x as f32 / float_width,
                        texture_y: packed_rect.y as f32 / float_height,
                        texture_width: packed_rect.width as f32 / float_width,
                        texture_height: packed_rect.height as f32 / float_height,
                    };

                    entry.insert(char_metadata);

                    self.glpyh_painter.write_to_texture(
                        queue,
                        &bitmap,
                        packed_rect.x as u32,
                        packed_rect.y as u32,
                        packed_rect.width as u32,
                        packed_rect.height as u32,
                    );

                    let glyph_missing = rasterizer.lookup_glyph_index(character) == 0;

                    if glyph_missing {
                        Ok(RasterizeResult::GlyphMissing)
                    } else {
                        Ok(RasterizeResult::Packed)
                    }
                } else {
                    // Couldn't pack into texture, resize it
                    println!("Couldn't pack char: {:?} into glyph texture", character);
                    Err(RasterizationError::NoTextureSpace)
                }
            },
        }
    }

    /// Call this for each "block" of text you want to render in a particular location.
    /// Each element in the `text` slice can have a different style and they are rendered
    /// one after the other so a given line of text can have multiple styles and colors.
    pub fn render_horizontal<'a, T: Borrow<StyledText<'a, F>>>(
        &mut self,
        text_alignment: TextAlignment,
        text_elements: &[T],
        render_pass: &mut wgpu::RenderPass,
        queue: &wgpu::Queue,
    ) {
        for text_element in text_elements {
            let text_element = text_element.borrow();

            self.font_data.create_rasterizer(text_element.font);

            for c in text_element.text.chars() {
                let styled_char = StyledCharacter { character: c, font: text_element.font };
                if let Err(err) = self.rasterize_and_cache(styled_char, queue) {
                    println!("Error rasterizing character: {:?} - {:?}", c, err);
                }
            }
        }

        let styles: Vec<_> = text_elements
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let t = t.borrow();
                TextStyle {
                    user_data: i,
                    text: t.text,
                    px: t.font.size() as f32,
                    font_index: self
                        .font_data
                        .font_index(&t.font)
                        .unwrap_or_else(|| panic!("Missing font index for font: {:?}", t.font)),
                }
            })
            .collect();

        let layout_settings =
            text_alignment.into_layout_settings((self.screen_width, self.screen_width));

        self.layout.reset(&layout_settings);
        let fonts = &self.font_data.rasterizers();
        for style in styles {
            self.layout.append(fonts, &style);
        }

        let glyphs = self.layout.glyphs();
        let char_metadata = &self.char_metadata;
        let font_data = &self.font_data;

        let position_data: Vec<_> = glyphs
            .iter()
            .filter_map(|d| {
                char_metadata
                    .get(&StyledCharacter {
                        character: d.key.c,
                        font: *font_data.font(d.key.font_index).unwrap_or_else(|| {
                            panic!(
                                "Should have a font for the given font index: {}",
                                d.key.font_index
                            )
                        }),
                    })
                    .map(|metadata| {
                        let color = text_elements[d.user_data].borrow().color;

                        PositionedGlyph {
                            x: d.x,
                            y: d.y,
                            width: d.width,
                            height: d.height,
                            texture_x: metadata.texture_x,
                            texture_y: metadata.texture_y,
                            texture_width: metadata.texture_width,
                            texture_height: metadata.texture_height,
                            color,
                        }
                    })
            })
            .collect();

        // TODO(bschwind) - Make an API for queueing up text to render, collect all
        // the output from fontdue, and then render it all at once to reduce GPU draw calls.
        self.glpyh_painter.render(
            &position_data,
            render_pass,
            queue,
            (self.screen_width, self.screen_height),
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
}

mod gpu {
    use super::{BITMAP_HEIGHT, BITMAP_WIDTH};
    use crate::{
        graphics::{screen_projection_matrix, text::PositionedGlyph},
        GraphicsDevice,
    };
    use bytemuck::{Pod, Zeroable};
    use glam::Mat4;
    use wgpu::{util::DeviceExt, BindGroup, Buffer, RenderPipeline, Texture};

    const MAX_INSTANCE_COUNT: usize = 40_000;

    /// Vertex attributes for instanced glyph data.
    #[repr(C)]
    #[derive(Debug, Copy, Clone, Pod, Zeroable)]
    struct GlyphInstanceData {
        /// XY position of the bottom left of the glyph in pixels
        pos: [f32; 2],

        /// The width and height of the rendered glyph, in pixels.
        size: [f32; 2],

        /// The UV coordinates of the top-left corner of the glpyh
        /// and the width/height of the glyph, both in texture space.
        uv_extents: [f32; 4],

        /// The color of the glyph, including alpha.
        color: [f32; 4],
    }

    impl Default for GlyphInstanceData {
        fn default() -> Self {
            GlyphInstanceData {
                pos: [0.0, 0.0],
                size: [0.0, 0.0],
                uv_extents: [0.0, 0.0, 0.0, 0.0],
                color: [1.0, 1.0, 1.0, 1.0],
            }
        }
    }

    /// Vertex attributes for our single glpyh quad.
    #[repr(C)]
    #[derive(Debug, Copy, Clone, Pod, Zeroable)]
    struct GlyphQuadVertex {
        /// UV coordinates for one vertex, in texture space.
        uv: [f32; 2],
    }

    /// This font renderer uses instanced rendering to draw quads for each
    /// glyph.
    /// Reference: https://learnopengl.com/Advanced-OpenGL/Instancing
    /// A single "unit quad" is stored in the vertex buffer. It only requires
    /// the default UV data for each vertex (0.0 - 1.0).
    /// There is also a dynamic vertex buffer. Each element in this buffer stores
    /// the data required to render one glyph. We update this buffer when the font
    /// system tells us where and how many glyphs to render.
    pub struct GlyphPainter {
        glyph_texture: Texture,
        glyph_vertex_buffer: Buffer,
        index_buffer: Buffer,
        instance_buffer: Buffer,
        uniform_buffer: wgpu::Buffer,
        bind_group: BindGroup,
        pipeline: RenderPipeline,
    }

    impl GlyphPainter {
        pub fn new(
            device: &wgpu::Device,
            target_format: wgpu::TextureFormat,
            depth_format: Option<wgpu::TextureFormat>,
        ) -> Self {
            let glyph_texture = Self::build_glyph_texture(device);
            let glyph_vertex_buffer = Self::build_vertex_buffer(device);
            let index_buffer = Self::build_index_buffer(device);
            let instance_buffer = Self::build_instance_buffer(device);
            let uniform_buffer = Self::build_uniform_buffer(device);

            let bind_group_layout =
                device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("GlyphPainter bind group layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: core::num::NonZeroU64::new(
                                    std::mem::size_of::<Mat4>() as u64,
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
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Linear,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("GlyphPainter bind group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: uniform_buffer.as_entire_binding(),
                    },
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

            let vertex_buffers = &[
                wgpu::VertexBufferLayout {
                    array_stride: (std::mem::size_of::<GlyphQuadVertex>()) as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &wgpu::vertex_attr_array![
                        0 => Float32x2, // UV
                    ],
                },
                wgpu::VertexBufferLayout {
                    array_stride: (std::mem::size_of::<GlyphInstanceData>()) as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Instance,
                    attributes: &wgpu::vertex_attr_array![
                        1 => Float32x2, // pos
                        2 => Float32x2, // size
                        3 => Float32x4, // uv_extents
                        4 => Float32x4, // color
                    ],
                },
            ];

            let draw_shader =
                GraphicsDevice::load_wgsl_shader(device, include_str!("shaders/wgsl/glyph.wgsl"));

            let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("GlyphPainter render pipeline"),
                layout: Some(&pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &draw_shader,
                    entry_point: Some("main_vs"),
                    buffers: vertex_buffers,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleStrip,
                    strip_index_format: Some(wgpu::IndexFormat::Uint16),
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Front),
                    polygon_mode: wgpu::PolygonMode::Fill,
                    conservative: false,
                    ..wgpu::PrimitiveState::default()
                },
                depth_stencil: depth_format.map(|f| wgpu::DepthStencilState {
                    format: f,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil: wgpu::StencilState::default(),
                    bias: wgpu::DepthBiasState::default(),
                }),
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment: Some(wgpu::FragmentState {
                    module: &draw_shader,
                    entry_point: Some("main_fs"),
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

            Self {
                glyph_texture,
                glyph_vertex_buffer,
                index_buffer,
                instance_buffer,
                uniform_buffer,
                bind_group,
                pipeline,
            }
        }

        pub fn render(
            &mut self,
            glyph_positions: &[PositionedGlyph],
            render_pass: &mut wgpu::RenderPass,
            queue: &wgpu::Queue,
            (width, height): (u32, u32),
        ) {
            if glyph_positions.len() > MAX_INSTANCE_COUNT {
                println!("Trying to render more glyphs than the maximum. Max = {}, attempted render count = {}", MAX_INSTANCE_COUNT, glyph_positions.len());
                return;
            }

            let instance_data: Vec<_> = glyph_positions
                .iter()
                .map(|g| GlyphInstanceData {
                    pos: [g.x, g.y],
                    size: [g.width as f32, g.height as f32],
                    uv_extents: [g.texture_x, g.texture_y, g.texture_width, g.texture_height],
                    color: [
                        g.color.red as f32 / 255.0,
                        g.color.green as f32 / 255.0,
                        g.color.blue as f32 / 255.0,
                        g.color.alpha as f32 / 255.0,
                    ],
                })
                .collect();

            queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instance_data));

            // TODO(bschwind) - Only write to the uniform buffer when the window resizes.
            let proj = screen_projection_matrix(width, height);
            queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(proj.as_ref()));

            render_pass.set_pipeline(&self.pipeline);
            render_pass.set_bind_group(0, &self.bind_group, &[]);
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.set_vertex_buffer(0, self.glyph_vertex_buffer.slice(..));
            render_pass.set_vertex_buffer(
                1,
                self.instance_buffer.slice(..std::mem::size_of_val(glyph_positions) as u64),
            );

            render_pass.draw_indexed(0..4u32, 0, 0..glyph_positions.len() as u32);
        }

        pub fn write_to_texture(
            &self,
            queue: &wgpu::Queue,
            bitmap: &[u8],
            x: u32,
            y: u32,
            width: u32,
            height: u32,
        ) {
            let bitmap_texture_extent = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };

            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &self.glyph_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x, y, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                bitmap,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(width),
                    rows_per_image: None,
                },
                bitmap_texture_extent,
            );
        }

        fn build_glyph_texture(device: &wgpu::Device) -> Texture {
            let glyph_texture_extent = wgpu::Extent3d {
                width: BITMAP_WIDTH,
                height: BITMAP_HEIGHT,
                depth_or_array_layers: 1,
            };

            device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Glyph texture"),
                size: glyph_texture_extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::R8Unorm,
                view_formats: &[],
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            })
        }

        fn build_vertex_buffer(device: &wgpu::Device) -> Buffer {
            let vertex_data = vec![
                GlyphQuadVertex { uv: [0.0, 1.0] },
                GlyphQuadVertex { uv: [0.0, 0.0] },
                GlyphQuadVertex { uv: [1.0, 0.0] },
                GlyphQuadVertex { uv: [1.0, 1.0] },
            ];

            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Glyph Vertex Buffer"),
                contents: bytemuck::cast_slice(&vertex_data),
                usage: wgpu::BufferUsages::VERTEX,
            })
        }

        fn build_index_buffer(device: &wgpu::Device) -> Buffer {
            let index_data = vec![0u16, 1, 3, 2];

            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Glyph Index Buffer"),
                contents: bytemuck::cast_slice(&index_data),
                usage: wgpu::BufferUsages::INDEX,
            })
        }

        fn build_instance_buffer(device: &wgpu::Device) -> Buffer {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Glyph Instance Buffer"),
                size: MAX_INSTANCE_COUNT as u64 * std::mem::size_of::<GlyphInstanceData>() as u64, // TODO - multiply by instance size?
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        }

        fn build_uniform_buffer(device: &wgpu::Device) -> Buffer {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Glyph Uniform Buffer"),
                size: std::mem::size_of::<Mat4>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            })
        }
    }
}
