use crate::*;

pub struct CmdAlloc {
    pool: vk::CommandPool,
}

impl CmdAlloc {
    pub fn new() -> Self {
        let pool_info = vk::CommandPoolCreateInfo::default()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(*QUEUE_FAMILY_INDEX);
        Self {
            pool: unsafe { DEVICE.create_command_pool(&pool_info, None).unwrap() },
        }
    }

    pub fn alloc(&self) -> vk::CommandBuffer {
        let cmd_alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(self.pool);
        let cmds = unsafe { DEVICE.allocate_command_buffers(&cmd_alloc_info).unwrap() };
        cmds[0]
    }

    pub fn dealloc(&self, cmd: vk::CommandBuffer) {
        unsafe { DEVICE.free_command_buffers(self.pool, &[cmd]) };
    }
}
