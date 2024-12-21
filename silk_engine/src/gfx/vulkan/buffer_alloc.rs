use std::collections::HashMap;

use super::{alloc_callbacks, gpu, QUEUE_FAMILY_INDEX};
use crate::{gpu_mem_props, log};
use ash::vk;
use vk::Handle;

unsafe impl Send for MappedRange {}
unsafe impl Sync for MappedRange {}

pub struct MappedRange {
    ptr: *mut u8,
    range: std::ops::Range<u64>,
}

impl MappedRange {
    fn new(ptr: *mut u8, range: &std::ops::Range<u64>) -> Self {
        Self {
            ptr,
            range: range.to_owned(),
        }
    }

    pub fn contains(&self, range: &std::ops::Range<u64>) -> bool {
        self.range.start <= range.start && self.range.end >= range.end
    }

    pub fn subrange(&self, range: &std::ops::Range<u64>) -> Self {
        Self::new(self.subrange_ptr(range), range)
    }

    pub fn subrange_ptr(&self, range: &std::ops::Range<u64>) -> *mut u8 {
        unsafe { self.ptr.offset((range.start - self.range.start) as isize) }
    }

    pub fn len(&self) -> u64 {
        self.range.end - self.range.start
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

struct BufMem {
    mem: vk::DeviceMemory,
    size: vk::DeviceSize,
    mapped_range: Option<MappedRange>,
    props: vk::MemoryPropertyFlags,
    usage: vk::BufferUsageFlags,
}

// TODO: make more efficient, implement actual allocator strategy and remove redundant calculations
// TODO: have different buffers for different properties of memory
// TODO: allocate vertex/index/uniform buffers from single pre-allocated buffer with suitable memory properties
// TODO: when buffer is full, allocate new buffer (maybe copy old data to new buffer)

pub struct BufferAlloc {
    buf_mems: HashMap<vk::DeviceSize, BufMem>,
}

impl Default for BufferAlloc {
    fn default() -> Self {
        Self::new()
    }
}

impl BufferAlloc {
    pub fn new() -> Self {
        Self {
            buf_mems: Default::default(),
        }
    }

    pub fn alloc(
        &mut self,
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        mem_props: vk::MemoryPropertyFlags,
    ) -> vk::Buffer {
        let buffer = unsafe {
            gpu()
                .create_buffer(
                    &vk::BufferCreateInfo::default()
                        .size(size)
                        .usage(usage)
                        .queue_family_indices(&[*QUEUE_FAMILY_INDEX])
                        .sharing_mode(vk::SharingMode::EXCLUSIVE),
                    alloc_callbacks(),
                )
                .unwrap()
        };
        let mem_reqs = unsafe { gpu().get_buffer_memory_requirements(buffer) };
        let mem_type_idx = Self::find_mem_type_idx(mem_reqs.memory_type_bits, mem_props);
        let mem = unsafe {
            gpu()
                .allocate_memory(
                    &vk::MemoryAllocateInfo::default()
                        .allocation_size(mem_reqs.size)
                        .memory_type_index(mem_type_idx)
                        .push_next(&mut vk::MemoryPriorityAllocateInfoEXT::default().priority(0.5)),
                    alloc_callbacks(),
                )
                .unwrap()
        };
        unsafe { gpu().bind_buffer_memory(buffer, mem, 0).unwrap() };
        self.buf_mems.insert(
            buffer.as_raw(),
            BufMem {
                mem,
                size,
                mapped_range: None,
                props: mem_props,
                usage,
            },
        );
        buffer
    }

    pub fn dealloc(&mut self, buffer: vk::Buffer) {
        let mem = self.buf_mems.remove(&buffer.as_raw()).unwrap();
        unsafe {
            gpu().destroy_buffer(buffer, alloc_callbacks());
            gpu().free_memory(mem.mem, alloc_callbacks());
        }
    }

    /// does not copy memory
    pub fn realloc(&mut self, buffer: vk::Buffer, new_size: vk::DeviceSize) -> vk::Buffer {
        let BufMem {
            mem: _,
            size: _,
            mapped_range: _,
            props,
            usage,
        } = self.buf_mems[&buffer.as_raw()];
        self.dealloc(buffer);
        self.alloc(new_size, usage, props)
    }

    pub fn map(&mut self, buffer: vk::Buffer) -> *mut u8 {
        self.map_range(buffer, 0, vk::WHOLE_SIZE)
    }

    pub fn map_range(
        &mut self,
        buffer: vk::Buffer,
        off: vk::DeviceSize,
        size: vk::DeviceSize,
    ) -> *mut u8 {
        let block = self.buf_mems.get_mut(&buffer.as_raw()).unwrap();
        let range = off..off + size;
        if let Some(mr) = &block.mapped_range {
            if mr.contains(&range) {
                return mr.subrange_ptr(&range);
            } else {
                unsafe { gpu().unmap_memory(block.mem) }
            }
        }
        block.mapped_range = Some(MappedRange::new(
            unsafe {
                gpu()
                    .map_memory(block.mem, off, size, vk::MemoryMapFlags::empty())
                    .unwrap() as *mut u8
            },
            &range,
        ));
        log!(
            "Mapped({:X}): off({off}), size({})",
            block.mem.as_raw(),
            if size == vk::WHOLE_SIZE {
                "WHOLE".to_string()
            } else {
                size.to_string()
            }
        );
        block.mapped_range.as_ref().unwrap().ptr
    }

    pub fn unmap(&mut self, buffer: vk::Buffer) {
        let block = self.buf_mems.get_mut(&buffer.as_raw()).unwrap();
        block.mapped_range = None;
        unsafe { gpu().unmap_memory(block.mem) }
    }

    pub fn write_mapped_off<T: ?Sized>(
        &mut self,
        buffer: vk::Buffer,
        data: &T,
        off: vk::DeviceSize,
    ) {
        unsafe {
            assert!(
                self.get_size(buffer) - off >= size_of_val(data) as vk::DeviceSize,
                "buffer size({}){} is too small for data({})",
                self.get_size(buffer) - off,
                if off > 0 {
                    format!("-off({off})")
                } else {
                    String::new()
                },
                size_of_val(data)
            );
            let mem_ptr = self.map(buffer).byte_add(off as usize);
            mem_ptr.copy_from_nonoverlapping(data as *const T as *const u8, size_of_val(data));
        }
    }

    pub fn read_mapped_off<T: ?Sized>(
        &mut self,
        buffer: vk::Buffer,
        data: &mut T,
        off: vk::DeviceSize,
    ) {
        unsafe {
            assert!(
                self.get_size(buffer) - off >= size_of_val(data) as vk::DeviceSize,
                "buffer size({}){} is too small for data({})",
                self.get_size(buffer) - off,
                if off > 0 {
                    format!("-off({off})")
                } else {
                    String::new()
                },
                size_of_val(data)
            );
            let mem_ptr = self.map(buffer).byte_add(off as usize);
            (data as *mut T as *mut u8).copy_from_nonoverlapping(mem_ptr, size_of_val(data));
        }
    }

    pub fn write_mapped<T: ?Sized>(&mut self, buffer: vk::Buffer, data: &T) {
        self.write_mapped_off(buffer, data, 0);
    }

    pub fn read_mapped<T: ?Sized>(&mut self, buffer: vk::Buffer, data: &mut T) {
        self.read_mapped_off(buffer, data, 0);
    }

    pub fn get_mem(&self, buffer: vk::Buffer) -> vk::DeviceMemory {
        self.buf_mems[&buffer.as_raw()].mem
    }

    pub fn get_size(&self, buffer: vk::Buffer) -> vk::DeviceSize {
        self.buf_mems[&buffer.as_raw()].size
    }

    pub fn get_props(&self, buffer: vk::Buffer) -> vk::MemoryPropertyFlags {
        self.buf_mems[&buffer.as_raw()].props
    }

    pub fn get_usage(&self, buffer: vk::Buffer) -> vk::BufferUsageFlags {
        self.buf_mems[&buffer.as_raw()].usage
    }

    pub fn is_mappable(&self, buffer: vk::Buffer) -> bool {
        self.buf_mems[&buffer.as_raw()]
            .props
            .contains(vk::MemoryPropertyFlags::HOST_VISIBLE)
    }

    pub fn get_mapped_range(&self, buffer: vk::Buffer) -> Option<&MappedRange> {
        self.buf_mems[&buffer.as_raw()].mapped_range.as_ref()
    }

    fn find_mem_type_idx(mem_type_bits: u32, props: vk::MemoryPropertyFlags) -> u32 {
        let need_device_local = props.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL);
        for (i, mem_type) in gpu_mem_props().memory_types.iter().enumerate() {
            // memory type not supported by required properties
            if (mem_type_bits & (1 << i)) == 0 {
                continue;
            }
            // memory type doesn't have required properties
            if !mem_type.property_flags.contains(props) {
                continue;
            }
            // try to use device local memory heap if requested, otherwise use any
            let mem_heap_flags = gpu_mem_props().memory_heaps[mem_type.heap_index as usize].flags;
            let is_device_local = mem_heap_flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL);
            if !need_device_local || is_device_local {
                return i as u32;
            }
        }
        panic!("Failed to find suitable memory type!")
    }
}
