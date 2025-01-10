use std::collections::HashMap;

use super::{alloc_callbacks, gpu, QUEUE_FAMILY_INDEX};
use crate::{buddy_alloc::BuddyAlloc, contain_range::ContainRange, gpu_mem_props, ImageInfo};
use ash::vk;
use vk::Handle;

#[derive(Clone)]
struct MemBlock {
    off: vk::DeviceSize,
    mem: vk::DeviceMemory,
    mapped_ranges: ContainRange,
    map_ptr: *mut u8,
}

impl MemBlock {
    fn new(off: vk::DeviceSize, size: vk::DeviceSize, mem_type_idx: u32) -> Self {
        let mem = unsafe {
            gpu()
                .allocate_memory(
                    &vk::MemoryAllocateInfo::default()
                        .allocation_size(size)
                        .memory_type_index(mem_type_idx)
                        .push_next(&mut vk::MemoryPriorityAllocateInfoEXT::default().priority(0.9)),
                    alloc_callbacks(),
                )
                .unwrap()
        };
        #[cfg(debug_assertions)]
        {
            let props = gpu_mem_props().memory_types[mem_type_idx as usize].property_flags;
            let gpu = props.contains(vk::MemoryPropertyFlags::DEVICE_LOCAL);
            let cpu = props.contains(vk::MemoryPropertyFlags::HOST_VISIBLE);
            let ty = match (cpu, gpu) {
                (true, true) => "cpu/gpu",
                (true, false) => "cpu",
                (false, true) => "gpu",
                (false, false) => "",
            };
            let cached = if props.contains(vk::MemoryPropertyFlags::HOST_CACHED) {
                " cached"
            } else {
                ""
            };
            crate::debug_name(
                &format!("{ty}{cached} mem block({})", crate::Mem::b(size as usize)),
                mem,
            );
        }
        Self {
            off,
            mem,
            mapped_ranges: Default::default(),
            map_ptr: std::ptr::null_mut(),
        }
    }
}

#[derive(Clone)]
struct MemPool {
    props: vk::MemoryPropertyFlags,
    mems: Vec<MemBlock>,
    buddy: BuddyAlloc,
    mem_type_idx: u32,
}

impl Default for MemPool {
    fn default() -> Self {
        Self {
            props: vk::MemoryPropertyFlags::empty(),
            mems: vec![],
            buddy: BuddyAlloc::new(0),
            mem_type_idx: 0,
        }
    }
}

/// starts with initial big mem block allocation
/// managed by buddy allocator, when mem block runs out
/// new mem block with 2x size is created managed by same buddy alloc
/// which mem block new alloc goes to is determined by it's offset in buddy alloc
/// TODO: dealloc blocks when empty (maybe after cooldown)
impl MemPool {
    fn new(mem_type_idx: u32) -> Self {
        let mut def = Self::default();
        def.mem_type_idx = mem_type_idx;
        def.props = gpu_mem_props().memory_types[mem_type_idx as usize].property_flags;
        def
    }

    fn init(&mut self) {
        if !self.mems.is_empty() {
            return;
        }
        const BLOCK_SIZE: vk::DeviceSize = 1024 * 1024; // 1 MiB
        let block_size = BLOCK_SIZE.next_power_of_two();
        self.mems = vec![MemBlock::new(0, block_size, self.mem_type_idx)];
        self.buddy = BuddyAlloc::new(block_size as usize);
    }

    fn find_off_mem_block(&mut self, off: vk::DeviceSize) -> &mut MemBlock {
        let mut last_mem_block_idx = 0;
        for i in 1..self.mems.len() {
            let last_mem_block = &self.mems[i - 1];
            if off >= last_mem_block.off && off < self.mems[i].off {
                return &mut self.mems[i - 1];
            }
            last_mem_block_idx = i;
        }
        &mut self.mems[last_mem_block_idx]
    }

