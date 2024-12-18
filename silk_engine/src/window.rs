use crate::gpu_idle;
use crate::scope_time;
use crate::DebugMarker;
use crate::GPU;
use crate::{DEVICE, ENTRY, INSTANCE};
use ash::khr;
use ash::vk;
use lazy_static::lazy_static;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use vk::Handle;
use winit::{
    raw_window_handle::{HasDisplayHandle, HasWindowHandle},
    window::Window,
};

lazy_static! {
    pub static ref SURFACE_LOADER: khr::surface::Instance =
        khr::surface::Instance::new(&ENTRY, &INSTANCE);
    pub static ref SURFACE_CAPS2: khr::get_surface_capabilities2::Instance =
        khr::get_surface_capabilities2::Instance::new(&ENTRY, &INSTANCE);
    static ref SURFACE_FORMATS: RwLock<HashMap<u64, Vec<vk::SurfaceFormatKHR>>> =
        RwLock::new(HashMap::new());
    pub static ref SURFACE_FORMAT: vk::SurfaceFormatKHR = {
        surface_formats(*SURFACE)
            .into_iter()
            .find(|format| format.format == vk::Format::B8G8R8A8_UNORM)
            .unwrap_or(vk::SurfaceFormatKHR {
                format: vk::Format::B8G8R8A8_UNORM,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            })
    };
    static ref SURFACE_PRESENT_MODES: RwLock<HashMap<u64, Vec<vk::PresentModeKHR>>> =
        RwLock::new(HashMap::new());
    pub static ref SWAPCHAIN_LOADER: khr::swapchain::Device =
        khr::swapchain::Device::new(&INSTANCE, &DEVICE);
    pub static ref WINDOW: RwLock<Option<Arc<Window>>> = RwLock::new(None);
    pub static ref SURFACE: vk::SurfaceKHR = unsafe {
        let window = WINDOW.read().unwrap();
        let window = window.as_ref().unwrap();
        ash_window::create_surface(
            &ENTRY,
            &INSTANCE,
            window.display_handle().unwrap().as_raw(),
            window.window_handle().unwrap().as_raw(),
            None,
        )
        .unwrap()
    };
    pub static ref SWAPCHAIN: RwLock<vk::SwapchainKHR> = RwLock::new(Default::default());
    pub static ref SWAPCHAIN_IMAGES: RwLock<Vec<vk::Image>> = RwLock::new(Default::default());
    pub static ref SWAPCHAIN_IMG_VIEWS: RwLock<Vec<vk::ImageView>> =
        RwLock::new(Default::default());
    pub static ref SWAPCHAIN_SIZE: RwLock<vk::Extent2D> = RwLock::new(Default::default());
    pub static ref SWAPCHAIN_IMG_IDX: RwLock<usize> = RwLock::new(Default::default());
}

pub fn swap_img_idx() -> usize {
    *SWAPCHAIN_IMG_IDX.read().unwrap()
}

pub fn swap_size() -> vk::Extent2D {
    *SWAPCHAIN_SIZE.read().unwrap()
}

pub fn swapchain() -> vk::SwapchainKHR {
    *SWAPCHAIN.read().unwrap()
}

pub fn cur_swap_img() -> vk::Image {
    SWAPCHAIN_IMAGES.read().unwrap()[swap_img_idx()]
}

pub fn cur_swap_img_view() -> vk::ImageView {
    SWAPCHAIN_IMG_VIEWS.read().unwrap()[swap_img_idx()]
}

pub fn surf_format() -> vk::Format {
    SURFACE_FORMAT.format
}

