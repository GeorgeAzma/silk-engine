use std::sync::{Arc, Mutex};

use ash::vk;

use super::{GraphicsPipeline, RenderContext};

#[repr(C)]
#[derive(Default, Clone, Copy)]
pub struct BatchVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub stroke_color: [f32; 4],
    pub stroke_width: f32,
    pub roundness: f32,
    pub rotation: f32,
    pub center: [f32; 2],
}
// TODO: tex_idx and textures
#[allow(unused)]
impl BatchVertex {
    fn pos(mut self, x: f32, y: f32) -> Self {
        self.pos = [x, y];
        self
    }

    fn uv(mut self, u: f32, v: f32) -> Self {
        self.uv = [u, v];
        self
    }

    fn col(mut self, color: [f32; 4]) -> Self {
        self.color = color;
        self
    }

    fn stk_col(mut self, stroke_color: [f32; 4]) -> Self {
        self.stroke_color = stroke_color;
        self
    }

    fn stk_w(mut self, stroke_width: f32) -> Self {
        self.stroke_width = stroke_width;
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

    fn with(batch_renderer: &BatchRenderer) -> Self {
        Self {
            pos: Default::default(),
            uv: Default::default(),
            color: batch_renderer.color,
            stroke_color: batch_renderer.stroke_color,
            stroke_width: batch_renderer.stroke_width,
            roundness: batch_renderer.roundness,
            rotation: batch_renderer.rotation,
            center: Default::default(),
        }
    }
}

// modify this in batch.wgsl too
pub struct BatchRenderer {
    render_ctx: Arc<Mutex<RenderContext>>,
    vertices: Vec<BatchVertex>,
    vert_cnt: usize,
    indices: Vec<u32>,
    idx_cnt: usize,
    vert_start: u32,
    pub color: [f32; 4],
    pub stroke_color: [f32; 4],
    pub stroke_width: f32,
    pub roundness: f32,
    pub rotation: f32,
    pub center: [f32; 2],
}

impl BatchRenderer {
    pub fn new(render_ctx: Arc<Mutex<RenderContext>>) -> Self {
        let vertices = vec![BatchVertex::default(); 1024];
        let indices = vec![0; 4096];
        {
            let mut ctx = render_ctx.lock().unwrap();
            ctx.add_buffer(
                "batch vao",
                (vertices.len() * size_of::<BatchVertex>()) as u64,
                vk::BufferUsageFlags::VERTEX_BUFFER
                    | vk::BufferUsageFlags::INDEX_BUFFER
                    | vk::BufferUsageFlags::TRANSFER_DST,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );
            ctx.add_shader("batch");
            let format = ctx.surface_format.format;
            ctx.add_pipeline(
                "batch",
                "batch",
                GraphicsPipeline::new()
                    .blend_attachment_empty()
                    .dyn_size()
                    .color_attachment(format),
                &[],
            );
        }
        Self {
            render_ctx,
            vertices,
            vert_cnt: 0,
            indices,
            idx_cnt: 0,
            vert_start: 0,
            color: [1.0, 1.0, 1.0, 1.0],
            stroke_color: [0.0, 0.0, 0.0, 0.0],
            stroke_width: 0.0,
            roundness: 0.0,
            rotation: 0.0,
            center: [0.0, 0.0],
        }
    }

    pub fn begin(&mut self) {
        self.vert_start = self.vert_cnt as u32;
    }

    pub fn calc_center(&mut self) {
        self.center = [0.0, 0.0];
        let verts = &self.vertices[self.vert_start as usize..self.vert_cnt];
        for v in verts.iter() {
            self.center[0] += v.pos[0];
            self.center[1] += v.pos[1];
        }
        self.center[0] /= verts.len() as f32;
        self.center[1] /= verts.len() as f32;
    }

