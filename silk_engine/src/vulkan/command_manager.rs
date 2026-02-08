use std::collections::HashMap;
use std::sync::{Arc, Mutex, Weak};
use std::thread;

use ash::vk::{self, Handle};

use crate::{
    prelude::ResultAny,
    vulkan::{command_pool::CommandPool, device::Device},
};

/// Manages command buffers and their state per thread\
/// command buffer state machine looks like this:
/// 1. **Initial**
///     - after `cmd.alloc()` or `cmd.reset()`
///     - after `cmd.reset()` or `cmd_pool.reset()`
/// 2. **Recording** after `cmd.begin()` is called
/// 3. **Executable**
///     - after `cmd.end()` is called
///     - after pending `cmd` finishes execution (when fence is signalled)
/// 4. **Pending** after `cmd.queue_submit()`
pub(crate) struct CommandManager {
    device: Weak<Device>,
    queue_family_index: u32,
    thread_data: Mutex<HashMap<thread::ThreadId, Arc<CommandThreadData>>>,
}

struct CommandThreadData {
    device: Weak<Device>,
    pool: Arc<CommandPool>,
    state: Mutex<PerThreadState>,
}

#[derive(Default)]
struct PerThreadState {
    initial_cmds: Vec<vk::CommandBuffer>,
    recording_cmd: vk::CommandBuffer,
    executable_cmds: Vec<vk::CommandBuffer>,
    pending_cmds: Vec<(vk::CommandBuffer, vk::Fence)>,
    invalid_cmds: Vec<vk::CommandBuffer>,
    finished_fences: Vec<vk::Fence>,
}

impl CommandThreadData {
    fn device(&self) -> Arc<Device> {
        self.device.upgrade().unwrap()
    }

    pub(crate) fn begin(&self) -> ResultAny<vk::CommandBuffer> {
        let mut state = self.state.lock().unwrap();

        if !state.recording_cmd.is_null() {
            return Err("failed to begin command buffer, it was recording elsewhere".into());
        }

        let cmd = if let Some(cmd) = state.initial_cmds.pop() {
            cmd
        } else {
            self.pool.alloc(1, vk::CommandBufferLevel::PRIMARY)?[0]
        };

        unsafe {
            self.device().device.begin_command_buffer(
                cmd,
                &vk::CommandBufferBeginInfo::default()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?
        };
        state.recording_cmd = cmd;

        Ok(cmd)
    }

    pub fn end(&self) -> ResultAny<vk::CommandBuffer> {
        let mut state = self.state.lock().unwrap();
        let cmd = state.recording_cmd;

        if cmd.is_null() {
            return Err("failed to end command buffer, no command buffer was recording".into());
        }

        state.executable_cmds.push(cmd);
        unsafe { self.device().device.end_command_buffer(cmd) }?;
        state.recording_cmd = vk::CommandBuffer::null();

        Ok(cmd)
    }

    pub(crate) fn submit(
        &self,
        queue: vk::Queue,
        cmd: vk::CommandBuffer,
        wait_info: &[vk::SemaphoreSubmitInfo],
        signal_info: &[vk::SemaphoreSubmitInfo],
    ) -> ResultAny {
        let mut state = self.state.lock().unwrap();

        if state.recording_cmd == cmd {
            state.executable_cmds.push(cmd);
            unsafe { self.device().device.end_command_buffer(cmd) }?;
            state.recording_cmd = vk::CommandBuffer::null();
        }

        let fence = if let Some(finished_fence) = state.finished_fences.pop() {
            finished_fence
        } else {
            unsafe {
                self.device().device.create_fence(
                    &vk::FenceCreateInfo::default(),
                    self.device().allocation_callbacks().as_ref(),
                )?
            }
        };

        unsafe {
            self.device().device.queue_submit2(
                queue,
                &[vk::SubmitInfo2::default()
                    .wait_semaphore_infos(wait_info)
                    .signal_semaphore_infos(signal_info)
                    .command_buffer_infos(&[
                        vk::CommandBufferSubmitInfo::default().command_buffer(cmd)
                    ])],
                fence,
            )
        }?;

        let Some(exec_cmd_idx) = state
            .executable_cmds
            .iter()
            .position(|&exec_cmd| exec_cmd == cmd)
        else {
            return Err("submitted command buffer is not executable".into());
        };

        state.executable_cmds.swap_remove(exec_cmd_idx);
        state.pending_cmds.push((cmd, fence));

        Ok(())
    }

    pub fn wait(&self, cmd: vk::CommandBuffer) -> ResultAny {
        let mut state = self.state.lock().unwrap();

        let Some(pending_cmd_idx) = state
            .pending_cmds
            .iter()
            .position(|&(pending_cmd, _)| pending_cmd == cmd)
        else {
            return Err("waited command buffer is not pending".into());
        };

        let (cmd, fence) = state.pending_cmds.swap_remove(pending_cmd_idx);

        unsafe {
            self.device()
                .device
                .wait_for_fences(&[fence], false, u64::MAX)
        }?;

        unsafe { self.device().device.reset_fences(&[fence]) }?;

        state.finished_fences.push(fence);
        state.invalid_cmds.push(cmd);

        Ok(())
    }

    pub fn reset(&self) -> ResultAny {
        let mut state = self.state.lock().unwrap();

        if !state.pending_cmds.is_empty() {
            return Err("attempted to reset cmd pool with pending cmds".into());
        }
        if !state.recording_cmd.is_null() {
            return Err("attempted to reset cmd pool with recording cmds".into());
        }

        self.pool.reset();
        let invalid = std::mem::take(&mut state.invalid_cmds);
        let executable = std::mem::take(&mut state.executable_cmds);
        state.initial_cmds.extend(invalid);
        state.initial_cmds.extend(executable);

        Ok(())
    }

    pub fn reserve(&self, count: usize) -> ResultAny<()> {
        let mut state = self.state.lock().unwrap();
        let current_count = state.initial_cmds.len();
        if current_count < count {
            let to_alloc = count - current_count;
            let new_cmds = self
                .pool
                .alloc(to_alloc as u32, vk::CommandBufferLevel::PRIMARY)?;
            state.initial_cmds.extend(new_cmds);
        }
        Ok(())
    }

    pub(crate) fn cmd(&self) -> ResultAny<vk::CommandBuffer> {
        let state = self.state.lock().unwrap();

        if state.recording_cmd.is_null() {
            return Err("no active command buffer".into());
        }

        Ok(state.recording_cmd)
    }
}

impl CommandManager {
    pub(crate) fn new(device: &Arc<Device>, queue_family_index: u32) -> ResultAny<Arc<Self>> {
        Ok(Arc::new(Self {
            device: Arc::downgrade(device),
            queue_family_index,
            thread_data: Mutex::new(HashMap::new()),
        }))
    }

