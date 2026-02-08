use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};

use ash::vk::{self, Handle};

use crate::{prelude::ResultAny, vulkan::device::Device};

pub(crate) struct Buffer {
    handle: vk::Buffer,
    memory: Mutex<vk::DeviceMemory>,
    size: AtomicU64,
    usage: vk::BufferUsageFlags,
    device: Arc<Device>,
    queue_family_indices: Vec<u32>,
    sharing_mode: vk::SharingMode,
    required_memory_properties: vk::MemoryPropertyFlags,
}

impl Buffer {
    pub(crate) fn new(
        device: &Arc<Device>,
        size: u64,
        usage: vk::BufferUsageFlags,
        queue_family_indices: &[u32],
        sharing_mode: vk::SharingMode,
        required_memory_properties: vk::MemoryPropertyFlags,
    ) -> ResultAny<Arc<Self>> {
        let buffer = Self::_new(
            device,
            size,
            usage,
            queue_family_indices,
            sharing_mode,
            required_memory_properties,
            vk::Buffer::null(),
        )?;
        device
            .allocator()
            .alloc_buf(&buffer, required_memory_properties)?;

        Ok(buffer)
    }

    pub(crate) fn new_unallocated(
        device: &Arc<Device>,
        size: u64,
        usage: vk::BufferUsageFlags,
        queue_family_indices: &[u32],
        sharing_mode: vk::SharingMode,
        required_memory_properties: vk::MemoryPropertyFlags,
    ) -> ResultAny<Arc<Self>> {
        Self::_new(
            device,
            size,
            usage,
            queue_family_indices,
            sharing_mode,
            required_memory_properties,
            vk::Buffer::null(),
        )
    }

    pub(crate) fn _new(
        device: &Arc<Device>,
        size: u64,
        usage: vk::BufferUsageFlags,
        queue_family_indices: &[u32],
        sharing_mode: vk::SharingMode,
        required_memory_properties: vk::MemoryPropertyFlags,
        mut buffer: vk::Buffer,
    ) -> ResultAny<Arc<Self>> {
        assert!(
            (sharing_mode == vk::SharingMode::EXCLUSIVE && queue_family_indices.len() == 1)
                || (sharing_mode == vk::SharingMode::CONCURRENT
                    && !queue_family_indices.is_empty()),
            "buffer's queue family index count must be 1 if using vk::SharingMode::Exclusive"
        );
        if buffer.is_null() {
            buffer = unsafe {
                device.device.create_buffer(
                    &vk::BufferCreateInfo::default()
                        .size(size)
                        .usage(usage)
                        .queue_family_indices(queue_family_indices)
                        .sharing_mode(sharing_mode),
                    device.allocation_callbacks().as_ref(),
                )
            }?;
        }

        Ok(Arc::new(Self {
            handle: buffer,
            memory: Mutex::new(vk::DeviceMemory::null()),
            size: AtomicU64::new(size),
            usage,
            device: Arc::clone(device),
            queue_family_indices: queue_family_indices.to_vec(),
            sharing_mode,
            required_memory_properties,
        }))
    }

    pub(crate) fn bind_memory(&self, memory: vk::DeviceMemory, offset: u64) -> ResultAny {
        unsafe {
            self.device()
                .device
                .bind_buffer_memory(self.handle, memory, offset)
        }?;
        *self.memory.lock().unwrap() = memory;
        Ok(())
    }

    /// recreates buffer with `size`, does not copy old data
    pub(crate) fn realloc(self: Arc<Self>, size: u64) -> ResultAny<Arc<Self>> {
        assert!(self.usage.contains(vk::BufferUsageFlags::TRANSFER_SRC));
        assert!(self.usage.contains(vk::BufferUsageFlags::TRANSFER_DST));
        if size == self.size.load(Ordering::SeqCst) {
            return Ok(self);
        }

        let new_buffer = Self::new(
            self.device(),
            size,
            self.usage,
            &self.queue_family_indices,
            self.sharing_mode,
            self.required_memory_properties,
        )?;

        Ok(new_buffer)
    }

    /// recreates buffer with `size` and copies old data
    pub(crate) fn resize(self: Arc<Self>, size: u64) -> ResultAny<Arc<Self>> {
        assert!(self.usage.contains(vk::BufferUsageFlags::TRANSFER_SRC));
        assert!(self.usage.contains(vk::BufferUsageFlags::TRANSFER_DST));
        if size == self.size.load(Ordering::SeqCst) {
            return Ok(self);
        }

        let new_buffer = Self::new(
            self.device(),
            size,
            self.usage,
            &self.queue_family_indices,
            self.sharing_mode,
            self.required_memory_properties,
        )?;

        let copy_size = self.size.load(Ordering::SeqCst).min(size);
        if copy_size > 0 {
            new_buffer.copy_from_buffer(&self, 0, 0, copy_size)?;
        }

        Ok(new_buffer)
    }

    /// write to buffer via map if it's host mappable or staging buffer if not
    /// uses `queue_family_indices[0]` with `queue[0]` for waited submission
    pub(crate) fn write<T: ?Sized>(&self, data: &T, offset: u64) -> ResultAny {
        if self.is_mappable() {
            self.write_mapped_off(data, offset);
        } else {
            let allocator = self.device().allocator();
            let staging_buffer = allocator.staging_buffer(size_of_val(data) as u64)?;
            staging_buffer.write_mapped(data);
            self.copy_from_buffer(&staging_buffer, 0, offset, size_of_val(data) as u64)?;
        }
        Ok(())
    }

