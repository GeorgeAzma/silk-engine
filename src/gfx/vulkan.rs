use crate::*;
pub use ash::vk;
use ash::{khr, vk::Handle};
mod buffer_alloc;
pub use buffer_alloc::*;
mod cmd_alloc;
pub use cmd_alloc::*;
mod desc_alloc;
pub use desc_alloc::*;
mod dsl_manager;
pub use dsl_manager::*;
mod gpu;
pub use gpu::*;
mod instance;
mod pipeline_layout_manager;
pub use instance::*;
use lazy_static::lazy_static;
pub use pipeline_layout_manager::*;
use std::sync::Mutex;
mod config;
use config::*;

lazy_static!(
    pub static ref ENTRY: ash::Entry = unsafe { ash::Entry::load().expect("Failed to load Vulkan") };

    static ref SURFACE_FORMATS: Mutex<HashMap<u64, Vec<vk::SurfaceFormatKHR>>> = Mutex::new(HashMap::new());
    static ref SURFACE_CAPABILITIES: Mutex<HashMap<u64, vk::SurfaceCapabilitiesKHR>> = Mutex::new(HashMap::new());
    static ref SURFACE_PRESENT_MODES: Mutex<HashMap<u64, Vec<vk::PresentModeKHR>>> = Mutex::new(HashMap::new());

    pub static ref QUEUE_FAMILIES: Vec<vk::QueueFamilyProperties> = unsafe {
        let queue_family_props_len = INSTANCE.get_physical_device_queue_family_properties2_len(*GPU);
        let mut queue_family_props = vec![vk::QueueFamilyProperties2::default(); queue_family_props_len];
        INSTANCE.get_physical_device_queue_family_properties2(*GPU, &mut queue_family_props);
        queue_family_props.into_iter().map(|qfp| qfp.queue_family_properties).collect()
    };
    pub static ref QUEUE_FAMILY_INDEX: u32 =
        QUEUE_FAMILIES
        .iter()
        .position(|&queue_family_props| {
            queue_family_props.queue_flags.contains(
                vk::QueueFlags::GRAPHICS
                    | vk::QueueFlags::COMPUTE
                    | vk::QueueFlags::TRANSFER,
            )
        })
        .unwrap_or_default() as u32;

    pub static ref DEVICE: ash::Device = unsafe {
            #[cfg(debug_assertions)]
            log_file!(
                "logs/gpu.log",
                "//////////////////// Properties ////////////////////\n{:#?}\n\n//////////////////// Features ////////////////////\n{:#?}\n\n//////////////////// Extensions ////////////////////\n{:#?}", *GPU_PROPS, *GPU_FEATURES, *GPU_EXTENSIONS
            );

            let required_gpu_extensions = required_vulkan_gpu_extensions();
            required_gpu_extensions
                .iter()
                .filter(|re| !GPU_EXTENSIONS.contains(re))
                .for_each(|re| panic!("Required vulkan gpu extension not found: {re:?}"));
            let mut preferred_gpu_extensions = preferred_vulkan_gpu_extensions();
            preferred_gpu_extensions
                .retain(|pe| {
                    GPU_EXTENSIONS
                        .contains(pe)
                        .then_some(true)
                        .unwrap_or_else(|| {
                            println!("Preferred vulkan gpu extension not found: {pe:?}");
                            false
                        })
                });

                let mut dyn_render = vk::PhysicalDeviceDynamicRenderingFeatures::default().dynamic_rendering(true);
                let mut sync2 = vk::PhysicalDeviceSynchronization2Features::default().synchronization2(true);
                #[cfg(debug_assertions)]
                let mut pipeline_exec_props = vk::PhysicalDevicePipelineExecutablePropertiesFeaturesKHR::default()
                .pipeline_executable_info(true);

                let gpu_exts: Vec<*const i8> = required_gpu_extensions
                    .iter()
                    .chain(preferred_gpu_extensions.iter())
                    .filter(|ext| GPU_EXTENSIONS.contains(ext))
                    .map(|ext| ext.as_ptr())
                    .collect();
                let queue_priorities = [1.0];
                let queue_infos = [
                    vk::DeviceQueueCreateInfo::default()
                        .queue_family_index(*QUEUE_FAMILY_INDEX)
                        .queue_priorities(&queue_priorities)
                ];
                let sampler_anisotropy = vk::PhysicalDeviceFeatures::default().sampler_anisotropy(true);
                let info = vk::DeviceCreateInfo::default()
                    .queue_create_infos(&queue_infos)
                    .enabled_extension_names(&gpu_exts)
                    .enabled_features(&sampler_anisotropy)
                    .push_next(&mut dyn_render)
                    .push_next(&mut sync2);
                #[cfg(debug_assertions)]
                let info = info.push_next(&mut pipeline_exec_props);
                INSTANCE.create_device(*GPU, &info, None)
                .expect("Failed to create VkDevice")
        };
    pub static ref QUEUE: vk::Queue = unsafe { DEVICE.get_device_queue(*QUEUE_FAMILY_INDEX, 0) };

    pub static ref IMAGE_AVAILABLE_SEMAPHORE: vk::Semaphore = unsafe { DEVICE.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap() };
    pub static ref RENDER_FINISHED_SEMAPHORE: vk::Semaphore = unsafe { DEVICE.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap() };
    pub static ref PREV_FRAME_FINISHED_FENCE: vk::Fence = unsafe { DEVICE.create_fence(&vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED), None).unwrap() };

    pub static ref SWAPCHAIN_LOADER: khr::swapchain::Device = khr::swapchain::Device::new(&INSTANCE, &DEVICE);
    pub static ref SURFACE_LOADER: khr::surface::Instance = khr::surface::Instance::new(&ENTRY, &INSTANCE);
    pub static ref PIPELINE_EXEC_PROPS_LOADER: khr::pipeline_executable_properties::Device =
        if cfg!(debug_assertions) {
            khr::pipeline_executable_properties::Device::new(&INSTANCE, &DEVICE)
        } else {
            #[allow(invalid_value)]
            unsafe { std::mem::zeroed() }
        };
    pub static ref SURFACE_CAPS2: khr::get_surface_capabilities2::Instance = {
        khr::get_surface_capabilities2::Instance::new(&ENTRY, &INSTANCE)
    };


    pub static ref DESC_ALLOC: DescAlloc = DescAlloc::new();
    pub static ref DSL_MANAGER: Mutex<DSLManager> = Mutex::new(DSLManager::new());
    pub static ref PIPELINE_LAYOUT_MANAGER: Mutex<PipelineLayoutManager> = Mutex::new(PipelineLayoutManager::new());
    pub static ref CMD_ALLOC: CmdAlloc = CmdAlloc::new();
    pub static ref BUFFER_ALLOC: Mutex<BufferAlloc> = Mutex::new(BufferAlloc::new());

    // FIXME: this is temporary remove later
    pub static ref UNIFORM_BUFFER: vk::Buffer = BUFFER_ALLOC.lock().unwrap().alloc(size_of::<Uniform>() as _, vk::BufferUsageFlags::UNIFORM_BUFFER, vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT);
);

