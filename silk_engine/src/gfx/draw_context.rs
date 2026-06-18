use std::sync::Arc;

use ash::vk;

use crate::{
    prelude::ResultAny,
    vulkan::{buffer::Buffer, device::Device},
};

use super::{Unit, Vertex};

pub(crate) struct Instances {
    inst_ptr: *mut Vertex,
    pub(crate) inst_len: usize,
    inst_cap: usize,
    pub(crate) vertex_buffer: Arc<Buffer>,
}

unsafe impl Send for Instances {}
unsafe impl Sync for Instances {}

impl Instances {
    fn new(device: &Arc<Device>, queue_family_index: u32, size: usize) -> ResultAny<Self> {
        let vertex_buffer = Buffer::new(
            device,
            (size * size_of::<Vertex>()) as u64,
            vk::BufferUsageFlags::VERTEX_BUFFER
                | vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::TRANSFER_SRC,
            &[queue_family_index],
            vk::SharingMode::EXCLUSIVE,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let inst_ptr = vertex_buffer.map() as *mut Vertex;

        Ok(Self {
            inst_ptr,
            inst_len: 0,
            inst_cap: size,
            vertex_buffer,
        })
    }

    pub(crate) fn add(&mut self, vertex: Vertex) -> ResultAny {
        unsafe {
            self.inst_ptr.add(self.inst_len).write(vertex);
        }
        self.inst_len += 1;
        if self.inst_len >= self.inst_cap {
            let new_cap = (self.inst_len + 1).next_power_of_two();
            let new_size = (new_cap * size_of::<Vertex>()) as vk::DeviceSize;
            self.vertex_buffer = self.vertex_buffer.clone().resize(new_size)?;
            self.inst_ptr = self.vertex_buffer.map() as *mut Vertex;
            self.inst_cap = new_cap;
        }

        Ok(())
    }

    pub(crate) fn reset(&mut self) {
        self.inst_len = 0;
    }
}

pub struct DrawContext {
    pub color: [u8; 4],
    pub roundness: f32,
    pub rotation: f32,
    pub stroke_width: f32,
    pub stroke_color: [u8; 4],
    pub bold: f32,
    pub blur: f32,
    pub stroke_blur: f32,
    pub gradient: [u8; 4],
    pub gradient_dir: f32,
    pub superellipse: f32,
    pub(crate) tex_coord: [u32; 2],
    pub(crate) font: String,

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
    old_superellipse: f32,
    old_tex_coord: [u32; 2],
    old_font: String,

    areas: Vec<[f32; 4]>,
    pub(crate) instances: Instances,
    width: f32,
    height: f32,
}

impl DrawContext {
    pub(crate) fn new(device: &Arc<Device>, queue_family_index: u32) -> ResultAny<Self> {
        let instances = Instances::new(device, queue_family_index, 1024)?;
        Ok(Self {
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
            superellipse: 2.0,
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
            old_superellipse: f32::MAX,
            old_tex_coord: [0, 0],
            old_font: String::new(),

            areas: vec![],
            instances,
            width: 0.0,
            height: 0.0,
        })
    }

    pub(crate) fn set_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }

    pub fn pc_x(&self, unit: Unit) -> f32 {
        match unit {
            Unit::Px(px) => px as f32 / self.width,
            Unit::Mn(mn) => mn * self.width.min(self.height) / self.width,
            Unit::Mx(mx) => mx * self.width.max(self.height) / self.width,
            Unit::Pc(pc) => pc,
        }
    }

    pub fn pc_y(&self, unit: Unit) -> f32 {
        match unit {
            Unit::Px(px) => px as f32 / self.height,
            Unit::Mn(mn) => mn * self.width.min(self.height) / self.height,
            Unit::Mx(mx) => mx * self.width.max(self.height) / self.height,
            Unit::Pc(pc) => pc,
        }
    }

    pub fn px_x(&self, unit: Unit) -> f32 {
        match unit {
            Unit::Px(px) => px as f32,
            Unit::Mn(mn) => mn * self.width.min(self.height),
            Unit::Mx(mx) => mx * self.width.max(self.height),
            Unit::Pc(pc) => pc * self.width,
        }
    }

    pub fn px_y(&self, unit: Unit) -> f32 {
        match unit {
            Unit::Px(px) => px as f32,
            Unit::Mn(mn) => mn * self.width.min(self.height),
            Unit::Mx(mx) => mx * self.width.max(self.height),
            Unit::Pc(pc) => pc * self.height,
        }
    }

    pub(crate) fn vert(&mut self, x: f32, y: f32, w: f32, h: f32) -> Vertex {
        Vertex {
            pos: Default::default(),
            scale: Default::default(),
            color: self.color,
            roundness: self.roundness,
            rotation: self.rotation,
            stroke_width: self.stroke_width,
            stroke_color: self.stroke_color,
            tex_coord: self.tex_coord,
            blur: self.blur,
            stroke_blur: self.stroke_blur,
            gradient: self.gradient,
            gradient_dir: self.gradient_dir,
            superellipse: self.superellipse,
        }
        .pos(x, y)
        .scale(w, h)
    }

    fn instance(&mut self, mut x: f32, mut y: f32, mut w: f32, mut h: f32) {
        let area = self.areas.last().unwrap_or(&[0.0, 0.0, 1.0, 1.0]);
        x = x * area[2] + area[0];
        y = y * area[3] + area[1];
        w *= area[2];
        h *= area[3];
        let vertex = self.vert(x, y, w, h);
        self.instances.add(vertex).unwrap();
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

    pub fn rrectc(&mut self, x: Unit, y: Unit, w: Unit, h: Unit, r: f32) {
        let old_roundness = self.roundness;
        self.roundness = r;
        self.rectc(x, y, w, h);
        self.roundness = old_roundness;
    }

    pub fn rrect(&mut self, x: Unit, y: Unit, w: Unit, h: Unit, r: f32) {
        let old_roundness = self.roundness;
        self.roundness = r;
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

    pub fn rline(&mut self, x0: Unit, y0: Unit, x1: Unit, y1: Unit, w: Unit) {
        let old_roundness = self.roundness;
        self.roundness = 0.999;
        self.line(x0, y0, x1, y1, w);
        self.roundness = old_roundness;
    }

    #[allow(clippy::too_many_arguments)]
    pub fn bezier(&mut self, x0: Unit, y0: Unit, x1: Unit, y1: Unit, x2: Unit, y2: Unit, w: Unit) {
        fn bezier(a: f32, b: f32, c: f32, t: f32) -> f32 {
            t * (t * (c - 2.0 * b + a) + 2.0 * (b - a)) + a
        }

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
            let x = bezier(x0, x1, x2, t);
            let y = bezier(y0, y1, y2, t);
            self.line(Pc(px), Pc(py), Pc(x), Pc(y), w);
            px = x;
            py = y;
        }
        self.roundness = old_roundness;
    }

    /// defines rendering sub-area inside full rendering area `[-1; 1]`
    pub fn area(&mut self, x: Unit, y: Unit, w: Unit, h: Unit) {
        let area = [self.pc_x(x), self.pc_y(y), self.pc_x(w), self.pc_y(h)];
        if self.areas.is_empty() {
            self.areas.push(area);
        } else {
            let last = self.areas.len() - 1;
            self.areas[last] = area;
        }
    }

    /// defines rendering sub-area inside last pushed rendering sub-area
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
        self.old_superellipse = self.superellipse;
    }

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
        self.superellipse = self.old_superellipse;
    }

    pub(crate) fn reset(&mut self) {
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
        self.superellipse = 2.0;

        self.instances.reset();
        self.begin_temp();
    }
}
