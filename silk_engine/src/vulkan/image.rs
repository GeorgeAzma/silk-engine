use std::sync::{Arc, Mutex};

use ash::vk::{self, Handle};

use crate::{prelude::ResultAny, vulkan::device::Device};

pub(crate) struct Image {
    handle: vk::Image,
    memory: Mutex<vk::DeviceMemory>,
    view: Mutex<vk::ImageView>,
    layout: Mutex<vk::ImageLayout>,
    owned: bool,
    flags: vk::ImageCreateFlags,
    image_type: vk::ImageType,
    format: vk::Format,
    extent: vk::Extent3D,
    mip_levels: u32,
    array_layers: u32,
    samples: vk::SampleCountFlags,
    tiling: vk::ImageTiling,
    usage: vk::ImageUsageFlags,
    sharing_mode: vk::SharingMode,
    queue_family_indices: Vec<u32>,
    aspect: vk::ImageAspectFlags,
    device: Arc<Device>,
}

impl Image {
    pub(crate) fn new(
        device: &Arc<Device>,
        create_info: &vk::ImageCreateInfo,
        required_memory_properties: vk::MemoryPropertyFlags,
    ) -> ResultAny<Arc<Self>> {
        let image = Self::_new(device, create_info, vk::Image::null())?;
        device
            .allocator()
            .alloc_img(&image, required_memory_properties)?;
        Ok(image)
    }

    pub(crate) fn new_unallocated(
        device: &Arc<Device>,
        create_info: &vk::ImageCreateInfo,
    ) -> ResultAny<Arc<Self>> {
        Self::_new(device, create_info, vk::Image::null())
    }

    pub(crate) fn new_with_image(
        device: &Arc<Device>,
        image: vk::Image,
        create_info: &vk::ImageCreateInfo,
    ) -> ResultAny<Arc<Self>> {
        Self::_new(device, create_info, image)
    }

    fn _new(
        device: &Arc<Device>,
        create_info: &vk::ImageCreateInfo,
        mut image: vk::Image,
    ) -> ResultAny<Arc<Self>> {
        let mut create_info = create_info.clone();
        create_info.samples = create_info.samples.max(vk::SampleCountFlags::TYPE_1);
        create_info.mip_levels = create_info.mip_levels.max(1);
        create_info.array_layers = create_info.array_layers.max(1);

        let owned = image.is_null();
        if owned {
            image = unsafe {
                device
                    .device
                    .create_image(&create_info, device.allocation_callbacks().as_ref())
            }?;
        }

        let queue_family_indices = if create_info.queue_family_index_count > 0 {
            unsafe {
                std::slice::from_raw_parts(
                    create_info.p_queue_family_indices,
                    create_info.queue_family_index_count as usize,
                )
                .to_vec()
            }
        } else {
            vec![]
        };

        Ok(Arc::new(Self {
            handle: image,
            memory: Mutex::new(vk::DeviceMemory::null()),
            view: Mutex::new(vk::ImageView::null()),
            layout: Mutex::new(create_info.initial_layout),
            owned,
            flags: create_info.flags,
            image_type: create_info.image_type,
            format: create_info.format,
            extent: create_info.extent,
            mip_levels: create_info.mip_levels,
            array_layers: create_info.array_layers,
            samples: create_info.samples,
            tiling: create_info.tiling,
            usage: create_info.usage,
            sharing_mode: create_info.sharing_mode,
            queue_family_indices,
            aspect: format_to_aspect(create_info.format),
            device: Arc::clone(device),
        }))
    }

    pub(crate) fn bind_memory(&self, memory: vk::DeviceMemory, offset: u64) -> ResultAny {
        unsafe {
            self.device()
                .device
                .bind_image_memory(self.handle, memory, offset)
        }?;
        *self.memory.lock().unwrap() = memory;
        Ok(())
    }

