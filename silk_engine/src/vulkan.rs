use ash::{khr, vk};

use std::{
    ffi::{CStr, CString},
    path::Path,
    sync::{Arc, Mutex},
};

use crate::{
    fatal,
    prelude::ResultAny,
    vulkan::{
        alloc_callback::{AllocHandler, AllocManager, NoOpAllocHandler},
        physical_device::PhysicalDevice,
    },
    warn,
};

pub(crate) mod alloc;
pub(crate) mod alloc_callback;
pub(crate) mod buffer;
pub(crate) mod command_manager;
pub(crate) mod command_pool;
pub(crate) mod device;
pub(crate) mod ds_alloc;
pub(crate) mod dsl_manager;
pub(crate) mod image;
pub(crate) mod physical_device;
pub(crate) mod pipeline;
pub(crate) mod pipeline_cache;
pub(crate) mod sampler_manager;
pub(crate) mod shader;
pub(crate) mod surface;
pub(crate) mod swapchain;
pub(crate) mod window;

pub enum PhysicalDeviceUse {
    General,
}

pub enum QueueFamilyUse {
    General,
    Graphics,
    Compute,
    Transfer,
    SparseBinding,
    VideoDecode,
    VideoEncode,
}

/// pretty prints vulkan errors/warnings with backtrace and logs info/verbose/general messages
#[cfg(debug_assertions)]
pub(crate) unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    use crate::util::print::backtrace_callers;

    let callback_data = unsafe { *p_callback_data };
    let msg_id = callback_data.message_id_number;
    if message_severity == vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
        || (message_severity == vk::DebugUtilsMessageSeverityFlagsEXT::INFO
            && message_type == vk::DebugUtilsMessageTypeFlagsEXT::GENERAL)
        || msg_id == 601872502  // validation active warn
        || msg_id == 615892639 // GPU assisted validation active warn
        || msg_id == 2132353751 // GPU assisted + core validation active warn
        || msg_id == 1734198062 // pipeline exec props ext active warn
        // not using combined image samplers warn (no wgsl support)
        || msg_id == -222910232
    {
        return vk::FALSE;
    }
    let mut message = unsafe { callback_data.message_as_c_str() }
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let full_message = message.clone();
    if let Some(i) = message.find(" (http") {
        message.truncate(i);
    }
    let match_str = format!("MessageID = 0x{:x} | ", msg_id);
    if let Some(i) = message.find(&match_str) {
        message = message[i + match_str.len()..].to_string();
    }
    message = message
        .replace("The Vulkan spec states: ", "")
        .replace("VK_", "")
        .replace("the ", "");

    type Severity = vk::DebugUtilsMessageSeverityFlagsEXT;

    let mut backtrace = backtrace_callers();
    backtrace.pop();
    let backtrace = backtrace.join(" > ");
    crate::log_sink!(
        "rotating_file",
        crate::util::print::Level::Trace,
        "{full_message}\n|> {backtrace}\n"
    );

    use std::sync::atomic::{AtomicU32, Ordering};
    static ERROR_COUNT: AtomicU32 = AtomicU32::new(0);
    match message_severity {
        Severity::ERROR => {
            crate::err!("{message}");
            let err_cnt = ERROR_COUNT.fetch_add(1, Ordering::SeqCst);
            if err_cnt > 8 {
                crate::fatal!("too many vulkan errors");
            }
        }
        Severity::WARNING => {
            crate::warn!("{message}");
            ERROR_COUNT.store(0, Ordering::SeqCst);
        }
        _ => ERROR_COUNT.store(0, Ordering::SeqCst),
    }

    vk::FALSE
}

pub struct VulkanConfig {
    pub required_instance_extensions: Vec<&'static CStr>,
    pub preferred_instance_extensions: Vec<&'static CStr>,
    pub validation_layers: Vec<&'static CStr>,
    pub api_version: u32,
    pub app_name: &'static CStr,
    pub app_version: u32,
    pub engine_name: &'static CStr,
    pub engine_version: u32,
    pub alloc_handler: Box<dyn AllocHandler>,
}

