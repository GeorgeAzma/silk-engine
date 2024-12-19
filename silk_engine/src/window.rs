use crate::debug_name;
use crate::gpu_idle;
use crate::physical_gpu;
use crate::scope_time;
use crate::QUEUE;
use crate::{gpu, ENTRY, INSTANCE};
use ash::khr;
use ash::vk;
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

pub struct WindowContext {
    surface_caps2_loader: khr::get_surface_capabilities2::Instance,
    pub surface: vk::SurfaceKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    surface_present_modes: Vec<vk::PresentModeKHR>,
    swapchain_loader: khr::swapchain::Device,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_img_views: Vec<vk::ImageView>,
    pub swapchain_size: vk::Extent2D,
    pub swapchain_img_idx: usize,
}

impl WindowContext {
    pub fn new(window: &Window) -> Self {
        let surface_loader = khr::surface::Instance::new(&ENTRY, &INSTANCE);
        let surface_caps2 = khr::get_surface_capabilities2::Instance::new(&ENTRY, &INSTANCE);
        let surface = unsafe {
            ash_window::create_surface(
                &ENTRY,
                &INSTANCE,
                window.display_handle().unwrap().as_raw(),
                window.window_handle().unwrap().as_raw(),
                None,
            )
            .unwrap()
        };
        let surface_formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(physical_gpu(), surface)
                .unwrap()
        };
        let surface_format = surface_formats
            .iter()
            .find(|&format| format.format == vk::Format::B8G8R8A8_UNORM)
            .cloned()
            .unwrap_or(vk::SurfaceFormatKHR {
                format: vk::Format::B8G8R8A8_UNORM,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            });
        let surface_present_modes = unsafe {
            surface_loader
                .get_physical_device_surface_present_modes(physical_gpu(), surface)
                .unwrap()
        };
        let swapchain_loader = khr::swapchain::Device::new(&INSTANCE, gpu());
        Self {
            surface_caps2_loader: surface_caps2,
            surface,
            surface_format,
            surface_present_modes,
            swapchain_loader,
            swapchain: Default::default(),
            swapchain_images: Default::default(),
            swapchain_img_views: Default::default(),
            swapchain_size: Default::default(),
            swapchain_img_idx: Default::default(),
        }
    }

    pub fn recreate_swapchain(&mut self) {
        let surface_capabilities = self.surface_capabilities();
        let size = self.swapchain_size;
        let surface_resolution = match surface_capabilities.current_extent.width {
            u32::MAX => vk::Extent2D {
                width: size.width,
                height: size.height,
            },
            _ => surface_capabilities.current_extent,
        };
        if surface_resolution.width == 0
            || surface_resolution.height == 0
            || surface_resolution == size
        {
            return;
        }
        self.swapchain_size = surface_resolution;
        scope_time!(
            "resize {}x{}",
            surface_resolution.width,
            surface_resolution.height
        );
        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };
        let present_mode = self
            .surface_present_modes
            .iter()
            .find(|&mode| *mode == vk::PresentModeKHR::MAILBOX)
            .copied()
            .unwrap_or(vk::PresentModeKHR::FIFO);
        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0 {
            desired_image_count = surface_capabilities
                .max_image_count
                .min(desired_image_count);
        }
        // Destroy old swap chain images
        let old_swapchain = self.swapchain;
        self.swapchain = unsafe {
            self.swapchain_loader
                .create_swapchain(
                    &vk::SwapchainCreateInfoKHR::default()
                        .surface(self.surface)
                        .min_image_count(desired_image_count)
                        .image_color_space(self.surface_format.color_space)
                        .image_format(self.surface_format.format)
                        .image_extent(surface_resolution)
                        .image_array_layers(1)
                        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                        .pre_transform(pre_transform)
                        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                        .present_mode(present_mode)
                        .old_swapchain(old_swapchain)
                        .clipped(true),
                    None,
                )
                .unwrap()
        };

        if old_swapchain != Default::default() {
            self.swapchain_images.clear();
            unsafe {
                self.swapchain_img_views
                    .drain(..)
                    .for_each(|image_view| gpu().destroy_image_view(image_view, None));
            }
            unsafe { self.swapchain_loader.destroy_swapchain(old_swapchain, None) };
        }

        self.swapchain_images = unsafe {
            self.swapchain_loader
                .get_swapchain_images(self.swapchain)
                .unwrap()
        };
        self.swapchain_img_views = self
            .swapchain_images
            .iter()
            .enumerate()
            .map(|(i, &swapchain_image)| unsafe {
                let img_view = gpu()
                    .create_image_view(
                        &vk::ImageViewCreateInfo::default()
                            .view_type(vk::ImageViewType::TYPE_2D)
                            .format(self.surface_format.format)
                            .components(vk::ComponentMapping {
                                r: vk::ComponentSwizzle::IDENTITY,
                                g: vk::ComponentSwizzle::IDENTITY,
                                b: vk::ComponentSwizzle::IDENTITY,
                                a: vk::ComponentSwizzle::IDENTITY,
                            })
                            .subresource_range(
                                vk::ImageSubresourceRange::default()
                                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                                    .layer_count(1)
                                    .level_count(1),
                            )
                            .image(swapchain_image),
                        None,
                    )
                    .unwrap();
                debug_name(&format!("swapchain img {i}"), swapchain_image);
                debug_name(&format!("swapchain img view {i}"), img_view);
                img_view
            })
            .collect();

        gpu_idle();
    }

    pub fn acquire_img(&mut self, signal: vk::Semaphore) {
        if self.swapchain == vk::SwapchainKHR::null() {
            self.recreate_swapchain();
        }
        unsafe {
            self.swapchain_img_idx = self
                .swapchain_loader
                .acquire_next_image(self.swapchain, u64::MAX, signal, vk::Fence::null())
                .unwrap()
                .0 as usize;
        }
    }

    pub fn present(&mut self, wait: &[vk::Semaphore]) {
        unsafe {
            self.swapchain_loader
                .queue_present(
                    *QUEUE,
                    &vk::PresentInfoKHR::default()
                        .wait_semaphores(wait)
                        .swapchains(&[self.swapchain])
                        .image_indices(&[self.swapchain_img_idx as u32]),
                )
                .unwrap_or_else(|_| {
                    self.recreate_swapchain();
                    false
                })
        };
    }

    pub fn cur_img(&self) -> vk::Image {
        self.swapchain_images[self.swapchain_img_idx]
    }

    pub fn cur_img_view(&self) -> vk::ImageView {
        self.swapchain_img_views[self.swapchain_img_idx]
    }

    fn surface_capabilities(&self) -> vk::SurfaceCapabilitiesKHR {
        let mut surface_caps = vk::SurfaceCapabilities2KHR::default();
        unsafe {
            self.surface_caps2_loader
                .get_physical_device_surface_capabilities2(
                    physical_gpu(),
                    &vk::PhysicalDeviceSurfaceInfo2KHR::default().surface(self.surface),
                    &mut surface_caps,
                )
                .unwrap()
        };
        surface_caps.surface_capabilities
    }
}
