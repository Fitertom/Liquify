@group(0) @binding(0) var my_sampler: sampler;
@group(0) @binding(1) var mask_texture: texture_2d<f32>;
@group(0) @binding(2) var<uniform> screen_size: vec2<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var pos = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0)
    );
    var out: VertexOutput;
    out.position = vec4<f32>(pos[vertex_index], 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(@builtin(position) frag_pos: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag_pos.xy / screen_size;
    let mask = textureSample(mask_texture, my_sampler, uv).a;
    let alpha = mask;
    if (alpha < 0.01) {
        discard;
    }
    let t = (frag_pos.x / screen_size.x + frag_pos.y / screen_size.y) * 0.5;
    let color_start = vec3<f32>(0.0, 0.0, 0.0);
    let color_end = vec3<f32>(1.0, 1.0, 1.0);
    let color = mix(color_start, color_end, t);
    return vec4<f32>(color, alpha);
}
