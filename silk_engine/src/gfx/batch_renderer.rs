use std::sync::{Arc, Mutex};

use ash::vk::{self, PolygonMode};

use super::{GraphicsPipeline, RenderContext};

#[repr(C)]
#[derive(Default, Clone)]
pub struct BatchVertex {
    pub pos: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
    pub stroke_color: [f32; 4],
    pub stroke_width: f32,
    pub roundness: f32,
}

pub struct BatchRenderer {
    render_ctx: Arc<Mutex<RenderContext>>,
    vertices: Vec<BatchVertex>,
    vert_cnt: usize,
    indices: Vec<u32>,
    idx_cnt: usize,
}

impl BatchRenderer {
    pub fn new(render_ctx: Arc<Mutex<RenderContext>>) -> Self {
        let vertices = vec![BatchVertex::default(); 1024];
        let indices = vec![0; 4096];
        {
            let mut ctx = render_ctx.lock().unwrap();
            ctx.add_buffer(
                "batch vbo",
                (vertices.len() * size_of::<BatchVertex>()) as u64,
                vk::BufferUsageFlags::VERTEX_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );
            ctx.add_buffer(
                "batch ibo",
                (indices.len() * size_of::<u32>()) as u64,
                vk::BufferUsageFlags::INDEX_BUFFER,
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
        }
    }

    pub fn add_vert(&mut self, vert: BatchVertex) {
        self.vertices[self.vert_cnt] = vert;
        self.vert_cnt += 1;
        if self.vert_cnt >= self.vertices.len() {
            self.vertices
                .resize(self.vertices.len() * 2, BatchVertex::default());
        }
    }

    pub fn add_idx(&mut self, idx: u32) {
        self.indices[self.idx_cnt] = idx;
        self.idx_cnt += 1;
        if self.idx_cnt >= self.indices.len() {
            self.indices.resize(self.indices.len() * 2, 0);
        }
    }

    pub fn render_dynamic(&mut self) {
        self.flush();
        self.render();
        self.reset();
    }

    pub fn render(&mut self) {
        if self.idx_cnt == 0 || self.vert_cnt == 0 {
            return;
        }
        let mut ctx = self.render_ctx.lock().unwrap();
        ctx.bind_pipeline("batch");
        ctx.bind_vert("batch vbo");
        ctx.bind_index("batch ibo");
        ctx.draw_indexed(self.idx_cnt as u32, 1);
    }

    fn flush(&mut self) {
        if self.vert_cnt != 0 {
            let mut ctx = self.render_ctx.lock().unwrap();
            let vbo = ctx.buffer("batch vbo");
            ctx.write_buffer(vbo, &self.vertices[..]);
        }
        if self.idx_cnt != 0 {
            let mut ctx = self.render_ctx.lock().unwrap();
            let ibo = ctx.buffer("batch ibo");
            ctx.write_buffer(ibo, &self.indices[..]);
        }
    }

    fn reset(&mut self) {
        self.vert_cnt = 0;
        self.idx_cnt = 0;
    }
}
