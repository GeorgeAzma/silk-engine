use std::{ffi::CString, sync::Arc};

use ash::vk;

use crate::{prelude::ResultAny, vulkan::Vulkan};

pub(crate) struct PhysicalDevice {
    pub(crate) physical_device: vk::PhysicalDevice,
    pub(crate) properties: vk::PhysicalDeviceProperties,
    pub(crate) features: vk::PhysicalDeviceFeatures,
    pub(crate) extensions: Vec<CString>,
    pub(crate) memory_properties: vk::PhysicalDeviceMemoryProperties,
    pub(crate) queue_family_properties: Vec<vk::QueueFamilyProperties>,
    pub(crate) vulkan: Arc<Vulkan>,
}

impl PhysicalDevice {
    pub(crate) fn get_surface_formats(
        &self,
        surface: vk::SurfaceKHR,
    ) -> ResultAny<Vec<vk::SurfaceFormatKHR>> {
        Ok(unsafe {
            self.vulkan()
                .surface_instance()
                .get_physical_device_surface_formats(self.physical_device, surface)
        }?)
    }

    pub(crate) fn get_surface_present_modes(
        &self,
        surface: vk::SurfaceKHR,
    ) -> ResultAny<Vec<vk::PresentModeKHR>> {
        Ok(unsafe {
            self.vulkan()
                .surface_instance()
                .get_physical_device_surface_present_modes(self.physical_device, surface)
        }?)
    }

    pub(crate) fn get_capabilities(&self, surface: vk::SurfaceKHR) -> vk::SurfaceCapabilitiesKHR {
        let mut capabilities = vk::SurfaceCapabilities2KHR::default();
        let surface_info = vk::PhysicalDeviceSurfaceInfo2KHR::default().surface(surface);
        unsafe {
            self.vulkan()
                .get_surface_capabilities2_instance()
                .get_physical_device_surface_capabilities2(
                    self.physical_device,
                    &surface_info,
                    &mut capabilities,
                )
                .unwrap()
        };
        capabilities.surface_capabilities
    }

    pub(crate) fn vulkan(&self) -> &Arc<Vulkan> {
        &self.vulkan
    }
}
