@group(0) @binding(0) var t_src: texture_2d<f32>;
@group(0) @binding(1) var s_src: sampler;
@group(0) @binding(2) var<uniform> params: BlurParams;

struct BlurParams {
    texel_size: vec2<f32>,  // vec2(1.0/width, 1.0/height)
    radius:     f32,        // радиус в пикселях, например 5.0
    direction:  f32,        // 0.0 = горизонталь, 1.0 = вертикаль
}

struct VertexOut {
    @builtin(position) pos: vec4<f32>,
    @location(0)       uv:  vec2<f32>,
}

// fullscreen triangle — никакого vertex buffer не нужно
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
    out.uv  = p * vec2(0.5, -0.5) + 0.5;  // NDC → UV, Y флипнут
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let r = i32(params.radius);
    let dir = select(
        vec2(params.texel_size.x, 0.0),  // direction == 0 → горизонталь
        vec2(0.0, params.texel_size.y),  // direction == 1 → вертикаль
        params.direction > 0.5,
    );

    var color = vec4(0.0);
    let diam  = f32(2 * r + 1);

    for (var i = -r; i <= r; i++) {
        let offset = dir * f32(i);
        color += textureSample(t_src, s_src, in.uv + offset);
    }

    return color / diam;
}