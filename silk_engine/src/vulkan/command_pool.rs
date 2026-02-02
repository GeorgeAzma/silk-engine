use std::sync::Arc;

use ash::vk;

use crate::{prelude::ResultAny, vulkan::device::Device};

pub(crate) struct CommandPool {
    pub(crate) command_pool: vk::CommandPool,
    device: Arc<Device>,
}

impl CommandPool {
    pub(crate) fn new(
        device: &Arc<Device>,
        queue_family_index: u32,
        flags: vk::CommandPoolCreateFlags,
    ) -> ResultAny<Arc<Self>> {
        let command_pool = unsafe {
            device.device.create_command_pool(
                &vk::CommandPoolCreateInfo::default()
                    .flags(flags)
                    .queue_family_index(queue_family_index),
                device.allocation_callbacks().as_ref(),
            )
        }?;

        Ok(Arc::new(Self {
            command_pool,
            device: Arc::clone(device),
        }))
    }

    pub(crate) fn alloc(
        &self,
        count: u32,
        level: vk::CommandBufferLevel,
    ) -> ResultAny<Vec<vk::CommandBuffer>> {
        Ok(unsafe {
            self.device().device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(self.command_pool)
                    .command_buffer_count(count)
                    .level(level),
            )
        }?)
    }

    pub(crate) fn free(&self, command_buffers: &[vk::CommandBuffer]) {
        unsafe {
            self.device()
                .device
                .free_command_buffers(self.command_pool, command_buffers)
        }
    }

    pub fn reset(&self) {
        unsafe {
            self.device()
                .device
                .reset_command_pool(self.command_pool, vk::CommandPoolResetFlags::empty())
                .unwrap()
        }
    }

    pub(crate) fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_command_pool(
                self.command_pool,
                self.device.allocation_callbacks().as_ref(),
            )
        }
    }
}
