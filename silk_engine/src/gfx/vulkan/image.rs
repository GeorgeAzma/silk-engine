use crate::{alloc_callbacks, queue_family_index, samples_u32_to_vk};

use super::gpu;
use ash::vk;

#[derive(Clone)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub depth: u32,
    pub layers: u32,
    pub levels: u32,
    pub format: vk::Format,
    pub samples: u32,
    pub flags: vk::ImageCreateFlags,
    pub usage: vk::ImageUsageFlags,
    pub init_layout: vk::ImageLayout,
}

impl Default for ImageInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl ImageInfo {
    pub fn new() -> Self {
        Self {
            width: 1,
            height: 1,
            depth: 1,
            layers: 1,
            levels: 1,
            samples: 1,
            format: vk::Format::B8G8R8A8_UNORM,
            flags: vk::ImageCreateFlags::empty(),
            usage: vk::ImageUsageFlags::empty(),
            init_layout: vk::ImageLayout::UNDEFINED,
        }
    }

    pub fn cube(mut self) -> Self {
        self.layers = 6;
        self.flags |= vk::ImageCreateFlags::CUBE_COMPATIBLE;
        self
    }

    pub fn width(mut self, width: u32) -> Self {
        assert!(width > 0, "width is 0");
        self.width = width;
        self
    }

    pub fn height(mut self, height: u32) -> Self {
        assert!(height > 0, "height is 0");
        self.height = height;
        self
    }

    pub fn depth(mut self, depth: u32) -> Self {
        assert!(depth > 0, "depth is 0");
        self.depth = depth;
        self
    }

    pub fn layers(mut self, layers: u32) -> Self {
        assert!(layers > 0, "layers is 0");
        self.layers = layers;
        self
    }

    pub fn levels(mut self, levels: u32) -> Self {
        assert!(levels > 0, "levels is 0");
        self.levels = levels;
        self
    }

    pub fn samples(mut self, samples: u32) -> Self {
        assert!(samples > 0, "samples is 0");
        self.samples = samples;
        self
    }

    pub fn format(mut self, format: vk::Format) -> Self {
        assert!(format != vk::Format::UNDEFINED, "image format is undefined");
        self.format = format;
        self
    }

    pub fn usage(mut self, usage: vk::ImageUsageFlags) -> Self {
        assert!(!self.usage.is_empty(), "image usage is empty");
        self.usage |= usage;
        self
    }

    pub fn init_layout(mut self, init_layout: vk::ImageLayout) -> Self {
        self.init_layout = init_layout;
        self
    }

    pub fn build(&self) -> vk::Image {
        unsafe {
            gpu()
                .create_image(
                    &vk::ImageCreateInfo::default()
                        .extent(vk::Extent3D {
                            width: self.width,
                            height: self.height,
                            depth: self.depth,
                        })
                        .image_type(match (self.width, self.height, self.depth) {
                            (1, 1, _) => vk::ImageType::TYPE_1D,
                            (1, _, _) => vk::ImageType::TYPE_2D,
                            (_, _, _) => vk::ImageType::TYPE_3D,
                        })
                        .array_layers(self.layers)
                        .mip_levels(self.levels)
                        .samples(samples_u32_to_vk(self.samples))
                        .format(self.format)
                        .flags(self.flags)
                        .usage(self.usage)
                        .initial_layout(self.init_layout)
                        .queue_family_indices(&[queue_family_index()])
                        .sharing_mode(vk::SharingMode::EXCLUSIVE),
                    alloc_callbacks(),
                )
                .unwrap()
        }
    }
}