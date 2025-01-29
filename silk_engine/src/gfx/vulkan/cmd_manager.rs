use std::ptr::null;

use ash::vk;

use super::{CmdAlloc, alloc_callbacks, gpu, queue};

#[derive(Default)]
pub struct CmdManager {
    cmd_alloc: CmdAlloc,
    init_cmds: Vec<vk::CommandBuffer>,
    rec_cmd: vk::CommandBuffer,
    exec_cmds: Vec<vk::CommandBuffer>,
    pending_cmds: Vec<(vk::CommandBuffer, vk::Fence)>,
    invalid_cmds: Vec<vk::CommandBuffer>,
    finished_fences: Vec<vk::Fence>,
}

impl CmdManager {
    pub fn new() -> Self {
        Self {
            cmd_alloc: CmdAlloc::new(),
            ..Default::default()
        }
    }

    pub fn begin(&mut self) -> vk::CommandBuffer {
        assert_eq!(
            self.rec_cmd,
            Default::default(),
            "failed to begin cmd, other cmd was recording"
        );
        let cmd = self
            .init_cmds
            .pop()
            .unwrap_or_else(|| self.cmd_alloc.alloc(1)[0]);
        self.rec_cmd = cmd;
        unsafe {
            gpu()
                .begin_command_buffer(
                    cmd,
                    &vk::CommandBufferBeginInfo::default()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .unwrap()
        };
        cmd
    }

    pub fn end(&mut self) -> vk::CommandBuffer {
        let cmd = self.rec_cmd;
        assert_ne!(
            cmd,
            Default::default(),
            "failed to end cmd, no cmd was recording"
        );
        self.exec_cmds.push(cmd);
        unsafe { gpu().end_command_buffer(cmd).unwrap() };
        self.rec_cmd = Default::default();
        cmd
    }

    pub fn submit(
        &mut self,
        cmd: vk::CommandBuffer,
        waits: &[vk::Semaphore],
        signals: &[vk::Semaphore],
        wait_dst_stage_mask: &[vk::PipelineStageFlags],
    ) {
        let exec_cmd_idx = self
            .exec_cmds
            .iter()
            .position(|&ec| ec == cmd)
            .unwrap_or_else(|| panic!("cmd is not executable: {cmd:?}"));
        self.exec_cmds.remove(exec_cmd_idx);
        let fence = self.finished_fences.pop().unwrap_or_else(|| unsafe {
            gpu()
                .create_fence(&vk::FenceCreateInfo::default(), alloc_callbacks())
                .unwrap()
        });
        self.pending_cmds.push((cmd, fence));
        unsafe {
            gpu()
                .queue_submit(
                    queue(),
                    &[vk::SubmitInfo {
                        wait_semaphore_count: waits.len() as u32,
                        p_wait_semaphores: if waits.is_empty() {
                            null()
                        } else {
                            waits.as_ptr()
                        },
                        signal_semaphore_count: signals.len() as u32,
                        p_signal_semaphores: if signals.is_empty() {
                            null()
                        } else {
                            signals.as_ptr()
                        },
                        ..Default::default()
                    }
                    .command_buffers(&[cmd])
                    .wait_dst_stage_mask(wait_dst_stage_mask)],
                    fence,
                )
                .unwrap()
        };
    }

    pub fn wait(&mut self, cmd: vk::CommandBuffer) {
        let pending_cmd_idx = self
            .pending_cmds
            .iter()
            .position(|(pc, _)| *pc == cmd)
            .unwrap_or_else(|| panic!("can't wait on cmd that isn't pending"));
        let (cmd, fence) = self.pending_cmds.remove(pending_cmd_idx);
        unsafe { gpu().wait_for_fences(&[fence], false, u64::MAX).unwrap() };
        unsafe { gpu().reset_fences(&[fence]).unwrap() };
        self.finished_fences.push(fence);
        self.invalid_cmds.push(cmd);
    }

    pub fn reset(&mut self) {
        assert!(
            self.pending_cmds.is_empty(),
            "attempted to reset cmd pool with pending cmds"
        );
        assert_eq!(
            self.rec_cmd,
            Default::default(),
            "attempted to reset cmd pool with recording cmds"
        );
        self.cmd_alloc.reset();
        self.init_cmds.append(&mut self.invalid_cmds);
        self.init_cmds.append(&mut self.exec_cmds);
    }

    pub fn cmd(&self) -> vk::CommandBuffer {
        assert_ne!(self.rec_cmd, Default::default(), "no active cmd");
        self.rec_cmd
    }
}
