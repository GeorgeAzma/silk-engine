use ash::vk::{self, Handle};
use std::collections::HashMap;
use std::hash::Hash;
use std::ptr::null;
use std::sync::{Arc, Mutex};

use crate::prelude::ResultAny;
use crate::vulkan::device::Device;

#[derive(Clone, Debug, Hash)]
pub(crate) struct DSLBinding {
    pub binding: u32,
    pub desc_ty: vk::DescriptorType,
    pub descriptor_count: u32,
    pub stage_flags: vk::ShaderStageFlags,
}

impl From<&DSLBinding> for vk::DescriptorSetLayoutBinding<'_> {
    fn from(value: &DSLBinding) -> Self {
        vk::DescriptorSetLayoutBinding {
            binding: value.binding,
            descriptor_type: value.desc_ty,
            descriptor_count: value.descriptor_count,
            stage_flags: value.stage_flags,
            p_immutable_samplers: null(),
            _marker: std::marker::PhantomData,
        }
    }
}

impl From<&vk::DescriptorSetLayoutBinding<'_>> for DSLBinding {
    fn from(value: &vk::DescriptorSetLayoutBinding) -> Self {
        DSLBinding {
            binding: value.binding,
            desc_ty: value.descriptor_type,
            descriptor_count: value.descriptor_count,
            stage_flags: value.stage_flags,
        }
    }
}

pub(crate) struct DSLManager {
    dsls: Mutex<HashMap<DSLBindings, vk::DescriptorSetLayout>>,
    pub(crate) device: Arc<Device>,
}

impl DSLManager {
    pub(crate) fn new(device: &Arc<Device>) -> Arc<Self> {
        Arc::new(Self {
            dsls: Mutex::new(HashMap::new()),
            device: Arc::clone(device),
        })
    }

    pub(crate) fn get(
        &self,
        bindings: &[vk::DescriptorSetLayoutBinding<'static>],
    ) -> ResultAny<vk::DescriptorSetLayout> {
        let info = vk::DescriptorSetLayoutCreateInfo::default().bindings(bindings);
        // TODO: think about descriptor indexing
        // let binding_flags = vk::DescriptorSetLayoutBindingFlagsCreateInfo::default().binding_flags(&[vk::DescriptorBindingFlags::PARTIALLY_BOUND | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT])
        // info.push_next(&mut binding_flags)
        // vk::DescriptorSetVariableDescriptorCountAllocateInfo::default().descriptor_counts(&[]);
        let dslbs = DSLBindings(bindings.iter().map(|binding| binding.into()).collect());
        Ok(*self.dsls.lock().unwrap().entry(dslbs).or_insert(unsafe {
            self.device()
                .create_descriptor_set_layout(&info, self.device.allocation_callbacks().as_ref())?
        }))
    }

    pub(crate) fn device(&self) -> &ash::Device {
        &self.device.device
    }
}

impl Drop for DSLManager {
    fn drop(&mut self) {
        for &dsl in self.dsls.lock().unwrap().values() {
            if !dsl.is_null() {
                unsafe {
                    self.device.device.destroy_descriptor_set_layout(
                        dsl,
                        self.device.allocation_callbacks().as_ref(),
                    )
                };
            }
        }
    }
}

#[derive(Hash)]
struct DSLBindings(Vec<DSLBinding>);

impl PartialEq for DSLBindings {
    fn eq(&self, other: &Self) -> bool {
        self.0.len() == other.0.len() && self.0.iter().zip(other.0.iter()).all(|(a, b)| {
            a.binding == b.binding
                && a.desc_ty == b.desc_ty
                && a.descriptor_count == b.descriptor_count
                && a.stage_flags == b.stage_flags
        })
    }
}

impl Eq for DSLBindings {}
