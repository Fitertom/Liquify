@group(0) @binding(0) var t_src: texture_2d<f32>;
@group(0) @binding(1) var s_src: sampler;
@group(0) @binding(2) var<uniform> params: BlurParams;

struct BlurParams {
    texel_size: vec2<f32>,
    radius: f32,
    direction: f32,
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
    let dir = select(
        vec2(params.texel_size.x, 0.0),
        vec2(0.0, params.texel_size.y),
        params.direction > 0.5,
    );

    // Optimized Gaussian: single sample per iteration
    // Performance restored to 1x sampling
    let sigma = max(params.radius / 2.0, 1.0);
    let r = i32(ceil(params.radius * 1.5));
    
    var color = vec4<f32>(0.0);
    var total_weight = 0.0;
    let two_sigma_sq = 2.0 * sigma * sigma;
    
    for (var i = -r; i <= r; i++) {
        let x = f32(i);
        let weight = exp(-(x * x) / two_sigma_sq);
        color += textureSample(t_src, s_src, in.uv + dir * x) * weight;
        total_weight += weight;
    }

    return color / total_weight;
}