impl Default for VulkanConfig {
    fn default() -> Self {
        Self {
            required_instance_extensions: vec![
                khr::surface::NAME,
                khr::get_surface_capabilities2::NAME,
                #[cfg(target_os = "windows")]
                khr::win32_surface::NAME,
                #[cfg(target_os = "linux")]
                khr::wayland_surface::NAME,
                #[cfg(target_os = "macos")]
                ash::mvk::macos_surface::NAME,
            ],
            preferred_instance_extensions: vec![
                #[cfg(debug_assertions)]
                ash::ext::debug_utils::NAME,
                // ash::ext::swapchain_colorspace::NAME,
            ],
            validation_layers: vec![
                #[cfg(debug_assertions)]
                c"VK_LAYER_KHRONOS_validation",
            ],
            api_version: ash::vk::make_api_version(0, 1, 3, 0),
            app_name: c"App",
            app_version: ash::vk::make_api_version(0, 0, 0, 0),
            engine_name: c"Engine",
            engine_version: ash::vk::make_api_version(0, 0, 0, 0),
            alloc_handler: Box::new(NoOpAllocHandler),
        }
    }
}

pub struct Vulkan {
    version: u32,
    entry: ash::Entry,
    instance: ash::Instance,
    alloc_manager: Arc<Mutex<AllocManager>>,
    allocation_callbacks: Option<vk::AllocationCallbacks<'static>>,
    physical_devices: Option<Vec<Arc<PhysicalDevice>>>,
    surface_instance: ash::khr::surface::Instance,
    get_surface_capabilities2_instance: ash::khr::get_surface_capabilities2::Instance,
}

impl Vulkan {
    pub fn new(config: VulkanConfig) -> ResultAny<Arc<Self>> {
        let version = config.api_version;

        let entry = unsafe { ash::Entry::load()? };
        let app_info = vk::ApplicationInfo::default()
            .application_name(config.app_name)
            .application_version(config.app_version)
            .engine_name(config.engine_name)
            .engine_version(config.engine_version)
            .api_version(config.api_version);

        let instance_ext_cache_path_str =
            format!("res/cache/vulkan/instance_extensions-{}.cache", crate::OS);
        let instance_ext_cache_path = Path::new(&instance_ext_cache_path_str);

        let available_instance_extensions: Vec<CString> = if instance_ext_cache_path.exists() {
            let content = std::fs::read_to_string(instance_ext_cache_path).unwrap();
            content
                .lines()
                .map(|line| CString::new(line).unwrap())
                .collect()
        } else {
            let instance_extensions: Vec<CString> =
                unsafe { entry.enumerate_instance_extension_properties(None) }
                    .unwrap()
                    .into_iter()
                    .map(|e| e.extension_name_as_c_str().unwrap().to_owned())
                    .collect(); // 10ms

            let instance_ext_cache: String = instance_extensions
                .iter()
                .map(|e| e.to_string_lossy())
                .collect::<Vec<_>>()
                .join("\n");
            std::fs::write(instance_ext_cache_path, instance_ext_cache)?;
            instance_extensions
        };

        for &ext in &config.required_instance_extensions {
            if !available_instance_extensions
                .iter()
                .any(|e| e.as_c_str() == ext)
            {
                fatal!("Unsupported vulkan instance extension: {ext:?}");
            }
        }

        let supported_preferred_extensions =
            config.preferred_instance_extensions.iter().filter(|&ext| {
                if available_instance_extensions
                    .iter()
                    .any(|e| e.as_c_str() == *ext)
                {
                    true
                } else {
                    warn!(
                        "Unsupported preferred vulkan instance exstension: {}",
                        ext.to_str().unwrap_or_default()
                    );
                    false
                }
            });

        let enabled_instance_extensions: Vec<*const i8> = config
            .required_instance_extensions
            .iter()
            .chain(supported_preferred_extensions)
            .map(|&ext| ext.as_ptr())
            .collect();

        let available_validation_layers: Vec<CString> = unsafe {
            entry
                .enumerate_instance_layer_properties()
                .unwrap_or_default()
                .into_iter()
                .map(|e| e.layer_name_as_c_str().unwrap().to_owned())
                .collect()
        };

        let enabled_validation_layers: Vec<*const i8> = config
            .validation_layers
            .into_iter()
            .filter(|&layer| {
                available_validation_layers
                    .iter()
                    .any(|l| layer == l.as_c_str())
            })
            .map(|l| l.as_ptr())
            .collect();

        let instance_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&enabled_instance_extensions)
            .enabled_layer_names(&enabled_validation_layers);

