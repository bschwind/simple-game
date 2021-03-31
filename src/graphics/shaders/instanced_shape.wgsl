[[block]]
struct Globals {
    proj: mat4x4<f32>;
};

// Uniforms
[[group(0), binding(0)]]
var<uniform> globals: Globals;

[[stage(vertex)]]
fn main(
    [[location(0)]] in_instance_data: vec4<f32>, // Per instance data
    [[location(1)]] in_vertex_pos: vec3<f32>, // Per mesh vertex data
) -> [[builtin(position)]] vec4<f32> {
    const cos_angle: f32 = cos(in_instance_data.w);
    const sin_angle: f32 = sin(in_instance_data.w);

    const rot_matrix: mat2x2<f32> = mat2x2<f32>(
        vec2<f32>(cos_angle, sin_angle),
        vec2<f32>(-sin_angle, cos_angle)
    );

    const rotated_pos: vec2<f32> = rot_matrix * (in_vertex_pos.xy * in_instance_data.z);

    const out_position = vec4<f32>(rotated_pos + vec2<f32>(in_instance_data.x, in_instance_data.y), 0.0, 1.0);
    return globals.proj * out_position;
}

[[stage(fragment)]]
fn main() -> [[location(0)]] vec4<f32> {
    const r: f32 = 1.0;
    const g: f32 = 1.0;
    const b: f32 = 1.0;

    return vec4<f32>(r, g, b, 1.0);
}
