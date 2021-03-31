[[block]]
struct Globals {
    proj: mat4x4<f32>;
};

// Uniforms
[[group(0), binding(0)]]
var<uniform> globals: Globals;

[[stage(vertex)]]
fn main(
    [[location(0)]] in_position: vec3<f32>,
) -> [[builtin(position)]] vec4<f32> {
    const out_position = vec4<f32>(in_position, 1.0);
    return globals.proj * out_position;
}

[[stage(fragment)]]
fn main() -> [[location(0)]] vec4<f32> {
    var r: f32 = 1.0;
    var g: f32 = 1.0;
    var b: f32 = 1.0;

    return vec4<f32>(r, g, b, 1.0);
}
