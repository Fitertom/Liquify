struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vs_main(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    // clip_position is passed directly as NDC from pipeline
    out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
    out.uv = model.uv;
    out.color = model.color;
    return out;
}

// MSDF atlas: multi-channel signed distance field stored in RGB.
// Generated with a tool like msdfgen (Chlumsky) or msdf-atlas-gen.
// Alpha channel is unused (or holds a conventional SDF — ignored here).
@group(0) @binding(0) var sdf_texture: texture_2d<f32>;
@group(0) @binding(1) var sdf_sampler: sampler;

// Median-of-three: the core of MSDF decoding.
//
// Bilinear interpolation corrupts SDF values at sharp corners because
// each channel encodes a *different* pseudo-distance that happens to agree
// on the boundary (value == 0.5). Taking the channel-wise median restores
// the correct boundary while preserving the superior corner sharpness that
// makes MSDF better than single-channel SDF at every scale.
fn median(r: f32, g: f32, b: f32) -> f32 {
    return max(min(r, g), min(max(r, g), b));
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Sample all three distance channels from the MSDF atlas.
    let msd = textureSample(sdf_texture, sdf_sampler, in.uv).rgb;

    // Reconstruct the signed distance at this fragment.
    // 0.5 == exactly on the glyph boundary; >0.5 == inside; <0.5 == outside.
    let sd = median(msd.r, msd.g, msd.b);

    // Derivative-based edge width: automatically scales the AA band from
    // sub-pixel sharpness at large sizes to smooth blending when tiny,
    // without needing any extra uniforms (pxRange, atlas size, etc.).
    let fw = fwidth(sd);
    let alpha = smoothstep(0.5 - fw, 0.5 + fw, sd);

    if (alpha < 0.05) {
        discard;
    }

    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
