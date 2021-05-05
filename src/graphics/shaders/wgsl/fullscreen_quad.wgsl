struct VertexInput {
    [[location(0)]] pos: vec2<f32>;
    [[location(1)]] uv: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] pos: vec4<f32>;
    [[location(0)]] uv: vec2<f32>;
};

[[stage(vertex)]]
fn vs_main(input: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.uv = input.uv;
    out.pos = vec4<f32>(input.pos, 0.0, 1.0);

    return out;
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return vec4<f32>(in.uv.x, in.uv.y, 1.0, 1.0);
}
