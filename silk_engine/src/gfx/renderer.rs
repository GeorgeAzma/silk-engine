use std::sync::{Arc, Mutex};

use ash::vk;

use crate::WindowResize;

use super::{GraphicsPipelineInfo, RenderCtx, Unit};

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct Vertex {
    pub pos: [f32; 2],
    pub scale: [f32; 2],
    pub color: [u8; 4],
    pub roundness: f32,
    pub rotation: f32,
    pub stroke_width: f32,
    pub stroke_color: [u8; 4],
}
// TODO: tex_idx and textures
#[allow(unused)]
impl Vertex {
    fn pos(mut self, x: f32, y: f32) -> Self {
        self.pos = [x, y];
        self
    }

    fn scale(mut self, w: f32, h: f32) -> Self {
        self.scale = [w, h];
        self
    }

    fn col(mut self, color: [u8; 4]) -> Self {
        self.color = color;
        self
    }

    fn rnd(mut self, roundness: f32) -> Self {
        self.roundness = roundness;
        self
    }

    fn rot(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    fn stk_col(mut self, stroke_color: [u8; 4]) -> Self {
        self.stroke_color = stroke_color;
        self
    }

    fn stk_w(mut self, stroke_width: f32) -> Self {
        self.stroke_width = stroke_width;
        self
    }

    fn with(renderer: &Renderer) -> Self {
        Self {
            pos: Default::default(),
            scale: Default::default(),
            color: renderer.color,
            roundness: renderer.roundness,
            rotation: renderer.rotation,
            stroke_width: renderer.stroke_width,
            stroke_color: renderer.stroke_color,
        }
    }
}

// modify this in batch.wgsl too
pub struct Renderer {
    ctx: Arc<Mutex<RenderCtx>>,
    vertices: Vec<Vertex>,
    vert_cnt: usize,
    instances: Vec<Vertex>,
    inst_cnt: usize,
    pub color: [u8; 4],
    pub roundness: f32,
    pub rotation: f32,
    pub stroke_width: f32,
    pub stroke_color: [u8; 4],
    width: f32,
    height: f32,
}

impl Renderer {
    pub fn new(ctx: Arc<Mutex<RenderCtx>>) -> Self {
        let vertices = vec![Vertex::default(); 1024];
        let instances = vec![Vertex::default(); 1024];
        {
            let mut ctx = ctx.lock().unwrap();
            ctx.add_buffer(
                "batch vbo",
                (vertices.len() * size_of::<Vertex>()) as vk::DeviceSize,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_COHERENT
                    | vk::MemoryPropertyFlags::HOST_CACHED,
            );
            ctx.add_buffer(
                "instance vbo",
                (instances.len() * size_of::<Vertex>()) as vk::DeviceSize,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_COHERENT
                    | vk::MemoryPropertyFlags::HOST_CACHED,
            );
            ctx.add_shader("render");
            let format = ctx.surface_format.format;
            ctx.add_pipeline(
                "render",
                "render",
                GraphicsPipelineInfo::new()
                    .blend_attachment_standard()
                    .dyn_size()
                    .color_attachment(format)
                    .topology(vk::PrimitiveTopology::TRIANGLE_STRIP),
                &[(true, vec![])],
            );
            ctx.add_desc_set("render ds", "render", 0);
            ctx.add_buffer(
                "render ubo",
                2 * size_of::<f32>() as vk::DeviceSize,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE
                    | vk::MemoryPropertyFlags::HOST_COHERENT
                    | vk::MemoryPropertyFlags::HOST_CACHED,
            );
            ctx.write_ds("render ds", "render ubo", 0);
        }
        Self {
            ctx,
            vertices,
            vert_cnt: 0,
            instances,
            inst_cnt: 0,
            color: [255, 255, 255, 255],
            roundness: 0.0,
            rotation: 0.0,
            stroke_width: 0.0,
            stroke_color: [0, 0, 0, 0],
            width: 0.0,
            height: 0.0,
        }
    }

    pub fn rgb(&mut self, r: u8, g: u8, b: u8) {
        self.color = [r, g, b, 255];
    }

    pub fn rgba(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.color = [r, g, b, a];
    }

    pub fn hex(&mut self, hex: u32) {
        self.color = hex.to_be_bytes()
    }

    pub fn stroke_rgb(&mut self, r: u8, g: u8, b: u8) {
        self.color = [r, g, b, 255];
    }

    pub fn stroke_rgba(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.color = [r, g, b, a];
    }

    pub fn stroke_hex(&mut self, hex: u32) {
        self.color = hex.to_be_bytes()
    }

    pub fn verts(&mut self, verts: &[Vertex]) {
        let new_vert_cnt = self.vert_cnt + verts.len();
        if new_vert_cnt >= self.vertices.len() {
            self.vertices
                .resize((new_vert_cnt + 1).next_power_of_two(), Vertex::default());
        }
        self.vertices[self.vert_cnt..new_vert_cnt].copy_from_slice(verts);
        self.vert_cnt = new_vert_cnt;
    }

