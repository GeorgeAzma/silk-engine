use std::{
    ops::{Deref, DerefMut},
    sync::{Arc, Weak},
};

use ash::vk;
use bevy_ecs::prelude::*;
use winit::{dpi::PhysicalPosition, window::WindowId};

use crate::{
    prelude::ResultAny,
    vulkan::{device::Device, surface::Surface, swapchain::Swapchain},
};

/// Frame context returned by begin_frame, consumed by end_frame
#[derive(Clone, Copy)]
pub struct Frame {
    pub image_index: u32,
    pub wait_semaphore: vk::Semaphore,
    pub signal_semaphore: vk::Semaphore,
}

struct FrameSync {
    image_available: Vec<vk::Semaphore>,
    render_finished: Vec<vk::Semaphore>,
    last_submitted_cmd: Vec<vk::CommandBuffer>,
    current_frame: usize,
    pending_frame: Option<Frame>,
}

impl FrameSync {
    fn new() -> Self {
        Self {
            image_available: vec![],
            render_finished: vec![],
            last_submitted_cmd: vec![],
            current_frame: 0,
            pending_frame: None,
        }
    }

    fn is_empty(&self) -> bool {
        self.image_available.is_empty()
    }

    fn wait_for_current_frame(&self, mut wait_fn: impl FnMut(vk::CommandBuffer)) {
        if let Some(&last_cmd) = self
            .last_submitted_cmd
            .get(self.current_frame)
            .filter(|&&cmd| cmd != vk::CommandBuffer::null())
        {
            wait_fn(last_cmd);
        }
    }

    fn wait_semaphore(&self) -> vk::Semaphore {
        self.image_available[self.current_frame]
    }

    fn signal_semaphore(&self, image_index: u32) -> vk::Semaphore {
        self.render_finished[image_index as usize]
    }

    fn set_pending(&mut self, frame: Frame) {
        self.pending_frame = Some(frame);
    }

    fn take_pending(&mut self) -> Option<Frame> {
        self.pending_frame.take()
    }

    fn remember_submitted(&mut self, submitted_cmd: vk::CommandBuffer) {
        if let Some(last_cmd) = self.last_submitted_cmd.get_mut(self.current_frame) {
            *last_cmd = submitted_cmd;
        }
    }

    fn advance_to_image(&mut self, image_index: u32) {
        self.current_frame = image_index as usize;
    }

    fn recreate(&mut self, device: &Device, image_count: usize) -> ResultAny {
        self.destroy(device);

        self.image_available
            .resize(image_count, vk::Semaphore::null());
        self.render_finished
            .resize(image_count, vk::Semaphore::null());
        self.last_submitted_cmd
            .resize(image_count, vk::CommandBuffer::null());

        let alloc_callbacks = device.allocation_callbacks();
        for i in 0..image_count {
            let image_available = unsafe {
                device.device.create_semaphore(
                    &vk::SemaphoreCreateInfo::default(),
                    alloc_callbacks.as_ref(),
                )
            }?;
            device.debug_name(image_available, &format!("image_available_{i}"));
            self.image_available[i] = image_available;

            let render_finished = unsafe {
                device.device.create_semaphore(
                    &vk::SemaphoreCreateInfo::default(),
                    alloc_callbacks.as_ref(),
                )
            }?;
            device.debug_name(render_finished, &format!("render_finished_{i}"));
            self.render_finished[i] = render_finished;
        }

        Ok(())
    }

    fn destroy(&mut self, device: &Device) {
        if self.image_available.is_empty() {
            return;
        }

        device.wait();
        let alloc_callbacks = device.allocation_callbacks();
        for &semaphore in &self.image_available {
            unsafe {
                device
                    .device
                    .destroy_semaphore(semaphore, alloc_callbacks.as_ref())
            };
        }
        for &semaphore in &self.render_finished {
            unsafe {
                device
                    .device
                    .destroy_semaphore(semaphore, alloc_callbacks.as_ref())
            };
        }

        self.image_available.clear();
        self.render_finished.clear();
        self.last_submitted_cmd.clear();
        self.current_frame = 0;
        self.pending_frame = None;
    }
}

fn with_default_surface_formats(
    mut formats: Vec<vk::SurfaceFormatKHR>,
) -> Vec<vk::SurfaceFormatKHR> {
    formats.extend([
        vk::SurfaceFormatKHR {
            format: vk::Format::B8G8R8A8_UNORM,
            color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
        },
        vk::SurfaceFormatKHR {
            format: vk::Format::B8G8R8A8_SRGB,
            color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
        },
    ]);
    formats
}

fn with_default_present_modes(mut modes: Vec<vk::PresentModeKHR>) -> Vec<vk::PresentModeKHR> {
    modes.extend([
        vk::PresentModeKHR::MAILBOX,
        vk::PresentModeKHR::FIFO,
        vk::PresentModeKHR::FIFO_RELAXED,
        vk::PresentModeKHR::IMMEDIATE,
    ]);
    modes
}

#[derive(Component)]
pub struct Window {
    pub id: WindowId,
    pub window: winit::window::Window,

    surface: Surface,
    swapchain: Swapchain,
    device: Weak<Device>,

    sync: FrameSync,
}

