use std::sync::RwLock;

use ash::khr;
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
use lazy_static::lazy_static;
pub(super) use pipeline_layout_manager::PipelineLayoutManager;
mod config;
mod debug_marker;
pub use debug_marker::*;
pub mod pipeline;
pub use pipeline::*;
mod render_pass;
pub(super) use render_pass::RenderPass;

use super::cur_cmd;

lazy_static! {
    pub static ref ENTRY: ash::Entry =
        unsafe { ash::Entry::load().expect("Failed to load Vulkan") };
    pub static ref QUEUE_FAMILIES: Vec<vk::QueueFamilyProperties> = unsafe {
        let queue_family_props_len =
            INSTANCE.get_physical_device_queue_family_properties2_len(*GPU);
        let mut queue_family_props =
            vec![vk::QueueFamilyProperties2::default(); queue_family_props_len];
        INSTANCE.get_physical_device_queue_family_properties2(*GPU, &mut queue_family_props);
        queue_family_props
            .into_iter()
            .map(|qfp| qfp.queue_family_properties)
            .collect()
    };
    pub static ref QUEUE_FAMILY_INDEX: u32 = QUEUE_FAMILIES
        .iter()
        .position(|&queue_family_props| {
            queue_family_props.queue_flags.contains(
                vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER,
            )
        })
        .unwrap_or_default() as u32;
    pub static ref QUEUE: vk::Queue = unsafe { DEVICE.get_device_queue(*QUEUE_FAMILY_INDEX, 0) };
    pub static ref PIPELINE_EXEC_PROPS_LOADER: khr::pipeline_executable_properties::Device =
        if cfg!(debug_assertions) {
            khr::pipeline_executable_properties::Device::new(&INSTANCE, &DEVICE)
        } else {
            #[allow(invalid_value)]
            unsafe {
                std::mem::zeroed()
            }
        };

    // allocators/managers (cached)
    pub static ref DESC_ALLOC: RwLock<DescAlloc> = RwLock::new(DescAlloc::new());
    pub static ref CMD_ALLOC: RwLock<CmdAlloc> = RwLock::new(CmdAlloc::new());
    pub static ref BUFFER_ALLOC: RwLock<BufferAlloc> = RwLock::new(BufferAlloc::new());
    pub static ref DSL_MANAGER: RwLock<DSLManager> = RwLock::new(DSLManager::new());
    pub static ref PIPELINE_LAYOUT_MANAGER: RwLock<PipelineLayoutManager> = RwLock::new(PipelineLayoutManager::new());
}

pub fn gpu_idle() {
    unsafe { DEVICE.device_wait_idle().unwrap() };
}

pub fn queue_idle() {
    unsafe { DEVICE.queue_wait_idle(*QUEUE).unwrap() };
}

pub fn transition_image_layout(
    image: vk::Image,
    old_layout: vk::ImageLayout,
    new_layout: vk::ImageLayout,
) {
    let (src_stage, src_access_mask, dst_stage, dst_access_mask) = match (old_layout, new_layout) {
        (vk::ImageLayout::UNDEFINED, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL) => (
            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            vk::AccessFlags2::NONE,
            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
        ),
        (vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL, vk::ImageLayout::PRESENT_SRC_KHR) => (
            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            vk::PipelineStageFlags2::BOTTOM_OF_PIPE,
            vk::AccessFlags2::NONE,
        ),
        _ => panic!("Unsupported layout transition!"),
    };
    unsafe {
        DEVICE.cmd_pipeline_barrier2(
            cur_cmd(),
            &vk::DependencyInfo::default().image_memory_barriers(&[
                vk::ImageMemoryBarrier2::default()
                    .dst_access_mask(dst_access_mask)
                    .src_access_mask(src_access_mask)
                    .src_stage_mask(src_stage)
                    .dst_stage_mask(dst_stage)
                    .image(image)
                    .subresource_range(
                        vk::ImageSubresourceRange::default()
                            .aspect_mask(vk::ImageAspectFlags::COLOR)
                            .layer_count(1)
                            .level_count(1),
                    )
                    .old_layout(old_layout)
                    .new_layout(new_layout),
            ]),
        );
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
