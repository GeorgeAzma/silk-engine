use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use ash::vk;

use crate::{
    prelude::ResultAny,
    util::{
        dirty::Dirty,
        font::Font,
        image_loader::ImageLoader,
        packer::{Guillotine, Packer, Rect},
    },
    vulkan::{buffer::Buffer, device::Device, image::Image},
};

pub struct TextureAtlas {
    atlas: Arc<Image>,
    atlas_staging: Mutex<Arc<Buffer>>,
    atlas_staging_end: u64,
    packer: Guillotine,
    pub(crate) imgs: HashMap<String, (u64, Dirty<&'static mut [u8]>, Rect)>,
    pub(crate) fonts: HashMap<String, (Font, HashMap<char, Rect>)>,
}

impl TextureAtlas {
    pub(crate) fn new(device: &Arc<Device>, queue_family_index: u32) -> ResultAny<Self> {
        let packer = Guillotine::new(4096, 4096);

        let atlas = Image::new(
            device,
            &vk::ImageCreateInfo::default()
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

        Ok(Self {
            atlas,
            atlas_staging: Mutex::new(Buffer::new(
                device,
                atlas_staging_size,
                vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST,
                &[queue_family_index],
                vk::SharingMode::EXCLUSIVE,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?),
            atlas_staging_end: 0,
            packer,
            imgs: HashMap::new(),
            fonts: HashMap::new(),
        })
    }

    pub fn add_img(
        &mut self,
        name: &str,
        width: u32,
        height: u32,
    ) -> (u64, &mut Dirty<&'static mut [u8]>, Rect) {
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
                    Dirty::new(slice),
                    Rect::new(x, y, width as u16, height as u16),
                )
            });
            (*off, tracked, *rect)
        } else {
            panic!("failed to add img to atlas, out of space")
        }
    }

    pub fn load_img(&mut self, name: &str) -> &mut Dirty<&'static mut [u8]> {
        let mut img_data = ImageLoader::load(name);
        if img_data.channels != 4 {
            img_data.img = ImageLoader::make4(&img_data.img, img_data.channels);
        }
        let tracked_img_data = self.add_img(name, img_data.width, img_data.height).1;
        tracked_img_data.copy_from_slice(&img_data.img);
        tracked_img_data
    }

    pub fn atlas_tex_coord(&mut self) -> [u32; 2] {
        let r = Rect::new(0, 0, self.packer.width(), self.packer.height()).packed_whxy();
        [(r >> 32) as u32, r as u32]
    }

    pub fn no_img_tex_coord(&self) -> [u32; 2] {
        [0, 0]
    }

    pub fn img(&mut self, name: &str) -> &mut Dirty<&'static mut [u8]> {
        let img_data = self
            .imgs
            .get_mut(name)
            .unwrap_or_else(|| panic!("img not found in atlas: {name}"));
        &mut img_data.1
    }

    pub fn img_tex_coord(&self, name: &str) -> [u32; 2] {
        let img_data = self
            .imgs
            .get(name)
            .unwrap_or_else(|| panic!("img not found in atlas: {name}"));
        let r = img_data.2.packed_whxy();
        [(r >> 32) as u32, r as u32]
    }

    pub fn img_data(&self, name: &str) -> (&[u8], u32, u32) {
        let img_data = self
            .imgs
            .get(name)
            .unwrap_or_else(|| panic!("img not found in atlas: {name}"));
        let (w, h) = img_data.2.wh();
        (&img_data.1, w as u32, h as u32)
    }

    pub fn add_font(&mut self, name: &str) {
        let font = Font::new(name);
        self.fonts.insert(name.to_string(), (font, HashMap::new()));
    }

    pub(crate) fn atlas_image(&self) -> &Arc<Image> {
        &self.atlas
    }

    pub(crate) fn atlas_staging(&self) -> &Mutex<Arc<Buffer>> {
        &self.atlas_staging
    }

    pub(crate) fn dirty_copies(&mut self) -> Vec<vk::BufferImageCopy> {
        let dirty_imgs = self.imgs.values_mut().filter(|i| i.1.is_dirty());
        dirty_imgs
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
            .collect()
    }
}