impl Window {
    pub(crate) fn new(
        device: &Arc<Device>,
        window: winit::window::Window,
        preferred_surface_formats: Vec<vk::SurfaceFormatKHR>,
        preferred_present_modes: Vec<vk::PresentModeKHR>,
    ) -> ResultAny<Self> {
        let mut surface = Surface::new(device.physical_device(), &window)?;

        let mut swapchain = Swapchain::new(device, vk::SwapchainCreateInfoKHR::default());

        let surface_formats = with_default_surface_formats(preferred_surface_formats);
        let present_modes = with_default_present_modes(preferred_present_modes);

        _ = swapchain.recreate_from_surface(
            &mut surface,
            window.inner_size().width,
            window.inner_size().height,
            &surface_formats,
            &present_modes,
        )?;

        Ok(Self {
            id: window.id(),
            window,
            surface,
            swapchain,
            device: Arc::downgrade(device),
            sync: FrameSync::new(),
        })
    }

    pub fn extent(&self) -> vk::Extent2D {
        self.swapchain.create_info().image_extent
    }

    pub fn x(&self) -> i32 {
        let pos = self.window.outer_position().unwrap();
        pos.x
    }

    pub fn y(&self) -> i32 {
        let pos = self.window.outer_position().unwrap();
        pos.y
    }

    pub fn width(&self) -> u32 {
        self.extent().width
    }

    pub fn height(&self) -> u32 {
        self.extent().height
    }

    pub fn move_pos(&self, x: i32, y: i32) {
        let pos = self.window.outer_position().unwrap();
        self.window
            .set_outer_position(PhysicalPosition::new(pos.x + x, pos.y + y));
    }

    pub fn is_minimized(&mut self) -> bool {
        let caps = self.surface.capabilities();
        caps.current_extent.width == 0 || caps.current_extent.height == 0
    }

    pub fn needs_resize(&self) -> bool {
        let current = self.surface.capabilities().current_extent;
        let swapchain = self.swapchain.create_info().image_extent;
        current.width != swapchain.width || current.height != swapchain.height
    }

    pub fn update_swapchain(&mut self) -> ResultAny<()> {
        let extent = self.surface.capabilities().current_extent;
        let _ = self.swapchain.recreate_from_surface(
            &mut self.surface,
            extent.width,
            extent.height,
            &[],
            &[],
        )?;
        self.recreate_sync_objects()?;
        Ok(())
    }

    /// Begin a new frame. Returns None if the window is minimized or the frame should be skipped.
    /// The wait_fn callback is called to wait for the previous frame's command buffer if any.
    /// The returned Frame must be passed to end_frame after rendering.
    pub fn begin_frame(&mut self, mut wait_fn: impl FnMut(vk::CommandBuffer)) -> Option<Frame> {
        if self.needs_resize() {
            self.update_swapchain().ok()?;
        }

        if self.is_minimized() {
            return None;
        }

        if self.sync.is_empty() {
            self.recreate_sync_objects().ok()?;
        }

        self.sync.wait_for_current_frame(&mut wait_fn);

        let device = self.device.upgrade()?;
        let wait_semaphore = self.sync.wait_semaphore();

        let (image_index, suboptimal) = match unsafe {
            device.swapchain_device().acquire_next_image(
                *self.swapchain,
                u64::MAX,
                wait_semaphore,
                vk::Fence::null(),
            )
        } {
            Ok(v) => v,
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR) => {
                self.update_swapchain().ok()?;
                return None;
            }
            Err(_) => return None,
        };

        if suboptimal {
            self.update_swapchain().ok()?;
            return None;
        }

        let signal_semaphore = self.sync.signal_semaphore(image_index);

        let frame = Frame {
            image_index,
            wait_semaphore,
            signal_semaphore,
        };

        self.sync.set_pending(frame);

        Some(frame)
    }

    /// End the current frame and present. Must be called after rendering is submitted.
    /// Pass the command buffer that was submitted so we can wait for it next frame.
    pub fn end_frame(&mut self, queue: vk::Queue, submitted_cmd: vk::CommandBuffer) {
        let Some(frame) = self.sync.take_pending() else {
            return;
        };

        self.sync.remember_submitted(submitted_cmd);

        let Some(device) = self.device.upgrade() else {
            return;
        };

        let swapchains = [*self.swapchain];
        let image_indices = [frame.image_index];
        let signal_semaphores = [frame.signal_semaphore];

        let present_info = vk::PresentInfoKHR::default()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(&image_indices);

        match unsafe {
            device
                .swapchain_device()
                .queue_present(queue, &present_info)
        } {
            Ok(_) => {}
            Err(vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR) => {
                let _ = self.update_swapchain();
            }
            Err(err) => panic!("{err}"),
        }

        self.sync.advance_to_image(frame.image_index);
    }

    fn recreate_sync_objects(&mut self) -> ResultAny {
        let device = self.device.upgrade().ok_or("device dropped")?;
        self.sync.recreate(&device, self.swapchain.images().len())
    }

    pub(crate) fn surface(&self) -> &Surface {
        &self.surface
    }

    pub(crate) fn format(&self) -> vk::Format {
        self.swapchain.create_info().image_format
    }

    pub(crate) fn swapchain(&mut self) -> &mut Swapchain {
        &mut self.swapchain
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        if let Some(device) = self.device.upgrade() {
            self.sync.destroy(&device);
        }
    }
}

impl Deref for Window {
    type Target = winit::window::Window;

    fn deref(&self) -> &Self::Target {
        &self.window
    }
}

impl DerefMut for Window {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.window
    }
}
