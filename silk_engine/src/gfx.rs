use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    prelude::ResultAny,
    util::{
        font::Font,
        image_loader::ImageLoader,
        packer::{Guillotine, Packer, Rect},
        tracked::Tracked,
    },
    vulkan::{
        PhysicalDeviceUse, QueueFamilyUse, Vulkan,
        buffer::Buffer,
        command_manager::CommandManager,
        device::Device,
        image::Image,
        physical_device::PhysicalDevice,
        pipeline::{Pipeline, PipelineConfig, PipelineLayout},
        shader::Shader,
        window::{Frame, Window},
    },
};
use ash::vk::{self, BufferUsageFlags, ImageCreateInfo, MemoryPropertyFlags};

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Unit {
    /// pixels
    Px(i32),
    /// 1.0 is min(width, height) pixels
    Mn(f32),
    /// 1.0 is max(width, height) pixels
    Mx(f32),
    /// screen is 0-1 range
    Pc(f32),
}

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
    fn pos(mut self, x: f32, y: f32) -> Self {
        self.pos =
            (x.clamp(0.0, 1.0) * 65535.0) as u32 | (((y.clamp(0.0, 1.0) * 65535.0) as u32) << 16);
        self
    }

    fn scale(mut self, w: f32, h: f32) -> Self {
        self.scale =
            (w.clamp(0.0, 1.0) * 65535.0) as u32 | (((h.clamp(0.0, 1.0) * 65535.0) as u32) << 16);
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

    fn superellipse(mut self, superellipse: f32) -> Self {
        self.superellipse = superellipse;
        self
    }
}

struct Instances {
    inst_ptr: *mut Vertex,
    inst_len: usize,
    inst_cap: usize,
    vertex_buffer: Arc<Buffer>,
}

