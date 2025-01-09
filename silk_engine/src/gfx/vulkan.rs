#[cfg(debug_assertions)]
use std::ffi::c_void;
use std::sync::LazyLock;

pub use ash::vk;
mod gpu_alloc;
pub(super) use gpu_alloc::GpuAlloc;
mod cmd_alloc;
pub(super) use cmd_alloc::CmdAlloc;
mod desc_alloc;
pub(super) use desc_alloc::DescAlloc;
mod dsl_manager;
pub(super) use dsl_manager::*;
mod gpu;
pub use gpu::*;
mod instance;
mod pipeline_layout_manager;
pub use instance::*;
pub(super) use pipeline_layout_manager::PipelineLayoutManager;
mod config;
pub mod pipeline;
pub use pipeline::*;
mod render_pass;
pub(super) use render_pass::RenderPass;
mod image;
pub use image::*;

use crate::{err, log};

#[cfg(debug_assertions)]
struct UserData {
    allocs: crate::HashMap<usize, (std::alloc::Layout, vk::SystemAllocationScope)>,
}

#[cfg(debug_assertions)]
impl UserData {
    const PRINT_SIZE: usize = 512 * 1024;
    fn new() -> Self {
        Self {
            allocs: Default::default(),
        }
    }

    fn sys_alloc_scope_str(sys_alloc_scope: vk::SystemAllocationScope) -> String {
        match sys_alloc_scope {
            vk::SystemAllocationScope::COMMAND => "command",
            vk::SystemAllocationScope::OBJECT => "object",
            vk::SystemAllocationScope::CACHE => "cache",
            vk::SystemAllocationScope::DEVICE => "device",
            vk::SystemAllocationScope::INSTANCE => "instance",
            _ => "unknown",
        }
        .to_string()
    }

    fn log_alloc(
        &mut self,
        layout: std::alloc::Layout,
        sas: vk::SystemAllocationScope,
        ptr: *mut u8,
    ) {
        if layout.size() > Self::PRINT_SIZE {
            log!(
                "vkAlloc: {:?}, align({}), {}, {:016x}",
                crate::util::Mem::b(layout.size()),
                layout.align(),
                Self::sys_alloc_scope_str(sas),
                ptr as usize,
            );
        }
        self.allocs.insert(ptr as usize, (layout, sas));
    }

    fn log_free(&mut self, ptr: *mut std::ffi::c_void) -> std::alloc::Layout {
        let (layout, sas) = self.allocs.remove(&(ptr as usize)).unwrap();
        if layout.size() > Self::PRINT_SIZE {
            log!(
                "vkFree: {:?}, align({}), {}, {:016x}",
                crate::util::Mem::b(layout.size()),
                layout.align(),
                Self::sys_alloc_scope_str(sas),
                ptr as usize,
            );
        }
        layout
    }

    fn log_alloc_internal(size: usize, sas: vk::SystemAllocationScope) {
        if size > Self::PRINT_SIZE {
            log!(
                "vkAlloc(internal): {}, {}",
                crate::util::Mem::b(size),
                Self::sys_alloc_scope_str(sas),
            );
        }
    }

    fn log_free_internal(size: usize, sas: vk::SystemAllocationScope) {
        if size > Self::PRINT_SIZE {
            log!(
                "vkFree(internal): {}, {}",
                crate::util::Mem::b(size),
                Self::sys_alloc_scope_str(sas),
            );
        }
    }
}

#[cfg(debug_assertions)]
unsafe extern "system" fn alloc(
    user_data: *mut c_void,
    size: usize,
    alignment: usize,
    sas: vk::SystemAllocationScope,
) -> *mut c_void {
    let layout = std::alloc::Layout::from_size_align(size, alignment).unwrap();
    let ptr = std::alloc::alloc_zeroed(layout);
    let user_data = &mut *(user_data as *mut UserData);
    user_data.log_alloc(layout, sas, ptr);
    ptr as *mut _
}

#[cfg(debug_assertions)]
unsafe extern "system" fn free(user_data: *mut c_void, ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let user_data = &mut *(user_data as *mut UserData);
    let layout = user_data.log_free(ptr);
    std::alloc::dealloc(ptr as *mut _, layout)
}

