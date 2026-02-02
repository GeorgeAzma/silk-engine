use std::{
    collections::HashMap,
    ptr::null_mut,
    sync::{Arc, Mutex, Weak},
};

use ash::vk::{self, Handle};

use crate::{
    prelude::ResultAny,
    util::{alloc::BuddyAlloc, mem::Mem, print::Level},
    vulkan::{buffer::Buffer, device::Device, image::Image},
};

struct MemBlock {
    off: vk::DeviceSize,
    size: vk::DeviceSize,
    mem: vk::DeviceMemory,
    map_ptr: *mut u8,
    device: Arc<Device>,
}

// SAFETY: map_ptr points to persistently mapped memory, never modified after creation
unsafe impl Send for MemBlock {}
unsafe impl Sync for MemBlock {}

impl MemBlock {
    fn new(
        device: &Arc<Device>,
        off: vk::DeviceSize,
        size: vk::DeviceSize,
        mem_type_idx: u32,
        mem_property_flags: vk::MemoryPropertyFlags,
    ) -> Self {
        let mem = unsafe {
            device
                .device
                .allocate_memory(
                    &vk::MemoryAllocateInfo::default()
                        .allocation_size(size)
                        .memory_type_index(mem_type_idx)
                        .push_next(&mut vk::MemoryPriorityAllocateInfoEXT::default().priority(0.8)),
                    device.allocation_callbacks().as_ref(),
                )
                .unwrap()
        };

        // persistently map HOST_VISIBLE memory
        let map_ptr = if mem_property_flags.contains(vk::MemoryPropertyFlags::HOST_VISIBLE) {
            unsafe {
                device
                    .device
                    .map_memory(mem, 0, size, vk::MemoryMapFlags::empty())
                    .unwrap() as *mut u8
            }
        } else {
            null_mut()
        };

        #[cfg(debug_assertions)]
        {
            crate::log_sink!(
                "rotating_file",
                crate::util::print::Level::Trace,
                "Allocated Memory Block {} (mapped: {})",
                Mem::b(size as usize),
                !map_ptr.is_null(),
            );
        }
        Self {
            off,
            size,
            mem,
            map_ptr,
            device: Arc::clone(device),
        }
    }

    fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl Drop for MemBlock {
    fn drop(&mut self) {
        if !self.mem.is_null() {
            // unmap before freeing if it was mapped
            if !self.map_ptr.is_null() {
                unsafe { self.device.device.unmap_memory(self.mem) };
            }
            unsafe {
                self.device
                    .device
                    .free_memory(self.mem, self.device.allocation_callbacks().as_ref())
            }
        }
    }
}

/// resizable gpu/cpu memory block with same memory properties, managed by buddy allocator.\
/// on OOM it adds new memory block with double the size without reallocating first one, which means memory stays pinned.\
/// buddy allocator returned offset determines which memory block to use for allocation
struct MemPool {
    mem_property_flags: vk::MemoryPropertyFlags,
    mem_blocks: Mutex<Vec<Arc<MemBlock>>>,
    buddy: Mutex<BuddyAlloc>,
    mem_type_idx: u32,
    device: Weak<Device>,
}

impl MemPool {
    const BLOCK_SIZE: vk::DeviceSize = 16 * (1 << 20); // 16 MiB
    const _ASSERT_BLOCK_SIZE: () =
        assert!(Self::BLOCK_SIZE.is_power_of_two() && Self::BLOCK_SIZE != 0);

    fn new(
        device: &Arc<Device>,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
        mem_type_idx: u32,
    ) -> Self {
        let mem_property_flags =
            memory_properties.memory_types[mem_type_idx as usize].property_flags;
        Self {
            mem_property_flags,
            mem_blocks: Mutex::new(vec![]),
            buddy: Mutex::new(BuddyAlloc::new(0)),
            mem_type_idx,
            device: Arc::downgrade(device),
        }
    }

    fn lazy_init(&self) {
        if !self.mem_blocks.lock().unwrap().is_empty() {
            return;
        }
        let block_size = Self::BLOCK_SIZE.next_power_of_two();
        *self.mem_blocks.lock().unwrap() = vec![Arc::new(MemBlock::new(
            &self.device(),
            0,
            block_size,
            self.mem_type_idx,
            self.mem_property_flags,
        ))];
        *self.buddy.lock().unwrap() = BuddyAlloc::new(block_size as usize);
    }

    fn find_off_mem_block(&self, off: vk::DeviceSize) -> Arc<MemBlock> {
        let mem_block_idx = (Self::BLOCK_SIZE.leading_zeros() + 1
            - off.max(Self::BLOCK_SIZE - 1).leading_zeros()) as usize;
        Arc::clone(&self.mem_blocks.lock().unwrap()[mem_block_idx])
        // let mem_blocks = self.mem_blocks.lock().unwrap();
        // for block in mem_blocks.iter() {
        //     if off >= block.off && off < block.off + block.size {
        //         return Arc::clone(block);
        //     }
        // }
        // panic!("No memory block found for offset {off}");
    }