pub fn surface_formats(surface: vk::SurfaceKHR) -> Vec<vk::SurfaceFormatKHR> {
    SURFACE_FORMATS
        .lock()
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
    *SURFACE_CAPABILITIES
        .lock()
        .unwrap()
        .entry(surface.as_raw())
        .or_insert(unsafe {
            let mut surface_caps = vk::SurfaceCapabilities2KHR::default();
            SURFACE_CAPS2
                .get_physical_device_surface_capabilities2(
                    *GPU,
                    &vk::PhysicalDeviceSurfaceInfo2KHR::default().surface(surface),
                    &mut surface_caps,
                )
                .unwrap();
            surface_caps.surface_capabilities
        })
}

pub fn surface_present_modes(surface: vk::SurfaceKHR) -> Vec<vk::PresentModeKHR> {
    SURFACE_PRESENT_MODES
        .lock()
        .unwrap()
        .entry(surface.as_raw())
        .or_insert(unsafe {
            SURFACE_LOADER
                .get_physical_device_surface_present_modes(*GPU, surface)
                .unwrap()
        })
        .clone()
}

#[repr(C)]
pub struct Uniform {
    pub resolution: [u32; 2],
    pub mouse_pos: [f32; 2],
    pub time: f32,
    pub dt: f32,
}