    fn alloc(&mut self, size: vk::DeviceSize) -> (vk::DeviceSize, &MemBlock) {
        self.init();
        let off = self.buddy.alloc(size as usize);
        if off == usize::MAX {
            let old_len = self.buddy.len();
            let size2 = (old_len + size as usize).next_power_of_two();
            crate::log!("Mem pool resized: {}", crate::Mem::b(size2));
            self.buddy.resize(size2);
            let new_mem_off = self.buddy.alloc(size as usize);
            let new_mem_size = size2 - old_len;
            self.mems.push(MemBlock::new(
                new_mem_off as vk::DeviceSize,
                new_mem_size as vk::DeviceSize,
                self.mem_type_idx,
            ));
            (0, &self.mems[self.mems.len() - 1])
        } else {
            let mem_block = self.find_off_mem_block(off as vk::DeviceSize);
            (off as vk::DeviceSize - mem_block.off, mem_block)
        }
    }

    fn dealloc(&mut self, offset: vk::DeviceSize, size: vk::DeviceSize) {
        self.buddy.dealloc(offset as usize, size as usize)
    }
}

impl Drop for MemPool {
    fn drop(&mut self) {
        for mem_block in self.mems.iter() {
            if !mem_block.mem.is_null() {
                unsafe { gpu().free_memory(mem_block.mem, alloc_callbacks()) }
            }
        }
    }
}

#[derive(Clone, Copy)]
struct BufferAlloc {
    mem_type_idx: u32,
    offset: vk::DeviceSize,
    buddy_offset: vk::DeviceSize,
    size: vk::DeviceSize,
    aligned_size: vk::DeviceSize,
    usage: vk::BufferUsageFlags,
    mapped_range: (usize, usize),
}

#[derive(Clone)]
struct ImageAlloc {
    mem_type_idx: u32,
    offset: vk::DeviceSize,
    aligned_size: vk::DeviceSize,
    #[allow(unused)] // will use later
    img_info: ImageInfo,
}

pub struct GpuAlloc {
    mem_pools: Vec<MemPool>,
    buf_allocs: HashMap<u64, BufferAlloc>,
    img_allocs: HashMap<u64, ImageAlloc>,
}

impl Default for GpuAlloc {
    fn default() -> Self {
        Self::new()
    }
}

impl GpuAlloc {
    pub fn new() -> Self {
        let mem_type_count = gpu_mem_props().memory_type_count;
        let mut mem_pools = vec![MemPool::default(); mem_type_count as usize];
        for i in 0..mem_type_count {
            mem_pools[i as usize] = MemPool::new(i);
        }
        Self {
            mem_pools,
            buf_allocs: Default::default(),
            img_allocs: Default::default(),
        }
    }

    pub fn alloc_img(
        &mut self,
        img_info: &ImageInfo,
        mem_props: vk::MemoryPropertyFlags,
    ) -> vk::Image {
        let image = img_info.build();
        let mem_reqs = unsafe { gpu().get_image_memory_requirements(image) };
        let mem_type_idx = Self::find_mem_type_idx(mem_reqs.memory_type_bits, mem_props);
        let pool = &mut self.mem_pools[mem_type_idx as usize];
        let aligned_size = mem_reqs.size;
        let (alloc_off, mem_block) = pool.alloc(aligned_size);
        unsafe {
            gpu()
                .bind_image_memory(image, mem_block.mem, alloc_off)
                .unwrap()
        };
        self.img_allocs.insert(
            image.as_raw(),
            ImageAlloc {
                mem_type_idx,
                offset: alloc_off,
                aligned_size,
                img_info: img_info.clone(),
            },
        );
        image
    }

    pub fn dealloc_img(&mut self, image: vk::Image) {
        let img_alloc = self.img_allocs.remove(&image.as_raw()).unwrap();
        self.mem_pools[img_alloc.mem_type_idx as usize]
            .dealloc(img_alloc.offset, img_alloc.aligned_size);
        unsafe {
            gpu().destroy_image(image, alloc_callbacks());
        }
    }

