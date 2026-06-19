#[repr(C)]
#[derive(Default, Debug, Clone, Copy)]
pub struct Vertex {
    pub pos: u32,
    pub scale: u32,
    pub color: [u8; 4],
    pub roundness: f32,
    pub rotation: f32,
    pub stroke_width: f32,
    pub stroke_color: [u8; 4],
    pub tex_coord: [u32; 2], // packed whxy
    pub blur: f32,
    pub stroke_blur: f32,
    pub gradient: [u8; 4],
    pub gradient_dir: f32,
    pub superellipse: f32,
}

#[allow(unused)]
impl Vertex {
    pub fn pos(mut self, x: f32, y: f32) -> Self {
        self.pos =
            (x.clamp(0.0, 1.0) * 65535.0) as u32 | (((y.clamp(0.0, 1.0) * 65535.0) as u32) << 16);
        self
    }

    pub fn scale(mut self, w: f32, h: f32) -> Self {
        self.scale =
            (w.clamp(0.0, 1.0) * 65535.0) as u32 | (((h.clamp(0.0, 1.0) * 65535.0) as u32) << 16);
        self
    }

    pub fn col(mut self, color: [u8; 4]) -> Self {
        self.color = color;
        self
    }

    pub fn rnd(mut self, roundness: f32) -> Self {
        self.roundness = roundness;
        self
    }

    pub fn rot(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    pub fn blur(mut self, blur: f32) -> Self {
        self.blur = blur;
        self
    }

    pub fn stk_col(mut self, stroke_color: [u8; 4]) -> Self {
        self.stroke_color = stroke_color;
        self
    }

    pub fn stk_w(mut self, stroke_width: f32) -> Self {
        self.stroke_width = stroke_width;
        self
    }

    pub fn stk_blur(mut self, stroke_blur: f32) -> Self {
        self.stroke_blur = stroke_blur;
        self
    }

    pub fn grad(mut self, gradient: [u8; 4]) -> Self {
        self.gradient = gradient;
        self
    }

    pub fn grad_dir(mut self, gradient_dir: f32) -> Self {
        self.gradient_dir = gradient_dir;
        self
    }

    pub fn superellipse(mut self, superellipse: f32) -> Self {
        self.superellipse = superellipse;
        self
    }
}