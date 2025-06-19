use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
};

use ash::vk;

use super::{alloc_callbacks, gpu};

#[derive(Default)]
pub struct SamplerManager {
    samplers: HashMap<SamplerInfo, vk::Sampler>,
}

impl SamplerManager {
    pub fn get(
        &mut self,
        addr_mode_u: vk::SamplerAddressMode,
        addr_mode_v: vk::SamplerAddressMode,
        min_filter: vk::Filter,
        mag_filter: vk::Filter,
        mip_filter: vk::SamplerMipmapMode,
    ) -> vk::Sampler {
        *self
            .samplers
            .entry(SamplerInfo {
                addr_mode_u,
                addr_mode_v,
                min_filter,
                mag_filter,
                mip_filter,
            })
            .or_insert(unsafe {
                gpu()
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
                        alloc_callbacks(),
                    )
                    .unwrap()
            })
    }
}

impl Drop for SamplerManager {
    fn drop(&mut self) {
        for &sampler in self.samplers.values() {
            unsafe {
                gpu().destroy_sampler(sampler, alloc_callbacks());
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
