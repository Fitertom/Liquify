struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) screen_uv: vec2<f32>, // x = icon_id, y = unused
    @location(3) color: vec4<f32>,
    @location(4) quad_size: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) icon_id: f32,
    @location(2) color: vec4<f32>,
};

@vertex
fn vs_main(model: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = vec4<f32>(model.position, 0.0, 1.0);
    out.uv = model.tex_coord; // 0..1 inside the icon quad
    out.icon_id = model.screen_uv.x;
    out.color = model.color;
    return out;
}

// --- SDF Primitives ---

fn sd_line(p: vec2<f32>, a: vec2<f32>, b: vec2<f32>) -> f32 {
    let pa = p - a;
    let ba = b - a;
    let h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return length(pa - ba * h);
}

fn sd_circle(p: vec2<f32>, r: f32) -> f32 {
    return length(p) - r;
}

fn sd_box(p: vec2<f32>, b: vec2<f32>, r: f32) -> f32 {
    let q = abs(p) - b + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

// --- Icons Definitions ---

fn draw_home(p: vec2<f32>) -> f32 {
    let t = 0.035;
    let d1 = sd_line(p, vec2<f32>(-0.375, -0.125), vec2<f32>(0.0, -0.416));
    let d2 = sd_line(p, vec2<f32>(0.0, -0.416), vec2<f32>(0.375, -0.125));
    let d3 = sd_line(p, vec2<f32>(0.375, -0.125), vec2<f32>(0.375, 0.416));
    let d4 = sd_line(p, vec2<f32>(0.375, 0.416), vec2<f32>(-0.375, 0.416));
    let d5 = sd_line(p, vec2<f32>(-0.375, 0.416), vec2<f32>(-0.375, -0.125));
    let d6 = sd_line(p, vec2<f32>(-0.125, 0.416), vec2<f32>(-0.125, 0.0));
    let d7 = sd_line(p, vec2<f32>(-0.125, 0.0), vec2<f32>(0.125, 0.0));
    let d8 = sd_line(p, vec2<f32>(0.125, 0.0), vec2<f32>(0.125, 0.416));
    return min(min(min(min(d1, d2), min(d3, d4)), d5), min(min(d6, d7), d8)) - t;
}

fn draw_search(p: vec2<f32>) -> f32 {
    let t = 0.035;
    let circle_dist = abs(sd_circle(p - vec2<f32>(-0.041, -0.041), 0.28)) - t;
    let search_handle = sd_line(p, vec2<f32>(0.375, 0.375), vec2<f32>(0.193, 0.193)) - t;
    return min(circle_dist, search_handle);
}

fn draw_library(p: vec2<f32>) -> f32 {
    let t = 0.035;
    let l1 = sd_line(p, vec2<f32>(-0.15, -0.25), vec2<f32>(0.38, -0.25));
    let l2 = sd_line(p, vec2<f32>(-0.15, 0.0), vec2<f32>(0.38, 0.0));
    let l3 = sd_line(p, vec2<f32>(-0.15, 0.25), vec2<f32>(0.38, 0.25));
    let d1 = sd_circle(p - vec2<f32>(-0.35, -0.25), 0.01);
    let d2 = sd_circle(p - vec2<f32>(-0.35, 0.0), 0.01);
    let d3 = sd_circle(p - vec2<f32>(-0.35, 0.25), 0.01);
    let lines = min(min(l1, l2), l3) - t;
    let dots = min(min(d1, d2), d3);
    return min(lines, dots);
}

fn draw_settings(p: vec2<f32>) -> f32 {
    let t = 0.035;
    let ring = abs(sd_circle(p, 0.12)) - t;
    let outer = abs(sd_circle(p, 0.33)) - t;
    return min(ring, outer);
}

fn draw_heart(p: vec2<f32>) -> f32 {
    let t = 0.035;
    let x = p.x * 1.5;
    let y = -p.y * 1.5 - 0.2;
    let a = x*x + y*y - 0.3;
    let d = a*a*a - x*x*y*y*y;
    return abs(d * 0.5) - t;
}

fn draw_play(p: vec2<f32>) -> f32 {
    let t = 0.035;
    let d1 = sd_line(p, vec2<f32>(-0.25, -0.35), vec2<f32>(0.35, 0.0));
    let d2 = sd_line(p, vec2<f32>(0.35, 0.0), vec2<f32>(-0.25, 0.35));
    let d3 = sd_line(p, vec2<f32>(-0.25, 0.35), vec2<f32>(-0.25, -0.35));
    return min(min(d1, d2), d3) - t;
}

fn draw_pause(p: vec2<f32>) -> f32 {
    let t = 0.035;
    let l1 = sd_line(p - vec2<f32>(-0.15, 0.0), vec2<f32>(0.0, -0.35), vec2<f32>(0.0, 0.35)) - 0.05;
    let l2 = sd_line(p - vec2<f32>(0.15, 0.0), vec2<f32>(0.0, -0.35), vec2<f32>(0.0, 0.35)) - 0.05;
    return min(l1, l2);
}

fn draw_prev(p: vec2<f32>) -> f32 {
    let t = 0.035;
    let d1 = sd_line(p, vec2<f32>(0.291, 0.333), vec2<f32>(-0.125, 0.0));
    let d2 = sd_line(p, vec2<f32>(-0.125, 0.0), vec2<f32>(0.291, -0.333));
    let d3 = sd_line(p, vec2<f32>(0.291, -0.333), vec2<f32>(0.291, 0.333));
    let line = sd_line(p, vec2<f32>(-0.291, -0.291), vec2<f32>(-0.291, 0.291));
    return min(min(min(d1, d2), d3), line) - t;
}

fn draw_next(p: vec2<f32>) -> f32 {
    let t = 0.035;
    let d1 = sd_line(p, vec2<f32>(-0.291, -0.333), vec2<f32>(0.125, 0.0));
    let d2 = sd_line(p, vec2<f32>(0.125, 0.0), vec2<f32>(-0.291, 0.333));
    let d3 = sd_line(p, vec2<f32>(-0.291, 0.333), vec2<f32>(-0.291, -0.333));
    let line = sd_line(p, vec2<f32>(0.291, -0.291), vec2<f32>(0.291, 0.291));
    return min(min(min(d1, d2), d3), line) - t;
}

fn draw_plus(p: vec2<f32>) -> f32 {
    let t = 0.035;
    let v = sd_line(p, vec2<f32>(0.0, -0.3), vec2<f32>(0.0, 0.3));
    let h = sd_line(p, vec2<f32>(-0.3, 0.0), vec2<f32>(0.3, 0.0));
    return min(v, h) - t;
}

fn draw_check(p: vec2<f32>) -> f32 {
    let t = 0.035;
    let d1 = sd_line(p, vec2<f32>(-0.333, 0.0), vec2<f32>(-0.125, 0.208));
    let d2 = sd_line(p, vec2<f32>(-0.125, 0.208), vec2<f32>(0.333, -0.25));
    return min(d1, d2) - t;
}

fn draw_more(p: vec2<f32>) -> f32 {
    let d1 = sd_circle(p - vec2<f32>(-0.25, 0.0), 0.04);
    let d2 = sd_circle(p, 0.04);
    let d3 = sd_circle(p - vec2<f32>(0.25, 0.0), 0.04);
    return min(min(d1, d2), d3);
}

fn draw_arrow_left(p: vec2<f32>) -> f32 {
    let t = 0.035;
    let line = sd_line(p, vec2<f32>(0.3, 0.0), vec2<f32>(-0.3, 0.0));
    let d1 = sd_line(p, vec2<f32>(0.0, -0.3), vec2<f32>(-0.3, 0.0));
    let d2 = sd_line(p, vec2<f32>(0.0, 0.3), vec2<f32>(-0.3, 0.0));
    return min(min(line, d1), d2) - t;
}

fn draw_music(p: vec2<f32>) -> f32 {
    let t = 0.035;
    let stem = sd_line(p, vec2<f32>(-0.125, 0.25), vec2<f32>(-0.125, -0.3));
    let top = sd_line(p, vec2<f32>(-0.125, -0.3), vec2<f32>(0.3, -0.4));
    let stem2 = sd_line(p, vec2<f32>(0.3, -0.4), vec2<f32>(0.3, 0.15));
    let note1 = sd_circle(p - vec2<f32>(-0.25, 0.25), 0.12);
    let note2 = sd_circle(p - vec2<f32>(0.18, 0.15), 0.12);
    return min(min(min(stem, top), stem2) - t, min(note1, note2));
}

fn draw_image(p: vec2<f32>) -> f32 {
    let t = 0.035;
    let b = abs(sd_box(p, vec2<f32>(0.375, 0.375), 0.05)) - t;
    let sun = sd_circle(p - vec2<f32>(-0.15, -0.15), 0.06);
    let mountain = sd_line(p, vec2<f32>(-0.3, 0.375), vec2<f32>(0.1, -0.1));
    let mountain2 = sd_line(p, vec2<f32>(0.1, -0.1), vec2<f32>(0.375, 0.15));
    return min(min(b, sun), min(mountain, mountain2) - t);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let p = in.uv - 0.5;
    var d: f32 = 1000.0;

    let id = u32(in.icon_id + 0.5);
    if (id == 0u) { d = draw_home(p); }
    else if (id == 1u) { d = draw_search(p); }
    else if (id == 2u) { d = draw_library(p); }
    else if (id == 3u) { d = draw_settings(p); }
    else if (id == 4u) { d = draw_heart(p); }
    else if (id == 5u) { d = draw_play(p); }
    else if (id == 6u) { d = draw_pause(p); }
    else if (id == 7u) { d = draw_prev(p); }
    else if (id == 8u) { d = draw_next(p); }
    else if (id == 9u) { d = draw_music(p); }
    else if (id == 10u) { d = draw_image(p); }
    else if (id == 11u) { d = draw_plus(p); }
    else if (id == 12u) { d = draw_check(p); }
    else if (id == 14u) { d = draw_arrow_left(p); }
    else if (id == 15u) { d = draw_more(p); }
    else { d = sd_circle(p, 0.25) - 0.035; }

    let fw = fwidth(d);
    let alpha = 1.0 - smoothstep(-fw, fw, d);

    if (alpha < 0.01) { discard; }
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
