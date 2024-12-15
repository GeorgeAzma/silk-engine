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

    pub fn get(
        &mut self,
        set_layouts: &[vk::DescriptorSetLayout],
        push_constant_ranges: &[vk::PushConstantRange],
    ) -> vk::PipelineLayout {
        *self
            .pipeline_layouts
            .entry(PipelineLayoutInfo {
                set_layouts: set_layouts.to_vec(),
                push_constant_ranges: push_constant_ranges.to_vec(),
            })
            .or_insert(unsafe {
                DEVICE
                    .create_pipeline_layout(
                        &vk::PipelineLayoutCreateInfo::default()
                            .set_layouts(set_layouts)
                            .push_constant_ranges(push_constant_ranges),
                        None,
                    )
                    .unwrap()
            })
    }
}

struct PipelineLayoutInfo {
    pub set_layouts: Vec<vk::DescriptorSetLayout>,
    pub push_constant_ranges: Vec<vk::PushConstantRange>,
}

impl Hash for PipelineLayoutInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.set_layouts.hash(state);
        let mut combined_hash = 0;
        for dsl in self.set_layouts.iter().cloned() {
            combined_hash ^= dsl.as_raw();
        }
        for pcr in self.push_constant_ranges.iter() {
            combined_hash ^= (pcr.size as u64) << 32 | (pcr.offset as u64);
            combined_hash ^= (pcr.stage_flags.as_raw() as u64) << 16;
        }
        state.write_u64(combined_hash);
    }
}

impl PartialEq for PipelineLayoutInfo {
    fn eq(&self, other: &Self) -> bool {
        for (a, b) in self
            .push_constant_ranges
            .iter()
            .zip(other.push_constant_ranges.iter())
        {
            if a.size != b.size || a.offset != b.offset || a.stage_flags != b.stage_flags {
                return false;
            }
        }
        self.set_layouts == other.set_layouts
    }
}

impl Eq for PipelineLayoutInfo {}
