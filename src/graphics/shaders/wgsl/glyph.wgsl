[[block]]
struct Globals {
    proj: mat4x4<f32>;
};

// Uniforms
[[group(0), binding(0)]]
var globals: Globals;

struct VertexInput {
    // Per-vertex data
    [[location(0)]] uv: vec2<f32>; // Default UV coords from the glyph quad
    // Per-instance data
    [[location(1)]] pos: vec2<f32>;
    [[location(2)]] size: vec2<f32>;
    [[location(3)]] uv_extents: vec4<f32>;
    [[location(4)]] color: vec4<f32>;
};

struct VertexOutput {
    [[builtin(position)]] pos: vec4<f32>;
    [[location(0)]] glyph_uv: vec2<f32>;
    [[location(1)]] glyph_color: vec4<f32>;
};

[[stage(vertex)]]
fn main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.glyph_uv = input.uv_extents.xy + (input.uv_extents.zw * input.uv);
    out.glyph_color = input.color;

    let output_pos = vec4<f32>(input.pos + (input.size * input.uv), 0.0, 1.0);
    out.pos = globals.proj * output_pos;

    return out;
}

[[group(0), binding(1)]]
var glyph_texture: texture_2d<f32>;
[[group(0), binding(2)]]
var glyph_texture_sampler: sampler;

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let glyph_alpha = textureSample(glyph_texture, glyph_texture_sampler, in.glyph_uv).r;
    return vec4<f32>(in.glyph_color.rgb, glyph_alpha * in.glyph_color.a);
}
