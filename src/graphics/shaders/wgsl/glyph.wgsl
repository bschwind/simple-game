struct Globals {
    proj: mat4x4<f32>,
};

// Uniforms
@group(0) @binding(0)
var<uniform> globals: Globals;

struct VertexInput {
    @location(0)
    pos: vec2<f32>,

    @location(1)
    tex_coords: vec2<f32>,

    @location(2)
    color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position)
    pos: vec4<f32>,

    @location(0)
    glyph_uv: vec2<f32>,

    @location(1)
    glyph_color: vec4<f32>,
};

@vertex
fn main_vs(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.glyph_uv = input.tex_coords;
    out.glyph_color = input.color;

    let output_pos = vec4<f32>(input.pos, 0.0, 1.0);
    out.pos = globals.proj * output_pos;

    return out;
}

@group(0) @binding(1)
var glyph_texture: texture_2d<f32>;
@group(0) @binding(2)
var glyph_texture_sampler: sampler;

@fragment
fn main_fs(in: VertexOutput) -> @location(0) vec4<f32> {
    let sampled_color = textureSample(glyph_texture, glyph_texture_sampler, in.glyph_uv);
    return vec4<f32>(sampled_color.rgb * in.glyph_color.a * in.glyph_color.rgb, sampled_color.a * in.glyph_color.a);
}
