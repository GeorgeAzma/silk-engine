use std::sync::{Arc, Mutex};

use ash::vk;
use winit::window::Window;

use crate::{err, window::WindowContext};

use super::{
    gpu, queue, vulkan::pipeline::GraphicsPipeline, write_desc_set_uniform_buffer_whole,
    RenderContext,
};

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
                gpu()
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                    .unwrap()
            },
            render_done: unsafe {
                gpu()
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                    .unwrap()
            },
            prev_frame_done: unsafe {
                gpu()
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
            gpu()
                .wait_for_fences(&[self.prev_frame_done], true, u64::MAX)
                .unwrap();
            gpu().reset_fences(&[self.prev_frame_done]).unwrap();
        }
    }
}

impl Default for Frame {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
#[derive(Default, Clone)]
pub struct GlobalUniform {
    pub resolution: [u32; 2],
    pub mouse_pos: [f32; 2],
    pub time: f32,
    pub dt: f32,
}

pub struct Renderer {
    render_ctx: Arc<Mutex<RenderContext>>,
    window_ctx: Arc<Mutex<WindowContext>>,
    frames: [Frame; Self::FRAMES],
    current_frame: usize,
}

impl Renderer {
    const FRAMES: usize = 1;

    pub(crate) fn new(
        render_ctx: Arc<Mutex<RenderContext>>,
        window_ctx: Arc<Mutex<WindowContext>>,
    ) -> Self {
        {
            std::panic::set_hook(Box::new(|panic_info| {
                if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
                    err!("panic occurred: {s:?}");
                } else if let Some(s) = panic_info.payload().downcast_ref::<String>() {
                    err!("panic occurred: {s:?}");
                } else {
                    err!("panicked");
                }
            }));
            let mut ctx = render_ctx.lock().unwrap();
            ctx.add_shader("shader");
            ctx.add_pipeline(
                "pipeline",
                "shader",
                GraphicsPipeline::new()
                    .dyn_size()
                    .color_attachment(window_ctx.lock().unwrap().surface_format.format)
                    .blend_attachment_empty(),
                &[],
            );
            let desc_set = ctx.add_desc_set("global uniform", "shader", 0);

            ctx.add_cmds_numbered("render", Self::FRAMES);
            ctx.add_cmd("init");

            let uniform_buffer = ctx.add_buffer(
                "global uniform",
                size_of::<GlobalUniform>() as u64,
                vk::BufferUsageFlags::UNIFORM_BUFFER,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );
            write_desc_set_uniform_buffer_whole(desc_set, uniform_buffer, 0);
        }
        Self {
            render_ctx,
            window_ctx,
            frames: [Frame::default(); Self::FRAMES],
            current_frame: 0,
        }
    }

    pub(crate) fn begin_render(&mut self) {
        let frame = &self.frames[self.current_frame];
        frame.wait();
        let mut window_ctx = self.window_ctx.lock().unwrap();
        window_ctx.acquire_img(frame.img_available);

        let mut ctx = self.render_ctx.lock().unwrap();
        let cmd_name = format!("render{}", self.current_frame);
        ctx.reset_cmd(&cmd_name);
        ctx.begin_cmd(&cmd_name);
        ctx.transition_image_layout(
            window_ctx.cur_img(),
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        );

        ctx.begin_render(
            window_ctx.swapchain_size.width,
            window_ctx.swapchain_size.height,
            window_ctx.cur_img_view(),
        );

        ctx.bind_pipeline("pipeline");
        ctx.bind_desc_set("global uniform");
        ctx.draw(3, 1);
    }

    pub(crate) fn end_render(&mut self, window: &Window) {
        let mut ctx = self.render_ctx.lock().unwrap();
        ctx.end_render();

        let mut window_ctx = self.window_ctx.lock().unwrap();
        ctx.transition_image_layout(
            window_ctx.cur_img(),
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
        );
        let cmd_name = ctx.cmd_name().to_owned();
        ctx.end_cmd();

        // wait(image_available), submit cmd, signal(render_finished)
        let frame = &self.frames[self.current_frame];
        ctx.submit_cmd(
            &cmd_name,
            queue(),
            &[frame.img_available],
            &[frame.render_done],
            &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
            frame.prev_frame_done,
        );

        window.pre_present_notify();
        window_ctx.present(&[frame.render_done]);

        // self.current_frame = (self.current_frame + 1) % Self::FRAMES;
    }

    pub fn clear(&self, image: vk::Image, color: [f32; 4]) {
        unsafe {
            gpu().cmd_clear_color_image(
                self.render_ctx.lock().unwrap().cmd(),
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
