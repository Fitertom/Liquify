@group(0) @binding(0) var t_src: texture_2d<f32>;
@group(0) @binding(1) var s_src: sampler;
@group(0) @binding(2) var<uniform> params: ChromaticParams;

struct ChromaticParams {
    strength: f32,
    _pad: vec3<f32>,
}

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) idx: u32) -> VertexOut {
    var positions = array<vec2<f32>, 3>(
        vec2(-1.0, -1.0),
        vec2( 3.0, -1.0),
        vec2(-1.0,  3.0),
    );
    let p = positions[idx];
    var out: VertexOut;
    out.pos = vec4(p, 0.0, 1.0);
    out.uv  = p * vec2(0.5, -0.5) + 0.5;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let dist_from_center = length(in.uv - 0.5);
    let offset = (in.uv - 0.5) * params.strength * dist_from_center;

    let r = textureSample(t_src, s_src, in.uv + offset).r;
    let g = textureSample(t_src, s_src, in.uv).g;
    let b = textureSample(t_src, s_src, in.uv - offset).b;

    return vec4<f32>(r, g, b, 1.0);
}