        let alloc_manager = Arc::new(Mutex::new(AllocManager::new(config.alloc_handler)));
        let allocation_callbacks = AllocManager::allocation_callbacks(Arc::clone(&alloc_manager));

        let instance =
            unsafe { entry.create_instance(&instance_info, allocation_callbacks.as_ref()) }?; // 12ms

        #[cfg(debug_assertions)]
        unsafe {
            ash::ext::debug_utils::Instance::new(&entry, &instance).create_debug_utils_messenger(
                &vk::DebugUtilsMessengerCreateInfoEXT::default()
                    .message_severity(
                        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                            | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                            | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                            | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
                    )
                    .message_type(
                        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                            | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                            | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
                    )
                    .pfn_user_callback(Some(crate::vulkan::debug_callback)),
                allocation_callbacks.as_ref(),
            )?;
        }

        let surface_instance = ash::khr::surface::Instance::new(&entry, &instance);

        let get_surface_capabilities2_instance =
            ash::khr::get_surface_capabilities2::Instance::new(&entry, &instance);

        Ok(Arc::new(Self {
            version,
            entry,
            instance,
            alloc_manager,
            allocation_callbacks,
            physical_devices: None,
            surface_instance,
            get_surface_capabilities2_instance,
        }))
    }

    pub(crate) fn best_physical_device<F>(
        self: &Arc<Self>,
        fitness: F,
    ) -> Option<Arc<PhysicalDevice>>
    where
        F: Fn(&PhysicalDevice) -> Option<u32>,
    {
        self.physical_devices()
            .ok()?
            .iter()
            .filter_map(|physical_device| {
                fitness(physical_device).map(|score| (physical_device, score))
            })
            .max_by(|(_, score_a), (_, score_b)| {
                score_a
                    .partial_cmp(score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(physical_device, _)| Arc::clone(physical_device))
    }

    pub(crate) fn best_physical_device_for(
        self: &Arc<Self>,
        usecase: PhysicalDeviceUse,
    ) -> Option<Arc<PhysicalDevice>> {
        self.best_physical_device(match usecase {
            PhysicalDeviceUse::General => |physical_device: &PhysicalDevice| {
                let mut score = 0;
                score += (physical_device.properties.device_type
                    == vk::PhysicalDeviceType::DISCRETE_GPU) as u32
                    * 1_000_000;
                let limits = &physical_device.properties.limits;
                score += limits.max_image_dimension2_d;
                score += limits.max_uniform_buffer_range / 64;
                score += limits.max_push_constants_size / 4;
                score += limits.max_compute_shared_memory_size / 16;
                score += limits.max_compute_work_group_invocations;
                Some(score)
            },
        })
    }

    pub fn best_queue_family<F>(
        &self,
        queue_family_properties: &[vk::QueueFamilyProperties],
        fitness: F,
    ) -> Option<u32>
    where
        F: Fn(&vk::QueueFamilyProperties) -> Option<u32>,
    {
        queue_family_properties
            .iter()
            .enumerate()
            .filter_map(|(idx, qfp)| fitness(qfp).map(|score| (idx, score)))
            .max_by(|(_, score_a), (_, score_b)| {
                score_a
                    .partial_cmp(score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(idx, _)| idx as u32)
    }

    pub fn best_queue_family_for(
        &self,
        queue_family_properties: &[vk::QueueFamilyProperties],
        usecase: QueueFamilyUse,
    ) -> Option<u32> {
        self.best_queue_family(queue_family_properties, |props| {
            let supports = |queue_flags: vk::QueueFlags| {
                props
                    .queue_flags
                    .contains(queue_flags)
                    .then_some(props.queue_count)
            };

            match usecase {
                QueueFamilyUse::General => supports(
                    vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER,
                ),
                QueueFamilyUse::Graphics => supports(vk::QueueFlags::GRAPHICS),
                QueueFamilyUse::Compute => supports(vk::QueueFlags::COMPUTE),
                QueueFamilyUse::Transfer => supports(vk::QueueFlags::TRANSFER),
                QueueFamilyUse::SparseBinding => supports(vk::QueueFlags::SPARSE_BINDING),
                QueueFamilyUse::VideoDecode => supports(vk::QueueFlags::VIDEO_DECODE_KHR),
                QueueFamilyUse::VideoEncode => supports(vk::QueueFlags::VIDEO_ENCODE_KHR),
            }
        })
    }

    pub(crate) fn physical_devices(self: &Arc<Self>) -> ResultAny<Vec<Arc<PhysicalDevice>>> {
        if let Some(physical_devices) = &self.physical_devices {
            Ok(physical_devices.clone())
        } else {
            let physical_devices = unsafe { self.instance.enumerate_physical_devices() }?;
            Ok(physical_devices
                .iter()
                .map(|&physical_device| {
                    let mut properties = vk::PhysicalDeviceProperties2::default();
                    unsafe {
                        self.instance
                            .get_physical_device_properties2(physical_device, &mut properties)
                    };
                    let properties = properties.properties;

                    let mut features = vk::PhysicalDeviceFeatures2::default();
                    unsafe {
                        self.instance
                            .get_physical_device_features2(physical_device, &mut features)
                    };
                    let features = features.features;

                    let extensions: Vec<CString> = unsafe {
                        self.instance
                            .enumerate_device_extension_properties(physical_device)
                    }
                    .unwrap_or_default()
                    .into_iter()
                    .map(|ext| ext.extension_name_as_c_str().unwrap_or_default().to_owned())
                    .collect();

                    let mut memory_properties = vk::PhysicalDeviceMemoryProperties2::default();
                    unsafe {
                        self.instance.get_physical_device_memory_properties2(
                            physical_device,
                            &mut memory_properties,
                        )
                    };
                    let memory_properties = memory_properties.memory_properties;

                    let queue_family_properties_len = unsafe {
                        self.instance
                            .get_physical_device_queue_family_properties2_len(physical_device)
                    };
                    let mut video_props = vk::QueueFamilyVideoPropertiesKHR::default();
                    let default_family_props =
                        vk::QueueFamilyProperties2::default().push_next(&mut video_props);
                    let mut queue_family_properties =
                        vec![default_family_props; queue_family_properties_len];
                    unsafe {
                        self.instance.get_physical_device_queue_family_properties2(
                            physical_device,
                            &mut queue_family_properties,
                        )
                    };
                    let queue_family_properties: Vec<vk::QueueFamilyProperties> =
                        queue_family_properties
                            .iter()
                            .map(|qfp2| qfp2.queue_family_properties)
                            .collect();

                    Arc::new(PhysicalDevice {
                        physical_device,
                        properties,
                        features,
                        extensions,
                        memory_properties,
                        queue_family_properties,
                        vulkan: Arc::clone(self),
                    })
                })
                .collect())
        }
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub(crate) fn instance(&self) -> &ash::Instance {
        &self.instance
    }

    pub(crate) fn entry(&self) -> &ash::Entry {
        &self.entry
    }

    pub(crate) fn allocation_callbacks(&self) -> Option<&vk::AllocationCallbacks<'static>> {
        self.allocation_callbacks.as_ref()
    }

    pub(crate) fn surface_instance(&self) -> &ash::khr::surface::Instance {
        &self.surface_instance
    }

    pub(crate) fn get_surface_capabilities2_instance(
        &self,
    ) -> &ash::khr::get_surface_capabilities2::Instance {
        &self.get_surface_capabilities2_instance
    }
}

pub struct MemProp;
impl MemProp {
    pub const GPU: vk::MemoryPropertyFlags = vk::MemoryPropertyFlags::DEVICE_LOCAL;
    pub const CPU_GPU: vk::MemoryPropertyFlags = vk::MemoryPropertyFlags::from_raw(
        vk::MemoryPropertyFlags::DEVICE_LOCAL.as_raw()
            | vk::MemoryPropertyFlags::HOST_VISIBLE.as_raw()
            | vk::MemoryPropertyFlags::HOST_COHERENT.as_raw(),
    );
    pub const CPU: vk::MemoryPropertyFlags = vk::MemoryPropertyFlags::from_raw(
        vk::MemoryPropertyFlags::HOST_VISIBLE.as_raw()
            | vk::MemoryPropertyFlags::HOST_COHERENT.as_raw(),
    );
    pub const CPU_CACHED: vk::MemoryPropertyFlags = vk::MemoryPropertyFlags::from_raw(
        vk::MemoryPropertyFlags::HOST_VISIBLE.as_raw()
            | vk::MemoryPropertyFlags::HOST_COHERENT.as_raw()
            | vk::MemoryPropertyFlags::HOST_CACHED.as_raw(),
    );
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
