[[block]]
struct Globals {
    proj: mat4x4<f32>;
};

// Uniforms
[[group(0), binding(0)]]
var<uniform> globals: Globals;

struct VertexInput {
    // Per-vertex data
    [[location(0)]] pos: vec3<f32>;

    // Per-instance data
    [[location(1)]] point_a: vec2<f32>;
    [[location(2)]] point_b: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] pos: vec4<f32>;
};

[[stage(vertex)]]
fn main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let width = 40.0;
    let a = input.point_a;
    let b = input.point_b;

    let x_basis = normalize(b - a);
    let y_basis = vec2<f32>(-x_basis.y, x_basis.x);

    let offset_a = a + width * (input.pos.x * x_basis + input.pos.y * y_basis);
    let offset_b = b + width * (input.pos.x * x_basis + input.pos.y * y_basis);

    let point = mix(offset_a, offset_b, vec2<f32>(input.pos.z));

    out.pos = globals.proj * vec4<f32>(point, 0.0, 1.0);

    return out;
}

[[stage(fragment)]]
fn main() -> [[location(0)]] vec4<f32> {
    let r = 1.0;
    let g = 1.0;
    let b = 1.0;

    return vec4<f32>(r, g, b, 1.0);
}