use crate::*;

// TODO: make more efficient, implement actual allocator strategy and remove redundant calculations
// TODO: have different buffers for different properties of memory
// TODO: allocate vertex/index/uniform buffers from single pre-allocated buffer with suitable memory properties
// TODO: when buffer is full, allocate new buffer (maybe copy old data to new buffer)
pub struct BufferAlloc {
    // mem_pools: Vec<vk::DeviceMemory>,
}

impl BufferAlloc {
    pub fn new() -> Self {
        Self {
            // mem_pools: Vec::new(),
        }
    }

    pub fn alloc(
        &self,
        size: u64,
        usage: vk::BufferUsageFlags,
        mem_props: vk::MemoryPropertyFlags,
    ) -> (vk::DeviceMemory, vk::Buffer) {
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
        }; // TODO: free
        unsafe { DEVICE.bind_buffer_memory(buffer, mem, 0).unwrap() };

        print::log(&format!(
            "Alloc {:?} bytes, {:?}, {:?}",
            mem_reqs.size, usage, mem_props
        ));
        (mem, buffer)
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
            if !need_device_local || (need_device_local && is_device_local) {
                return i as u32;
            }
        }
        panic!("Failed to find suitable memory type!")
    }
}