    fn get_thread_data(&self) -> ResultAny<Arc<CommandThreadData>> {
        let thread_id = thread::current().id();
        let mut map = self.thread_data.lock().unwrap();
        if let Some(thread_data) = map.get(&thread_id) {
            return Ok(thread_data.clone());
        }

        let pool = CommandPool::new(
            &self.device.upgrade().unwrap(),
            self.queue_family_index,
            vk::CommandPoolCreateFlags::empty(),
        )?;

        let thread_data = Arc::new(CommandThreadData {
            device: self.device.clone(),
            pool,
            state: Mutex::new(PerThreadState::default()),
        });
        map.insert(thread_id, thread_data.clone());
        Ok(thread_data)
    }

    pub(crate) fn begin(&self) -> ResultAny<vk::CommandBuffer> {
        self.get_thread_data()?.begin()
    }

    pub fn end(&self) -> ResultAny<vk::CommandBuffer> {
        self.get_thread_data()?.end()
    }

    pub(crate) fn submit(
        &self,
        queue: vk::Queue,
        cmd: vk::CommandBuffer,
        wait_info: &[vk::SemaphoreSubmitInfo],
        signal_info: &[vk::SemaphoreSubmitInfo],
    ) -> ResultAny {
        self.get_thread_data()?
            .submit(queue, cmd, wait_info, signal_info)
    }

    pub fn wait(&self, cmd: vk::CommandBuffer) -> ResultAny {
        self.get_thread_data()?.wait(cmd)
    }

    pub fn reset(&self) -> ResultAny {
        self.get_thread_data()?.reset()
    }

    pub fn reserve(&self, count: usize) -> ResultAny<()> {
        self.get_thread_data()?.reserve(count)
    }

    pub(crate) fn cmd(&self) -> ResultAny<vk::CommandBuffer> {
        self.get_thread_data()?.cmd()
    }
}