#[cfg(debug_assertions)]
unsafe extern "system" fn realloc(
    _user_data: *mut c_void,
    ptr: *mut c_void,
    size: usize,
    alignment: usize,
    _sas: vk::SystemAllocationScope,
) -> *mut c_void {
    let layout = std::alloc::Layout::from_size_align(size, alignment).unwrap();
    let new_ptr = std::alloc::realloc(ptr as *mut _, layout, size);
    if new_ptr.is_null() {
        std::alloc::dealloc(ptr as *mut _, layout);
        return std::ptr::null_mut();
    }
    new_ptr as *mut _
}

#[cfg(debug_assertions)]
unsafe extern "system" fn internal_alloc(
    _user_data: *mut c_void,
    size: usize,
    _alloc_type: vk::InternalAllocationType,
    sas: vk::SystemAllocationScope,
) {
    UserData::log_alloc_internal(size, sas);
}

#[cfg(debug_assertions)]
unsafe extern "system" fn internal_free(
    _user_data: *mut c_void,
    size: usize,
    _alloc_type: vk::InternalAllocationType,
    sas: vk::SystemAllocationScope,
) {
    UserData::log_free_internal(size, sas);
}

#[cfg(debug_assertions)]
static ALLOC_CALLBACKS: LazyLock<Option<vk::AllocationCallbacks<'static>>> = LazyLock::new(|| {
    let user_data = Box::new(UserData::new());
    let user_data_ptr = Box::into_raw(user_data) as *mut c_void;
    Some(
        vk::AllocationCallbacks::default()
            .pfn_allocation(Some(alloc))
            .pfn_free(Some(free))
            .pfn_internal_allocation(Some(internal_alloc))
            .pfn_internal_free(Some(internal_free))
            .pfn_reallocation(Some(realloc))
            .user_data(user_data_ptr),
    )
});

pub fn alloc_callbacks() -> Option<&'static vk::AllocationCallbacks<'static>> {
    #[cfg(debug_assertions)]
    {
        ALLOC_CALLBACKS.as_ref()
    }
    #[cfg(not(debug_assertions))]
    None
}

static ENTRY: LazyLock<ash::Entry> =
    LazyLock::new(|| unsafe { ash::Entry::load().expect("Failed to load Vulkan") });

static QUEUE_FAMILY_PROPS: LazyLock<Vec<vk::QueueFamilyProperties>> = LazyLock::new(|| unsafe {
    let queue_family_props_len =
        instance().get_physical_device_queue_family_properties2_len(physical_gpu());
    let mut queue_family_props =
        vec![vk::QueueFamilyProperties2::default(); queue_family_props_len];
    instance()
        .get_physical_device_queue_family_properties2(physical_gpu(), &mut queue_family_props);
    queue_family_props
        .into_iter()
        .map(|qfp| qfp.queue_family_properties)
        .collect()
});

static QUEUE_FAMILY_INDEX: LazyLock<u32> = LazyLock::new(|| {
    QUEUE_FAMILY_PROPS
        .iter()
        .position(|&queue_family_props| {
            queue_family_props.queue_flags.contains(
                vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER,
            )
        })
        .unwrap_or_default() as u32
});

static QUEUE: LazyLock<vk::Queue> =
    LazyLock::new(|| unsafe { gpu().get_device_queue(*QUEUE_FAMILY_INDEX, 0) });

pub fn gpu_idle() {
    crate::scope_time!("GPU idle");
    unsafe { gpu().device_wait_idle().unwrap() };
}

pub fn queue_idle() {
    crate::scope_time!("Queue idle");
    unsafe { gpu().queue_wait_idle(*QUEUE).unwrap() };
}

pub fn entry() -> &'static ash::Entry {
    &ENTRY
}

pub fn queue_family_index() -> u32 {
    *QUEUE_FAMILY_INDEX
}

pub fn queue() -> vk::Queue {
    *QUEUE
}

