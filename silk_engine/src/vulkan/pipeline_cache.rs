use std::sync::Arc;

use ash::vk::{self, Handle};

use crate::{prelude::ResultAny, vulkan::device::Device};

pub(crate) struct PipelineCache {
    handle: vk::PipelineCache,
    device: Arc<Device>,
    path: String,
}

impl PipelineCache {
    pub(crate) fn new(device: &Arc<Device>, path: &str) -> ResultAny<Self> {
        let data = std::fs::read(path).unwrap_or_default();
        let pipeline_cache_info = vk::PipelineCacheCreateInfo::default().initial_data(&data);
        let pipeline_cache = unsafe {
            device
                .device
                .create_pipeline_cache(&pipeline_cache_info, device.allocation_callbacks().as_ref())
        }?;
        Ok(Self {
            handle: pipeline_cache,
            device: Arc::clone(device),
            path: path.to_string(),
        })
    }

    pub(crate) fn save(&self) {
        let data = unsafe { self.device().device.get_pipeline_cache_data(self.handle) };
        match data {
            Ok(data) => std::fs::write(&self.path, data)
                .unwrap_or_else(|err| crate::warn!("failed to save pipeline cache: {err}")),
            Err(err) => crate::warn!("failed to get pipeline cache data: {err}"),
        }
    }

    pub(crate) fn device(&self) -> &Arc<Device> {
        &self.device
    }

    pub(crate) fn handle(&self) -> vk::PipelineCache {
        self.handle
    }
}

impl Drop for PipelineCache {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            self.save();
            unsafe {
                self.device.device.destroy_pipeline_cache(
                    self.handle,
                    self.device.allocation_callbacks().as_ref(),
                )
            };
        }
    }
}
