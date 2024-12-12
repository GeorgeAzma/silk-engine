use crate::*;

// TODO: make more efficient, implement actual allocator strategy and remove redundant calculations
pub struct GPUAlloc;

impl GPUAlloc {
    pub fn alloc(
        size: u64,
        usage: vk::BufferUsageFlags,
        mem_props: vk::MemoryPropertyFlags,
    ) -> (vk::DeviceMemory, vk::Buffer) {
        let queue_family_indices = [*QUEUE_FAMILY_INDEX];
        let buffer_info = vk::BufferCreateInfo::default()
            .size(size)
            .usage(usage)
            .queue_family_indices(&queue_family_indices)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);
        let buffer = unsafe { DEVICE.create_buffer(&buffer_info, None).unwrap() };

        let mem_reqs = unsafe { DEVICE.get_buffer_memory_requirements(buffer) };
        print::log(&format!(
            "Alloc {:?} bytes, {:?}, {:?}",
            mem_reqs.size, usage, mem_props
        ));

        let alloc_info = vk::MemoryAllocateInfo::default()
            .allocation_size(mem_reqs.size)
            .memory_type_index(Self::find_mem_type_idx(
                mem_reqs.memory_type_bits,
                mem_props,
            ));
        let mem = unsafe { DEVICE.allocate_memory(&alloc_info, None).unwrap() }; // TODO: free
        unsafe { DEVICE.bind_buffer_memory(buffer, mem, 0).unwrap() };
        (mem, buffer)
    }

    fn find_mem_type_idx(mem_type_bits: u32, props: vk::MemoryPropertyFlags) -> u32 {
        let mem_props = unsafe { INSTANCE.get_physical_device_memory_properties(*GPU) };
        for (i, mem_type) in mem_props.memory_types.iter().enumerate() {
            if (mem_type_bits & (1 << i)) != 0 && mem_type.property_flags.contains(props) {
                return i as u32;
            }
        }
        panic!("Failed to find suitable memory type!")
    }
}
