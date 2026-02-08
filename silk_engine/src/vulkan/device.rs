use std::collections::HashMap;
use std::ffi::CString;
use std::sync::{Arc, Mutex, OnceLock};

use ash::vk::{self, Handle};

use crate::vulkan::command_manager::CommandManager;
use crate::vulkan::ds_alloc::DSAlloc;
use crate::vulkan::dsl_manager::DSLManager;
use crate::vulkan::pipeline_cache::PipelineCache;
use crate::vulkan::sampler_manager::SamplerManager;
use crate::{
    prelude::ResultAny,
    vulkan::{PhysicalDevice, alloc::VulkanAlloc},
};

pub(crate) struct Device {
    pub(crate) device: ash::Device,
    physical_device: Arc<PhysicalDevice>,
    debug_utils: Option<ash::ext::debug_utils::Device>,
    swapchain_device: Option<ash::khr::swapchain::Device>,
    cached_queues: Mutex<Vec<Vec<vk::Queue>>>,
    sampler_manager: OnceLock<Arc<SamplerManager>>,
    dsl_manager: OnceLock<Arc<DSLManager>>,
    ds_alloc: OnceLock<Arc<DSAlloc>>,
    command_managers: Mutex<HashMap<u32, Arc<CommandManager>>>,
    allocator: OnceLock<Arc<VulkanAlloc>>,
    pipeline_cache: OnceLock<Arc<PipelineCache>>,
}

impl Device {
    pub(crate) fn new(
        physical_device: &Arc<PhysicalDevice>,
        create_info: &vk::DeviceCreateInfo,
    ) -> ResultAny<Arc<Self>> {
        let vulkan = physical_device.vulkan();

        let device = unsafe {
            vulkan.instance().create_device(
                physical_device.physical_device,
                create_info,
                vulkan.allocation_callbacks(),
            )
        }?;

        let cached_queues = physical_device
            .queue_family_properties
            .iter()
            .map(|queue_family_props| {
                vec![vk::Queue::null(); queue_family_props.queue_count as usize]
            })
            .collect();

        Ok(Arc::new(Self {
            device,
            physical_device: physical_device.clone(),
            debug_utils: None,
            swapchain_device: None,
            cached_queues: Mutex::new(cached_queues),
            sampler_manager: OnceLock::new(),
            dsl_manager: OnceLock::new(),
            ds_alloc: OnceLock::new(),
            command_managers: Mutex::new(HashMap::new()),
            allocator: OnceLock::new(),
            pipeline_cache: OnceLock::new(),
        }))
    }

    pub(crate) fn wait(&self) {
        unsafe { self.device.device_wait_idle().unwrap() }
    }

    pub(crate) fn get_queue(&self, family_index: u32, queue_index: u32) -> vk::Queue {
        let mut cache = self.cached_queues.lock().unwrap();
        let queue = cache[family_index as usize][queue_index as usize];
        if queue.is_null() {
            let queue = unsafe { self.device.get_device_queue(family_index, queue_index) };
            cache[family_index as usize][queue_index as usize] = queue;
            queue
        } else {
            queue
        }
    }

    pub(crate) fn get_sampler(
        self: &Arc<Self>,
        addr_mode_u: vk::SamplerAddressMode,
        addr_mode_v: vk::SamplerAddressMode,
        min_filter: vk::Filter,
        mag_filter: vk::Filter,
        mip_filter: vk::SamplerMipmapMode,
    ) -> vk::Sampler {
        let mgr = self
            .sampler_manager
            .get_or_init(|| SamplerManager::new(self));
        mgr.get(addr_mode_u, addr_mode_v, min_filter, mag_filter, mip_filter)
    }

    pub(crate) fn dsl_manager(self: &Arc<Self>) -> Arc<DSLManager> {
        self.dsl_manager
            .get_or_init(|| DSLManager::new(self))
            .clone()
    }

    pub(crate) fn alloc_ds(
        self: &Arc<Self>,
        dsls: &[vk::DescriptorSetLayout],
    ) -> Vec<vk::DescriptorSet> {
        let alloc = self.ds_alloc.get_or_init(|| DSAlloc::new(self));
        alloc.alloc(dsls)
    }

    pub(crate) fn allocator(self: &Arc<Self>) -> Arc<VulkanAlloc> {
        self.allocator
            .get_or_init(|| VulkanAlloc::new(self, &self.physical_device().memory_properties))
            .clone()
    }

    pub(crate) fn command_manager(
        self: &Arc<Self>,
        queue_family_index: u32,
    ) -> Arc<CommandManager> {
        let mut managers = self.command_managers.lock().unwrap();
        managers
            .entry(queue_family_index)
            .or_insert_with(|| CommandManager::new(self, queue_family_index).unwrap())
            .clone()
    }

    pub(crate) fn pipeline_cache(self: &Arc<Self>) -> Arc<PipelineCache> {
        self.pipeline_cache
            .get_or_init(|| Arc::new(PipelineCache::new(self, "res/cache/pipeline.cache").unwrap()))
            .clone()
    }

    pub(crate) fn ds_layout(
        self: &Arc<Self>,
        bindings: &[vk::DescriptorSetLayoutBinding<'static>],
    ) -> ResultAny<vk::DescriptorSetLayout> {
        self.dsl_manager().get(bindings)
    }

    pub(crate) fn debug_name<T: vk::Handle>(&self, handle: T, name: &str) {
        if !cfg!(debug_assertions) {
            return;
        }
        let Ok(name) = CString::new(name) else {
            return;
        };
        unsafe {
            self.debug_utils()
                .set_debug_utils_object_name(
                    &vk::DebugUtilsObjectNameInfoEXT::default()
                        .object_handle(handle)
                        .object_name(name.as_c_str()),
                )
                .unwrap_or_default()
        }
    }

    pub(crate) fn debug_utils(&self) -> ash::ext::debug_utils::Device {
        self.debug_utils.clone().unwrap_or_else(|| {
            ash::ext::debug_utils::Device::new(
                &self.physical_device().vulkan().instance,
                &self.device,
            )
        })
    }

    pub(crate) fn swapchain_device(&self) -> ash::khr::swapchain::Device {
        self.swapchain_device.clone().unwrap_or_else(|| {
            ash::khr::swapchain::Device::new(
                &self.physical_device().vulkan().instance,
                &self.device,
            )
        })
    }

    pub(crate) fn physical_device(&self) -> &Arc<PhysicalDevice> {
        &self.physical_device
    }

    pub(crate) fn allocation_callbacks(&self) -> Option<vk::AllocationCallbacks<'static>> {
        self.physical_device().vulkan().allocation_callbacks
    }
}
