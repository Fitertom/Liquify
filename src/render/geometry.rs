use std::sync::LazyLock;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub tex_coord: [f32; 2],
    pub screen_uv: [f32; 2],
    pub color: [f32; 4],
    pub quad_size: [f32; 3],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use wgpu::VertexFormat;
        static ATTRIBUTES: LazyLock<[wgpu::VertexAttribute; 5]> = LazyLock::new(|| {
            [
                wgpu::VertexAttribute { offset: 0, format: VertexFormat::Float32x2, shader_location: 0 },
                wgpu::VertexAttribute { offset: 8, format: VertexFormat::Float32x2, shader_location: 1 },
                wgpu::VertexAttribute { offset: 16, format: VertexFormat::Float32x2, shader_location: 2 },
                wgpu::VertexAttribute { offset: 24, format: VertexFormat::Float32x4, shader_location: 3 },
                wgpu::VertexAttribute { offset: 40, format: VertexFormat::Float32x3, shader_location: 4 },
            ]
        });
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as _,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &*ATTRIBUTES,
        }
    }
}

  pub fn build_quad(x: f32, y: f32, w: f32, h: f32, color: [f32; 4]) -> [Vertex; 6] {
    let (xmin, ymin) = (x, y);
    let (xmax, ymax) = (x + w, y + h);
    let c = [color[0], color[1], color[2], 0.0];
    let qs = [w, h, 0.0];
    [
        Vertex { position: [xmin, ymin], tex_coord: [0.0, 0.0], screen_uv: [0.0, 0.0], color: c, quad_size: qs },
        Vertex { position: [xmax, ymin], tex_coord: [1.0, 0.0], screen_uv: [0.0, 0.0], color: c, quad_size: qs },
        Vertex { position: [xmax, ymax], tex_coord: [1.0, 1.0], screen_uv: [0.0, 0.0], color: c, quad_size: qs },
        Vertex { position: [xmin, ymin], tex_coord: [0.0, 0.0], screen_uv: [0.0, 0.0], color: c, quad_size: qs },
        Vertex { position: [xmax, ymax], tex_coord: [1.0, 1.0], screen_uv: [0.0, 0.0], color: c, quad_size: qs },
        Vertex { position: [xmin, ymax], tex_coord: [0.0, 1.0], screen_uv: [0.0, 0.0], color: c, quad_size: qs },
    ]
}

pub fn build_background_quad(w: f32, h: f32) -> [Vertex; 6] {
    let qs = [w, h, 0.0];
    [
        Vertex { position: [0.0, 0.0], tex_coord: [0.0, 0.0], screen_uv: [0.0, 0.0], color: [1.0, 1.0, 1.0, -1.0], quad_size: qs },
        Vertex { position: [w, 0.0], tex_coord: [1.0, 0.0], screen_uv: [1.0, 0.0], color: [1.0, 1.0, 1.0, -1.0], quad_size: qs },
        Vertex { position: [w, h], tex_coord: [1.0, 1.0], screen_uv: [1.0, 1.0], color: [1.0, 1.0, 1.0, -1.0], quad_size: qs },
        Vertex { position: [0.0, 0.0], tex_coord: [0.0, 0.0], screen_uv: [0.0, 0.0], color: [1.0, 1.0, 1.0, -1.0], quad_size: qs },
        Vertex { position: [w, h], tex_coord: [1.0, 1.0], screen_uv: [1.0, 1.0], color: [1.0, 1.0, 1.0, -1.0], quad_size: qs },
        Vertex { position: [0.0, h], tex_coord: [0.0, 1.0], screen_uv: [0.0, 1.0], color: [1.0, 1.0, 1.0, -1.0], quad_size: qs },
    ]
}

pub fn build_glass_quad(x: f32, y: f32, w: f32, h: f32, win_w: f32, win_h: f32, tint: [f32; 3], strength: f32, radius: f32) -> [Vertex; 6] {
    let (xmin, ymin) = (x, y);
    let (xmax, ymax) = (x + w, y + h);
    let uv0 = [xmin / win_w, ymin / win_h];
    let uv1 = [xmax / win_w, ymax / win_h];
    let qs = [w, h, radius];
    [
        Vertex { position: [xmin, ymin], tex_coord: [0.0, 0.0], screen_uv: [uv0[0], uv0[1]], color: [tint[0], tint[1], tint[2], -(1.0 + strength)], quad_size: qs },
        Vertex { position: [xmax, ymin], tex_coord: [1.0, 0.0], screen_uv: [uv1[0], uv0[1]], color: [tint[0], tint[1], tint[2], -(1.0 + strength)], quad_size: qs },
        Vertex { position: [xmax, ymax], tex_coord: [1.0, 1.0], screen_uv: [uv1[0], uv1[1]], color: [tint[0], tint[1], tint[2], -(1.0 + strength)], quad_size: qs },
        Vertex { position: [xmin, ymin], tex_coord: [0.0, 0.0], screen_uv: [uv0[0], uv0[1]], color: [tint[0], tint[1], tint[2], -(1.0 + strength)], quad_size: qs },
        Vertex { position: [xmax, ymax], tex_coord: [1.0, 1.0], screen_uv: [uv1[0], uv1[1]], color: [tint[0], tint[1], tint[2], -(1.0 + strength)], quad_size: qs },
        Vertex { position: [xmin, ymax], tex_coord: [0.0, 1.0], screen_uv: [uv0[0], uv1[1]], color: [tint[0], tint[1], tint[2], -(1.0 + strength)], quad_size: qs },
    ]
}

pub fn build_shadow_quad(x: f32, y: f32, w: f32, h: f32, spread: f32, opacity: f32) -> [Vertex; 6] {
    let (xmin, ymin) = (x - spread, y - spread);
    let (xmax, ymax) = (x + w + spread, y + h + spread);
    let c = [0.0, 0.0, 0.0, -opacity.clamp(0.0, 0.99)];
    let qs = [w + spread * 2.0, h + spread * 2.0, 0.0];
    [
        Vertex { position: [xmin, ymin], tex_coord: [0.0, 0.0], screen_uv: [0.0, 0.0], color: c, quad_size: qs },
        Vertex { position: [xmax, ymin], tex_coord: [1.0, 0.0], screen_uv: [0.0, 0.0], color: c, quad_size: qs },
        Vertex { position: [xmax, ymax], tex_coord: [1.0, 1.0], screen_uv: [0.0, 0.0], color: c, quad_size: qs },
        Vertex { position: [xmin, ymin], tex_coord: [0.0, 0.0], screen_uv: [0.0, 0.0], color: c, quad_size: qs },
        Vertex { position: [xmax, ymax], tex_coord: [1.0, 1.0], screen_uv: [0.0, 0.0], color: c, quad_size: qs },
        Vertex { position: [xmin, ymax], tex_coord: [0.0, 1.0], screen_uv: [0.0, 0.0], color: c, quad_size: qs },
    ]
}