    fn alloc(&self, size: vk::DeviceSize) -> (vk::DeviceSize, Arc<MemBlock>) {
        self.lazy_init();
        let mut off = self.buddy.lock().unwrap().alloc(size as usize);
        while off == usize::MAX {
            let mut buddy = self.buddy.lock().unwrap();
            let old_len = buddy.len();
            buddy.resize(old_len * 2);
            off = buddy.alloc(size as usize);
            self.mem_blocks.lock().unwrap().push(Arc::new(MemBlock::new(
                &self.device(),
                old_len as vk::DeviceSize,
                (buddy.len() - old_len) as vk::DeviceSize,
                self.mem_type_idx,
                self.mem_property_flags,
            )));
        }
        crate::log_sink!(
            "rotating_file",
            Level::Trace,
            "Memory Pool({:?}) Alloc: off({}), size({})",
            self.mem_property_flags,
            Mem::b(off),
            Mem::b(size as usize)
        );
        assert_ne!(off, usize::MAX); // there is enough free space in some memory block
        let mem_block = self.find_off_mem_block(off as vk::DeviceSize); // find memory block which had enough free space
        let off_in_block = off as vk::DeviceSize - mem_block.off;
        assert!(off_in_block + size <= mem_block.size);
        (off_in_block, mem_block)
    }

    fn dealloc(&self, offset: vk::DeviceSize, size: vk::DeviceSize) {
        crate::log_sink!(
            "rotating_file",
            Level::Trace,
            "Memory Pool({:?}) Dealloc: off({}), size({})",
            self.mem_property_flags,
            Mem::b(offset as usize),
            Mem::b(size as usize)
        );
        self.buddy
            .lock()
            .unwrap()
            .dealloc(offset as usize, size as usize)
    }

    pub(crate) fn device(&self) -> Arc<Device> {
        self.device.upgrade().unwrap()
    }
}

#[derive(Clone, Copy)]
struct BufferAlloc {
    mem_type_idx: u32,
    mem_block_off: vk::DeviceSize,
    buddy_off: vk::DeviceSize,
    size: vk::DeviceSize,
    aligned_size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
}

#[derive(Clone)]
struct ImageAlloc {
    mem_type_idx: u32,
    buddy_off: vk::DeviceSize,
    aligned_size: vk::DeviceSize,
}

pub struct VulkanAlloc {
    mem_pools: Vec<MemPool>,
    buf_allocs: Mutex<HashMap<u64, BufferAlloc>>,
    img_allocs: Mutex<HashMap<u64, ImageAlloc>>,
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    staging_buffer: Mutex<Option<Arc<Buffer>>>,
    device: Weak<Device>,
}

/// manages vulkan objects allocated from memory pool with suitable memory type index
impl VulkanAlloc {
    pub(crate) fn new(
        device: &Arc<Device>,
        memory_properties: &vk::PhysicalDeviceMemoryProperties,
    ) -> Arc<Self> {
        let mem_type_count = memory_properties.memory_type_count;
        let mem_pools: Vec<MemPool> = (0..mem_type_count)
            .map(|mem_type_idx| MemPool::new(device, memory_properties, mem_type_idx))
            .collect();
        Arc::new(Self {
            mem_pools,
            buf_allocs: Mutex::new(Default::default()),
            img_allocs: Mutex::new(Default::default()),
            memory_properties: *memory_properties,
            staging_buffer: Mutex::new(None),
            device: Arc::downgrade(device),
        })
    }

    pub(crate) fn alloc_buf(
        &self,
        buffer: &Buffer,
        required_memory_properties: vk::MemoryPropertyFlags,
    ) -> ResultAny {
        let mem_reqs = buffer.get_memory_requirements();
        let mem_type_idx = self
            .find_mem_type_idx(mem_reqs.memory_type_bits, required_memory_properties)
            .ok_or(format!(
                "buffer memory with these properties not found: {required_memory_properties:?}"
            ))?;

        let pool = &self.mem_pools[mem_type_idx as usize];
        let aligned_size = mem_reqs.size;
        let (alloc_off, mem_block) = pool.alloc(aligned_size);
        buffer.bind_memory(mem_block.mem, alloc_off)?;

        self.buf_allocs.lock().unwrap().insert(
            buffer.handle().as_raw(),
            BufferAlloc {
                mem_type_idx,
                mem_block_off: alloc_off,
                buddy_off: mem_block.off + alloc_off,
                size: buffer.size(),
                aligned_size,
                usage: buffer.usage(),
            },
        );

        Ok(())
    }

    pub(crate) fn dealloc_buf(&self, buf: vk::Buffer) {
        if let Some(buf_alloc) = self.buf_allocs.lock().unwrap().remove(&buf.as_raw()) {
            self.mem_pools[buf_alloc.mem_type_idx as usize]
                .dealloc(buf_alloc.buddy_off, buf_alloc.aligned_size);
        }
    }