impl Instances {
    fn new(device: &Arc<Device>, queue_family_index: u32, size: usize) -> ResultAny<Self> {
        let vertex_buffer = Buffer::new(
            &device,
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

    fn add(&mut self, vertex: Vertex) -> ResultAny {
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

    fn reset(&mut self) {
        self.inst_len = 0;
    }
}

pub struct Gfx {
    pub(crate) physical_device: Arc<PhysicalDevice>,
    pub(crate) device: Arc<Device>,
    queue: vk::Queue,
    surface_format: Option<vk::Format>,
    command_manager: Arc<CommandManager>,
    shader: Shader,
    pipeline: Option<Pipeline>,
    pipeline_layout: Option<PipelineLayout>,
    uniform: Arc<Buffer>,
    atlas: Arc<Image>,
    width: f32,
    height: f32,

    pub color: [u8; 4],
    /// negative for text
    pub roundness: f32,
    pub rotation: f32,
    pub stroke_width: f32,
    pub stroke_color: [u8; 4],
    /// [-1, 0, 1] = [thin, normal, bold]
    pub bold: f32,
    /// negative = glow
    pub blur: f32,
    pub stroke_blur: f32,
    pub gradient: [u8; 4],
    /// f32::MAX = gradient
    pub gradient_dir: f32,
    pub superellipse: f32,
    /// packed whxy
    tex_coord: [u32; 2],
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
    old_superellipse: f32,
    old_tex_coord: [u32; 2],
    old_font: String,

    areas: Vec<[f32; 4]>,
    packer: Guillotine,
    imgs: HashMap<String, (u64, Tracked<&'static mut [u8]>, Rect)>,
    fonts: HashMap<String, (Font, HashMap<char, Rect>)>,

    instances: Instances,
    atlas_staging: Mutex<Arc<Buffer>>,
    atlas_staging_end: u64,
    descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    descriptor_sets: Vec<vk::DescriptorSet>,
}

impl Gfx {
    pub fn new(vulkan: &Arc<Vulkan>) -> ResultAny<Self> {
        let physical_device = vulkan
            .best_physical_device_for(PhysicalDeviceUse::General)
            .ok_or("no suitable GPU found")?;

        let queue_family_index = vulkan
            .best_queue_family_for(
                &physical_device.queue_family_properties,
                QueueFamilyUse::General,
            )
            .ok_or("no suitable Queue Family found")?;

        let queue_create_infos = [vk::DeviceQueueCreateInfo::default()
            .queue_family_index(queue_family_index)
            .queue_priorities(&[1.0])];

        let mut descriptor_indexing = vk::PhysicalDeviceDescriptorIndexingFeatures::default()
            .shader_sampled_image_array_non_uniform_indexing(true)
            .shader_storage_buffer_array_non_uniform_indexing(true)
            .shader_storage_image_array_non_uniform_indexing(true)
            .descriptor_binding_sampled_image_update_after_bind(true)
            .descriptor_binding_storage_buffer_update_after_bind(true)
            .descriptor_binding_storage_image_update_after_bind(true)
            .descriptor_binding_partially_bound(true)
            .descriptor_binding_variable_descriptor_count(true)
            .runtime_descriptor_array(true);

        let mut features13 = vk::PhysicalDeviceVulkan13Features::default()
            .synchronization2(true)
            .dynamic_rendering(true);

        let mut enabled_device_features = vk::PhysicalDeviceFeatures2::default()
            .push_next(&mut descriptor_indexing)
            .push_next(&mut features13);

        let enabled_device_extensions = [ash::khr::swapchain::NAME.as_ptr()];

        let device_info = vk::DeviceCreateInfo::default()
            .queue_create_infos(&queue_create_infos)
            .enabled_extension_names(&enabled_device_extensions)
            .push_next(&mut enabled_device_features);

        let device = Device::new(&physical_device, &device_info)?;

        let queue = device.get_queue(queue_family_index, 0);
        device.debug_name(queue, "gfx");

        let command_manager = device.command_manager(queue_family_index);

        let shader = Shader::new(&["test.vert", "test.frag"], &device)?;

        let uniform = Buffer::new(
            &device,
            (2 * size_of::<f32>()) as u64,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
            &[queue_family_index],
            vk::SharingMode::EXCLUSIVE,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
        )?;

        let packer = Guillotine::new(4096, 4096);

        let atlas = Image::new(
            &device,
            &ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .extent(vk::Extent3D {
                    width: packer.width() as u32,
                    height: packer.height() as u32,
                    depth: 1,
                })
                .format(vk::Format::R8G8B8A8_UNORM)
                .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED),
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let atlas_staging_size = packer.width() as u64 * packer.height() as u64 * 4;

        let atlas_view = atlas.create_view()?;
        let atlas_sampler = device.get_sampler(
            vk::SamplerAddressMode::REPEAT,
            vk::SamplerAddressMode::REPEAT,
            vk::Filter::LINEAR,
            vk::Filter::LINEAR,
            vk::SamplerMipmapMode::LINEAR,
        );

        let descriptor_set_layouts = shader
            .reflect_descriptor_set_layouts()?
            .into_values()
            .collect::<Vec<_>>();
        let descriptor_sets = device.alloc_ds(&descriptor_set_layouts);
        let uniform_info = vk::DescriptorBufferInfo::default()
            .buffer(uniform.handle())
            .offset(0)
            .range(vk::WHOLE_SIZE);
        let atlas_info = vk::DescriptorImageInfo::default()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(atlas_view)
            .sampler(atlas_sampler);
        let ds_write_uniform = vk::WriteDescriptorSet::default()
            .dst_set(descriptor_sets[0])
            .dst_binding(0)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(std::slice::from_ref(&uniform_info));
        let ds_write_atlas = vk::WriteDescriptorSet::default()
            .dst_set(descriptor_sets[0])
            .dst_binding(1)
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .image_info(std::slice::from_ref(&atlas_info));
        unsafe {
            device
                .device
                .update_descriptor_sets(&[ds_write_uniform, ds_write_atlas], &[])
        };

        Ok(Self {
            physical_device,
            device: device.clone(),
            queue,
            surface_format: None,
            command_manager,
            shader,
            pipeline: None,
            pipeline_layout: None,
            uniform,
            atlas,
            width: 0.0,
            height: 0.0,

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
            packer, // TODO: resizable packer
            imgs: HashMap::new(),
            fonts: HashMap::new(),

            instances: Instances::new(&device, queue_family_index, 1024)?,
            atlas_staging: Mutex::new(Buffer::new(
                &device,
                atlas_staging_size,
                BufferUsageFlags::TRANSFER_SRC | BufferUsageFlags::TRANSFER_DST,
                &[queue_family_index],
                vk::SharingMode::EXCLUSIVE,
                MemoryPropertyFlags::HOST_VISIBLE | MemoryPropertyFlags::HOST_COHERENT,
            )?),
            atlas_staging_end: 0,
            descriptor_set_layouts,
            descriptor_sets,
        })
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

    pub fn add_img(
        &mut self,
        name: &str,
        width: u32,
        height: u32,
    ) -> (u64, &mut Tracked<&'static mut [u8]>, Rect) {
        assert!(!self.imgs.contains_key(name), "img already in atlas");
        if let Some((x, y)) = self.packer.pack(width as u16, height as u16) {
            let (off, tracked, rect) = self.imgs.entry(name.to_string()).or_insert_with(|| {
                let size = width as u64 * height as u64 * 4;
                let ptr = self.atlas_staging.lock().unwrap().map();
                let slice = unsafe {
                    std::slice::from_raw_parts_mut(
                        ptr.add(self.atlas_staging_end as usize),
                        size as usize,
                    )
                };
                self.atlas_staging_end += size;
                (
                    self.atlas_staging_end - size,
                    Tracked::new(slice),
                    Rect::new(x, y, width as u16, height as u16),
                )
            });
            (*off, tracked, rect.clone())
        } else {
            panic!("failed to add img to atlas, out of space")
        }
    }

    pub fn load_img(&mut self, name: &str) -> &mut Tracked<&'static mut [u8]> {
        let mut img_data = ImageLoader::load(name);
        if img_data.channels != 4 {
            img_data.img = ImageLoader::make4(&mut img_data.img, img_data.channels);
        }
        let tracked_img_data = self.add_img(name, img_data.width, img_data.height).1;
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

    pub fn img(&mut self, name: &str) -> &mut Tracked<&'static mut [u8]> {
        let img_data = self
            .imgs
            .get_mut(name)
            .unwrap_or_else(|| panic!("img not found in atlas: {name}"));
        let r = img_data.2.packed_whxy();
        self.tex_coord = [(r >> 32) as u32, r as u32];
        &mut img_data.1
    }

    pub fn img_data(&self, name: &str) -> (&[u8], u32, u32) {
        let img_data = self
            .imgs
            .get(name)
            .unwrap_or_else(|| panic!("img not found in atlas: {name}"));
        let (w, h) = img_data.2.wh();
        (&img_data.1, w as u32, h as u32)
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

    fn vert(&mut self, x: f32, y: f32, w: f32, h: f32) -> Vertex {
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
        self.roundness = r;
        self.rectc(x, y, w, h);
        self.roundness = old_roundness;
    }

    /// rounded rect
    pub fn rrect(&mut self, x: Unit, y: Unit, w: Unit, h: Unit, r: f32) {
        let old_roundness = self.roundness;
        self.roundness = r;
        self.rect(x, y, w, h);
        self.roundness = old_roundness;
    }

    /// centered square
    pub fn squarec(&mut self, x: Unit, y: Unit, w: Unit) {
        self.rectc(x, y, w, w)
    }

    pub fn square(&mut self, x: Unit, y: Unit, w: Unit) {
        self.rect(x, y, w, w)
    }

    /// rounded square
    pub fn rsquare(&mut self, x: Unit, y: Unit, w: Unit, r: f32) {
        self.rrect(x, y, w, w, r)
    }

    /// rounded centered square
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

    /// Renders text. Returns bounding rect in pixels
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
        let self_ptr = self as *mut Self;
        let (font, char_rects) = unsafe { &mut *self_ptr }
            .fonts
            .entry(self.font.clone())
            .or_insert_with(|| {
                if let Ok(true) = std::fs::exists(format!("res/fonts/{}.ttf", self.font)) {
                    (Font::new(&self.font), HashMap::new())
                } else {
                    panic!(
                        "failed to render text \"{text}\", font does not exist: {}",
                        self.font
                    )
                }
            });

        const SDF_PX: u32 = 64;
        let px = SDF_PX as f32;
        let (mut ax, mut ay, mut bx, mut by) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        let layout = font.layout(text);
        for (i, c) in text.chars().enumerate() {
            if !font.is_char_graphic(c) {
                continue;
            }

            let rect = *char_rects.entry(c).or_insert_with(|| {
                let sdf_img = font.gen_char_sdf(c, SDF_PX);
                let name = format!("{}-{c}", font.name());
                let (_, img, rect) = self.add_img(&name, sdf_img.width, sdf_img.height);
                img.copy_from_slice(&ImageLoader::make4(&sdf_img.img, 1));
                rect
            });

            let (lx, ly) = layout[i];
            let (rw, rh) = rect.wh();
            let (rw, rh) = (rw as f32 / px * w, rh as f32 / px * h);
            let r = rect.packed_whxy();
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

    /// defines rendering sub-area inside full rendering area `[-1; 1]`\
    /// primitives following this will render inside sub-area as if it were full rendering area
    pub fn area(&mut self, x: Unit, y: Unit, w: Unit, h: Unit) {
        let area = [self.pc_x(x), self.pc_y(y), self.pc_x(w), self.pc_y(h)];
        if self.areas.is_empty() {
            self.areas.push(area);
        } else {
            let last = self.areas.len() - 1;
            self.areas[last] = area;
        }
    }

    /// defines rendering sub-area inside last pushed rendering sub-area\
    /// primitives following this will render inside sub-area as if it were full rendering area
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

    /// saves rendering parameters to reset to when end_temp() is called
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

    /// resets rendering parameters to values before begin_temp() was called
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

    pub(crate) fn flush(&mut self) -> ResultAny {
        let dirty_imgs = self.imgs.values_mut().filter(|i| i.1.is_dirty());
        let buf_copies = dirty_imgs
            .map(|(off, tracked, rect)| {
                let (x, y, w, h) = rect.xywh();
                let copy = vk::BufferImageCopy {
                    buffer_offset: *off,
                    buffer_row_length: 0,
                    buffer_image_height: 0,
                    image_subresource: vk::ImageSubresourceLayers::default()
                        .aspect_mask(vk::ImageAspectFlags::COLOR)
                        .layer_count(1),
                    image_offset: vk::Offset3D {
                        x: x as i32,
                        y: y as i32,
                        z: 0,
                    },
                    image_extent: vk::Extent3D {
                        width: w as u32,
                        height: h as u32,
                        depth: 1,
                    },
                };
                tracked.reset();
                copy
            })
            .collect::<Vec<_>>();
        if !buf_copies.is_empty() {
            let cmd = self.command_manager.begin()?;
            self.atlas
                .transition(cmd, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
            self.atlas.copy_from_buffer_cmd(
                cmd,
                self.atlas_staging.lock().unwrap().as_ref(),
                &buf_copies,
            );
            self.atlas
                .transition(cmd, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
            self.command_manager
                .submit(self.queue, cmd, &[], &[], &[])?;
            self.command_manager.wait(cmd)?;
        }
        Ok(())
    }

    fn recreate_render_data(&mut self) -> ResultAny {
        let surface_format = self.surface_format.unwrap_or(vk::Format::UNDEFINED);
        let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(std::slice::from_ref(&surface_format));

        let pipeline_layout = PipelineLayout::new(&self.device, &self.descriptor_set_layouts, &[])?;
        self.pipeline_layout = Some(pipeline_layout);

        let spec_info = vk::SpecializationInfo::default();
        let mut pipeline_info = PipelineConfig::default();
        let pipeline_info = pipeline_info
            .with_shader(&self.shader, &spec_info)?
            .with_auto_vertex_inputs()?
            .add_color_blend_disabled_attachment()
            .build(self.pipeline_layout.as_ref().unwrap().handle())
            .push_next(&mut rendering_info);

        let pipeline = Pipeline::new(&self.device, &pipeline_info)?;
        self.pipeline = Some(pipeline);

        Ok(())
    }

    fn record_command_buffer(
        &mut self,
        window: &mut Window,
        frame: &Frame,
    ) -> ResultAny<vk::CommandBuffer> {
        let extent = window.extent();
        let cmd = self.command_manager.begin()?;
        {
            let swapchain_image = &mut window.swapchain().images()[frame.image_index as usize];
            let color_attachment = vk::RenderingAttachmentInfo::default()
                .image_view(swapchain_image.view())
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.1, 0.0, 0.3, 1.0],
                    },
                });
            let color_attachments = [color_attachment];
            let rendering_info = vk::RenderingInfo::default()
                .render_area(vk::Rect2D::default().extent(extent))
                .layer_count(1)
                .color_attachments(&color_attachments);

            swapchain_image.transition(cmd, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

            self.atlas
                .transition(cmd, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

            unsafe { self.device().cmd_begin_rendering(cmd, &rendering_info) };
            {
                unsafe {
                    self.device().cmd_set_viewport(
                        cmd,
                        0,
                        &[vk::Viewport::default()
                            .y(extent.height as f32)
                            .width(extent.width as f32)
                            .height(-(extent.height as f32))],
                    )
                };
                unsafe {
                    self.device()
                        .cmd_set_scissor(cmd, 0, &[vk::Rect2D::default().extent(extent)])
                };
                unsafe {
                    self.device().cmd_bind_descriptor_sets(
                        cmd,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline_layout.as_ref().unwrap().handle(),
                        0,
                        &self.descriptor_sets,
                        &[],
                    )
                };
                unsafe {
                    self.device().cmd_bind_pipeline(
                        cmd,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline.as_ref().unwrap().handle(),
                    )
                };
                unsafe {
                    self.device().cmd_bind_vertex_buffers(
                        cmd,
                        1,
                        &[self.instances.vertex_buffer.handle()],
                        &[0],
                    );
                }
                unsafe {
                    self.device()
                        .cmd_draw(cmd, 4, self.instances.inst_len as u32, 0, 0)
                };
            }
            unsafe { self.device().cmd_end_rendering(cmd) };

            swapchain_image.transition(cmd, vk::ImageLayout::PRESENT_SRC_KHR);
        }
        self.command_manager.end()?;

        Ok(cmd)
    }

    pub fn render(&mut self, window: &mut Window) {
        if self.width as u32 != window.width() || self.height as u32 != window.height() {
            self.width = window.width() as f32;
            self.height = window.height() as f32;
            self.uniform.write_mapped(&[self.width, self.height]);
        }

        self.flush().unwrap();

        let Some(frame) = window.begin_frame(|cmd| {
            self.command_manager.wait(cmd).unwrap();
        }) else {
            return;
        };

        let surface_format = window.format();
        if surface_format != self.surface_format.unwrap_or(vk::Format::UNDEFINED) {
            self.surface_format = Some(surface_format);
            self.recreate_render_data().unwrap();
        }

        let cmd = self.record_command_buffer(window, &frame).unwrap();

        self.command_manager
            .submit(
                self.queue,
                cmd,
                std::slice::from_ref(&frame.wait_semaphore),
                std::slice::from_ref(&frame.signal_semaphore),
                &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
            )
            .unwrap();

        window.end_frame(self.queue, cmd);
        self.reset();
    }

    pub fn device(&self) -> &ash::Device {
        &self.device.device
    }
}
