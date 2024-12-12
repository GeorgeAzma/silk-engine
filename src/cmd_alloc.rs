use crate::*;

pub struct CmdAlloc {
    pool: vk::CommandPool,
}

impl CmdAlloc {
    pub fn new() -> Self {
        let pool_create_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(*QUEUE_FAMILY_INDEX);
        Self {
            pool: unsafe { DEVICE.create_command_pool(&pool_create_info, None).unwrap() },
        }
    }

    pub fn alloc(&self) -> vk::CommandBuffer {
        let command_buffer_alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(self.pool);
        let command_buffers = unsafe {
            DEVICE
                .allocate_command_buffers(&command_buffer_alloc_info)
                .unwrap()
        };
        command_buffers[0]
    }
}