    pub(crate) fn alloc_img(
        &self,
        image: &Image,
        required_memory_properties: vk::MemoryPropertyFlags,
    ) -> ResultAny {
        let mem_reqs = image.get_memory_requirements();
        let mem_type_idx = self
            .find_mem_type_idx(mem_reqs.memory_type_bits, required_memory_properties)
            .ok_or(format!(
                "image memory with these properties not found: {required_memory_properties:?}"
            ))?;

        let pool = &self.mem_pools[mem_type_idx as usize];
        let aligned_size = mem_reqs.size;
        let (alloc_off, mem_block) = pool.alloc(aligned_size);
        image.bind_memory(mem_block.mem, alloc_off)?;

        self.img_allocs.lock().unwrap().insert(
            image.handle().as_raw(),
            ImageAlloc {
                mem_type_idx,
                buddy_off: alloc_off + mem_block.off,
                aligned_size,
            },
        );

        Ok(())
    }

    pub(crate) fn dealloc_img(&self, image: &Image) {
        if let Some(img_alloc) = self
            .img_allocs
            .lock()
            .unwrap()
            .remove(&image.handle().as_raw())
        {
            self.mem_pools[img_alloc.mem_type_idx as usize]
                .dealloc(img_alloc.buddy_off, img_alloc.aligned_size);
        }
    }

    pub(crate) fn mapped_ptr(&self, buffer: vk::Buffer, offset: vk::DeviceSize) -> Option<*mut u8> {
        let buf_allocs = self.buf_allocs.lock().unwrap();
        let buf_alloc = buf_allocs.get(&buffer.as_raw()).unwrap();

        let pool = &self.mem_pools[buf_alloc.mem_type_idx as usize];
        let mem_block = pool.find_off_mem_block(buf_alloc.buddy_off);

        if mem_block.map_ptr.is_null() {
            return None;
        }

        let block_buf_off = offset + buf_alloc.mem_block_off;
        Some(unsafe { mem_block.map_ptr.byte_add(block_buf_off as usize) })
    }

    pub(crate) fn map_ptr(&self, buffer: vk::Buffer, offset: vk::DeviceSize) -> *mut u8 {
        self.mapped_ptr(buffer, offset)
            .expect("Buffer is not mappable (not HOST_VISIBLE)")
    }

    pub(crate) fn is_mappable(&self, buffer: vk::Buffer) -> bool {
        self.buf_props(buffer)
            .contains(vk::MemoryPropertyFlags::HOST_VISIBLE)
    }

    pub(crate) fn img_props(&self, image: vk::Image) -> vk::MemoryPropertyFlags {
        let img_alloc = &self.img_allocs.lock().unwrap()[&image.as_raw()];
        self.mem_pools[img_alloc.mem_type_idx as usize].mem_property_flags
    }

    pub(crate) fn buf_size(&self, buffer: vk::Buffer) -> vk::DeviceSize {
        let buf_alloc = &self.buf_allocs.lock().unwrap()[&buffer.as_raw()];
        buf_alloc.size
    }

    pub(crate) fn buf_props(&self, buffer: vk::Buffer) -> vk::MemoryPropertyFlags {
        let buf_alloc = &self.buf_allocs.lock().unwrap()[&buffer.as_raw()];
        self.mem_pools[buf_alloc.mem_type_idx as usize].mem_property_flags
    }

    pub(crate) fn buf_usage(&self, buffer: vk::Buffer) -> vk::BufferUsageFlags {
        let buf_alloc = &self.buf_allocs.lock().unwrap()[&buffer.as_raw()];
        buf_alloc.usage
    }

    fn find_mem_type_idx(
        &self,
        mem_type_bits: u32,
        required_mem_props: vk::MemoryPropertyFlags,
    ) -> Option<u32> {
        self.memory_properties
            .memory_types
            .iter()
            .enumerate()
            .find_map(|(i, mem_type)| {
                (mem_type_bits & (1 << i) != 0
                    && mem_type.property_flags.contains(required_mem_props))
                .then_some(i as u32)
            })
    }

    pub(crate) fn staging_buffer(self: &Arc<Self>, min_size: u64) -> ResultAny<Arc<Buffer>> {
        let mut staging_buffer_guard = self.staging_buffer.lock().unwrap();
        if let Some(staging_buffer) = staging_buffer_guard.as_ref() {
            if staging_buffer.size() < min_size {
                let new_buffer = staging_buffer
                    .clone()
                    .realloc(min_size.next_power_of_two())?;
                *staging_buffer_guard = Some(new_buffer.clone());
                Ok(new_buffer)
            } else {
                Ok(staging_buffer.clone())
            }
        } else {
            let new_buffer = Buffer::new(
                &self.device(),
                min_size.next_power_of_two().max(8192),
                vk::BufferUsageFlags::TRANSFER_SRC | vk::BufferUsageFlags::TRANSFER_DST,
                &[],
                vk::SharingMode::EXCLUSIVE,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            )?;
            *staging_buffer_guard = Some(new_buffer.clone());
            Ok(new_buffer)
        }
    }

    pub(crate) fn device(&self) -> Arc<Device> {
        self.device.upgrade().unwrap()
    }
}