    /// does not copy memory
    pub fn realloc_img(&mut self, image: vk::Image, new_img_info: &ImageInfo) -> vk::Image {
        let pool_props = self.img_props(image);
        self.dealloc_img(image);
        self.alloc_img(new_img_info, pool_props)
    }

    pub fn alloc_buf(
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
        let pool = &mut self.mem_pools[mem_type_idx as usize];
        let aligned_size = mem_reqs.size;
        let (alloc_off, mem_block) = pool.alloc(aligned_size);
        unsafe {
            gpu()
                .bind_buffer_memory(buffer, mem_block.mem, alloc_off)
                .unwrap()
        };
        self.buf_allocs.insert(
            buffer.as_raw(),
            BufferAlloc {
                mem_type_idx,
                offset: alloc_off,
                buddy_offset: mem_block.off + alloc_off,
                size,
                aligned_size,
                usage,
                mapped_range: (0, 0),
            },
        );
        buffer
    }

    pub fn dealloc_buf(&mut self, buffer: vk::Buffer) {
        let buf_alloc = self.buf_allocs.remove(&buffer.as_raw()).unwrap();
        self.mem_pools[buf_alloc.mem_type_idx as usize]
            .dealloc(buf_alloc.offset, buf_alloc.aligned_size);
        unsafe {
            gpu().destroy_buffer(buffer, alloc_callbacks());
        }
    }