pub fn format_size(format: vk::Format) -> u32 {
    match format {
        vk::Format::R4G4_UNORM_PACK8 => 1,
        vk::Format::R4G4B4A4_UNORM_PACK16 => 2,
        vk::Format::B4G4R4A4_UNORM_PACK16 => 2,
        vk::Format::R5G6B5_UNORM_PACK16 => 2,
        vk::Format::B5G6R5_UNORM_PACK16 => 2,
        vk::Format::R5G5B5A1_UNORM_PACK16 => 2,
        vk::Format::B5G5R5A1_UNORM_PACK16 => 2,
        vk::Format::A1R5G5B5_UNORM_PACK16 => 2,
        vk::Format::R8_UNORM => 1,
        vk::Format::R8_SNORM => 1,
        vk::Format::R8_USCALED => 1,
        vk::Format::R8_SSCALED => 1,
        vk::Format::R8_UINT => 1,
        vk::Format::R8_SINT => 1,
        vk::Format::R8_SRGB => 1,
        vk::Format::R8G8_UNORM => 2,
        vk::Format::R8G8_SNORM => 2,
        vk::Format::R8G8_USCALED => 2,
        vk::Format::R8G8_SSCALED => 2,
        vk::Format::R8G8_UINT => 2,
        vk::Format::R8G8_SINT => 2,
        vk::Format::R8G8_SRGB => 2,
        vk::Format::R8G8B8_UNORM => 3,
        vk::Format::R8G8B8_SNORM => 3,
        vk::Format::R8G8B8_USCALED => 3,
        vk::Format::R8G8B8_SSCALED => 3,
        vk::Format::R8G8B8_UINT => 3,
        vk::Format::R8G8B8_SINT => 3,
        vk::Format::R8G8B8_SRGB => 3,
        vk::Format::B8G8R8_UNORM => 3,
        vk::Format::B8G8R8_SNORM => 3,
        vk::Format::B8G8R8_USCALED => 3,
        vk::Format::B8G8R8_SSCALED => 3,
        vk::Format::B8G8R8_UINT => 3,
        vk::Format::B8G8R8_SINT => 3,
        vk::Format::B8G8R8_SRGB => 3,
        vk::Format::R8G8B8A8_UNORM => 4,
        vk::Format::R8G8B8A8_SNORM => 4,
        vk::Format::R8G8B8A8_USCALED => 4,
        vk::Format::R8G8B8A8_SSCALED => 4,
        vk::Format::R8G8B8A8_UINT => 4,
        vk::Format::R8G8B8A8_SINT => 4,
        vk::Format::R8G8B8A8_SRGB => 4,
        vk::Format::B8G8R8A8_UNORM => 4,
        vk::Format::B8G8R8A8_SNORM => 4,
        vk::Format::B8G8R8A8_USCALED => 4,
        vk::Format::B8G8R8A8_SSCALED => 4,
        vk::Format::B8G8R8A8_UINT => 4,
        vk::Format::B8G8R8A8_SINT => 4,
        vk::Format::B8G8R8A8_SRGB => 4,
        vk::Format::A8B8G8R8_UNORM_PACK32 => 4,
        vk::Format::A8B8G8R8_SNORM_PACK32 => 4,
        vk::Format::A8B8G8R8_USCALED_PACK32 => 4,
        vk::Format::A8B8G8R8_SSCALED_PACK32 => 4,
        vk::Format::A8B8G8R8_UINT_PACK32 => 4,
        vk::Format::A8B8G8R8_SINT_PACK32 => 4,
        vk::Format::A8B8G8R8_SRGB_PACK32 => 4,
        vk::Format::A2R10G10B10_UNORM_PACK32 => 4,
        vk::Format::A2R10G10B10_SNORM_PACK32 => 4,
        vk::Format::A2R10G10B10_USCALED_PACK32 => 4,
        vk::Format::A2R10G10B10_SSCALED_PACK32 => 4,
        vk::Format::A2R10G10B10_UINT_PACK32 => 4,
        vk::Format::A2R10G10B10_SINT_PACK32 => 4,
        vk::Format::A2B10G10R10_UNORM_PACK32 => 4,
        vk::Format::A2B10G10R10_SNORM_PACK32 => 4,
        vk::Format::A2B10G10R10_USCALED_PACK32 => 4,
        vk::Format::A2B10G10R10_SSCALED_PACK32 => 4,
        vk::Format::A2B10G10R10_UINT_PACK32 => 4,
        vk::Format::A2B10G10R10_SINT_PACK32 => 4,
        vk::Format::R16_UNORM => 2,
        vk::Format::R16_SNORM => 2,
        vk::Format::R16_USCALED => 2,
        vk::Format::R16_SSCALED => 2,
        vk::Format::R16_UINT => 2,
        vk::Format::R16_SINT => 2,
        vk::Format::R16_SFLOAT => 2,
        vk::Format::R16G16_UNORM => 4,
        vk::Format::R16G16_SNORM => 4,
        vk::Format::R16G16_USCALED => 4,
        vk::Format::R16G16_SSCALED => 4,
        vk::Format::R16G16_UINT => 4,
        vk::Format::R16G16_SINT => 4,
        vk::Format::R16G16_SFLOAT => 4,
        vk::Format::R16G16B16_UNORM => 6,
        vk::Format::R16G16B16_SNORM => 6,
        vk::Format::R16G16B16_USCALED => 6,
        vk::Format::R16G16B16_SSCALED => 6,
        vk::Format::R16G16B16_UINT => 6,
        vk::Format::R16G16B16_SINT => 6,
        vk::Format::R16G16B16_SFLOAT => 6,
        vk::Format::R16G16B16A16_UNORM => 8,
        vk::Format::R16G16B16A16_SNORM => 8,
        vk::Format::R16G16B16A16_USCALED => 8,
        vk::Format::R16G16B16A16_SSCALED => 8,
        vk::Format::R16G16B16A16_UINT => 8,
        vk::Format::R16G16B16A16_SINT => 8,
        vk::Format::R16G16B16A16_SFLOAT => 8,
        vk::Format::R32_UINT => 4,
        vk::Format::R32_SINT => 4,
        vk::Format::R32_SFLOAT => 4,
        vk::Format::R32G32_UINT => 8,
        vk::Format::R32G32_SINT => 8,
        vk::Format::R32G32_SFLOAT => 8,
        vk::Format::R32G32B32_UINT => 12,
        vk::Format::R32G32B32_SINT => 12,
        vk::Format::R32G32B32_SFLOAT => 12,
        vk::Format::R32G32B32A32_UINT => 16,
        vk::Format::R32G32B32A32_SINT => 16,
        vk::Format::R32G32B32A32_SFLOAT => 16,
        vk::Format::R64_UINT => 8,
        vk::Format::R64_SINT => 8,
        vk::Format::R64_SFLOAT => 8,
        vk::Format::R64G64_UINT => 16,
        vk::Format::R64G64_SINT => 16,
        vk::Format::R64G64_SFLOAT => 16,
        vk::Format::R64G64B64_UINT => 24,
        vk::Format::R64G64B64_SINT => 24,
        vk::Format::R64G64B64_SFLOAT => 24,
        vk::Format::R64G64B64A64_UINT => 32,
        vk::Format::R64G64B64A64_SINT => 32,
        vk::Format::R64G64B64A64_SFLOAT => 32,
        vk::Format::B10G11R11_UFLOAT_PACK32 => 4,
        vk::Format::E5B9G9R9_UFLOAT_PACK32 => 4,
        vk::Format::D16_UNORM => 2,
        vk::Format::X8_D24_UNORM_PACK32 => 4,
        vk::Format::D32_SFLOAT => 4,
        vk::Format::S8_UINT => 1,
        vk::Format::D16_UNORM_S8_UINT => 3,
        vk::Format::D24_UNORM_S8_UINT => 4,
        vk::Format::D32_SFLOAT_S8_UINT => 5,
        _ => 0,
    }
}