pub fn recreate_swapchain() {
    let surface_capabilities = surface_capabilities(*SURFACE);
    let size = swap_size();
    let surface_resolution = match surface_capabilities.current_extent.width {
        u32::MAX => vk::Extent2D {
            width: size.width,
            height: size.height,
        },
        _ => surface_capabilities.current_extent,
    };
    if surface_resolution.width == 0 || surface_resolution.height == 0 || surface_resolution == size
    {
        return;
    }
    *SWAPCHAIN_SIZE.write().unwrap() = surface_resolution;
    let surface_present_modes = surface_present_modes(*SURFACE);
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
    // Destroy old swap chain images
    let old_swapchain = *SWAPCHAIN.read().unwrap();
    *SWAPCHAIN.write().unwrap() = unsafe {
        SWAPCHAIN_LOADER
            .create_swapchain(
                &vk::SwapchainCreateInfoKHR::default()
                    .surface(*SURFACE)
                    .min_image_count(desired_image_count)
                    .image_color_space(SURFACE_FORMAT.color_space)
                    .image_format(SURFACE_FORMAT.format)
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
        assert!(
            SWAPCHAIN_IMAGES.read().unwrap().len() == SWAPCHAIN_IMG_VIEWS.read().unwrap().len(),
            "Mismatched images and image views"
        );
        assert!(
            !SWAPCHAIN_IMAGES.read().unwrap().is_empty(),
            "No images to destroy"
        );
        SWAPCHAIN_IMAGES.write().unwrap().clear();
        unsafe {
            SWAPCHAIN_IMG_VIEWS
                .write()
                .unwrap()
                .drain(..)
                .for_each(|image_view| DEVICE.destroy_image_view(image_view, None));
        }
        unsafe { SWAPCHAIN_LOADER.destroy_swapchain(old_swapchain, None) };
    }

    *SWAPCHAIN_IMAGES.write().unwrap() = unsafe {
        SWAPCHAIN_LOADER
            .get_swapchain_images(*SWAPCHAIN.read().unwrap())
            .unwrap()
    };
    *SWAPCHAIN_IMG_VIEWS.write().unwrap() = SWAPCHAIN_IMAGES
        .read()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, &swapchain_image)| unsafe {
            let img_view = DEVICE
                .create_image_view(
                    &vk::ImageViewCreateInfo::default()
                        .view_type(vk::ImageViewType::TYPE_2D)
                        .format(surf_format())
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
            DebugMarker::name(&format!("swapchain img {i}"), swapchain_image);
            DebugMarker::name(&format!("swapchain img view {i}"), img_view);
            img_view
        })
        .collect();

    gpu_idle();
}

pub fn acquire_img(signal: vk::Semaphore) {
    if *SWAPCHAIN.read().unwrap() == vk::SwapchainKHR::null() {
        recreate_swapchain();
    }
    unsafe {
        *SWAPCHAIN_IMG_IDX.write().unwrap() = SWAPCHAIN_LOADER
            .acquire_next_image(
                *SWAPCHAIN.read().unwrap(),
                u64::MAX,
                signal,
                vk::Fence::null(),
            )
            .unwrap()
            .0 as usize;
    }
}

pub fn surface_formats(surface: vk::SurfaceKHR) -> Vec<vk::SurfaceFormatKHR> {
    SURFACE_FORMATS
        .write()
        .unwrap()
        .entry(surface.as_raw())
        .or_insert(unsafe {
            SURFACE_LOADER
                .get_physical_device_surface_formats(*GPU, surface)
                .unwrap()
        })
        .clone()
}

pub fn surface_capabilities(surface: vk::SurfaceKHR) -> vk::SurfaceCapabilitiesKHR {
    let mut surface_caps = vk::SurfaceCapabilities2KHR::default();
    unsafe {
        SURFACE_CAPS2
            .get_physical_device_surface_capabilities2(
                *GPU,
                &vk::PhysicalDeviceSurfaceInfo2KHR::default().surface(surface),
                &mut surface_caps,
            )
            .unwrap()
    };
    surface_caps.surface_capabilities
}

pub fn surface_present_modes(surface: vk::SurfaceKHR) -> Vec<vk::PresentModeKHR> {
    SURFACE_PRESENT_MODES
        .write()
        .unwrap()
        .entry(surface.as_raw())
        .or_insert(unsafe {
            SURFACE_LOADER
                .get_physical_device_surface_present_modes(*GPU, surface)
                .unwrap()
        })
        .clone()
}
