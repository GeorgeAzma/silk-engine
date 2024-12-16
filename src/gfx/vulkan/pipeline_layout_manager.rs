use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

use ash::vk;
use vk::Handle;

use super::DEVICE;

#[derive(Default)]
pub struct PipelineLayoutManager {
    pipeline_layouts: HashMap<PipelineLayoutInfo, vk::PipelineLayout>,
}

impl PipelineLayoutManager {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get(&mut self, set_layouts: &[vk::DescriptorSetLayout]) -> vk::PipelineLayout {
        *self
            .pipeline_layouts
            .entry(PipelineLayoutInfo {
                set_layouts: set_layouts.to_vec(),
            })
            .or_insert(unsafe {
                DEVICE
                    .create_pipeline_layout(
                        &vk::PipelineLayoutCreateInfo::default().set_layouts(set_layouts),
                        None,
                    )
                    .unwrap()
            })
    }
}

struct PipelineLayoutInfo {
    pub set_layouts: Vec<vk::DescriptorSetLayout>,
}

impl Hash for PipelineLayoutInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut combined_hash = 0;
        for dsl in self.set_layouts.iter().cloned() {
            combined_hash ^= dsl.as_raw();
        }
        state.write_u64(combined_hash);
    }
}

impl PartialEq for PipelineLayoutInfo {
    fn eq(&self, other: &Self) -> bool {
        let mut dsl_a = self.set_layouts.clone();
        let mut dsl_b = other.set_layouts.clone();
        dsl_a.sort_by_key(|dsl| dsl.as_raw());
        dsl_b.sort_by_key(|dsl| dsl.as_raw());
        dsl_a == dsl_b
    }
}

impl Eq for PipelineLayoutInfo {}
