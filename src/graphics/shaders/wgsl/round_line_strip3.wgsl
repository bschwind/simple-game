struct Globals {
    proj: mat4x4<f32>,
    resolution: vec4<f32>, // Only XY is used, screen width and height.
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
    point_a: vec4<f32>,

    @location(2)
    point_b: vec4<f32>,
};

struct VertexOutput {
    @builtin(position)
    pos: vec4<f32>,
};

@vertex
fn main_vs(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    let a_width = input.point_a.w;
    let b_width = input.point_b.w;

    // vec4 clip0 = projection * view * model * vec4(pointA, 1.0);
    // vec4 clip1 = projection * view * model * vec4(pointB, 1.0);

    // Transform the segment endpoints to clip space
    let clip0 = globals.proj * vec4<f32>(input.point_a.xyz, 1.0);
    let clip1 = globals.proj * vec4<f32>(input.point_b.xyz, 1.0);

    // vec2 screen0 = resolution * (0.5 * clip0.xy/clip0.w + 0.5);
    // vec2 screen1 = resolution * (0.5 * clip1.xy/clip1.w + 0.5);

    // Transform the segment endpoints to screen space
    let a = globals.resolution.xy * (0.5 * clip0.xy / clip0.w + 0.5);
    let b = globals.resolution.xy * (0.5 * clip1.xy / clip1.w + 0.5);

    // let a = input.point_a.xy;
    // let b = input.point_b.xy;

    let x_basis = normalize(b - a);
    let y_basis = vec2<f32>(-x_basis.y, x_basis.x);

    let offset_a = a + a_width * (input.pos.x * x_basis + input.pos.y * y_basis);
    let offset_b = b + b_width * (input.pos.x * x_basis + input.pos.y * y_basis);

    let final_pos = mix(offset_a, offset_b, vec2<f32>(input.pos.z));

    // vec4 clip = mix(clip0, clip1, position.z);
    let clip = mix(clip0, clip1, vec4<f32>(input.pos.z));

    // gl_Position = vec4(clip.w * ((2.0 * pt) / resolution - 1.0), clip.z, clip.w);
    out.pos = vec4<f32>(clip.w * ((2.0 * final_pos) / globals.resolution.xy - 1.0), clip.z, clip.w);

    // out.pos = globals.proj * vec4<f32>(final_pos, 0.0, 1.0);

    return out;
}

@fragment
fn main_fs() -> @location(0) vec4<f32> {
    let r = 1.0;
    let g = 1.0;
    let b = 1.0;

    return vec4<f32>(r, g, b, 1.0);
}