    /// read from buffer via map if it's host mappable or staging buffer if not
    /// uses `queue_family_indices[0]` with `queue[0]` for waited submission
    pub(crate) fn read<T: ?Sized>(&self, data: &mut T, offset: u64) -> ResultAny {
        if self.is_mappable() {
            self.read_mapped_off(data, offset);
        } else {
            let allocator = self.device().allocator();
            let staging_buffer = allocator.staging_buffer(size_of_val(data) as u64)?;
            staging_buffer.copy_from_buffer(self, offset, 0, size_of_val(data) as u64)?;
            staging_buffer.read_mapped(data);
        }
        Ok(())
    }

    pub(crate) fn map_ptr(&self, offset: u64) -> *mut u8 {
        self.device().allocator().map_ptr(self.handle, offset)
    }

    pub(crate) fn map(&self) -> *mut u8 {
        self.map_ptr(0)
    }

    pub(crate) fn write_mapped_off<T: ?Sized>(&self, data: &T, offset: vk::DeviceSize) -> *mut u8 {
        unsafe {
            assert!(
                self.size.load(Ordering::SeqCst) - offset >= size_of_val(data) as vk::DeviceSize,
                "buffer size({}){} is too small for data({})",
                self.size.load(Ordering::SeqCst) - offset,
                if offset > 0 {
                    format!("-off({offset})")
                } else {
                    String::new()
                },
                size_of_val(data)
            );
            let mem_ptr = self.map().byte_add(offset as usize);
            mem_ptr.copy_from_nonoverlapping(data as *const T as *const u8, size_of_val(data));
            mem_ptr
        }
    }

    pub(crate) fn read_mapped_off<T: ?Sized>(
        &self,
        data: &mut T,
        offset: vk::DeviceSize,
    ) -> *mut u8 {
        unsafe {
            assert!(
                self.size.load(Ordering::SeqCst) - offset >= size_of_val(data) as vk::DeviceSize,
                "buffer size({}){} is too small for data({})",
                self.size.load(Ordering::SeqCst) - offset,
                if offset > 0 {
                    format!("-off({offset})")
                } else {
                    String::new()
                },
                size_of_val(data)
            );
            let mem_ptr = self.map().byte_add(offset as usize);
            (data as *mut T as *mut u8).copy_from_nonoverlapping(mem_ptr, size_of_val(data));
            mem_ptr
        }
    }

    pub(crate) fn write_mapped<T: ?Sized>(&self, data: &T) -> *mut u8 {
        self.write_mapped_off(data, 0)
    }

    pub(crate) fn read_mapped<T: ?Sized>(&self, data: &mut T) -> *mut u8 {
        self.read_mapped_off(data, 0)
    }

    /// Copy from source buffer with automatic command buffer management\
    /// uses `queue_family_indices[0]` with `queue[0]` for waited submission
    pub(crate) fn copy_from_buffer(
        &self,
        source: &Buffer,
        src_offset: u64,
        dst_offset: u64,
        size: u64,
    ) -> ResultAny<()> {
        self.copy_from_buffer_regions(
            source,
            &[vk::BufferCopy {
                src_offset,
                dst_offset,
                size,
            }],
        )
    }

    /// Copy multiple regions from source buffer with automatic command buffer management\
    /// uses `queue_family_indices[0]` with `queue[0]` for waited submission
    pub(crate) fn copy_from_buffer_regions(
        &self,
        source: &Buffer,
        regions: &[vk::BufferCopy],
    ) -> ResultAny<()> {
        let queue_family_index = self.queue_family_indices[0];
        let cmd_manager = self.device().command_manager(queue_family_index);
        let cmd = cmd_manager.begin()?;

        self.copy_from_buffer_cmd(source, regions, cmd);

        let queue = self.device().get_queue(queue_family_index, 0);
        cmd_manager.submit(queue, cmd, &[], &[])?;
        cmd_manager.wait(cmd)?;
        Ok(())
    }

    /// Copy from source buffer using provided command buffer (for batching)
    pub(crate) fn copy_from_buffer_cmd(
        &self,
        source: &Buffer,
        regions: &[vk::BufferCopy],
        command_buffer: vk::CommandBuffer,
    ) {
        unsafe {
            self.device().device.cmd_copy_buffer(
                command_buffer,
                source.handle(),
                self.handle,
                regions,
            );
        }
    }

    pub(crate) fn get_memory_requirements(&self) -> vk::MemoryRequirements {
        unsafe {
            self.device()
                .device
                .get_buffer_memory_requirements(self.handle)
        }
    }

    pub(crate) fn is_mappable(&self) -> bool {
        self.device().allocator().is_mappable(self.handle)
    }

    /// Returns the persistently mapped pointer at the given offset, or None if not HOST_VISIBLE
    pub(crate) fn mapped_ptr(&self, offset: vk::DeviceSize) -> Option<*mut u8> {
        self.device().allocator().mapped_ptr(self.handle, offset)
    }

    pub(crate) fn handle(&self) -> vk::Buffer {
        self.handle
    }

    pub(crate) fn size(&self) -> u64 {
        self.size.load(Ordering::SeqCst)
    }

    pub(crate) fn usage(&self) -> vk::BufferUsageFlags {
        self.usage
    }

    pub(crate) fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.device.allocator().dealloc_buf(self.handle);

        unsafe {
            self.device
                .device
                .destroy_buffer(self.handle, self.device.allocation_callbacks().as_ref())
        };
    }
}
