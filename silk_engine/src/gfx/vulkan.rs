use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{LazyLock, Mutex};

pub use ash::vk;
mod buffer_alloc;
pub(super) use buffer_alloc::BufferAlloc;
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

use crate::log;

static ALLOCS: LazyLock<Mutex<HashMap<usize, std::alloc::Layout>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(debug_assertions)]
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

unsafe extern "system" fn alloc(
    _user_data: *mut c_void,
    size: usize,
    alignment: usize,
    #[allow(unused)] sys_alloc_scope: vk::SystemAllocationScope,
) -> *mut c_void {
    let layout = std::alloc::Layout::from_size_align(size, alignment).unwrap();
    let ptr = std::alloc::alloc_zeroed(layout);
    if size > 1024 {
        log!(
            "vkAlloc: {:?}, align({alignment}), {}, {ptr:?}",
            crate::util::Mem::b(size),
            sys_alloc_scope_str(sys_alloc_scope)
        );
    }
    ALLOCS.lock().unwrap().insert(ptr as usize, layout);
    ptr as *mut _
}

unsafe extern "system" fn free(_user_data: *mut c_void, ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let layout = ALLOCS.lock().unwrap().remove(&(ptr as usize)).unwrap();
    if layout.size() > 1024 {
        log!(
            "vkFree: {}, align({}), {ptr:?}",
            crate::util::Mem::b(layout.size()),
            layout.align()
        );
    }
    std::alloc::dealloc(ptr as *mut _, layout)
}

unsafe extern "system" fn realloc(
    _user_data: *mut c_void,
    ptr: *mut c_void,
    size: usize,
    alignment: usize,
    #[allow(unused)] sys_alloc_scope: vk::SystemAllocationScope,
) -> *mut c_void {
    let layout = std::alloc::Layout::from_size_align(size, alignment).unwrap();
    ALLOCS
        .lock()
        .unwrap()
        .entry(ptr as usize)
        .and_modify(|l| *l = layout)
        .or_insert(layout);
    let new_ptr = std::alloc::realloc(ptr as *mut _, layout, size);
    if new_ptr.is_null() {
        std::alloc::dealloc(ptr as *mut _, layout);
        return std::ptr::null_mut();
    }
    if size > 1024 {
        log!(
            "vkRealloc: {}, align({alignment}), {}, {new_ptr:?}",
            crate::util::Mem::b(size),
            sys_alloc_scope_str(sys_alloc_scope)
        );
    }
    new_ptr as *mut _
}

unsafe extern "system" fn internal_alloc(
    _user_data: *mut c_void,
    size: usize,
    _alloc_type: vk::InternalAllocationType,
    #[allow(unused)] sys_alloc_scope: vk::SystemAllocationScope,
) {
    if size > 1024 {
        log!(
            "vkAlloc(internal): {}, {}",
            crate::util::Mem::b(size),
            sys_alloc_scope_str(sys_alloc_scope)
        );
    }
}

unsafe extern "system" fn internal_free(
    _user_data: *mut c_void,
    size: usize,
    _alloc_type: vk::InternalAllocationType,
    #[allow(unused)] sys_alloc_scope: vk::SystemAllocationScope,
) {
    if size > 1024 {
        log!(
            "vkFree(internal): {}, {}",
            crate::util::Mem::b(size),
            sys_alloc_scope_str(sys_alloc_scope)
        );
    }
}

static ALLOC_CALLBACKS: LazyLock<Option<vk::AllocationCallbacks<'static>>> = LazyLock::new(|| {
    Some(
        vk::AllocationCallbacks::default()
            .pfn_allocation(Some(alloc))
            .pfn_free(Some(free))
            .pfn_internal_allocation(Some(internal_alloc))
            .pfn_internal_free(Some(internal_free))
            .pfn_reallocation(Some(realloc))
            .user_data(std::ptr::null_mut()),
    )
});

pub fn alloc_callbacks() -> Option<&'static vk::AllocationCallbacks<'static>> {
    ALLOC_CALLBACKS.as_ref()
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
    unsafe { gpu().device_wait_idle().unwrap() };
}

pub fn queue_idle() {
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
