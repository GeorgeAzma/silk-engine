mod font;
mod packer;
mod render_ctx;
mod shader;
mod unit;
mod vulkan;

pub use font::Font;
use packer::Rect;
pub use packer::{Guillotine, Packer, Shelf};
pub use render_ctx::{BufferImageCopy, DebugScope, RenderCtx, debug_name, debug_tag};
pub use unit::Unit;
pub use unit::Unit::*;
pub use vulkan::*;

use core::f32;
// TODO:
// - make roundness Unit
// - make stroke_width Unit
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use ash::vk;

use crate::{
    event::WindowResize,
    util::{Bezier, ImageLoader, Tracked},
};

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
}

#[allow(unused)]
impl Vertex {
    fn pos(mut self, x: f32, y: f32) -> Self {
        self.pos = (x * 65535.0) as u32 | (((y * 65535.0) as u32) << 16);
        self
    }

    fn scale(mut self, w: f32, h: f32) -> Self {
        self.scale = (w * 65535.0) as u32 | (((h * 65535.0) as u32) << 16);
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

    fn blur(mut self, blur: f32) -> Self {
        self.blur = blur;
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

    fn stk_blur(mut self, stroke_blur: f32) -> Self {
        self.stroke_blur = stroke_blur;
        self
    }

    fn grad(mut self, gradient: [u8; 4]) -> Self {
        self.gradient = gradient;
        self
    }

    fn grad_dir(mut self, gradient_dir: f32) -> Self {
        self.gradient_dir = gradient_dir;
        self
    }

    fn with(gfx: &Gfx) -> Self {
        Self {
            pos: Default::default(),
            scale: Default::default(),
            color: gfx.color,
            roundness: gfx.roundness,
            rotation: gfx.rotation,
            stroke_width: gfx.stroke_width,
            stroke_color: gfx.stroke_color,
            tex_coord: gfx.tex_coord,
            blur: gfx.blur,
            stroke_blur: gfx.stroke_blur,
            gradient: gfx.gradient,
            gradient_dir: gfx.gradient_dir,
        }
    }
}

pub struct Gfx {
    ctx: Arc<Mutex<RenderCtx>>,
    instances: *mut Vertex,
    inst_cnt: usize,
    inst_len: usize,
    pub color: [u8; 4],
    pub roundness: f32,
    pub rotation: f32,
    pub stroke_width: f32,
    pub stroke_color: [u8; 4],
    /// [-1, 0, 1] = [thin, normal, bold]
    pub bold: f32,
    /// negative is glow
    pub blur: f32,
    pub stroke_blur: f32,
    pub gradient: [u8; 4],
    /// f32::MAX means no gradient
    pub gradient_dir: f32,
    tex_coord: [u32; 2], // packed whxy
    font: String,
    old_color: [u8; 4],
    old_roundness: f32,
    old_rotation: f32,
    old_stroke_width: f32,
    old_stroke_color: [u8; 4],
    old_bold: f32,
    old_blur: f32,
    old_stroke_blur: f32,
    old_gradient: [u8; 4],
    old_gradient_dir: f32,
    old_tex_coord: [u32; 2],
    old_font: String,
    areas: Vec<[f32; 4]>,
    width: f32,
    height: f32,
    packer: Guillotine,
    imgs: HashMap<String, (Tracked<Vec<u8>>, Rect)>,
    fonts: HashMap<String, (Font, HashMap<char, Rect>)>,
    batch_idx: usize,
}

const SDF_PX: u32 = 64;

impl Gfx {
    pub fn new(ctx: Arc<Mutex<RenderCtx>>) -> Self {
        // TODO: resizable packer
        let packer = Guillotine::new(1024, 1024);
        {
            let mut ctx = ctx.lock().unwrap();
            ctx.add_buf(
                "instance vbo",
                (1024 * size_of::<Vertex>()) as vk::DeviceSize,
                BufUsage::VERT,
                MemProp::CPU_GPU,
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
                MemProp::CPU,
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
            ctx.add_sampler(
                "atlas sampler",
                vk::SamplerAddressMode::CLAMP_TO_EDGE,
                vk::SamplerAddressMode::CLAMP_TO_EDGE,
                vk::Filter::LINEAR,
                vk::Filter::LINEAR,
                vk::SamplerMipmapMode::NEAREST,
            );

            ctx.write_ds_img("render ds", "atlas view", ImgLayout::SHADER_READ, 1);
            ctx.write_ds_sampler("render ds", "atlas sampler", 2);
        }
        let instance_buf = ctx.lock().unwrap().buf("instance vbo");
        let inst_len = ctx.lock().unwrap().buf_size("instance vbo") as usize / size_of::<Vertex>();
        let instances = ctx.lock().unwrap().gpu_alloc.map(instance_buf) as *mut Vertex;
        Self {
            ctx,
            instances,
            inst_cnt: 0,
            inst_len,
            color: [255, 255, 255, 255],
            roundness: 0.0,
            rotation: 0.0,
            stroke_width: 0.0,
            stroke_color: [0, 0, 0, 0],
            bold: 0.0,
            blur: 0.0,
            stroke_blur: 0.0,
            gradient: [255, 255, 255, 255],
            gradient_dir: f32::MAX,
            tex_coord: [0, 0],
            font: String::new(),
            old_color: [255, 255, 255, 255],
            old_roundness: 0.0,
            old_rotation: 0.0,
            old_stroke_width: 0.0,
            old_stroke_color: [0, 0, 0, 0],
            old_bold: 0.0,
            old_blur: 0.0,
            old_stroke_blur: 0.0,
            old_gradient: [255, 255, 255, 255],
            old_gradient_dir: f32::MAX,
            old_tex_coord: [0, 0],
            old_font: String::new(),
            areas: Vec::new(),
            width: 0.0,
            height: 0.0,
            packer,
            imgs: HashMap::new(),
            fonts: HashMap::new(),
            batch_idx: usize::MAX,
        }
    }

    pub fn alpha(&mut self, a: u8) {
        self.color[3] = a;
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

    pub fn glow(&mut self, glow: f32) {
        self.blur = -glow;
    }

    pub fn stroke_alpha(&mut self, a: u8) {
        self.stroke_color[3] = a;
    }

    pub fn stroke_rgb(&mut self, r: u8, g: u8, b: u8) {
        self.stroke_color = [r, g, b, 255];
    }

    pub fn stroke_rgba(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.stroke_color = [r, g, b, a];
    }

    pub fn stroke_hex(&mut self, hex: u32) {
        self.stroke_color = hex.to_be_bytes()
    }

    pub fn gradient_alpha(&mut self, a: u8) {
        self.gradient[3] = a;
    }

    pub fn gradient_rgb(&mut self, r: u8, g: u8, b: u8) {
        self.gradient = [r, g, b, 255];
    }

    pub fn gradient_rgba(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.gradient = [r, g, b, a];
    }

    pub fn gradient_hex(&mut self, hex: u32) {
        self.gradient = hex.to_be_bytes()
    }

    pub fn no_gradient(&mut self) {
        self.gradient_dir = f32::MAX;
    }

    pub fn font(&mut self, font: &str) {
        self.font = font.to_string();
    }

    pub fn add_font(&mut self, name: &str) {
        let font = Font::new(name);
        self.font = name.to_string();
        self.fonts.insert(self.font.clone(), (font, HashMap::new()));
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
            img_data.img = ImageLoader::make4(&mut img_data.img, img_data.channels);
        }
        let tracked_img_data = self.add_img(name, img_data.width, img_data.height);
        tracked_img_data.copy_from_slice(&img_data.img);
        tracked_img_data
    }

    pub fn atlas(&mut self) {
        let r = Rect::new(0, 0, self.packer.width(), self.packer.height()).packed_whxy();
        self.tex_coord = [(r >> 32) as u32, r as u32];
    }

    pub fn no_img(&mut self) {
        self.tex_coord = [0, 0];
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

    pub fn img_data(&self, name: &str) -> (&[u8], u32, u32) {
        let img_data = self
            .imgs
            .get(name)
            .unwrap_or_else(|| panic!("img not found in atlas: {name}"));
        let (w, h) = img_data.1.wh();
        (&img_data.0, w as u32, h as u32)
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
        let vert = Vertex::with(self).pos(x, y).scale(w, h);
        unsafe {
            self.instances.add(self.inst_cnt).write(vert);
        }
        self.inst_cnt += 1;
        if self.inst_cnt >= self.inst_len {
            let new_len = (self.inst_cnt + 1).next_power_of_two();
            let new_size = (new_len * size_of::<Vertex>()) as vk::DeviceSize;
            let mut ctx = self.ctx.lock().unwrap();
            ctx.resize_buf("instance vbo", new_size);
            self.instances = ctx.map_buf("instance vbo") as *mut Vertex;
            self.inst_len = new_len;
        }
    }

    pub fn begin_batch(&mut self) {
        self.batch_idx = self.inst_cnt;
    }

    pub fn end_batch(&mut self) -> Vec<Vertex> {
        assert!(
            self.batch_idx != usize::MAX,
            "can't end instance batch that has not begun"
        );
        assert!(self.batch_idx < self.inst_cnt, "no instances in batch");
        let batch = unsafe {
            std::slice::from_raw_parts(
                self.instances.add(self.batch_idx),
                self.inst_cnt - self.batch_idx,
            )
        }
        .to_vec();
        self.inst_cnt = self.batch_idx;
        self.batch_idx = usize::MAX;
        batch
    }
    pub fn batch(&mut self, instances: &[Vertex]) {
        let new_inst_cnt = self.inst_cnt + instances.len();
        if new_inst_cnt >= self.inst_len {
            let new_len = (new_inst_cnt + 1).next_power_of_two();
            let new_size = (new_len * size_of::<Vertex>()) as vk::DeviceSize;
            let mut ctx = self.ctx.lock().unwrap();
            ctx.resize_buf("instance vbo", new_size);
            self.instances = ctx.map_buf("instance vbo") as *mut Vertex;
            self.inst_len = new_len;
        }
        unsafe {
            std::ptr::copy_nonoverlapping(
                instances.as_ptr(),
                self.instances.add(self.inst_cnt),
                instances.len(),
            )
        };
        self.inst_cnt = new_inst_cnt;
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
        let old_roundness = self.roundness;
        self.roundness = r.min(0.999);
        self.rectc(x, y, w, h);
        self.roundness = old_roundness;
    }

    /// rounded rect
    pub fn rrect(&mut self, x: Unit, y: Unit, w: Unit, h: Unit, r: f32) {
        let old_roundness = self.roundness;
        self.roundness = r.min(0.999);
        self.rect(x, y, w, h);
        self.roundness = old_roundness;
    }

    pub fn squarec(&mut self, x: Unit, y: Unit, w: Unit) {
        self.rectc(x, y, w, w)
    }

    pub fn square(&mut self, x: Unit, y: Unit, w: Unit) {
        self.rect(x, y, w, w)
    }

    pub fn rsquare(&mut self, x: Unit, y: Unit, w: Unit, r: f32) {
        self.rrect(x, y, w, w, r)
    }

    pub fn rsquarec(&mut self, x: Unit, y: Unit, w: Unit, r: f32) {
        self.rrect(x, y, w, w, r)
    }

    pub fn aabb(&mut self, x0: Unit, y0: Unit, x1: Unit, y1: Unit) {
        let (x0, y0, x1, y1) = (self.pc_x(x0), self.pc_y(y0), self.pc_x(x1), self.pc_y(y1));
        let (w, h) = ((x1 - x0) * 0.5, (y1 - y0) * 0.5);
        let (x, y) = (x0 + w, y0 + h);
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

    fn char_rect(
        c: char,
        font: &Font,
        char_rects: &mut HashMap<char, Rect>,
        imgs: &mut HashMap<String, (Tracked<Vec<u8>>, Rect)>,
        packer: &mut Guillotine,
    ) -> Rect {
        if !font.is_char_graphic(c) {
            return Rect::default();
        }
        *char_rects.entry(c).or_insert_with(|| {
            let img = font.gen_char_sdf(c, SDF_PX);
            if let Some((x, y)) = packer.pack(img.width as u16, img.height as u16) {
                let img_data = &mut imgs.entry(format!("{c}: {x}, {y}")).or_insert((
                    Tracked::new(vec![0; img.width as usize * img.height as usize * 4]),
                    Rect::new(x, y, img.width as u16, img.height as u16),
                ));
                img_data.0.copy_from_slice(&ImageLoader::make4(&img.img, 1));
                img_data.1
            } else {
                panic!("failed to add img to atlas, out of space")
            }
        })
    }

    /// returns bounding rect in pixels
    pub fn text(&mut self, text: &str, x: Unit, y: Unit, w: Unit) -> (i32, i32, i32, i32) {
        let old_tex_coord = self.tex_coord;
        assert!(
            self.font.as_str() != "",
            "failed to render text, no font is active"
        );
        let old_roundness = self.roundness;
        self.roundness = -(self.bold + 1.0 + 1e-5);
        let (x, y) = (self.pc_x(x), self.pc_y(y));
        let (w, h) = (self.pc_x(w), self.pc_y(w));
        let (font, char_rects) = self.fonts.entry(self.font.clone()).or_insert_with(|| {
            use crate::RES_PATH;
            if let Ok(true) = std::fs::exists(format!("{RES_PATH}/fonts/{}.ttf", self.font)) {
                (Font::new(&self.font), HashMap::new())
            } else {
                panic!(
                    "failed to render text \"{text}\", font does not exist: {}",
                    self.font
                )
            }
        });
        let rects_glyph_sizes = text
            .chars()
            .zip(font.layout(text).iter())
            .map(|(c, &(lx, ly))| {
                let r = Self::char_rect(c, font, char_rects, &mut self.imgs, &mut self.packer);
                (lx, ly, r)
            })
            .collect::<Vec<_>>();
        _ = &*char_rects;
        let px = SDF_PX as f32;
        let (mut ax, mut ay, mut bx, mut by) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for (lx, ly, r) in rects_glyph_sizes {
            if r == Default::default() {
                continue;
            }
            let (rw, rh) = r.wh();
            let (rw, rh) = (rw as f32 / px * w, rh as f32 / px * h);
            let r = r.packed_whxy();
            self.tex_coord = [(r >> 32) as u32, r as u32];
            let (x, y) = (x + lx * w, y + ly * h);
            let (w, h) = (rw, rh);
            self.instance(x + w, y + h, w, h);
            ax = ax.min(x);
            ay = ay.min(y);
            bx = bx.max(x + w * 2.0);
            by = by.max(y + h * 2.0);
        }
        self.roundness = old_roundness;
        self.tex_coord = old_tex_coord;
        use Unit::*;
        let (ax, ay, bx, by) = (
            self.px_x(Pc(ax)),
            self.px_y(Pc(ay)),
            self.px_x(Pc(bx)),
            self.px_y(Pc(by)),
        );
        (ax as i32, ay as i32, bx as i32, by as i32)
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
        self.old_font = self.font.clone();
        self.old_bold = self.bold;
        self.old_blur = self.blur;
        self.old_stroke_blur = self.stroke_blur;
        self.old_gradient = self.gradient;
        self.old_gradient_dir = self.gradient_dir;
    }

    /// resets render params to values before begin_temp() was called
    pub fn end_temp(&mut self) {
        self.color = self.old_color;
        self.stroke_color = self.old_stroke_color;
        self.stroke_width = self.old_stroke_width;
        self.roundness = self.old_roundness;
        self.rotation = self.old_rotation;
        self.tex_coord = self.old_tex_coord;
        self.font = self.old_font.clone();
        self.bold = self.old_bold;
        self.blur = self.old_blur;
        self.stroke_blur = self.old_stroke_blur;
        self.gradient = self.old_gradient;
        self.gradient_dir = self.old_gradient_dir;
    }

    pub(crate) fn render(&mut self) {
        if self.inst_cnt == 0 {
            return;
        }
        let mut ctx = self.ctx.lock().unwrap();
        ctx.bind_pipeline("render");
        ctx.bind_ds("render ds");
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
        self.inst_cnt = 0;
        self.color = [255, 255, 255, 255];
        self.stroke_color = [0; 4];
        self.stroke_width = 0.0;
        self.roundness = 0.0;
        self.rotation = 0.0;
        self.areas = Vec::new();
        self.tex_coord = [0, 0];
        self.font = String::new();
        self.bold = 0.0;
        self.blur = 0.0;
        self.stroke_blur = 0.0;
        self.gradient = [255, 255, 255, 255];
        self.gradient_dir = f32::MAX;

        self.old_color = self.color;
        self.old_stroke_color = self.stroke_color;
        self.old_stroke_width = self.stroke_width;
        self.old_roundness = self.roundness;
        self.old_rotation = self.rotation;
        self.old_tex_coord = self.tex_coord;
        self.old_font = self.font.clone();
        self.old_bold = self.bold;
        self.old_blur = self.blur;
        self.old_stroke_blur = self.stroke_blur;
        self.old_gradient = self.gradient;
        self.old_gradient_dir = self.gradient_dir;

        self.batch_idx = usize::MAX;
    }
}
