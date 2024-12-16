use super::render_context::RenderContext;
use super::vulkan::pipeline::GraphicsPipelineInfo;

use crate::*;

lazy_static! {
    pub static ref CTX: Mutex<RenderContext> = Mutex::new(RenderContext::new());
}

#[derive(Clone, Copy)]
struct Frame {
    img_available: vk::Semaphore,
    render_done: vk::Semaphore,
    prev_frame_done: vk::Fence,
}

impl Frame {
    fn new() -> Self {
        Self {
            img_available: unsafe {
                DEVICE
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                    .unwrap()
            },
            render_done: unsafe {
                DEVICE
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                    .unwrap()
            },
            prev_frame_done: unsafe {
                DEVICE
                    .create_fence(
                        &vk::FenceCreateInfo::default().flags(vk::FenceCreateFlags::SIGNALED),
                        None,
                    )
                    .unwrap()
            },
        }
    }

    fn wait(&self) {
        unsafe {
            DEVICE
                .wait_for_fences(&[self.prev_frame_done], true, u64::MAX)
                .unwrap();
            DEVICE.reset_fences(&[self.prev_frame_done]).unwrap();
        }
    }

    fn acquire_img(&self, window: &mut WindowData) -> u32 {
        if window.swapchain.swapchain == vk::SwapchainKHR::null() {
            window.recreate_swapchain();
        }
        unsafe {
            SWAPCHAIN_LOADER
                .acquire_next_image(
                    window.swapchain.swapchain,
                    u64::MAX,
                    self.img_available,
                    vk::Fence::null(),
                )
                .unwrap()
                .0
        }
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Renderer {
    frames: [Frame; Self::FRAMES],
    current_frame: usize,
    image_index: u32,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    const FRAMES: usize = 1;

    pub fn new() -> Self {
        let ctx = &mut *CTX.lock().unwrap();
        ctx.add_shader("screen");
        ctx.add_pipeline(
            "main",
            "screen",
            GraphicsPipelineInfo::new().dyn_size().blend_attachment(),
            &[],
        );
        let desc_set = ctx.add_desc_set("global uniform", "screen", 0);
        ctx.add_cmds_numbered("render", Self::FRAMES);

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
            frames: [Frame::default(); Self::FRAMES],
            current_frame: 0,
            image_index: 0,
        }
    }

    pub fn begin_render(&mut self, window: &mut WindowData) {
        let ctx = &mut *CTX.lock().unwrap();
        unsafe {
            let frame = &self.frames[self.current_frame];
            frame.wait();
            self.image_index = frame.acquire_img(window);
            let img_view = window.swapchain.image_views[self.image_index as usize];

            // record command buffer
            let cmd_name = format!("render{}", self.current_frame);
            ctx.reset_cmd(&cmd_name);
            ctx.begin_cmd(&cmd_name);

            // UNDEFINED -> COLOR_ATTACHMENT_OPTIMAL
            DEVICE.cmd_pipeline_barrier2(
                ctx.cmd(),
                &vk::DependencyInfo::default().image_memory_barriers(&[
                    vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                        .src_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                        .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                        .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
                        .old_layout(vk::ImageLayout::UNDEFINED)
                        .new_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .image(window.swapchain.images[self.image_index as usize])
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

    pub fn end_render(&mut self, window: &mut WindowData) {
        let ctx = &mut *CTX.lock().unwrap();
        let cmd_name = ctx.cmd_name().to_owned();

        ctx.end_render();

        // COLOR_ATTACHMENT_OPTIMAL -> PRESENT_SRC_KHR
        unsafe {
            DEVICE.cmd_pipeline_barrier2(
                ctx.cmd(),
                &vk::DependencyInfo::default().image_memory_barriers(&[
                    vk::ImageMemoryBarrier2::default()
                        .src_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                        .src_access_mask(vk::AccessFlags2::NONE)
                        .dst_stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT)
                        .dst_access_mask(vk::AccessFlags2::COLOR_ATTACHMENT_WRITE)
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
        let frame = &self.frames[self.current_frame];
        ctx.submit_cmd(
            &cmd_name,
            *QUEUE,
            &[frame.img_available],
            &[frame.render_done],
            &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
            frame.prev_frame_done,
        );

        // wait(render_finished), present rendered image
        unsafe {
            SWAPCHAIN_LOADER
                .queue_present(
                    *QUEUE,
                    &vk::PresentInfoKHR::default()
                        .wait_semaphores(&[frame.render_done])
                        .swapchains(&[window.swapchain.swapchain])
                        .image_indices(&[self.image_index]),
                )
                .unwrap_or_else(|_| {
                    window.recreate_swapchain();
                    false
                })
        };

        // self.current_frame = (self.current_frame + 1) % Self::FRAMES;
    }

    pub fn clear(&self, image: vk::Image, color: [f32; 4]) {
        unsafe {
            DEVICE.cmd_clear_color_image(
                CTX.lock().unwrap().cmd(),
                image,
                vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                &vk::ClearColorValue { float32: color },
                &[vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(1)
                    .level_count(1)],
            );
        }
    }
}
