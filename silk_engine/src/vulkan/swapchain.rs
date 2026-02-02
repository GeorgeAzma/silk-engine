use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Weak},
};

use ash::vk;

use crate::{
    prelude::ResultAny,
    vulkan::{device::Device, image::Image, surface::Surface},
};

pub(crate) struct Swapchain {
    swapchain: vk::SwapchainKHR,
    images: Vec<Arc<Image>>,
    create_info: vk::SwapchainCreateInfoKHR<'static>,
    device: Weak<Device>,
}

impl Deref for Swapchain {
    type Target = vk::SwapchainKHR;
    fn deref(&self) -> &Self::Target {
        &self.swapchain
    }
}

impl DerefMut for Swapchain {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.swapchain
    }
}

impl Swapchain {
    pub(crate) fn new(
        device: &Arc<Device>,
        create_info: vk::SwapchainCreateInfoKHR<'static>,
    ) -> Self {
        Self {
            swapchain: vk::SwapchainKHR::null(),
            images: vec![],
            create_info,
            device: Arc::downgrade(device),
        }
    }

    pub(crate) fn recreate(
        &mut self,
        create_info: &vk::SwapchainCreateInfoKHR<'static>,
    ) -> ResultAny {
        if create_info.image_extent.width == 0 || create_info.image_extent.height == 0 {
            return Ok(());
        }
        let device = self.device();
        let alloc_callbacks = device.allocation_callbacks();
        self.swapchain = unsafe {
            self.device()
                .swapchain_device()
                .create_swapchain(create_info, alloc_callbacks.as_ref())
        }?;
        let old_swapchain = create_info.old_swapchain;
        self.create_info = *create_info;

        // Destroy old swapchain and image views
        if old_swapchain != Default::default() {
            unsafe { device.device.device_wait_idle() }?;
            self.images.clear(); // destroys images
            unsafe {
                self.device()
                    .swapchain_device()
                    .destroy_swapchain(old_swapchain, alloc_callbacks.as_ref())
            };
        }

        let images = unsafe {
            self.device()
                .swapchain_device()
                .get_swapchain_images(self.swapchain)
        }?;
        self.images.reserve_exact(images.len());
        for image in images {
            let queue_family_indices = if create_info.queue_family_index_count > 0 {
                unsafe {
                    std::slice::from_raw_parts(
                        create_info.p_queue_family_indices,
                        create_info.queue_family_index_count as usize,
                    )
                    .to_vec()
                }
            } else {
                vec![]
            };
            let image = Image::new_with_image(
                &device,
                image,
                &vk::ImageCreateInfo::default()
                    .format(self.create_info.image_format)
                    .image_type(vk::ImageType::TYPE_2D)
                    .extent(vk::Extent3D {
                        width: create_info.image_extent.width,
                        height: create_info.image_extent.height,
                        depth: 1,
                    })
                    .array_layers(create_info.image_array_layers)
                    .mip_levels(1)
                    .samples(vk::SampleCountFlags::TYPE_1)
                    .usage(create_info.image_usage)
                    .sharing_mode(create_info.image_sharing_mode)
                    .queue_family_indices(&queue_family_indices)
                    .initial_layout(vk::ImageLayout::UNDEFINED),
            )?;
            image.create_view()?;
            self.images.push(image);
        }

        Ok(())
    }

    pub(crate) fn recreate_from_surface(
        &mut self,
        surface: &mut Surface,
        width: u32,
        height: u32,
        preferred_formats: &[vk::SurfaceFormatKHR],
        preferred_present_modes: &[vk::PresentModeKHR],
    ) -> ResultAny<vk::Extent2D> {
        let surface_capabilities = *surface.update_capabilities();

        let surface_resolution = if surface_capabilities.current_extent.width == u32::MAX {
            vk::Extent2D {
                width: width.clamp(
                    surface_capabilities.min_image_extent.width,
                    surface_capabilities.max_image_extent.width,
                ),
                height: height.clamp(
                    surface_capabilities.min_image_extent.height,
                    surface_capabilities.max_image_extent.height,
                ),
            }
        } else {
            surface_capabilities.current_extent
        };

        let created_before = self.create_info.image_format != vk::Format::default();
        let format = if preferred_formats.is_empty() && created_before {
            vk::SurfaceFormatKHR {
                format: self.create_info.image_format,
                color_space: self.create_info.image_color_space,
            }
        } else {
            surface.choose_format(preferred_formats)?
        };
        let present_mode = if preferred_present_modes.is_empty() && created_before {
            self.create_info.present_mode
        } else {
            surface.choose_present_mode(preferred_present_modes)?
        };

        let same_size = surface_resolution.width == self.create_info.image_extent.width
            && surface_resolution.height == self.create_info.image_extent.height;
        let same_format = format.format == self.create_info.image_format
            && format.color_space == self.create_info.image_color_space;
        let same_present_mode = present_mode == self.create_info.present_mode;

        if same_size && same_format && same_present_mode && created_before {
            return Ok(surface_resolution);
        }

        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };

        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0 {
            desired_image_count = surface_capabilities
                .max_image_count
                .min(desired_image_count);
        }

        let old_swapchain = self.swapchain;
        let mut swapchain_info = self
            .create_info
            .surface(**surface)
            .min_image_count(desired_image_count)
            .image_color_space(format.color_space)
            .image_format(format.format)
            .image_extent(surface_resolution)
            .pre_transform(pre_transform)
            .present_mode(present_mode)
            .old_swapchain(old_swapchain);

        if !created_before {
            swapchain_info.image_array_layers = 1;
            swapchain_info.composite_alpha = vk::CompositeAlphaFlagsKHR::OPAQUE;
            swapchain_info.image_usage =
                vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_DST;
            swapchain_info.image_sharing_mode = vk::SharingMode::EXCLUSIVE;
            swapchain_info.clipped = true.into();
        }

        self.recreate(&swapchain_info)?;

        Ok(surface_resolution)
    }

    pub(crate) fn images(&mut self) -> &mut [Arc<Image>] {
        &mut self.images
    }

    pub(crate) fn create_info(&self) -> &vk::SwapchainCreateInfoKHR<'static> {
        &self.create_info
    }

    pub(crate) fn device(&self) -> Arc<Device> {
        self.device.upgrade().unwrap()
    }
}
