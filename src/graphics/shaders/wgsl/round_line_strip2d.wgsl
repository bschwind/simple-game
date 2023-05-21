struct Globals {
    proj: mat4x4<f32>,
};

// Uniforms
@group(0) @binding(0)
var<uniform> globals: Globals;

struct VertexInput {
    // Per-vertex data
    @location(0)
    pos: vec3<f32>,

    // Per-instance data
    @location(1)
    point_a: vec3<f32>,

    @location(2)
    point_b: vec3<f32>,
};

struct VertexOutput {
    @builtin(position)
    pos: vec4<f32>,
};

@vertex
fn main_vs(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let a_width = input.point_a.z;
    let b_width = input.point_b.z;

    let a = input.point_a.xy;
    let b = input.point_b.xy;

    let x_basis = normalize(b - a);
    let y_basis = vec2<f32>(-x_basis.y, x_basis.x);

    let offset_a = a + a_width * (input.pos.x * x_basis + input.pos.y * y_basis);
    let offset_b = b + b_width * (input.pos.x * x_basis + input.pos.y * y_basis);

    let final_pos = mix(offset_a, offset_b, vec2<f32>(input.pos.z));

    out.pos = globals.proj * vec4<f32>(final_pos, 0.0, 1.0);

    return out;
}

@fragment
fn main_fs() -> @location(0) vec4<f32> {
    let r = 1.0;
    let g = 1.0;
    let b = 1.0;

    return vec4<f32>(r, g, b, 1.0);
}
