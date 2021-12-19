struct Globals {
    proj: mat4x4<f32>;
};

// Uniforms
[[group(0), binding(0)]]
var<uniform> globals: Globals;

struct VertexInput {
    [[location(0)]] instance_data: vec4<f32>; // Per instance data (x, y, radius, rotation)
    [[location(1)]] pos: vec3<f32>; // Per mesh vertex data
};

struct VertexOutput {
    [[builtin(position)]] pos: vec4<f32>;
};

[[stage(vertex)]]
fn main_vs(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let cos_angle = cos(input.instance_data.w);
    let sin_angle = sin(input.instance_data.w);

    let rot_matrix = mat2x2<f32>(
        vec2<f32>(cos_angle, sin_angle),
        vec2<f32>(-sin_angle, cos_angle)
    );

    let rotated_pos = rot_matrix * (input.pos.xy * input.instance_data.z);

    let out_position = vec4<f32>(rotated_pos + vec2<f32>(input.instance_data.x, input.instance_data.y), 0.0, 1.0);
    out.pos = globals.proj * out_position;

    return out;
}

[[stage(fragment)]]
fn main_fs() -> [[location(0)]] vec4<f32> {
    let r = 1.0;
    let g = 1.0;
    let b = 1.0;

    return vec4<f32>(r, g, b, 1.0);
}
