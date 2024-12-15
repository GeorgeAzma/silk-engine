use crate::{gfx::*, scope_time};
use ash::vk;
use std::sync::Arc;
use winit::{
    event_loop::ActiveEventLoop,
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

#[derive(Default)]
pub struct Swapchain {
    pub swapchain: vk::SwapchainKHR,
    pub images: Vec<vk::Image>,
    pub image_views: Vec<vk::ImageView>,
}

impl Swapchain {
    fn recreate(
        &mut self,
        surface: vk::SurfaceKHR,
        min_image_count: u32,
        image_color_space: vk::ColorSpaceKHR,
        image_format: vk::Format,
        image_extent: vk::Extent2D,
        pre_transform: vk::SurfaceTransformFlagsKHR,
        present_mode: vk::PresentModeKHR,
    ) {
        // Destroy old swap chain images
        let old_swapchain = self.swapchain;
        self.swapchain = unsafe {
            SWAPCHAIN_LOADER
                .create_swapchain(
                    &vk::SwapchainCreateInfoKHR::default()
                        .surface(surface)
                        .min_image_count(min_image_count)
                        .image_color_space(image_color_space)
                        .image_format(image_format)
                        .image_extent(image_extent)
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
            assert!(
                self.images.len() == self.image_views.len(),
                "Mismatched images and image views"
            );
            assert!(self.images.len() > 0, "No images to destroy");
            self.images.clear();
            unsafe {
                self.image_views
                    .drain(..)
                    .for_each(|image_view| DEVICE.destroy_image_view(image_view, None));
            }
            unsafe { SWAPCHAIN_LOADER.destroy_swapchain(old_swapchain, None) };
        }

        self.images = unsafe {
            SWAPCHAIN_LOADER
                .get_swapchain_images(self.swapchain)
                .unwrap()
        };
        self.image_views = self
            .images
            .iter()
            .map(|&swapchain_image| unsafe {
                DEVICE
                    .create_image_view(
                        &vk::ImageViewCreateInfo::default()
                            .view_type(vk::ImageViewType::TYPE_2D)
                            .format(image_format)
                            .components(vk::ComponentMapping {
                                r: vk::ComponentSwizzle::R,
                                g: vk::ComponentSwizzle::G,
                                b: vk::ComponentSwizzle::B,
                                a: vk::ComponentSwizzle::A,
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
                    .unwrap()
            })
            .collect();

        wait_idle();
    }
}

impl std::ops::Deref for Swapchain {
    type Target = vk::SwapchainKHR;

    fn deref(&self) -> &Self::Target {
        &self.swapchain
    }
}

impl std::ops::DerefMut for Swapchain {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.swapchain
    }
}

pub struct WindowData {
    pub window: Arc<Window>,
    pub surface: vk::SurfaceKHR,
    pub swapchain: Swapchain,
    pub size: vk::Extent2D,
}

impl WindowData {
    pub fn new(event_loop: &ActiveEventLoop) -> Self {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes())
                .unwrap(),
        );

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

        Self {
            window,
            surface,
            swapchain: Swapchain::default(),
            size: vk::Extent2D::default(),
        }
    }

    pub fn recreate_swapchain(&mut self) {
        let surface_capabilities = surface_capabilities(self.surface);
        let surface_resolution = match surface_capabilities.current_extent.width {
            u32::MAX => vk::Extent2D {
                width: self.width(),
                height: self.height(),
            },
            _ => surface_capabilities.current_extent,
        };
        if surface_resolution.width == 0
            || surface_resolution.height == 0
            || surface_resolution == self.size
        {
            return;
        }
        self.size = surface_resolution;
        let surface_formats = surface_formats(self.surface);
        let surface_format = surface_formats
            .iter()
            .copied()
            .find(|format| format.format == vk::Format::B8G8R8A8_UNORM)
            .unwrap_or(surface_formats[0]);
        let surface_present_modes = surface_present_modes(self.surface);
        scope_time!(
            "resize {}x{}",
            self.window.inner_size().width,
            self.window.inner_size().height
        );
        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };
        let present_mode = surface_present_modes
            .iter()
            .copied()
            .find(|&mode| mode == vk::PresentModeKHR::MAILBOX)
            .unwrap_or(vk::PresentModeKHR::FIFO);
        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0 {
            desired_image_count = surface_capabilities
                .max_image_count
                .min(desired_image_count);
        }

        self.swapchain.recreate(
            self.surface,
            desired_image_count,
            surface_format.color_space,
            surface_format.format,
            surface_resolution,
            pre_transform,
            present_mode,
        );
    }

    pub fn width(&self) -> u32 {
        self.size.width
    }

    pub fn height(&self) -> u32 {
        self.size.height
    }
}