    pub(crate) fn create_view(&self) -> ResultAny<vk::ImageView> {
        let create_info = vk::ImageViewCreateInfo::default()
            .image(self.handle)
            .view_type(match self.image_type {
                vk::ImageType::TYPE_1D => vk::ImageViewType::TYPE_1D,
                vk::ImageType::TYPE_2D => vk::ImageViewType::TYPE_2D,
                vk::ImageType::TYPE_3D => vk::ImageViewType::TYPE_3D,
                _ => vk::ImageViewType::TYPE_2D,
            })
            .format(self.format)
            .subresource_range(
                vk::ImageSubresourceRange::default()
                    .aspect_mask(self.aspect)
                    .base_array_layer(0)
                    .base_mip_level(0)
                    .layer_count(self.array_layers)
                    .level_count(self.mip_levels),
            )
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::IDENTITY,
                g: vk::ComponentSwizzle::IDENTITY,
                b: vk::ComponentSwizzle::IDENTITY,
                a: vk::ComponentSwizzle::IDENTITY,
            });

        let view = unsafe {
            self.device()
                .device
                .create_image_view(&create_info, self.device().allocation_callbacks().as_ref())
        }?;

        *self.view.lock().unwrap() = view;

        Ok(view)
    }

    pub(crate) fn transition(&self, cmd: vk::CommandBuffer, new_layout: vk::ImageLayout) {
        let old_layout = *self.layout.lock().unwrap();
        if old_layout == new_layout {
            return;
        }

        let (src_stage, src_access) = layout_to_stage_access(old_layout, false);
        let (dst_stage, dst_access) = layout_to_stage_access(new_layout, true);

        let barrier = vk::ImageMemoryBarrier2 {
            src_stage_mask: src_stage,
            src_access_mask: src_access,
            dst_stage_mask: dst_stage,
            dst_access_mask: dst_access,
            old_layout,
            new_layout,
            image: self.handle,
            subresource_range: vk::ImageSubresourceRange {
                aspect_mask: self.aspect,
                base_mip_level: 0,
                level_count: self.mip_levels,
                base_array_layer: 0,
                layer_count: self.array_layers,
            },
            ..Default::default()
        };

        let dep = vk::DependencyInfo {
            image_memory_barrier_count: 1,
            p_image_memory_barriers: &barrier,
            ..Default::default()
        };

        unsafe { self.device().device.cmd_pipeline_barrier2(cmd, &dep) };

        *self.layout.lock().unwrap() = new_layout;
    }

    pub(crate) fn get_memory_requirements(&self) -> vk::MemoryRequirements {
        unsafe {
            self.device()
                .device
                .get_image_memory_requirements(self.handle)
        }
    }

    /// write to image via staging buffer\
    /// uses `queue_family_indices[0]` with `queue[0]` for waited submission
    pub(crate) fn write<T: ?Sized>(&self, data: &T, offset: vk::Offset3D) -> ResultAny {
        let allocator = self.device().allocator();
        let staging_buffer = allocator.staging_buffer(size_of_val(data) as u64)?;
        staging_buffer.write_mapped(data);

        let queue_family_index = self.queue_family_indices[0];
        let cmd_manager = self.device().command_manager(queue_family_index);
        let cmd = cmd_manager.begin()?;

        // transition to TRANSFER_DST_OPTIMAL for copy
        self.transition(cmd, vk::ImageLayout::TRANSFER_DST_OPTIMAL);

        self.copy_from_buffer_cmd(
            cmd,
            &staging_buffer,
            &[vk::BufferImageCopy {
                buffer_offset: 0,
                buffer_row_length: 0,
                buffer_image_height: 0,
                image_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: self.aspect,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: self.array_layers,
                },
                image_offset: offset,
                image_extent: self.extent,
            }],
        );

        let queue = self.device().get_queue(queue_family_index, 0);
        cmd_manager.submit(queue, cmd, &[], &[], &[])?;
        cmd_manager.wait(cmd)?;

        Ok(())
    }

    /// read from image via staging buffer\
    /// uses `queue_family_indices[0]` with `queue[0]` for waited submission
    pub(crate) fn read<T: ?Sized>(&self, data: &mut T, offset: vk::Offset3D) -> ResultAny {
        let allocator = self.device().allocator();
        let staging_buffer = allocator.staging_buffer(size_of_val(data) as u64)?;

        let queue_family_index = self.queue_family_indices[0];
        let cmd_manager = self.device().command_manager(queue_family_index);
        let cmd = cmd_manager.begin()?;

        // transition to TRANSFER_SRC_OPTIMAL for copy
        self.transition(cmd, vk::ImageLayout::TRANSFER_SRC_OPTIMAL);

        self.copy_to_buffer_cmd(
            cmd,
            &staging_buffer,
            &[vk::BufferImageCopy {
                buffer_offset: 0,
                buffer_row_length: 0,
                buffer_image_height: 0,
                image_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: self.aspect,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: self.array_layers,
                },
                image_offset: offset,
                image_extent: self.extent,
            }],
        );

        let queue = self.device().get_queue(queue_family_index, 0);
        cmd_manager.submit(queue, cmd, &[], &[], &[])?;
        cmd_manager.wait(cmd)?;

        staging_buffer.read_mapped(data);

        Ok(())
    }

    /// Copy from buffer with automatic command buffer management
    /// uses `queue_family_indices[0]` with `queue[0]` for waited submission
    pub(crate) fn copy_from_buffer(
        &self,
        source: &super::buffer::Buffer,
        buffer_offset: u64,
        image_offset: vk::Offset3D,
    ) -> ResultAny<()> {
        self.copy_from_buffer_regions(
            source,
            &[vk::BufferImageCopy {
                buffer_offset,
                buffer_row_length: 0,
                buffer_image_height: 0,
                image_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: self.aspect,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: self.array_layers,
                },
                image_offset,
                image_extent: self.extent,
            }],
        )
    }

    /// Copy multiple regions from buffer with automatic command buffer management
    /// uses `queue_family_indices[0]` with `queue[0]` for waited submission
    pub(crate) fn copy_from_buffer_regions(
        &self,
        source: &super::buffer::Buffer,
        regions: &[vk::BufferImageCopy],
    ) -> ResultAny<()> {
        let queue_family_index = self.queue_family_indices[0];
        let cmd_manager = self.device().command_manager(queue_family_index);
        let cmd = cmd_manager.begin()?;

        self.transition(cmd, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
        self.copy_from_buffer_cmd(cmd, source, regions);

        let queue = self.device().get_queue(queue_family_index, 0);
        cmd_manager.submit(queue, cmd, &[], &[], &[])?;
        cmd_manager.wait(cmd)?;
        Ok(())
    }

    /// Copy from buffer using provided command buffer (for batching)
    pub(crate) fn copy_from_buffer_cmd(
        &self,
        command_buffer: vk::CommandBuffer,
        source: &super::buffer::Buffer,
        regions: &[vk::BufferImageCopy],
    ) {
        unsafe {
            self.device().device.cmd_copy_buffer_to_image(
                command_buffer,
                source.handle(),
                self.handle,
                *self.layout.lock().unwrap(),
                regions,
            );
        }
    }

    /// Copy to buffer with automatic command buffer management
    /// uses `queue_family_indices[0]` with `queue[0]` for waited submission
    pub(crate) fn copy_to_buffer(
        &self,
        dest: &super::buffer::Buffer,
        buffer_offset: u64,
        image_offset: vk::Offset3D,
    ) -> ResultAny<()> {
        self.copy_to_buffer_regions(
            dest,
            &[vk::BufferImageCopy {
                buffer_offset,
                buffer_row_length: 0,
                buffer_image_height: 0,
                image_subresource: vk::ImageSubresourceLayers {
                    aspect_mask: self.aspect,
                    mip_level: 0,
                    base_array_layer: 0,
                    layer_count: self.array_layers,
                },
                image_offset,
                image_extent: self.extent,
            }],
        )
    }

    /// Copy multiple regions to buffer with automatic command buffer management
    /// uses `queue_family_indices[0]` with `queue[0]` for waited submission
    pub(crate) fn copy_to_buffer_regions(
        &self,
        dest: &super::buffer::Buffer,
        regions: &[vk::BufferImageCopy],
    ) -> ResultAny<()> {
        let queue_family_index = self.queue_family_indices[0];
        let cmd_manager = self.device().command_manager(queue_family_index);
        let cmd = cmd_manager.begin()?;

        self.transition(cmd, vk::ImageLayout::TRANSFER_SRC_OPTIMAL);
        self.copy_to_buffer_cmd(cmd, dest, regions);

        let queue = self.device().get_queue(queue_family_index, 0);
        cmd_manager.submit(queue, cmd, &[], &[], &[])?;
        cmd_manager.wait(cmd)?;
        Ok(())
    }

    /// Copy to buffer using provided command buffer (for batching)
    pub(crate) fn copy_to_buffer_cmd(
        &self,
        command_buffer: vk::CommandBuffer,
        dest: &super::buffer::Buffer,
        regions: &[vk::BufferImageCopy],
    ) {
        unsafe {
            self.device().device.cmd_copy_image_to_buffer(
                command_buffer,
                self.handle,
                *self.layout.lock().unwrap(),
                dest.handle(),
                regions,
            );
        }
    }

    pub(crate) fn handle(&self) -> vk::Image {
        self.handle
    }

    pub(crate) fn view(&self) -> vk::ImageView {
        *self.view.lock().unwrap()
    }

    pub(crate) fn device(&self) -> &Arc<Device> {
        &self.device
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        self.device.allocator().dealloc_img(self);

        if self.owned && !self.handle.is_null() {
            unsafe {
                self.device
                    .device
                    .destroy_image(self.handle, self.device.allocation_callbacks().as_ref())
            }
        }

        if !self.view().is_null() {
            unsafe {
                self.device
                    .device
                    .destroy_image_view(self.view(), self.device.allocation_callbacks().as_ref())
            };
        }
    }
}

