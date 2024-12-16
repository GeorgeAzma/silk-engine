use std::sync::RwLock;

use super::render_context::RenderContext;
use super::vulkan::pipeline::GraphicsPipeline;

use crate::*;

lazy_static! {
    static ref CTX: RwLock<RenderContext> = RwLock::new(RenderContext::new());
}

pub fn ctx<'a>() -> std::sync::RwLockWriteGuard<'a, RenderContext> {
    CTX.write().unwrap()
}

pub fn ctxr<'a>() -> std::sync::RwLockReadGuard<'a, RenderContext> {
    CTX.read().unwrap()
}

pub fn cur_cmd() -> vk::CommandBuffer {
    CTX.read().unwrap().cmd()
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
}

impl Default for Frame {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Renderer {
    frames: [Frame; Self::FRAMES],
    current_frame: usize,
}

impl Default for Renderer {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer {
    const FRAMES: usize = 1;

    pub fn new() -> Self {
        ctx().add_shader("shader");
        // let render_pass = ctx().add_render_pass(
        //     "render pass",
        //     RenderPass::new().add(
        //         vk::Format::B8G8R8A8_UNORM,
        //         vk::ImageLayout::UNDEFINED,
        //         vk::ImageLayout::PRESENT_SRC_KHR,
        //         vk::AttachmentLoadOp::CLEAR,
        //         vk::AttachmentStoreOp::STORE,
        //     ),
        // );

        ctx().add_pipeline(
            "pipeline",
            "shader",
            GraphicsPipeline::new().dyn_size().blend_attachment(),
            // .render_pass(render_pass),
            &[],
        );
        let desc_set = ctx().add_desc_set("global uniform", "shader", 0);
        ctx().add_cmds_numbered("render", Self::FRAMES);

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
        }
    }

    pub fn begin_render(&mut self) {
        let frame = &self.frames[self.current_frame];
        frame.wait();
        acquire_img(frame.img_available);
        // record command buffer
        let cmd_name = format!("render{}", self.current_frame);
        ctx().reset_cmd(&cmd_name);
        ctx().begin_cmd(&cmd_name);
        transition_image_layout(
            cur_swap_img(),
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        );

        // ctx().begin_rp(
        //     "render pass",
        //     size.width,
        //     size.height,
        //     &[cur_swap_img_view()],
        // );
        ctx().begin_render(swap_size().width, swap_size().height, cur_swap_img_view());

        ctx().bind_pipeline("pipeline");
        ctx().bind_desc_set("global uniform");
        unsafe {
            DEVICE.cmd_draw(cur_cmd(), 3, 1, 0, 0);
        }
    }

    pub fn end_render(&mut self) {
        let cmd_name = ctxr().cmd_name().to_owned();

        ctx().end_render();

        transition_image_layout(
            cur_swap_img(),
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
        );
        ctx().end_cmd();

        // wait(image_available), submit cmd, signal(render_finished)
        let frame = &self.frames[self.current_frame];
        ctx().submit_cmd(
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
                        .swapchains(&[*SWAPCHAIN.read().unwrap()])
                        .image_indices(&[swap_img_idx() as u32]),
                )
                .unwrap_or_else(|_| {
                    recreate_swapchain();
                    false
                })
        };

        // self.current_frame = (self.current_frame + 1) % Self::FRAMES;
    }

    pub fn clear(&self, image: vk::Image, color: [f32; 4]) {
        unsafe {
            DEVICE.cmd_clear_color_image(
                cur_cmd(),
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
