[[block]]
struct Globals {
    proj: mat4x4<f32>;
};

// Uniforms
[[group(0), binding(0)]]
var globals: Globals;

struct VertexInput {
    [[location(0)]] pos: vec2<f32>;
    [[location(1)]] uv: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] pos: vec4<f32>;
    [[location(0)]] uv: vec2<f32>;
};

[[stage(vertex)]]
fn main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.uv = input.uv;
    out.pos = globals.proj * vec4<f32>(input.pos, 0.0, 1.0);

    return out;
}

[[group(1), binding(0)]]
var image_texture: texture_2d<f32>;
[[group(1), binding(1)]]
var image_texture_sampler: sampler;

[[stage(fragment)]]
fn main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    let color = textureSample(image_texture, image_texture_sampler, in.uv);
    return color;
}
