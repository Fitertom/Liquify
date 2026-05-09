@group(0) @binding(0)
var my_sampler: sampler;

@group(0) @binding(1)
var my_texture: texture_2d<f32>;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(my_texture, my_sampler, input.tex_coord);
    let alpha = tex_color.r * input.color.a;
    return vec4<f32>(input.color.rgb * alpha, alpha);
}
