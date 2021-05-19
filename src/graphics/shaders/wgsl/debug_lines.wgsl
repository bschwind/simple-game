[[block]]
struct Globals {
    proj: mat4x4<f32>;
};

// Uniforms
[[group(0), binding(0)]]
var<uniform> globals: Globals;

struct VertexInput {
    [[location(0)]] pos: vec3<f32>;
};

struct VertexOutput {
    [[builtin(position)]] pos: vec4<f32>;
};

[[stage(vertex)]]
fn main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let out_position = vec4<f32>(input.pos, 1.0);
    out.pos = globals.proj * out_position;

    return out;
}

[[stage(fragment)]]
fn main() -> [[location(0)]] vec4<f32> {
    let r = 1.0;
    let g = 1.0;
    let b = 1.0;

    return vec4<f32>(r, g, b, 1.0);
}