    /// does not copy memory
    pub fn realloc_buf(&mut self, buffer: vk::Buffer, new_size: vk::DeviceSize) -> vk::Buffer {
        let buf_alloc = *self.buf_alloc(buffer);
        let pool_props = self.mem_pools[buf_alloc.mem_type_idx as usize].props;
        self.dealloc_buf(buffer);
        self.alloc_buf(new_size, buf_alloc.usage, pool_props)
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
        let buf_alloc = self.buf_allocs.get_mut(&buffer.as_raw()).unwrap();
        let pool = &mut self.mem_pools[buf_alloc.mem_type_idx as usize];
        let mem_block = pool.find_off_mem_block(buf_alloc.buddy_offset);
        let mapped_range = mem_block.mapped_ranges.range();
        let off = off + buf_alloc.offset;
        let size = if size == vk::WHOLE_SIZE {
            buf_alloc.size
        } else {
            size
        };
        let (start, end) = (off as usize, (off + size) as usize);
        if start >= mapped_range.start && end <= mapped_range.end {
            return unsafe { mem_block.map_ptr.byte_add(off as usize) };
        }
        if !(mapped_range.start == 0 && mapped_range.end == 0) {
            unsafe { gpu().unmap_memory(mem_block.mem) }
        }
        let (old_start, old_end) = buf_alloc.mapped_range;
        if old_end > 0 {
            mem_block.mapped_ranges.remove(old_start, old_end);
        }
        buf_alloc.mapped_range = (start, end);
        mem_block.mapped_ranges.add(start, end);
        let mapped_range = mem_block.mapped_ranges.range();
        let mapped_ptr = unsafe {
            gpu()
                .map_memory(
                    mem_block.mem,
                    mapped_range.start as u64,
                    (mapped_range.end - mapped_range.start) as u64,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap()
        } as *mut u8;
        mem_block.map_ptr = unsafe { mapped_ptr.byte_offset(-(mapped_range.start as isize)) };
        crate::log!(
            "Mapped({:x}): off({off}), size({})",
            mem_block.mem.as_raw(),
            if size == vk::WHOLE_SIZE {
                "WHOLE".to_string()
            } else {
                size.to_string()
            }
        );
        unsafe { mem_block.map_ptr.byte_add(off as usize) }
    }

    pub fn unmap(&mut self, buffer: vk::Buffer) {
        let buf_alloc = self.buf_allocs.get_mut(&buffer.as_raw()).unwrap();
        let pool = &mut self.mem_pools[buf_alloc.mem_type_idx as usize];
        let mem_block = pool.find_off_mem_block(buf_alloc.buddy_offset);
        mem_block.mapped_ranges.remove(
            buf_alloc.offset as usize,
            (buf_alloc.offset + buf_alloc.size) as usize,
        );
        let mapped_range = mem_block.mapped_ranges.range();
        if mapped_range.start == 0 && mapped_range.end == 0 {
            unsafe { gpu().unmap_memory(mem_block.mem) }
        }
    }

    pub fn write_mapped_off<T: ?Sized>(
        &mut self,
        buffer: vk::Buffer,
        data: &T,
        off: vk::DeviceSize,
    ) {
        unsafe {
            assert!(
                self.buf_size(buffer) - off >= size_of_val(data) as vk::DeviceSize,
                "buffer size({}){} is too small for data({})",
                self.buf_size(buffer) - off,
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
                self.buf_size(buffer) - off >= size_of_val(data) as vk::DeviceSize,
                "buffer size({}){} is too small for data({})",
                self.buf_size(buffer) - off,
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

    fn img_alloc(&self, image: vk::Image) -> &ImageAlloc {
        &self.img_allocs[&image.as_raw()]
    }

    fn img_pool(&self, image: vk::Image) -> &MemPool {
        &self.mem_pools[self.img_alloc(image).mem_type_idx as usize]
    }

    fn img_props(&self, image: vk::Image) -> vk::MemoryPropertyFlags {
        self.img_pool(image).props
    }

    fn buf_alloc(&self, buffer: vk::Buffer) -> &BufferAlloc {
        &self.buf_allocs[&buffer.as_raw()]
    }

    fn buf_pool(&self, buffer: vk::Buffer) -> &MemPool {
        &self.mem_pools[self.buf_alloc(buffer).mem_type_idx as usize]
    }

    pub fn buf_size(&self, buffer: vk::Buffer) -> vk::DeviceSize {
        self.buf_alloc(buffer).size
    }

    pub fn buf_props(&self, buffer: vk::Buffer) -> vk::MemoryPropertyFlags {
        self.buf_pool(buffer).props
    }

    pub fn buf_usage(&self, buffer: vk::Buffer) -> vk::BufferUsageFlags {
        self.buf_alloc(buffer).usage
    }

    pub fn is_mappable(&self, buffer: vk::Buffer) -> bool {
        self.buf_props(buffer)
            .contains(vk::MemoryPropertyFlags::HOST_VISIBLE)
    }

    fn find_mem_type_idx(mem_type_bits: u32, props: vk::MemoryPropertyFlags) -> u32 {
        let mut mem_type_scores: Vec<(u32, u32)> = gpu_mem_props()
            .memory_types
            .iter()
            .enumerate()
            .filter_map(|(i, mem_type)| {
                if mem_type_bits & (1 << i) != 0 && mem_type.property_flags.contains(props) {
                    let prop_flags = mem_type.property_flags.as_raw();
                    let score = 32 - ((prop_flags ^ props.as_raw()).count_ones());
                    Some((score, i as u32))
                } else {
                    None
                }
            })
            .collect();
        mem_type_scores.sort_unstable_by_key(|(score, _)| *score);
        if let Some((_, best_idx)) = mem_type_scores.last() {
            *best_idx
        } else {
            panic!("Failed to find suitable memory type!")
        }
    }
}

impl Drop for GpuAlloc {
    fn drop(&mut self) {
        for &buf_hnd in self.buf_allocs.keys() {
            if buf_hnd != 0 {
                unsafe { gpu().destroy_buffer(vk::Buffer::from_raw(buf_hnd), alloc_callbacks()) }
            }
        }
        for &img_hnd in self.img_allocs.keys() {
            if img_hnd != 0 {
                unsafe { gpu().destroy_image(vk::Image::from_raw(img_hnd), alloc_callbacks()) }
            }
        }
    }
}
