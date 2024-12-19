use super::config::*;
use std::ffi::CString;

use crate::*;

lazy_static! {
    static ref GPUS: Vec<vk::PhysicalDevice> = unsafe {
        INSTANCE
            .enumerate_physical_devices()
            .expect("No GPUs found")
    };
    static ref GPU_STUFF: (vk::PhysicalDevice, vk::PhysicalDeviceProperties, vk::PhysicalDeviceFeatures) = {
        // Selects first discrete GPU (non-integrated)
        let (gpu, gpu_props) = GPUS
            .iter()
            .map(|&gpu| {
                let mut props = vk::PhysicalDeviceProperties2::default();
                unsafe { INSTANCE.get_physical_device_properties2(gpu, &mut props) };
                let props = props.properties;
                let mut score = 0;
                score += (props.device_type == vk::PhysicalDeviceType::DISCRETE_GPU) as u32 * 1_000_000;
                score += props.limits.max_image_dimension2_d;
                score += props.limits.max_uniform_buffer_range / 64;
                score += props.limits.max_push_constants_size / 4;
                score += props.limits.max_compute_shared_memory_size / 16;
                score += props.limits.max_compute_work_group_invocations;
                (gpu, props, score)
            }).max_by_key(|(_, _, score)| *score).map(|(gpu, props, _)| (gpu, props)).unwrap();
        let mut features = vk::PhysicalDeviceFeatures2::default();
        unsafe { INSTANCE.get_physical_device_features2(gpu, &mut features) };
        (gpu, gpu_props, features.features)
    };

    pub static ref GPU: vk::PhysicalDevice = GPU_STUFF.0;
    pub static ref GPU_PROPS: vk::PhysicalDeviceProperties = GPU_STUFF.1;
    pub static ref GPU_LIMITS: vk::PhysicalDeviceLimits = GPU_PROPS.limits;
    pub static ref GPU_FEATURES: vk::PhysicalDeviceFeatures = GPU_STUFF.2;
    pub static ref GPU_EXTENSIONS: Vec<CString> = unsafe {
        INSTANCE
            .enumerate_device_extension_properties(*GPU)
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.extension_name_as_c_str().unwrap().to_owned())
            .collect()
    };
    pub static ref GPU_MEMORY_PROPS: vk::PhysicalDeviceMemoryProperties = unsafe {
        let mut mem_props = vk::PhysicalDeviceMemoryProperties2::default();
        INSTANCE.get_physical_device_memory_properties2(*GPU, &mut mem_props);
        mem_props.memory_properties
    };
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
            let sampler_anisotropy = vk::PhysicalDeviceFeatures::default().sampler_anisotropy(true).shader_clip_distance(true);
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
}
