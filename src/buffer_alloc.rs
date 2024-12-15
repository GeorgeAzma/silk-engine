use vk::Handle;

use crate::*;

unsafe impl Send for MappedRange {}

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
}

struct MemBlock {
    mem: vk::DeviceMemory,
    size: u64,
    mapped_range: Option<MappedRange>,
}

// TODO: make more efficient, implement actual allocator strategy and remove redundant calculations
// TODO: have different buffers for different properties of memory
// TODO: allocate vertex/index/uniform buffers from single pre-allocated buffer with suitable memory properties
// TODO: when buffer is full, allocate new buffer (maybe copy old data to new buffer)
#[derive(Default)]
pub struct BufferAlloc {
    buf_mems: HashMap<u64, MemBlock>,
}

impl BufferAlloc {
    pub fn new() -> Self {
        Self {
            buf_mems: HashMap::new(),
        }
    }

    pub fn alloc(
        &mut self,
        size: u64,
        usage: vk::BufferUsageFlags,
        mem_props: vk::MemoryPropertyFlags,
    ) -> vk::Buffer {
        let buffer = unsafe {
            DEVICE
                .create_buffer(
                    &vk::BufferCreateInfo::default()
                        .size(size)
                        .usage(usage)
                        .queue_family_indices(&[*QUEUE_FAMILY_INDEX])
                        .sharing_mode(vk::SharingMode::EXCLUSIVE),
                    None,
                )
                .unwrap()
        };
        let mem_reqs = unsafe { DEVICE.get_buffer_memory_requirements(buffer) };
        let mem_type_idx = Self::find_mem_type_idx(mem_reqs.memory_type_bits, mem_props);
        let mem = unsafe {
            DEVICE
                .allocate_memory(
                    &vk::MemoryAllocateInfo::default()
                        .allocation_size(mem_reqs.size)
                        .memory_type_index(mem_type_idx),
                    None,
                )
                .unwrap()
        };
        unsafe { DEVICE.bind_buffer_memory(buffer, mem, 0).unwrap() };
        self.buf_mems.insert(
            buffer.as_raw(),
            MemBlock {
                mem,
                size,
                mapped_range: None,
            },
        );
        log!(
            "Alloc {:?} bytes, {:?}, {:?}",
            mem_reqs.size,
            usage,
            mem_props
        );
        buffer
    }

    pub fn dealloc(&mut self, buffer: vk::Buffer) {
        let mem = self.buf_mems.remove(&buffer.as_raw()).unwrap();
        unsafe {
            DEVICE.destroy_buffer(buffer, None);
            DEVICE.free_memory(mem.mem, None);
        }
    }

    pub fn map(&mut self, buffer: vk::Buffer) -> *mut u8 {
        self.map_range(buffer, 0..0)
    }

    pub fn map_range(&mut self, buffer: vk::Buffer, mut range: std::ops::Range<u64>) -> *mut u8 {
        let MemBlock {
            mem,
            size,
            mapped_range,
        } = self.buf_mems.get_mut(&buffer.as_raw()).unwrap();
        if range.end == 0 {
            range.end = *size;
        }
        if let Some(mr) = mapped_range {
            if mr.contains(&range) {
                return mr.subrange_ptr(&range);
            } else {
                unsafe { DEVICE.unmap_memory(*mem) }
            }
        }
        *mapped_range = Some(MappedRange::new(
            unsafe {
                DEVICE
                    .map_memory(
                        *mem,
                        range.start,
                        range.end - range.start,
                        vk::MemoryMapFlags::empty(),
                    )
                    .unwrap() as *mut u8
            },
            &range,
        ));
        mapped_range.as_ref().unwrap().ptr
    }

    pub fn unmap(&mut self, buffer: vk::Buffer) {
        let MemBlock {
            mem,
            size: _,
            mapped_range,
        } = self.buf_mems.get_mut(&buffer.as_raw()).unwrap();
        *mapped_range = None;
        unsafe { DEVICE.unmap_memory(*mem) }
    }

    pub fn copy<T>(&mut self, buffer: vk::Buffer, data: &T) {
        unsafe {
            let mem_ptr = self.map(buffer);
            mem_ptr.copy_from_nonoverlapping(
                data as *const T as *mut _,
                self.get_size(buffer) as usize,
            );
        }
    }

    pub fn get_mem(&self, buffer: vk::Buffer) -> vk::DeviceMemory {
        self.buf_mems[&buffer.as_raw()].mem
    }

    pub fn get_size(&self, buffer: vk::Buffer) -> u64 {
        self.buf_mems[&buffer.as_raw()].size
    }

    pub fn get_mapped_range(&self, buffer: vk::Buffer) -> Option<&MappedRange> {
        self.buf_mems[&buffer.as_raw()].mapped_range.as_ref()
    }

    fn find_mem_type_idx(mem_type_bits: u32, props: vk::MemoryPropertyFlags) -> u32 {
        let need_device_local = props.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL);
        for (i, mem_type) in GPU_MEMORY_PROPS.memory_types.iter().enumerate() {
            // memory type not supported by required properties
            if (mem_type_bits & (1 << i)) == 0 {
                continue;
            }
            // memory type doesn't have required properties
            if !mem_type.property_flags.contains(props) {
                continue;
            }
            // try to use device local memory heap if requested, otherwise use any
            let mem_heap_flags = GPU_MEMORY_PROPS.memory_heaps[mem_type.heap_index as usize].flags;
            let is_device_local = mem_heap_flags.contains(vk::MemoryHeapFlags::DEVICE_LOCAL);
            if !need_device_local || is_device_local {
                return i as u32;
            }
        }
        panic!("Failed to find suitable memory type!")
    }
}
