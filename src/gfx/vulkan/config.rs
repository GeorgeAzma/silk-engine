use std::ffi::{CStr, CString};

use ash::khr;

pub fn required_vulkan_instance_extensions() -> Vec<CString> {
    [
        khr::surface::NAME,
        khr::get_surface_capabilities2::NAME,
        #[cfg(target_os = "windows")]
        khr::win32_surface::NAME,
        #[cfg(target_os = "linux")]
        khr::wayland_surface::NAME,
        #[cfg(target_os = "macos")]
        mvk::macos_surface::NAME,
    ]
    .into_iter()
    .map(|e| e.to_owned())
    .collect()
}

pub fn preferred_vulkan_instance_extensions() -> Vec<CString> {
    [
        #[cfg(debug_assertions)]
        ash::ext::debug_utils::NAME,
    ]
    .into_iter()
    .map(|e: &CStr| e.to_owned())
    .collect()
}

pub fn enabled_layers() -> Vec<CString> {
    [
        #[cfg(debug_assertions)]
        "VK_LAYER_KHRONOS_validation",
    ]
    .into_iter()
    .map(|e: &str| CString::new(e).unwrap())
    .collect()
}

pub fn required_vulkan_gpu_extensions() -> Vec<CString> {
    [khr::swapchain::NAME]
        .into_iter()
        .map(|e| e.to_owned())
        .collect()
}

pub fn preferred_vulkan_gpu_extensions() -> Vec<CString> {
    [
        // khr::draw_indirect_count::NAME,
        #[cfg(debug_assertions)]
        khr::pipeline_executable_properties::NAME,
    ]
    .into_iter()
    .map(|e: &CStr| e.to_owned())
    .collect()
}
