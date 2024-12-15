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
}
