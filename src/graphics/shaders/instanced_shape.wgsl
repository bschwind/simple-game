[[block]]
struct Globals {
    proj: mat4x4<f32>;
};

// Uniforms
[[group(0), binding(0)]]
var<uniform> globals: Globals;

// Per instance data
[[location(0)]]
var<in> in_instance_data: vec4<f32>;

// Per mesh vertex data
[[location(1)]]
var<in> in_vertex_pos: vec3<f32>;

// Vertex shader output
[[builtin(position)]]
var<out> out_position: vec4<f32>;

[[stage(vertex)]]
fn main() {
    var cos_angle: f32 = cos(in_instance_data.w);
    var sin_angle: f32 = sin(in_instance_data.w);

    var rot_matrix: mat2x2<f32> = mat2x2<f32>(
        vec2<f32>(cos_angle, sin_angle),
        vec2<f32>(-sin_angle, cos_angle)
    );

    var rotated_pos: vec2<f32> = rot_matrix * (in_vertex_pos.xy * in_instance_data.z);

    out_position = vec4<f32>(rotated_pos + vec2<f32>(in_instance_data.x, in_instance_data.y), 0.0, 1.0);
    out_position = globals.proj * out_position;
}

[[location(0)]]
var<out> out_color: vec4<f32>;

[[stage(fragment)]]
fn main() {
    var r: f32 = 1.0;
    var g: f32 = 1.0;
    var b: f32 = 1.0;

    out_color = vec4<f32>(r, g, b, 1.0);
}
