use std::hash::{Hash, Hasher};

use crate::*;

#[derive(Clone)]
pub struct DSLBinding {
    pub binding: u32,
    pub descriptor_type: vk::DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: vk::ShaderStageFlags,
}

impl<'a> From<&DSLBinding> for vk::DescriptorSetLayoutBinding<'a> {
    fn from(value: &DSLBinding) -> Self {
        vk::DescriptorSetLayoutBinding {
            binding: value.binding,
            descriptor_type: value.descriptor_type,
            descriptor_count: value.descriptor_count,
            stage_flags: value.stage_flags,
            p_immutable_samplers: null(),
            _marker: std::marker::PhantomData,
        }
    }
}

#[derive(Default)]
pub struct DSLManager {
    dsls: HashMap<DSLBindings, vk::DescriptorSetLayout>,
}

impl DSLManager {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get(&mut self, bindings: &[DSLBinding]) -> vk::DescriptorSetLayout {
        let dslbs = bindings.iter().map(|b| b.into()).collect::<Vec<_>>();
        let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(&dslbs);
        let dslbs = DSLBindings(bindings.to_vec());
        *self
            .dsls
            .entry(dslbs)
            .or_insert(unsafe { DEVICE.create_descriptor_set_layout(&info, None).unwrap() })
    }
}

impl Drop for DSLManager {
    fn drop(&mut self) {
        for dsl in self.dsls.values() {
            unsafe { DEVICE.destroy_descriptor_set_layout(*dsl, None) };
        }
    }
}

struct DSLBindings(Vec<DSLBinding>);

impl Hash for DSLBindings {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut combined_hash = 0;
        for dslb in self.0.iter() {
            combined_hash ^= dslb.binding as u64
                | (dslb.descriptor_type.as_raw() as u64) << 6
                | (dslb.descriptor_count as u64) << 12
                | (dslb.stage_flags.as_raw() as u64) << 18;
        }
        state.write_u64(combined_hash);
    }
}

impl PartialEq for DSLBindings {
    fn eq(&self, other: &Self) -> bool {
        for (a, b) in self.0.iter().zip(other.0.iter()) {
            if !(a.binding == b.binding
                && a.descriptor_type == b.descriptor_type
                && a.descriptor_count == b.descriptor_count
                && a.stage_flags == b.stage_flags)
            {
                return false;
            }
        }
        true
    }
}

impl Eq for DSLBindings {}
