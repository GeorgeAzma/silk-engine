use std::sync::{Arc, Mutex};

use ash::vk;
use winit::window::Window;

use crate::window::WindowContext;

use super::{gpu, queue, RenderContext};

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
            let mut ctx = render_ctx.lock().unwrap();
            ctx.add_cmds_numbered("render", Self::FRAMES);
            ctx.add_cmd("init");
        }
        Self {
            render_ctx,
            window_ctx,
            frames: [Frame::default(); Self::FRAMES],
            current_frame: 0,
        }
    }

    pub(crate) fn begin_frame(&mut self) {
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
    }

    pub(crate) fn end_frame(&mut self, window: &Window) {
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

        #[allow(clippy::modulo_one)]
        {
            self.current_frame = (self.current_frame + 1) % Self::FRAMES;
        }
    }
}
