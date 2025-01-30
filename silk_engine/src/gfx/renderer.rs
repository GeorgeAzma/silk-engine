// TODO: make roundness Unit
// TODO: make stroke_width Unit
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use ash::vk;

use crate::{
    event::WindowResize,
    util::{Funcs, ImageLoader, Tracked},
};

use super::{
    BufUsage, GraphicsPipelineInfo, ImageInfo, ImgLayout, ImgUsage, MSAA, MemProp, RenderCtx, Unit,
    packer::{Guillotine, Packer, Rect},
    render_ctx::BufferImageCopy,
};

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
    tex_coord: [u32; 2], // packed whxy
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
            tex_coord: renderer.tex_coord,
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
    tex_coord: [u32; 2], // packed whxy
    areas: Vec<[f32; 4]>,
    old_color: [u8; 4],
    old_roundness: f32,
    old_rotation: f32,
    old_stroke_width: f32,
    old_stroke_color: [u8; 4],
    old_tex_coord: [u32; 2],
    width: f32,
    height: f32,
    packer: Guillotine,
    imgs: HashMap<String, (Tracked<Vec<u8>>, Rect)>,
}

impl Renderer {
    pub fn new(ctx: Arc<Mutex<RenderCtx>>) -> Self {
        let vertices = vec![Vertex::default(); 1024];
        let instances = vec![Vertex::default(); 1024];

        // TODO: resizable packer
        let packer = Guillotine::new(1024, 1024);
        {
            let mut ctx = ctx.lock().unwrap();
            ctx.add_buf(
                "batch vbo",
                (vertices.len() * size_of::<Vertex>()) as vk::DeviceSize,
                BufUsage::VERT,
                MemProp::CPU_CACHED,
            );
            ctx.add_buf(
                "instance vbo",
                (instances.len() * size_of::<Vertex>()) as vk::DeviceSize,
                BufUsage::VERT,
                MemProp::CPU_CACHED,
            );
            ctx.add_shader("render");
            let format = ctx.surface_format.format;
            ctx.add_pipeline(
                "render",
                "render",
                GraphicsPipelineInfo::new()
                    .blend_attachment_standard()
                    .dyn_size()
                    .samples(MSAA)
                    .color_attachment(format)
                    .topology(vk::PrimitiveTopology::TRIANGLE_STRIP),
                &[(true, vec![])],
            );
            ctx.add_desc_set("render ds", "render", 0);
            ctx.add_buf(
                "render ubo",
                2 * size_of::<f32>() as vk::DeviceSize,
                BufUsage::UNIFORM,
                MemProp::CPU_CACHED,
            );
            ctx.write_ds_buf("render ds", "render ubo", 0);
            ctx.add_img(
                "atlas",
                &ImageInfo::new()
                    .width(packer.width() as u32)
                    .height(packer.height() as u32)
                    .format(vk::Format::R8G8B8A8_UNORM)
                    .usage(ImgUsage::DST | ImgUsage::SAMPLED),
                MemProp::GPU,
            );
            ctx.add_img_view("atlas view", "atlas");

            ctx.write_ds_img("render ds", "atlas view", ImgLayout::SHADER_READ, 1);
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
            tex_coord: [0, 0],
            old_color: [255, 255, 255, 255],
            old_roundness: 0.0,
            old_rotation: 0.0,
            old_stroke_width: 0.0,
            old_stroke_color: [0, 0, 0, 0],
            old_tex_coord: [0, 0],
            areas: Vec::new(),
            width: 0.0,
            height: 0.0,
            packer,
            imgs: HashMap::new(),
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

    pub fn add_img(&mut self, name: &str, width: u32, height: u32) -> &mut Tracked<Vec<u8>> {
        assert!(!self.imgs.contains_key(name), "img already in atlas");
        if let Some((x, y)) = self.packer.pack(width as u16, height as u16) {
            let tracked_img_data = &mut self
                .imgs
                .entry(name.to_string())
                .or_insert((
                    Tracked::new(vec![0; width as usize * height as usize * 4]),
                    Rect::new(x, y, width as u16, height as u16),
                ))
                .0;
            tracked_img_data
        } else {
            panic!("failed to add img to atlas, out of space")
        }
    }

    pub fn load_img(&mut self, name: &str) -> &mut Tracked<Vec<u8>> {
        let mut img_data = ImageLoader::load(name);
        if img_data.channels != 4 {
            img_data.img = ImageLoader::make4(&mut img_data.img);
        }
        let tracked_img_data = self.add_img(name, img_data.width, img_data.height);
        tracked_img_data.copy_from_slice(&img_data.img);
        tracked_img_data
    }

    pub fn img(&mut self, name: &str) -> &mut Tracked<Vec<u8>> {
        let img_data = self
            .imgs
            .get_mut(name)
            .unwrap_or_else(|| panic!("img not found in atlas: {name}"));
        let r = img_data.1.packed_whxy();
        self.tex_coord = [(r >> 32) as u32, r as u32];
        &mut img_data.0
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

    fn pc_x(&self, unit: Unit) -> f32 {
        match unit {
            Unit::Px(px) => px as f32 / self.width,
            Unit::Mn(mn) => mn * self.width.min(self.height) / self.width,
            Unit::Mx(mx) => mx * self.width.max(self.height) / self.width,
            Unit::Pc(pc) => pc,
        }
    }

    fn pc_y(&self, unit: Unit) -> f32 {
        match unit {
            Unit::Px(px) => px as f32 / self.height,
            Unit::Mn(mn) => mn * self.width.min(self.height) / self.height,
            Unit::Mx(mx) => mx * self.width.max(self.height) / self.height,
            Unit::Pc(pc) => pc,
        }
    }

    fn px_x(&self, unit: Unit) -> f32 {
        match unit {
            Unit::Px(px) => px as f32,
            Unit::Mn(mn) => mn * self.width.min(self.height),
            Unit::Mx(mx) => mx * self.width.max(self.height),
            Unit::Pc(pc) => pc * self.width,
        }
    }

    fn px_y(&self, unit: Unit) -> f32 {
        match unit {
            Unit::Px(px) => px as f32,
            Unit::Mn(mn) => mn * self.width.min(self.height),
            Unit::Mx(mx) => mx * self.width.max(self.height),
            Unit::Pc(pc) => pc * self.height,
        }
    }

    fn instance(&mut self, mut x: f32, mut y: f32, mut w: f32, mut h: f32) {
        let area = self.areas.last().unwrap_or(&[0.0, 0.0, 1.0, 1.0]);
        x = x * area[2] + area[0];
        y = y * area[3] + area[1];
        w *= area[2];
        h *= area[3];
        self.instances[self.inst_cnt] = Vertex::with(self).pos(x, y).scale(w, h);
        self.inst_cnt += 1;
        if self.inst_cnt >= self.instances.len() {
            self.instances
                .resize((self.inst_cnt + 1).next_power_of_two(), Vertex::default());
        }
    }

    /// centered rect
    pub fn rectc(&mut self, x: Unit, y: Unit, w: Unit, h: Unit) {
        let (x, y, w, h) = (self.pc_x(x), self.pc_y(y), self.pc_x(w), self.pc_y(h));
        self.instance(x, y, w, h)
    }

    pub fn rect(&mut self, x: Unit, y: Unit, w: Unit, h: Unit) {
        let (x, y, w, h) = (
            self.pc_x(x),
            self.pc_y(y),
            self.pc_x(w) * 0.5,
            self.pc_y(h) * 0.5,
        );
        self.instance(x + w, y + h, w, h)
    }

    /// rounded centered rect
    pub fn rrectc(&mut self, x: Unit, y: Unit, w: Unit, h: Unit, r: f32) {
        self.roundness += r.min(0.999);
        self.rectc(x, y, w, h);
        self.roundness -= r.min(0.999);
    }

    /// rounded rect
    pub fn rrect(&mut self, x: Unit, y: Unit, w: Unit, h: Unit, r: f32) {
        self.roundness += r.min(0.999);
        self.rect(x, y, w, h);
        self.roundness -= r.min(0.999);
    }

    pub fn aabb(&mut self, x0: Unit, y0: Unit, x1: Unit, y1: Unit) {
        let (x0, y0, x1, y1) = (self.pc_x(x0), self.pc_y(y0), self.pc_x(x1), self.pc_y(y1));
        let (w, h) = ((x1 - x0) * 0.5, (y1 - y0) * 0.5);
        let (x, y) = (x0 - h, y0 - w);
        self.instance(x, y, w, h);
    }

    pub fn circle(&mut self, x: Unit, y: Unit, r: Unit) {
        self.roundness += 1.0;
        self.rectc(x, y, r, r);
        self.roundness -= 1.0;
    }

    pub fn line(&mut self, x0: Unit, y0: Unit, x1: Unit, y1: Unit, w: Unit) {
        let (x0, y0) = (self.px_x(x0), self.px_y(y0));
        let (x1, y1) = (self.px_x(x1), self.px_y(y1));
        let (dx, dy) = (x1 - x0, y1 - y0);
        let an = dy.atan2(dx);
        self.rotation += an;
        let (rw, rh) = (self.width, self.height);
        let len = (dx * dx + dy * dy).sqrt() / rw * 0.5;
        let dw = self.pc_y(w) * 0.5;
        self.instance(
            (x0 + x1) * 0.5 / rw,
            (y0 + y1) * 0.5 / rh,
            len + self.pc_x(w) * 0.5,
            dw,
        );
        self.rotation -= an;
    }

    /// rounded line
    pub fn rline(&mut self, x0: Unit, y0: Unit, x1: Unit, y1: Unit, w: Unit) {
        let old_roundness = self.roundness;
        self.roundness = 0.999;
        self.line(x0, y0, x1, y1, w);
        self.roundness = old_roundness;
    }

    pub fn bezier(&mut self, x0: Unit, y0: Unit, x1: Unit, y1: Unit, x2: Unit, y2: Unit, w: Unit) {
        let (x0, y0) = (self.pc_x(x0), self.pc_y(y0));
        let (x1, y1) = (self.pc_x(x1), self.pc_y(y1));
        let (x2, y2) = (self.pc_x(x2), self.pc_y(y2));
        use Unit::Pc;
        let (mut px, mut py) = (x0, y0);
        let old_roundness = self.roundness;
        self.roundness = 0.999;
        const ITERS: usize = 32;
        for i in 0..ITERS {
            let t = (i + 1) as f32 / ITERS as f32;
            let x = x0.bezier(x1, x2, t);
            let y = y0.bezier(y1, y2, t);
            self.line(Pc(px), Pc(py), Pc(x), Pc(y), w);
            px = x;
            py = y;
        }
        self.roundness = old_roundness;
    }

    pub fn area(&mut self, x: Unit, y: Unit, w: Unit, h: Unit) {
        let area = [self.pc_x(x), self.pc_y(y), self.pc_x(w), self.pc_y(h)];
        if self.areas.is_empty() {
            self.areas.push(area);
        } else {
            let last = self.areas.len() - 1;
            self.areas[last] = area;
        }
    }

    pub fn push_area(&mut self, x: Unit, y: Unit, w: Unit, h: Unit) {
        let mut area = [self.pc_x(x), self.pc_y(y), self.pc_x(w), self.pc_y(h)];
        if let Some(last) = self.areas.last() {
            area[0] += last[0];
            area[1] += last[1];
            area[2] *= last[2];
            area[3] *= last[3];
        }
        self.areas.push(area);
    }

    pub fn pop_area(&mut self) {
        self.areas.pop();
    }

    /// saves old render params to reset to when end_temp() is called
    pub fn begin_temp(&mut self) {
        self.old_color = self.color;
        self.old_stroke_color = self.stroke_color;
        self.old_stroke_width = self.stroke_width;
        self.old_roundness = self.roundness;
        self.old_rotation = self.rotation;
        self.old_tex_coord = self.tex_coord;
    }

    /// resets render params to values before begin_temp() was called
    pub fn end_temp(&mut self) {
        self.color = self.old_color;
        self.stroke_color = self.old_stroke_color;
        self.stroke_width = self.old_stroke_width;
        self.roundness = self.old_roundness;
        self.rotation = self.old_rotation;
        self.tex_coord = self.old_tex_coord;
    }

    pub(crate) fn render(&mut self) {
        if self.vert_cnt != 0 && self.inst_cnt == 0 {
            return;
        }
        let mut ctx = self.ctx.lock().unwrap();
        ctx.bind_pipeline("render");
        ctx.bind_ds("render ds");
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
            .write_buf("render ubo", &resolution);
    }

    pub(crate) fn flush(&mut self) {
        // update instance buffers
        let mut ctx = self.ctx.lock().unwrap();
        if self.vert_cnt != 0 {
            let vbo_size = (self.vertices.len() * size_of::<Vertex>()) as vk::DeviceSize;
            if ctx.buf_size("batch vbo") < vbo_size {
                ctx.recreate_buf("batch vbo", vbo_size);
            }
            ctx.write_buf("batch vbo", &self.vertices[..self.vert_cnt]);
        }
        if self.inst_cnt != 0 {
            let inst_vbo_size = (self.instances.len() * size_of::<Vertex>()) as vk::DeviceSize;
            if ctx.buf_size("instance vbo") < inst_vbo_size {
                ctx.recreate_buf("instance vbo", inst_vbo_size);
            }
            ctx.write_buf("instance vbo", &self.instances[..self.inst_cnt]);
        }
        // update atlas
        let img_datas = self.imgs.values_mut().filter(|i| i.0.is_dirty());
        let mut off = 0;
        let buf_copies = img_datas
            .map(|i| {
                let (x, y, w, h) = i.1.xywh();
                let buf_width = w as u32;
                let copy = BufferImageCopy {
                    buf_off: off,
                    img_off_x: x as u32,
                    img_off_y: y as u32,
                    buf_width,
                    buf_height: h as u32,
                };
                off += 4 * buf_width as vk::DeviceSize * h as vk::DeviceSize;
                i.0.reset();
                (copy, &i.0)
            })
            .collect::<Vec<_>>();
        let staging = &ctx.staging_buf(off);
        for (copy, data) in buf_copies.iter() {
            ctx.write_buf_off(staging, &data[..], copy.buf_off);
        }
        let wrong_layout = ctx.img("atlas").info.layout != ImgLayout::SHADER_READ;
        let copy = !buf_copies.is_empty();
        if copy || wrong_layout {
            ctx.begin_cmd();
        }
        if copy {
            ctx.set_img_layout(
                "atlas",
                ImgLayout::DST,
                vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                vk::PipelineStageFlags2::TRANSFER,
                vk::AccessFlags2::NONE,
                vk::AccessFlags2::TRANSFER_WRITE,
            );
            ctx.copy_buf_to_img(
                staging,
                "atlas",
                &buf_copies.into_iter().map(|(c, _)| c).collect::<Vec<_>>(),
            );
        }
        if copy || wrong_layout {
            if !copy {
                // avoids validation warning:
                // atlas is used for reading but has undefined layout
                // which discards prev content, reading from discarded content makes no sense
                // but in shader we don't read atlas unless it was written to
                // but vulkan doesn't know that, so convert img layout to transfer dst
                // but don't actually write to it, just swindle vulkan
                ctx.set_img_layout(
                    "atlas",
                    ImgLayout::DST,
                    vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
                    vk::PipelineStageFlags2::TRANSFER,
                    vk::AccessFlags2::NONE,
                    vk::AccessFlags2::TRANSFER_WRITE,
                );
            }
            ctx.set_img_layout(
                "atlas",
                ImgLayout::SHADER_READ,
                vk::PipelineStageFlags2::TRANSFER,
                vk::PipelineStageFlags2::FRAGMENT_SHADER,
                vk::AccessFlags2::TRANSFER_WRITE,
                vk::AccessFlags2::SHADER_READ,
            );
            ctx.finish_cmd();
        }
    }

    pub(crate) fn reset(&mut self) {
        self.vert_cnt = 0;
        self.inst_cnt = 0;
        self.color = [255, 255, 255, 255];
        self.stroke_color = [0; 4];
        self.stroke_width = 0.0;
        self.roundness = 0.0;
        self.rotation = 0.0;
        self.areas = Vec::new();
        self.tex_coord = [0, 0];

        self.old_color = self.color;
        self.old_stroke_color = self.stroke_color;
        self.old_stroke_width = self.stroke_width;
        self.old_roundness = self.roundness;
        self.old_rotation = self.rotation;
        self.old_tex_coord = self.tex_coord;
    }
}
