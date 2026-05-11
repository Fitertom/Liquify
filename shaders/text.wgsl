@group(0) @binding(0)
var my_sampler: sampler;

@group(0) @binding(1)
var font_texture: texture_2d<f32>;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) screen_uv: vec2<f32>,
    @location(3) color: vec4<f32>,
    @location(4) quad_size: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) screen_pos: vec2<f32>,
};

@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.position = vec4<f32>(input.position, 0.0, 1.0);
    output.tex_coord = input.tex_coord;
    output.color = input.color;
    output.screen_pos = input.position;
    return output;
}

fn get_alpha(dist: f32, fw: f32, edge: f32) -> f32 {
    return smoothstep(edge - fw, edge + fw, dist);
}

@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    let uv = input.tex_coord;
    
    // --- ВЫСОКОКАЧЕСТВЕННОЕ СГЛАЖИВАНИЕ (SDF) ---
    // Читаем расстояние. 0.5 - граница.
    let dist = textureSample(font_texture, my_sampler, uv).r;
    
    // fwidth(dist) даёт скорость изменения расстояния на пиксель экрана.
    // Мы используем его для автоматического подбора мягкости края (AA).
    // Умножаем на 0.7-1.0 для контроля резкости. 
    // Меньше 1.0 = резче, Больше 1.0 = мягче.
    let fw = fwidth(dist) * 0.85; 
    
    // Основная маска текста
    let threshold = 0.5;
    var alpha = get_alpha(dist, fw, threshold);

    // --- ЭФФЕКТЫ ---
    // 1. Outline (Контур)
    // Чтобы включить контур, можно было бы передавать параметры в quad_size,
    // но сделаем универсальную базу.
    let outline_width = 0.0; // Значение > 0 создаст контур внутри/снаружи
    if (outline_width > 0.0) {
        let outline_alpha = get_alpha(dist, fw, threshold - outline_width);
        alpha = outline_alpha - alpha; // Оставляем только разницу
    }

    // 2. Тень / Свечение (SDF позволяет делать это дешево)
    // Для реальной тени нужно второе чтение со смещением, но мы можем сделать "свечение"
    // просто расширив область smoothstep.
    let glow_width = 0.08;
    let glow = smoothstep(threshold - glow_width, threshold, dist) * 0.3;
    
    // Смешиваем основной текст и свечение
    let final_alpha = max(alpha, glow) * input.color.a;

    // --- ГАММА-КОРРЕКЦИЯ ---
    // Текст часто выглядит слишком тонким или "грязным" из-за линейного смешивания.
    // Небольшая коррекция делает его более "браузерным" и плотным.
    let corrected_alpha = pow(final_alpha, 1.1);

    if (corrected_alpha < 0.01) {
        discard;
    }

    // Применяем цвет
    return vec4<f32>(input.color.rgb, corrected_alpha);
}
