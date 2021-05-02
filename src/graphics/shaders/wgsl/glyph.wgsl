struct VertexOutput {
    [[location(0)]] glyph_uv: vec2<f32>;
    [[location(1)]] glyph_color: vec4<f32>;
    [[builtin(position)]] my_pos: vec4<f32>;
};

[[block]]
struct Globals {
    proj: mat4x4<f32>;
};

// Uniforms
[[group(0), binding(0)]]
var globals: Globals;

[[stage(vertex)]]
fn vs_main(
    // Per-vertex data
    [[location(0)]] uv: vec2<f32>, // Default UV coords from the glyph quad
    // Per-instance data
    [[location(1)]] pos: vec2<f32>,
    [[location(2)]] size: vec2<f32>,
    [[location(3)]] uv_extents: vec4<f32>,
    [[location(4)]] color: vec4<f32>,
) -> VertexOutput {
    var out: VertexOutput;

    out.glyph_uv = uv_extents.xy + (uv_extents.zw * uv);
    out.glyph_color = color;

    let output_pos: vec4<f32> = vec4<f32>(pos + (size * uv), 0.0, 1.0);
    out.my_pos = globals.proj * output_pos;

    return out;
}

[[group(0), binding(1)]]
var glyph_texture: texture_2d<f32>;
[[group(0), binding(2)]]
var glyph_texture_sampler: sampler;

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let glyph_alpha = textureSample(glyph_texture, glyph_texture_sampler, in.glyph_uv).r;
    return vec4<f32>(in.glyph_color.rgb, glyph_alpha * in.glyph_color.a);
}
