use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    sync::{Arc, Mutex},
};

use ash::vk;

use crate::vulkan::device::Device;

pub struct SamplerManager {
    samplers: Mutex<HashMap<SamplerInfo, vk::Sampler>>,
    device: Arc<Device>,
}

impl SamplerManager {
    pub fn new(device: &Arc<Device>) -> Arc<Self> {
        Arc::new(Self {
            samplers: Mutex::new(HashMap::new()),
            device: device.clone(),
        })
    }

    pub fn get(
        &self,
        addr_mode_u: vk::SamplerAddressMode,
        addr_mode_v: vk::SamplerAddressMode,
        min_filter: vk::Filter,
        mag_filter: vk::Filter,
        mip_filter: vk::SamplerMipmapMode,
    ) -> vk::Sampler {
        let device = self.device().clone();
        *self
            .samplers
            .lock()
            .unwrap()
            .entry(SamplerInfo {
                addr_mode_u,
                addr_mode_v,
                min_filter,
                mag_filter,
                mip_filter,
            })
            .or_insert_with(|| unsafe {
                device
                    .create_sampler(
                        &vk::SamplerCreateInfo::default()
                            .address_mode_u(addr_mode_u)
                            .address_mode_v(addr_mode_v)
                            .address_mode_w(vk::SamplerAddressMode::REPEAT)
                            .min_filter(min_filter)
                            .mag_filter(mag_filter)
                            .mipmap_mode(mip_filter)
                            .max_anisotropy(16.0)
                            .border_color(vk::BorderColor::FLOAT_TRANSPARENT_BLACK)
                            .compare_enable(false)
                            .compare_op(vk::CompareOp::ALWAYS)
                            .mip_lod_bias(0.0)
                            .min_lod(0.0)
                            .max_lod(1.0)
                            .unnormalized_coordinates(false),
                        self.device.allocation_callbacks().as_ref(),
                    )
                    .unwrap()
            })
    }

    pub fn device(&self) -> &ash::Device {
        &self.device.device
    }
}

impl Drop for SamplerManager {
    fn drop(&mut self) {
        for &sampler in self.samplers.lock().unwrap().values() {
            unsafe {
                self.device()
                    .destroy_sampler(sampler, self.device.allocation_callbacks().as_ref());
            }
        }
    }
}

#[derive(PartialEq, Eq)]
pub struct SamplerInfo {
    addr_mode_u: vk::SamplerAddressMode,
    addr_mode_v: vk::SamplerAddressMode,
    min_filter: vk::Filter,
    mag_filter: vk::Filter,
    mip_filter: vk::SamplerMipmapMode,
}

impl Hash for SamplerInfo {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let mut hash = 0;
        hash ^= self.addr_mode_u.as_raw();
        hash ^= self.addr_mode_v.as_raw() << 2;
        hash ^= self.min_filter.as_raw() << 4;
        hash ^= self.mag_filter.as_raw() << 5;
        hash ^= self.mip_filter.as_raw() << 6;
        state.write_i32(hash);
    }
}