    pub fn vert(&mut self, vert: Vertex) {
        self.verts(&[vert]);
    }

    fn to_x(&self, unit: Unit) -> f32 {
        match unit {
            Unit::Px(px) => px as f32 / self.width,
            Unit::Mn(mn) => mn * self.height / self.width.max(self.height),
            Unit::Mx(mx) => mx * self.height / self.width.min(self.height),
            Unit::Pc(pc) => pc,
        }
    }

    fn to_y(&self, unit: Unit) -> f32 {
        match unit {
            Unit::Px(px) => px as f32 / self.height,
            Unit::Mn(mn) => mn * self.width / self.width.max(self.height),
            Unit::Mx(mx) => mx * self.width / self.width.min(self.height),
            Unit::Pc(pc) => pc,
        }
    }

    fn inst(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.instances[self.inst_cnt] = Vertex::with(self).pos(x, y).scale(w, h);
        self.inst_cnt += 1;
        if self.inst_cnt >= self.instances.len() {
            self.instances
                .resize((self.inst_cnt + 1).next_power_of_two(), Vertex::default());
        }
    }

    /// centered rect
    pub fn rectc(&mut self, x: Unit, y: Unit, w: Unit, h: Unit) {
        let (x, y, w, h) = (self.to_x(x), self.to_y(y), self.to_x(w), self.to_y(h));
        self.inst(x, y, w, h)
    }

    pub fn rect(&mut self, x: Unit, y: Unit, w: Unit, h: Unit) {
        let (x, y, w, h) = (
            self.to_x(x),
            self.to_y(y),
            self.to_x(w) * 0.5,
            self.to_y(h) * 0.5,
        );
        self.inst(x + w, y + h, w, h)
    }

    /// rounded centered rect
    pub fn rrectc(&mut self, x: Unit, y: Unit, w: Unit, h: Unit, r: f32) {
        let old = self.roundness;
        self.roundness = r;
        self.rectc(x, y, w, h);
        self.roundness = old;
    }

    /// rounded rect
    pub fn rrect(&mut self, x: Unit, y: Unit, w: Unit, h: Unit, r: f32) {
        let old = self.roundness;
        self.roundness = r;
        self.rect(x, y, w, h);
        self.roundness = old;
    }

    pub fn aabb(&mut self, x0: Unit, y0: Unit, x1: Unit, y1: Unit) {
        let (x0, y0, x1, y1) = (self.to_x(x0), self.to_y(y0), self.to_x(x1), self.to_y(y1));
        let (w, h) = ((x1 - x0) * 0.5, (y1 - y0) * 0.5);
        let (x, y) = (x0 - h, y0 - w);
        self.inst(x, y, w, h);
    }

    pub fn circle(&mut self, x: Unit, y: Unit, r: Unit) {
        let old = self.roundness;
        self.roundness = 1.0;
        self.rectc(x, y, r, r);
        self.roundness = old;
    }

    pub fn render(&mut self) {
        if self.vert_cnt != 0 && self.inst_cnt == 0 {
            return;
        }
        let mut ctx = self.ctx.lock().unwrap();
        ctx.bind_pipeline("render");
        ctx.bind_desc_set("render ds");
        if self.vert_cnt != 0 {
            ctx.bind_vbo("batch vbo");
            ctx.draw(self.vert_cnt as u32, 1);
        }
        if self.inst_cnt != 0 {
            ctx.bind_vbo("instance vbo");
            ctx.draw(4, self.inst_cnt as u32);
        }
    }

    pub(crate) fn on_resize(&mut self, e: &WindowResize) {
        if e.width == 0 || e.height == 0 {
            return;
        }
        self.width = e.width as f32;
        self.height = e.height as f32;
        let resolution = [e.width as f32, e.height as f32];
        self.ctx
            .lock()
            .unwrap()
            .write_buffer("render ubo", &resolution);
    }

    pub fn flush(&mut self) {
        if self.vert_cnt != 0 {
            let mut ctx = self.ctx.lock().unwrap();
            let vbo_size = (self.vertices.len() * size_of::<Vertex>()) as vk::DeviceSize;
            if ctx.buffer_size("batch vbo") < vbo_size {
                ctx.recreate_buffer("batch vbo", vbo_size);
            }
            ctx.write_buffer("batch vbo", &self.vertices[..self.vert_cnt]);
        }
        if self.inst_cnt != 0 {
            let mut ctx = self.ctx.lock().unwrap();
            let inst_vbo_size = (self.instances.len() * size_of::<Vertex>()) as vk::DeviceSize;
            if ctx.buffer_size("instance vbo") < inst_vbo_size {
                ctx.recreate_buffer("instance vbo", inst_vbo_size);
            }
            ctx.write_buffer("instance vbo", &self.instances[..self.inst_cnt]);
        }
    }

    pub fn reset(&mut self) {
        self.vert_cnt = 0;
        self.inst_cnt = 0;
        self.color = [255, 255, 255, 255];
        self.stroke_color = [0, 0, 0, 0];
        self.stroke_width = 0.0;
        self.roundness = 0.0;
        self.rotation = 0.0;
    }
}
