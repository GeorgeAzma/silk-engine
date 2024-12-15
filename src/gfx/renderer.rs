use super::pipeline::GraphicsPipelineInfo;
use super::render_context::RenderContext;

use crate::*;

pub struct Renderer {
    context: RenderContext,
    image_index: u32,
}

impl Renderer {
    pub fn new() -> Self {
        let mut context = RenderContext::new();
        context.add_shader("screen");
        context.add_pipeline(
            "main",
            "screen",
            GraphicsPipelineInfo::new().dyn_size(),
            &[],
        );
        let desc_set = context.add_desc_set("global uniform", "screen", 0);
        context.add_cmd("render");

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
                    .dst_set(desc_set)],
                &[],
            )
        };

        Self {
            context,
            image_index: 0,
        }
    }

    pub fn begin_render(&mut self, window: &WindowData) {
        let ctx = &mut self.context;
        unsafe {
            // wait prev frame
            DEVICE
                .wait_for_fences(&[*PREV_FRAME_FINISHED_FENCE], false, u64::MAX)
                .unwrap();
            DEVICE.reset_fences(&[*PREV_FRAME_FINISHED_FENCE]).unwrap();

            // acquire next image
            let (image_index, suboptimal) = SWAPCHAIN_LOADER
                .acquire_next_image(
                    window.swapchain.swapchain,
                    u64::MAX,
                    *IMAGE_AVAILABLE_SEMAPHORE,
                    vk::Fence::null(),
                )
                .unwrap();
            if suboptimal {
                warn!("suboptimal swapchain");
            }
            self.image_index = image_index;
            let img_view = window.swapchain.image_views[image_index as usize];

            // record command buffer
            // CMD_ALLOC.reset(ctx.get_cmd("render"));
            ctx.begin_cmd("render");

            // UNDEFINED -> COLOR_ATTACHMENT_OPTIMAL
            DEVICE.cmd_pipeline_barrier2(
                ctx.cmd(),
                &vk::DependencyInfo::default().image_memory_barriers(&[
                    vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                        .src_access_mask(vk::AccessFlags2::NONE)
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

            ctx.begin_render(window.width(), window.height(), img_view);

            ctx.bind_pipeline("main");
            ctx.bind_desc_set("global uniform");
            DEVICE.cmd_draw(ctx.cmd(), 3, 1, 0, 0);
        }
    }

    pub fn end_render(&mut self, window: &WindowData) {
        let ctx = &mut self.context;

        ctx.end_render();

        // COLOR_ATTACHMENT_OPTIMAL -> PRESENT_SRC_KHR
        unsafe {
            DEVICE.cmd_pipeline_barrier2(
                ctx.cmd(),
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
            )
        };

        ctx.end_cmd();

        // wait(image_available), submit cmd, signal(render_finished)
        ctx.submit_cmd(
            "render",
            *QUEUE,
            &[*IMAGE_AVAILABLE_SEMAPHORE],
            &[*RENDER_FINISHED_SEMAPHORE],
            &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
            *PREV_FRAME_FINISHED_FENCE,
        );

        // wait(render_finished), present rendered image
        unsafe {
            SWAPCHAIN_LOADER
                .queue_present(
                    *QUEUE,
                    &vk::PresentInfoKHR::default()
                        .wait_semaphores(&[*RENDER_FINISHED_SEMAPHORE])
                        .swapchains(&[window.swapchain.swapchain])
                        .image_indices(&[self.image_index]),
                )
                .unwrap()
        };
    }
}
