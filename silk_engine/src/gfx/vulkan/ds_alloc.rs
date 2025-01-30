use crate::gfx::alloc_callbacks;

use super::gpu;
use ash::vk::{self, Handle};

// TODO: manage DSLs and desc pools, free them together
// TODO: track desc allocs and create new pools based on that
// TODO: allocate desc sets together
// For now it just uses single large desc pool
pub struct DescAlloc {
    pool: vk::DescriptorPool,
}

impl Default for DescAlloc {
    fn default() -> Self {
        Self::new()
    }
}

macro_rules! dps {
    ($dty:ident, $cnt:literal) => {
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::$dty,
            descriptor_count: $cnt,
        }
    };
}

impl DescAlloc {
    pub fn new() -> Self {
        const MAX_SETS: u32 = 16;
        const POOL_SIZES: [vk::DescriptorPoolSize; 2] =
            [dps!(UNIFORM_BUFFER, 32), dps!(STORAGE_BUFFER, 16)];
        Self {
            pool: unsafe {
                gpu()
                    .create_descriptor_pool(
                        &vk::DescriptorPoolCreateInfo::default()
                            .max_sets(MAX_SETS)
                            .pool_sizes(&POOL_SIZES),
                        alloc_callbacks(),
                    )
                    .unwrap()
            },
        }
    }

    pub fn alloc(&self, dsls: &[vk::DescriptorSetLayout]) -> Vec<vk::DescriptorSet> {
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.pool)
            .set_layouts(dsls);
        let desc = unsafe { gpu().allocate_descriptor_sets(&alloc_info) };
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

    pub fn alloc_one(&self, dsl: vk::DescriptorSetLayout) -> vk::DescriptorSet {
        self.alloc(&[dsl])[0]
    }
}

impl Drop for DescAlloc {
    fn drop(&mut self) {
        if !self.pool.is_null() {
            unsafe { gpu().destroy_descriptor_pool(self.pool, alloc_callbacks()) }
        }
    }
}
