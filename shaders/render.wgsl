@group(0) @binding(0)
var my_sampler: sampler;

@group(0) @binding(1)
var background_texture: texture_2d<f32>;

@group(0) @binding(2)
var blurred_texture: texture_2d<f32>;

@group(0) @binding(3)
var icon_texture: texture_2d<f32>;

@group(0) @binding(4)
var cover_texture: texture_2d<f32>;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) screen_uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) size: vec2<f32>,
    @location(5) radii: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) screen_uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) size: vec2<f32>,
    @location(4) radii: vec4<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.tex_coord = input.tex_coord;
    output.screen_uv = input.screen_uv;
    output.color = input.color;
    output.size = input.size;
    output.radii = input.radii;
    return output;
}

fn get_sd_round_box(p: vec2<f32>, b: vec2<f32>, r: vec4<f32>) -> f32 {
    var radius: f32;
    if (p.x < 0.0) {
        if (p.y < 0.0) {
            radius = r.x; // Top-Left
        } else {
            radius = r.z; // Bottom-Left
        }
    } else {
        if (p.y < 0.0) {
            radius = r.y; // Top-Right
        } else {
            radius = r.w; // Bottom-Right
        }
    }
    
    let q = abs(p) - b + vec2<f32>(radius, radius);
    return min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0, 0.0))) - radius;
}

fn median(r: f32, g: f32, b: f32) -> f32 {
    return max(min(r, g), min(max(r, g), b));
}

fn calculate_glass_displacement(p: vec2<f32>, half_size: vec2<f32>, d: f32, strength: f32) -> vec2<f32> {
    let radial_dir = p / max(half_size.x, half_size.y);
    let edge_influence = smoothstep(-40.0, 5.0, d);
    let center_bulge = (1.0 - smoothstep(0.0, 1.0, length(p / half_size))) * 0.2;
    return radial_dir * (edge_influence + center_bulge) * 0.02 * strength;
}

fn apply_chromatic_aberration(uv: vec2<f32>, norm_p: vec2<f32>, edge: f32, strength: f32) -> vec3<f32> {
    let offset = norm_p * edge * 0.008 * strength;
    let r = textureSample(blurred_texture, my_sampler, clamp(uv + offset, vec2<f32>(0.0), vec2<f32>(1.0))).r;
    let g = textureSample(blurred_texture, my_sampler, uv).g;
    let b = textureSample(blurred_texture, my_sampler, clamp(uv - offset, vec2<f32>(0.0), vec2<f32>(1.0))).b;
    return vec3<f32>(r, g, b);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    // --- BACKGROUND PASS ---
    if (input.color.a == -1.0) {
        return textureSample(background_texture, my_sampler, input.tex_coord);
    }

    // --- PHOTO / COVER PASS (With Rounding) ---
    if (input.screen_uv.x == -2.0) {
        let p = (input.tex_coord - 0.5) * input.size;
        let d = get_sd_round_box(p, input.size * 0.5, input.radii);
        let mask = 1.0 - smoothstep(-1.0, 0.5, d);
        if (mask <= 0.0) { discard; }
        
        let tex_color = textureSample(cover_texture, my_sampler, input.tex_coord);
        return vec4<f32>(tex_color.rgb * input.color.rgb, tex_color.a * input.color.a * mask);
    }

    // --- SHADOW PASS ---
    if (input.color.a < 0.0 && input.color.a > -1.0) {
        let p = (input.tex_coord - 0.5) * input.size;
        let d = get_sd_round_box(p, input.size * 0.5 - 8.0, vec4<f32>(min(input.size.x, input.size.y) * 0.1));
        return vec4<f32>(0.0, 0.0, 0.0, abs(input.color.a) * (1.0 - smoothstep(-2.0, 15.0, d)));
    }

    // --- UI / GLASS PASS ---
    if (input.color.a < -1.0) {
        let strength = abs(input.color.a) - 1.0;
        let half_size = input.size * 0.5;
        let p = (input.tex_coord - 0.5) * input.size;
        let d = get_sd_round_box(p, half_size, input.radii);

        let mask = 1.0 - smoothstep(-1.0, 0.5, d);
        if (mask <= 0.0) { discard; }

        let displacement = calculate_glass_displacement(p, half_size, d, strength);
        let sample_uv = clamp(input.screen_uv + displacement, vec2<f32>(0.0), vec2<f32>(1.0));

        let norm_p = p / half_size;
        let edge = smoothstep(-25.0, 0.0, d);

        let base_color = apply_chromatic_aberration(sample_uv, norm_p, edge, strength);

        let tint_strength = max(input.color.r, max(input.color.g, input.color.b));
        let border = smoothstep(-3.0, 0.0, d);
        let highlight = (1.0 - smoothstep(0.0, 1.5, length(input.tex_coord - 0.2))) * 0.08 * tint_strength;
        let rim = border * 0.12 * tint_strength;
        let tint = input.color.rgb * 0.03;

        let glass = base_color + vec3<f32>(highlight + rim) + tint;

        return vec4<f32>(glass, mask);
    }

    // --- ICONS (MSDF) ---
    if (input.screen_uv.x == -1.0) {
        let msd = textureSample(icon_texture, my_sampler, input.tex_coord).rgb;
        let sd = median(msd.r, msd.g, msd.b);
        let fw = fwidth(sd) * 0.75;
        let alpha = smoothstep(0.5 - fw, 0.5 + fw, sd) * input.color.a;
        return vec4<f32>(input.color.rgb, alpha);
    }

    return vec4<f32>(input.color.rgb, 1.0);
}
