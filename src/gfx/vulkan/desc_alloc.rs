use super::DEVICE;
use ash::vk;

// TODO: manage DSLs and descriptor pools, free them together
// TODO: track descriptor allocs and create new pools based on that
// TODO: allocate descriptor sets together
// For now it just uses single large descriptor pool
#[derive(Default)]
pub struct DescAlloc {
    pool: vk::DescriptorPool,
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
                DEVICE
                    .create_descriptor_pool(
                        &vk::DescriptorPoolCreateInfo::default()
                            .max_sets(MAX_SETS)
                            .pool_sizes(&POOL_SIZES),
                        None,
                    )
                    .unwrap()
            },
        }
    }

    pub fn alloc(&self, dsls: &[vk::DescriptorSetLayout]) -> Vec<vk::DescriptorSet> {
        let alloc_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.pool)
            .set_layouts(dsls);
        let desc = unsafe { DEVICE.allocate_descriptor_sets(&alloc_info) };
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

    pub fn alloc_single(&self, dsl: vk::DescriptorSetLayout) -> vk::DescriptorSet {
        self.alloc(&[dsl])[0]
    }
}