pub fn samples_u32_to_vk(samples: u32) -> vk::SampleCountFlags {
    match samples {
        1 => vk::SampleCountFlags::TYPE_1,
        2 => vk::SampleCountFlags::TYPE_2,
        4 => vk::SampleCountFlags::TYPE_4,
        8 => vk::SampleCountFlags::TYPE_8,
        16 => vk::SampleCountFlags::TYPE_16,
        32 => vk::SampleCountFlags::TYPE_32,
        64 => vk::SampleCountFlags::TYPE_64,
        _ => {
            err!("invalid sample count");
            vk::SampleCountFlags::TYPE_1
        }
    }
}

pub fn format_size(format: vk::Format) -> u32 {
    match format {
        vk::Format::R4G4_UNORM_PACK8 => 1,
        vk::Format::R4G4B4A4_UNORM_PACK16 => 2,
        vk::Format::B4G4R4A4_UNORM_PACK16 => 2,
        vk::Format::R5G6B5_UNORM_PACK16 => 2,
        vk::Format::B5G6R5_UNORM_PACK16 => 2,
        vk::Format::R5G5B5A1_UNORM_PACK16 => 2,
        vk::Format::B5G5R5A1_UNORM_PACK16 => 2,
        vk::Format::A1R5G5B5_UNORM_PACK16 => 2,
        vk::Format::R8_UNORM => 1,
        vk::Format::R8_SNORM => 1,
        vk::Format::R8_USCALED => 1,
        vk::Format::R8_SSCALED => 1,
        vk::Format::R8_UINT => 1,
        vk::Format::R8_SINT => 1,
        vk::Format::R8_SRGB => 1,
        vk::Format::R8G8_UNORM => 2,
        vk::Format::R8G8_SNORM => 2,
        vk::Format::R8G8_USCALED => 2,
        vk::Format::R8G8_SSCALED => 2,
        vk::Format::R8G8_UINT => 2,
        vk::Format::R8G8_SINT => 2,
        vk::Format::R8G8_SRGB => 2,
        vk::Format::R8G8B8_UNORM => 3,
        vk::Format::R8G8B8_SNORM => 3,
        vk::Format::R8G8B8_USCALED => 3,
        vk::Format::R8G8B8_SSCALED => 3,
        vk::Format::R8G8B8_UINT => 3,
        vk::Format::R8G8B8_SINT => 3,
        vk::Format::R8G8B8_SRGB => 3,
        vk::Format::B8G8R8_UNORM => 3,
        vk::Format::B8G8R8_SNORM => 3,
        vk::Format::B8G8R8_USCALED => 3,
        vk::Format::B8G8R8_SSCALED => 3,
        vk::Format::B8G8R8_UINT => 3,
        vk::Format::B8G8R8_SINT => 3,
        vk::Format::B8G8R8_SRGB => 3,
        vk::Format::R8G8B8A8_UNORM => 4,
        vk::Format::R8G8B8A8_SNORM => 4,
        vk::Format::R8G8B8A8_USCALED => 4,
        vk::Format::R8G8B8A8_SSCALED => 4,
        vk::Format::R8G8B8A8_UINT => 4,
        vk::Format::R8G8B8A8_SINT => 4,
        vk::Format::R8G8B8A8_SRGB => 4,
        vk::Format::B8G8R8A8_UNORM => 4,
        vk::Format::B8G8R8A8_SNORM => 4,
        vk::Format::B8G8R8A8_USCALED => 4,
        vk::Format::B8G8R8A8_SSCALED => 4,
        vk::Format::B8G8R8A8_UINT => 4,
        vk::Format::B8G8R8A8_SINT => 4,
        vk::Format::B8G8R8A8_SRGB => 4,
        vk::Format::A8B8G8R8_UNORM_PACK32 => 4,
        vk::Format::A8B8G8R8_SNORM_PACK32 => 4,
        vk::Format::A8B8G8R8_USCALED_PACK32 => 4,
        vk::Format::A8B8G8R8_SSCALED_PACK32 => 4,
        vk::Format::A8B8G8R8_UINT_PACK32 => 4,
        vk::Format::A8B8G8R8_SINT_PACK32 => 4,
        vk::Format::A8B8G8R8_SRGB_PACK32 => 4,
        vk::Format::A2R10G10B10_UNORM_PACK32 => 4,
        vk::Format::A2R10G10B10_SNORM_PACK32 => 4,
        vk::Format::A2R10G10B10_USCALED_PACK32 => 4,
        vk::Format::A2R10G10B10_SSCALED_PACK32 => 4,
        vk::Format::A2R10G10B10_UINT_PACK32 => 4,
        vk::Format::A2R10G10B10_SINT_PACK32 => 4,
        vk::Format::A2B10G10R10_UNORM_PACK32 => 4,
        vk::Format::A2B10G10R10_SNORM_PACK32 => 4,
        vk::Format::A2B10G10R10_USCALED_PACK32 => 4,
        vk::Format::A2B10G10R10_SSCALED_PACK32 => 4,
        vk::Format::A2B10G10R10_UINT_PACK32 => 4,
        vk::Format::A2B10G10R10_SINT_PACK32 => 4,
        vk::Format::R16_UNORM => 2,
        vk::Format::R16_SNORM => 2,
        vk::Format::R16_USCALED => 2,
        vk::Format::R16_SSCALED => 2,
        vk::Format::R16_UINT => 2,
        vk::Format::R16_SINT => 2,
        vk::Format::R16_SFLOAT => 2,
        vk::Format::R16G16_UNORM => 4,
        vk::Format::R16G16_SNORM => 4,
        vk::Format::R16G16_USCALED => 4,
        vk::Format::R16G16_SSCALED => 4,
        vk::Format::R16G16_UINT => 4,
        vk::Format::R16G16_SINT => 4,
        vk::Format::R16G16_SFLOAT => 4,
        vk::Format::R16G16B16_UNORM => 6,
        vk::Format::R16G16B16_SNORM => 6,
        vk::Format::R16G16B16_USCALED => 6,
        vk::Format::R16G16B16_SSCALED => 6,
        vk::Format::R16G16B16_UINT => 6,
        vk::Format::R16G16B16_SINT => 6,
        vk::Format::R16G16B16_SFLOAT => 6,
        vk::Format::R16G16B16A16_UNORM => 8,
        vk::Format::R16G16B16A16_SNORM => 8,
        vk::Format::R16G16B16A16_USCALED => 8,
        vk::Format::R16G16B16A16_SSCALED => 8,
        vk::Format::R16G16B16A16_UINT => 8,
        vk::Format::R16G16B16A16_SINT => 8,
        vk::Format::R16G16B16A16_SFLOAT => 8,
        vk::Format::R32_UINT => 4,
        vk::Format::R32_SINT => 4,
        vk::Format::R32_SFLOAT => 4,
        vk::Format::R32G32_UINT => 8,
        vk::Format::R32G32_SINT => 8,
        vk::Format::R32G32_SFLOAT => 8,
        vk::Format::R32G32B32_UINT => 12,
        vk::Format::R32G32B32_SINT => 12,
        vk::Format::R32G32B32_SFLOAT => 12,
        vk::Format::R32G32B32A32_UINT => 16,
        vk::Format::R32G32B32A32_SINT => 16,
        vk::Format::R32G32B32A32_SFLOAT => 16,
        vk::Format::R64_UINT => 8,
        vk::Format::R64_SINT => 8,
        vk::Format::R64_SFLOAT => 8,
        vk::Format::R64G64_UINT => 16,
        vk::Format::R64G64_SINT => 16,
        vk::Format::R64G64_SFLOAT => 16,
        vk::Format::R64G64B64_UINT => 24,
        vk::Format::R64G64B64_SINT => 24,
        vk::Format::R64G64B64_SFLOAT => 24,
        vk::Format::R64G64B64A64_UINT => 32,
        vk::Format::R64G64B64A64_SINT => 32,
        vk::Format::R64G64B64A64_SFLOAT => 32,
        vk::Format::B10G11R11_UFLOAT_PACK32 => 4,
        vk::Format::E5B9G9R9_UFLOAT_PACK32 => 4,
        vk::Format::D16_UNORM => 2,
        vk::Format::X8_D24_UNORM_PACK32 => 4,
        vk::Format::D32_SFLOAT => 4,
        vk::Format::S8_UINT => 1,
        vk::Format::D16_UNORM_S8_UINT => 3,
        vk::Format::D24_UNORM_S8_UINT => 4,
        vk::Format::D32_SFLOAT_S8_UINT => 5,
        _ => 0,
    }
}
