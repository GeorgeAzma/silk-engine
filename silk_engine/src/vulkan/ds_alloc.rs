use std::sync::Arc;

use ash::vk::{self, Handle};

use crate::vulkan::device::Device;

pub struct DSAlloc {
    pool: vk::DescriptorPool,
    device: Arc<Device>,
}

macro_rules! dps {
    ($dty:ident, $cnt:literal) => {
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::$dty,
            descriptor_count: $cnt,
        }
    };
}

impl DSAlloc {
    pub fn new(device: &Arc<Device>) -> Arc<Self> {
        const MAX_SETS: u32 = 16;
        const POOL_SIZES: [vk::DescriptorPoolSize; 6] = [
            dps!(UNIFORM_BUFFER, 32),
            dps!(STORAGE_BUFFER, 16),
            dps!(SAMPLED_IMAGE, 16),
            dps!(COMBINED_IMAGE_SAMPLER, 16),
            dps!(SAMPLER, 8),
            dps!(STORAGE_IMAGE, 8),
        ];
        Arc::new(Self {
            pool: unsafe {
                device
                    .device
                    .create_descriptor_pool(
                        &vk::DescriptorPoolCreateInfo::default()
                            .max_sets(MAX_SETS)
                            .pool_sizes(&POOL_SIZES),
                        device.allocation_callbacks().as_ref(),
                    )
                    .unwrap()
            },
            device: device.clone(),
        })
    }

    pub fn alloc(&self, dsls: &[vk::DescriptorSetLayout]) -> Vec<vk::DescriptorSet> {
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.pool)
            .set_layouts(dsls);
        let desc = unsafe { self.device().allocate_descriptor_sets(&alloc_info) };
        match desc {
            Ok(descs) => descs,
            Err(e) => match e {
                vk::Result::ERROR_OUT_OF_POOL_MEMORY | vk::Result::ERROR_FRAGMENTED_POOL => {
                    panic!("{e}")
                }
                _ => panic!("error while allocating descriptor set"),
            },
        }
    }

    pub fn device(&self) -> &ash::Device {
        &self.device.device
    }
}

impl Drop for DSAlloc {
    fn drop(&mut self) {
        if !self.pool.is_null() {
            unsafe {
                self.device()
                    .destroy_descriptor_pool(self.pool, self.device.allocation_callbacks().as_ref())
            }
        }
    }
}
