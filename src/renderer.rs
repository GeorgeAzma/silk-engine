use crate::*;

pub struct Renderer {
    pub command_buffer: vk::CommandBuffer,
    image_index: u32,
}

impl Renderer {
    pub fn new() -> Self {
        let _ = *PIPELINE;
        let _ = *VERTEX_BUFFER;
        // TODO: figure out simpler way for this
        // TODO: have single ubo that is used to create all ubos, same with ssbo etc.
        // which buffer, it's range, which binding, what desc type
        // desc type can be figured out from desc index and binding
        unsafe {
            DEVICE.update_descriptor_sets(
                &[vk::WriteDescriptorSet::default()
                    .buffer_info(&[vk::DescriptorBufferInfo::default()
                        .buffer(*UNIFORM_BUFFER)
                        .offset(0)
                        .range(vk::WHOLE_SIZE)])
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
                    .dst_binding(0)
                    .dst_set((*DESCRIPTORS)[0])],
                &[],
            )
        };

        Self {
            command_buffer: CMD_ALLOCATOR.alloc(),
            image_index: 0,
        }
    }

    pub fn begin_render(&mut self, window: &WindowData) {
        unsafe {
            // wait for previous frame
            DEVICE
                .wait_for_fences(&[*PREV_FRAME_FINISHED_FENCE], false, u64::MAX)
                .unwrap_or_default();
            DEVICE.reset_fences(&[*PREV_FRAME_FINISHED_FENCE]).unwrap();

            // acquire next image
            let (image_index, _suboptimal) = SWAPCHAIN_LOADER
                .acquire_next_image(
                    window.swapchain.swapchain,
                    u64::MAX,
                    *IMAGE_AVAILABLE_SEMAPHORE,
                    vk::Fence::null(),
                )
                .unwrap();
            self.image_index = image_index;

            // record command buffer
            DEVICE
                .begin_command_buffer(
                    self.command_buffer,
                    &vk::CommandBufferBeginInfo::default()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .unwrap();

            // UNDEFINED -> COLOR_ATTACHMENT_OPTIMAL
            DEVICE.cmd_pipeline_barrier2(
                self.command_buffer,
                &vk::DependencyInfo::default().image_memory_barriers(&[
                    vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::TOP_OF_PIPE)
                        .src_access_mask(vk::AccessFlags2::empty())
                        .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                        .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .image(window.swapchain.images[image_index as usize])
                        .subresource_range(
                            vk::ImageSubresourceRange::default()
                                .aspect_mask(vk::ImageAspectFlags::COLOR)
                                .level_count(1)
                                .layer_count(1),
                        ),
                ]),
            );

            DEVICE.cmd_set_scissor(
                self.command_buffer,
                0,
                &[vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent: vk::Extent2D {
                        width: window.width(),
                        height: window.height(),
                    },
                }],
            );

            DEVICE.cmd_set_viewport(
                self.command_buffer,
                0,
                &[vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: window.width() as f32,
                    height: window.height() as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                }],
            );

            DYNAMIC_RENDERING.cmd_begin_rendering(
                self.command_buffer,
                &vk::RenderingInfo::default()
                    .render_area(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent: vk::Extent2D {
                            width: window.width(),
                            height: window.height(),
                        },
                    })
                    .layer_count(1)
                    .color_attachments(&[vk::RenderingAttachmentInfo::default()
                        .load_op(vk::AttachmentLoadOp::CLEAR)
                        .store_op(vk::AttachmentStoreOp::STORE)
                        .clear_value(vk::ClearValue {
                            color: vk::ClearColorValue {
                                float32: [0.0, 0.0, 0.0, 0.0],
                            },
                        })
                        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .image_view(window.swapchain.image_views[image_index as usize])]),
            );

            DEVICE.cmd_bind_pipeline(
                self.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                *PIPELINE,
            );
            DEVICE.cmd_bind_descriptor_sets(
                self.command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                *PIPELINE_LAYOUT,
                0,
                &[(*DESCRIPTORS)[0]],
                &[],
            );
            DEVICE.cmd_bind_vertex_buffers(self.command_buffer, 0, &[*VERTEX_BUFFER], &[0]);
            DEVICE.cmd_draw(self.command_buffer, 3, 1, 0, 0);
        }
    }

    pub fn end_render(&self, window: &WindowData) {
        unsafe {
            DYNAMIC_RENDERING.cmd_end_rendering(self.command_buffer);

            // COLOR_ATTACHMENT_OPTIMAL -> PRESENT_SRC_KHR
            DEVICE.cmd_pipeline_barrier2(
                self.command_buffer,
                &vk::DependencyInfo::default().image_memory_barriers(&[
                    vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                        .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                        .dst_stage_mask(vk::PipelineStageFlags2::BOTTOM_OF_PIPE)
                        .dst_access_mask(vk::AccessFlags2::empty())
                        .old_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
                        .image(window.swapchain.images[self.image_index as usize])
                        .subresource_range(
                            vk::ImageSubresourceRange::default()
                                .aspect_mask(vk::ImageAspectFlags::COLOR)
                                .level_count(1)
                                .layer_count(1),
                        ),
                ]),
            );

            DEVICE.end_command_buffer(self.command_buffer).unwrap();

            // wait(image_available), submit command buffer, signal(render_finished)
            DEVICE
                .queue_submit(
                    *QUEUE,
                    &[vk::SubmitInfo::default()
                        .command_buffers(&[self.command_buffer])
                        .wait_semaphores(&[*IMAGE_AVAILABLE_SEMAPHORE])
                        .signal_semaphores(&[*RENDER_FINISHED_SEMAPHORE])
                        .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])],
                    *PREV_FRAME_FINISHED_FENCE,
                )
                .unwrap();

            // wait(render_finished), present rendered image
            SWAPCHAIN_LOADER
                .queue_present(
                    *QUEUE,
                    &vk::PresentInfoKHR::default()
                        .wait_semaphores(&[*RENDER_FINISHED_SEMAPHORE])
                        .swapchains(&[window.swapchain.swapchain])
                        .image_indices(&[self.image_index]),
                )
                .unwrap();
        }
    }
}
