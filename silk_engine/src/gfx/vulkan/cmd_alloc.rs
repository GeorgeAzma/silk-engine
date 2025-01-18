use crate::debug_name;

use super::{QUEUE_FAMILY_INDEX, alloc_callbacks, gpu};
use ash::vk::{self, Handle};

pub struct CmdAlloc {
    pool: vk::CommandPool,
}

impl Default for CmdAlloc {
    fn default() -> Self {
        Self::new()
    }
}

impl CmdAlloc {
    pub fn new() -> Self {
        let pool_info =
            vk::CommandPoolCreateInfo::default().queue_family_index(*QUEUE_FAMILY_INDEX);
        let pool = unsafe {
            gpu()
                .create_command_pool(&pool_info, alloc_callbacks())
                .unwrap()
        };
        debug_name("cmd pool", pool);
        Self { pool }
    }

    pub fn alloc(&self, count: u32) -> Vec<vk::CommandBuffer> {
        let cmd_alloc_info = vk::CommandBufferAllocateInfo::default()
            .command_buffer_count(count)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(self.pool);
        unsafe { gpu().allocate_command_buffers(&cmd_alloc_info).unwrap() }
    }

    pub fn reset(&self) {
        unsafe {
            gpu()
                .reset_command_pool(self.pool, vk::CommandPoolResetFlags::empty())
                .unwrap()
        }
    }
}

impl Drop for CmdAlloc {
    fn drop(&mut self) {
        if !self.pool.is_null() {
            unsafe { gpu().destroy_command_pool(self.pool, alloc_callbacks()) }
        }
    }
}