pub(crate) fn format_to_aspect(format: vk::Format) -> vk::ImageAspectFlags {
    match format {
        vk::Format::D16_UNORM => vk::ImageAspectFlags::DEPTH,
        vk::Format::X8_D24_UNORM_PACK32 => vk::ImageAspectFlags::DEPTH,
        vk::Format::D32_SFLOAT => vk::ImageAspectFlags::DEPTH,
        vk::Format::S8_UINT => vk::ImageAspectFlags::STENCIL,
        vk::Format::D16_UNORM_S8_UINT => {
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
        }
        vk::Format::D24_UNORM_S8_UINT => {
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
        }
        vk::Format::D32_SFLOAT_S8_UINT => {
            vk::ImageAspectFlags::DEPTH | vk::ImageAspectFlags::STENCIL
        }
        _ => vk::ImageAspectFlags::COLOR,
    }
}

/// Returns (stage, access) flags for a given image layout
/// `is_dst` indicates whether this is the destination (true) or source (false) of a transition
fn layout_to_stage_access(
    layout: vk::ImageLayout,
    is_dst: bool,
) -> (vk::PipelineStageFlags2, vk::AccessFlags2) {
    match layout {
        vk::ImageLayout::UNDEFINED => (vk::PipelineStageFlags2::NONE, vk::AccessFlags2::NONE),
        vk::ImageLayout::GENERAL => (
            vk::PipelineStageFlags2::ALL_COMMANDS,
            vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
        ),
        vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL => (
            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            if is_dst {
                vk::AccessFlags2::COLOR_ATTACHMENT_WRITE
            } else {
                vk::AccessFlags2::COLOR_ATTACHMENT_READ | vk::AccessFlags2::COLOR_ATTACHMENT_WRITE
            },
        ),
        vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL => (
            vk::PipelineStageFlags2::EARLY_FRAGMENT_TESTS
                | vk::PipelineStageFlags2::LATE_FRAGMENT_TESTS,
            if is_dst {
                vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE
            } else {
                vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_READ
                    | vk::AccessFlags2::DEPTH_STENCIL_ATTACHMENT_WRITE
            },
        ),
        vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => (
            vk::PipelineStageFlags2::FRAGMENT_SHADER,
            vk::AccessFlags2::SHADER_READ,
        ),
        vk::ImageLayout::TRANSFER_SRC_OPTIMAL => (
            vk::PipelineStageFlags2::TRANSFER,
            vk::AccessFlags2::TRANSFER_READ,
        ),
        vk::ImageLayout::TRANSFER_DST_OPTIMAL => (
            vk::PipelineStageFlags2::TRANSFER,
            vk::AccessFlags2::TRANSFER_WRITE,
        ),
        vk::ImageLayout::PRESENT_SRC_KHR => (vk::PipelineStageFlags2::NONE, vk::AccessFlags2::NONE),
        _ => (
            vk::PipelineStageFlags2::ALL_COMMANDS,
            vk::AccessFlags2::MEMORY_READ | vk::AccessFlags2::MEMORY_WRITE,
        ),
    }
}