    pub fn verts(&mut self, verts: &[BatchVertex]) {
        let new_vert_cnt = self.vert_cnt + verts.len();
        if new_vert_cnt >= self.vertices.len() {
            self.vertices.resize(
                (new_vert_cnt + 1).next_power_of_two(),
                BatchVertex::default(),
            );
        }
        self.vertices[self.vert_cnt..][..verts.len()].copy_from_slice(verts);
        self.vert_cnt = new_vert_cnt;
    }

    pub fn vert(&mut self, vert: BatchVertex) {
        self.verts(&[vert]);
    }

    pub fn idxs(&mut self, idxs: &[u32]) {
        let mut idxs = idxs.to_owned();
        for i in idxs.iter_mut() {
            *i += self.vert_start;
        }
        let new_idx_cnt = self.idx_cnt + idxs.len();
        if new_idx_cnt >= self.indices.len() {
            self.indices
                .resize((new_idx_cnt + 1).next_power_of_two(), 0);
        }
        self.indices[self.idx_cnt..][..idxs.len()].copy_from_slice(&idxs);
        self.idx_cnt = new_idx_cnt;
    }

    pub fn idx(&mut self, idx: u32) {
        self.idxs(&[idx]);
    }

    pub fn aabb(&mut self, x0: f32, y0: f32, x1: f32, y1: f32) {
        self.begin();
        self.verts(&[
            BatchVertex::with(self).pos(x0, y0),
            BatchVertex::with(self).pos(x1, y0),
            // BatchVertex::with(self).pos(x1, y1),
            BatchVertex::with(self).pos(x1, y1),
            // BatchVertex::with(self).pos(x0, y0),
            BatchVertex::with(self).pos(x0, y1),
        ]);
        if self.rotation.abs() > 0.0 {
            self.calc_center();
        }
        self.idxs(&[0, 1, 2, 2, 0, 3]);
    }

    pub fn rect(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.aabb(x, y, x + w, y + h);
    }

    pub fn rect_center(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.aabb(x - w, y - h, x + w, y + h);
    }

    pub fn circle(&mut self, x: f32, y: f32, r: f32) {
        let old = self.roundness;
        self.roundness = 1.0;
        self.rect_center(x, y, r, r);
        self.roundness = old;
    }

    pub fn render(&mut self) {
        if self.vert_cnt == 0 {
            return;
        }
        let mut ctx = self.render_ctx.lock().unwrap();
        ctx.bind_pipeline("batch");
        if self.idx_cnt != 0 {
            ctx.bind_vao(
                "batch vao",
                (self.vert_cnt * size_of::<BatchVertex>()) as vk::DeviceSize,
            );
            ctx.draw_indexed(self.idx_cnt as u32, 1);
        } else {
            ctx.bind_vbo("batch vao");
            ctx.draw(self.vert_cnt as u32, 1);
        }
    }

    pub fn flush(&mut self) {
        if self.vert_cnt == 0 {
            return;
        }
        let mut ctx = self.render_ctx.lock().unwrap();
        let idx_size = (self.indices.len() * size_of::<u32>()) as vk::DeviceSize;
        let vert_size = (self.vertices.len() * size_of::<BatchVertex>()) as vk::DeviceSize;
        let vao_size = vert_size + if self.idx_cnt == 0 { 0 } else { idx_size };
        if ctx.buffer_size("batch vao") < vao_size {
            ctx.recreate_buffer("batch vao", vao_size);
        }
        // TODO: join as single write
        ctx.write_buffer("batch vao", &self.vertices[..self.vert_cnt]);
        if self.idx_cnt != 0 {
            ctx.write_buffer_off(
                "batch vao",
                &self.indices[..self.idx_cnt],
                (self.vert_cnt * size_of::<BatchVertex>()) as vk::DeviceSize,
            );
        }
    }

    pub fn reset(&mut self) {
        self.vert_cnt = 0;
        self.idx_cnt = 0;
        self.vert_start = 0;
        self.color = [1.0, 1.0, 1.0, 1.0];
        self.stroke_color = [0.0, 0.0, 0.0, 0.0];
        self.stroke_width = 0.0;
        self.roundness = 0.0;
        self.rotation = 0.0;
        self.center = [0.0, 0.0];
    }
}
