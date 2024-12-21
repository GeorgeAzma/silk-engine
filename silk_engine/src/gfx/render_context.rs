use std::collections::HashMap;

use ash::vk;
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

use crate::util::Mem;
use crate::{gpu, gpu_idle, scope_time};

use super::shader::Shader;
use super::vulkan::pipeline::{GraphicsPipeline, PipelineStageInfo};
use super::{
    alloc_callbacks, entry, instance, physical_gpu, queue, BufferAlloc, CmdAlloc, DSLManager,
    DescAlloc, PipelineLayoutManager, RenderPass,
};

#[cfg(debug_assertions)]
static DEBUG_UTILS_LOADER: std::sync::LazyLock<ash::ext::debug_utils::Device> =
    std::sync::LazyLock::new(|| ash::ext::debug_utils::Device::new(crate::instance(), gpu()));

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
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), alloc_callbacks())
                    .unwrap()
            },
            render_done: unsafe {
                gpu()
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), alloc_callbacks())
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

struct ShaderData {
    shader: Shader,
    dsls: Vec<vk::DescriptorSetLayout>,
    pipeline_layout: vk::PipelineLayout,
    pipeline_stages: Vec<PipelineStageInfo>,
}

#[derive(Default, Clone)]
struct PipelineData {
    pipeline: vk::Pipeline,
    info: GraphicsPipeline,
    bind_point: vk::PipelineBindPoint,
}

#[derive(Default)]
struct CmdState {
    cmd: vk::CommandBuffer,
    cmd_name: String,
    pipeline_data: PipelineData,
    desc_sets: Vec<vk::DescriptorSet>,
    render_area: vk::Rect2D,
    render_pass: RenderPass,
}

pub struct RenderContext {
    cmd_state: CmdState,
    // allocators
    desc_alloc: DescAlloc,
    cmd_alloc: CmdAlloc,
    pub buffer_alloc: BufferAlloc,
    // managers (hashmap cached)
    dsl_manager: DSLManager,
    pipeline_layout_manager: PipelineLayoutManager,
    // named cached objects
    shaders: HashMap<String, ShaderData>,
    pipelines: HashMap<String, PipelineData>,
    desc_sets: HashMap<String, vk::DescriptorSet>,
    cmds: HashMap<String, vk::CommandBuffer>,
    buffers: HashMap<String, vk::Buffer>,
    render_passes: HashMap<String, RenderPass>,
    // window context
    surface_caps2_loader: ash::khr::get_surface_capabilities2::Instance,
    pub surface: vk::SurfaceKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    surface_present_modes: Vec<vk::PresentModeKHR>,
    swapchain_loader: ash::khr::swapchain::Device,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_images: Vec<vk::Image>,
    pub swapchain_img_views: Vec<vk::ImageView>,
    pub swapchain_size: vk::Extent2D,
    pub swapchain_img_idx: usize,
    // rendering
    frames: [Frame; 1],
    cur_frame: usize,
}

impl RenderContext {
    pub fn new(window: &Window) -> Self {
        let surface_loader = ash::khr::surface::Instance::new(entry(), instance());
        let surface_caps2 = ash::khr::get_surface_capabilities2::Instance::new(entry(), instance());
        let surface = unsafe {
            ash_window::create_surface(
                entry(),
                instance(),
                window.display_handle().unwrap().as_raw(),
                window.window_handle().unwrap().as_raw(),
                None,
            )
            .unwrap()
        };
        let surface_formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(physical_gpu(), surface)
                .unwrap()
        };
        let surface_format = surface_formats
            .iter()
            .find(|&format| format.format == vk::Format::B8G8R8A8_UNORM)
            .cloned()
            .unwrap_or(vk::SurfaceFormatKHR {
                format: vk::Format::B8G8R8A8_UNORM,
                color_space: vk::ColorSpaceKHR::SRGB_NONLINEAR,
            });
        let surface_present_modes = unsafe {
            surface_loader
                .get_physical_device_surface_present_modes(physical_gpu(), surface)
                .unwrap()
        };
        let swapchain_loader = ash::khr::swapchain::Device::new(instance(), gpu());
        let mut slf = Self {
            cmd_state: CmdState::default(),
            desc_alloc: DescAlloc::default(),
            cmd_alloc: CmdAlloc::default(),
            buffer_alloc: BufferAlloc::default(),
            dsl_manager: DSLManager::default(),
            pipeline_layout_manager: PipelineLayoutManager::default(),
            shaders: Default::default(),
            pipelines: Default::default(),
            desc_sets: Default::default(),
            cmds: Default::default(),
            buffers: Default::default(),
            render_passes: Default::default(),
            surface_caps2_loader: surface_caps2,
            surface,
            surface_format,
            surface_present_modes,
            swapchain_loader,
            swapchain: Default::default(),
            swapchain_images: Default::default(),
            swapchain_img_views: Default::default(),
            swapchain_size: Default::default(),
            swapchain_img_idx: Default::default(),
            frames: Default::default(),
            cur_frame: 0,
        };
        {
            slf.add_cmds_numbered("render", slf.frames.len());
            slf.add_cmd("init");
            slf.add_buffer(
                "staging",
                *Mem::mb(64),
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );
        }
        slf
    }

    pub(crate) fn begin_frame(&mut self) {
        let frame = &self.frames[self.cur_frame];
        frame.wait();
        self.acquire_img(frame.img_available);

        let cmd_name = format!("render{}", self.cur_frame);
        self.reset_cmd(&cmd_name);
        self.begin_cmd(&cmd_name);
        self.transition_image_layout(
            self.cur_img(),
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
        );

        let width = self.swapchain_size.width;
        let height = self.swapchain_size.height;
        let img_view = self.cur_img_view();
        self.begin_render(width, height, img_view);
    }

    pub(crate) fn end_frame(&mut self, window: &Window) {
        self.end_render();

        self.transition_image_layout(
            self.cur_img(),
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
        );
        let cmd_name = self.cmd_name().to_owned();
        self.end_cmd();

        // wait(image_available), submit cmd, signal(render_finished)
        let frame = self.frames[self.cur_frame];
        self.submit_cmd(
            &cmd_name,
            queue(),
            &[frame.img_available],
            &[frame.render_done],
            &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
            frame.prev_frame_done,
        );

        window.pre_present_notify();
        self.present(&[frame.render_done]);

        self.cur_frame = (self.cur_frame + 1) % self.frames.len();
    }

    pub fn shader(&self, name: &str) -> &Shader {
        &self.shaders[name].shader
    }

    pub fn add_shader(&mut self, name: &str) -> &Shader {
        let shader_data = {
            let shader = Shader::new(name);
            let dsls = shader
                .get_dsl_bindings()
                .iter()
                .map(|dslb| self.dsl_manager.get(dslb))
                .collect::<Vec<_>>();
            let module = shader.create_module();
            debug_name(name, module);
            let pipeline_layout = self.pipeline_layout_manager.get(&dsls);
            let pipeline_stages = shader.get_pipeline_stages(module);
            ShaderData {
                shader,
                dsls,
                pipeline_layout,
                pipeline_stages,
            }
        };
        let shader_data = self.shaders.entry(name.to_string()).or_insert(shader_data);
        &shader_data.shader
    }

    pub fn add_render_pass(&mut self, name: &str, mut render_pass: RenderPass) -> vk::RenderPass {
        let rp = render_pass.build();
        let inserted = self
            .render_passes
            .insert(name.to_string(), render_pass)
            .is_none();
        assert!(inserted, "render pass already exists: {name}");
        debug_name(name, rp);
        rp
    }

    pub fn render_pass(&mut self, name: &str) -> vk::RenderPass {
        self.render_passes[name].render_pass
    }

    pub fn add_pipeline(
        &mut self,
        name: &str,
        shader_name: &str,
        pipeline_info: GraphicsPipeline,
        vert_input_bindings: &[(bool, Vec<u32>)],
    ) -> vk::Pipeline {
        scope_time!("Create pipeline({name})");
        let shader_data = &self.shaders[shader_name];
        let pipeline_info = pipeline_info
            .layout(shader_data.pipeline_layout)
            .stages(&shader_data.pipeline_stages)
            .vert_layout(&shader_data.shader, vert_input_bindings);
        let pipeline = pipeline_info.build();
        let inserted = self
            .pipelines
            .insert(
                name.to_string(),
                PipelineData {
                    pipeline,
                    info: pipeline_info,
                    bind_point: vk::PipelineBindPoint::GRAPHICS,
                },
            )
            .is_none();
        assert!(inserted, "pipeline already exists: {name}");
        debug_name(name, pipeline);
        pipeline
    }

    pub fn add_desc_set(
        &mut self,
        name: &str,
        shader_name: &str,
        group: usize,
    ) -> vk::DescriptorSet {
        let dsl = self.shaders[shader_name].dsls[group];
        let desc_set = self.desc_alloc.alloc_one(dsl);
        let inserted = self.desc_sets.insert(name.to_string(), desc_set).is_none();
        assert!(inserted, "desc set already exists: {name}");
        debug_name(name, desc_set);
        desc_set
    }

    pub fn add_desc_sets(&mut self, names: &[&str], shader_name: &str) -> Vec<vk::DescriptorSet> {
        let dsls = &self.shaders[shader_name].dsls;
        let desc_sets = self.desc_alloc.alloc(dsls);
        for (name, &desc_set) in names.iter().zip(desc_sets.iter()) {
            let inserted = self.desc_sets.insert(name.to_string(), desc_set).is_none();
            assert!(inserted, "desc set already exists: {name}");
            debug_name(name, desc_set);
        }
        desc_sets
    }

    pub fn desc_set(&self, name: &str) -> vk::DescriptorSet {
        self.desc_sets[name]
    }

    pub fn add_cmds(&mut self, names: &[&str]) -> Vec<vk::CommandBuffer> {
        let cmds = self.cmd_alloc.alloc(names.len() as u32);
        for (&cmd, &name) in cmds.iter().zip(names.iter()) {
            let inserted = self.cmds.insert(name.to_string(), cmd).is_none();
            assert!(inserted, "cmd buf already exists: {name}");
            debug_name(name, cmd);
        }
        cmds
    }

    pub fn add_cmds_numbered(&mut self, name: &str, count: usize) -> Vec<vk::CommandBuffer> {
        let names: Vec<_> = (0..count).map(|i| format!("{name}{i}")).collect();
        self.add_cmds(&names.iter().map(|s| s.as_str()).collect::<Vec<_>>())
    }

    pub fn add_cmd(&mut self, name: &str) -> vk::CommandBuffer {
        self.add_cmds(&[name])[0]
    }

    pub fn remove_cmds(&mut self, names: &[&str]) {
        let cmds: Vec<_> = names
            .iter()
            .map(|name| self.cmds.remove(name.to_owned()).unwrap())
            .collect();
        self.cmd_alloc.dealloc(&cmds);
    }

    pub fn remove_cmd(&mut self, name: &str) {
        self.remove_cmds(&[name])
    }

    pub fn reset_cmds(&mut self, names: &[&str]) {
        for name in names.iter() {
            self.cmd_alloc.reset(self.cmds[&name.to_string()]);
        }
    }

    pub fn reset_cmd(&mut self, name: &str) {
        self.reset_cmds(&[name])
    }

    pub fn add_buffer(
        &mut self,
        name: &str,
        size: u64,
        usage: vk::BufferUsageFlags,
        mem_props: vk::MemoryPropertyFlags,
    ) -> vk::Buffer {
        let buf = self.buffer_alloc.alloc(size, usage, mem_props);
        let inserted = self.buffers.insert(name.to_string(), buf).is_none();
        assert!(inserted, "buffer already exists: {name}");
        debug_name(name, buf);
        buf
    }

    pub fn remove_buffer(&mut self, name: &str) {
        let buf = self.buffers.remove(name).unwrap();
        self.buffer_alloc.dealloc(buf);
    }

    pub fn recreate_buffer(&mut self, name: &str, size: u64) -> vk::Buffer {
        let buffer = self.buffer(name);
        *self
            .buffers
            .entry(name.to_string())
            .and_modify(|e| *e = self.buffer_alloc.realloc(buffer, size))
            .or_default()
    }

    pub fn buffer(&self, name: &str) -> vk::Buffer {
        self.buffers[name]
    }

    pub fn buffer_size(&self, name: &str) -> u64 {
        self.buffer_alloc.get_size(self.buffer(name))
    }

    pub fn get_cmd(&self, name: &str) -> vk::CommandBuffer {
        self.cmds[name]
    }

    pub fn cmd(&self) -> vk::CommandBuffer {
        let cmd = self.cmd_state.cmd;
        assert_ne!(cmd, Default::default());
        cmd
    }

    pub fn cmd_name(&self) -> &str {
        &self.cmd_state.cmd_name
    }

    pub fn begin_cmd(&mut self, name: &str) -> vk::CommandBuffer {
        let cmd = self.get_cmd(name);
        if self.cmd_state.cmd == cmd {
            return cmd;
        }
        assert!(
            self.cmd_state.cmd == Default::default(),
            "cmd begun when other cmd was running: {name}"
        );
        self.cmd_state.cmd = cmd;
        self.cmd_state.cmd_name = name.to_string();
        unsafe {
            gpu()
                .begin_command_buffer(
                    self.cmd_state.cmd,
                    &vk::CommandBufferBeginInfo::default()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .unwrap();
        }
        self.cmd_state.cmd
    }

    pub fn end_cmd(&mut self) {
        if self.cmd_state.cmd == Default::default() {
            return;
        }
        if self.cmd_state.render_area != Default::default() {
            self.end_render();
        }
        unsafe {
            gpu().end_command_buffer(self.cmd()).unwrap();
        }
        self.cmd_state = Default::default();
    }

    pub fn wait_cmd(&mut self) {
        let cmd = self.cmd_name().to_string();
        let fence = unsafe {
            gpu()
                .create_fence(&vk::FenceCreateInfo::default(), alloc_callbacks())
                .unwrap()
        };
        self.submit_cmd(&cmd, queue(), &[], &[], &[], fence);
        unsafe { gpu().wait_for_fences(&[fence], false, u64::MAX).unwrap() };
        unsafe { gpu().destroy_fence(fence, alloc_callbacks()) };
    }

    pub fn submit_cmd(
        &mut self,
        name: &str,
        queue: vk::Queue,
        wait_semaphores: &[vk::Semaphore],
        signal_semaphores: &[vk::Semaphore],
        wait_dst_stage_mask: &[vk::PipelineStageFlags],
        fence: vk::Fence,
    ) {
        self.submit_cmds(
            &[name],
            queue,
            wait_semaphores,
            signal_semaphores,
            wait_dst_stage_mask,
            fence,
        );
    }

    pub fn submit_cmds(
        &mut self,
        names: &[&str],
        queue: vk::Queue,
        wait_semaphores: &[vk::Semaphore],
        signal_semaphores: &[vk::Semaphore],
        wait_dst_stage_mask: &[vk::PipelineStageFlags],
        fence: vk::Fence,
    ) {
        let cmds = names
            .iter()
            .map(|name| self.get_cmd(name))
            .collect::<Vec<_>>();
        let needs_end = cmds.iter().any(|&cmd| cmd == self.cmd_state.cmd);
        if needs_end {
            self.end_cmd();
        }
        unsafe {
            gpu()
                .queue_submit(
                    queue,
                    &[vk::SubmitInfo::default()
                        .command_buffers(&cmds)
                        .wait_semaphores(wait_semaphores)
                        .signal_semaphores(signal_semaphores)
                        .wait_dst_stage_mask(wait_dst_stage_mask)],
                    fence,
                )
                .unwrap();
        }
    }

    pub fn begin_render(&mut self, width: u32, height: u32, image_view: vk::ImageView) {
        self.cmd_state.render_area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D { width, height },
        };
        unsafe {
            gpu().cmd_begin_rendering(
                self.cmd(),
                &vk::RenderingInfo::default()
                    .render_area(self.cmd_state.render_area)
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
                        .image_view(image_view)]),
            )
        };
    }

    pub fn end_render(&mut self) {
        assert!(
            self.cmd_state.render_area != Default::default(),
            "can't end rendering that has not begun"
        );
        self.cmd_state.render_area = Default::default();
        unsafe {
            if self.cmd_state.render_pass.render_pass == Default::default() {
                gpu().cmd_end_rendering(self.cmd());
            } else {
                gpu().cmd_end_render_pass(self.cmd());
                self.cmd_state.render_pass = Default::default();
            }
        }
    }

    // TODO:
    // pub fn begin_rp(&mut self, name: &str, width: u32, height: u32, img_views: &[vk::ImageView]) {
    //     self.cmd_state.render_area = vk::Rect2D {
    //         offset: vk::Offset2D { x: 0, y: 0 },
    //         extent: vk::Extent2D { width, height },
    //     };
    //     let img_cnt = SWAPCHAIN_IMAGES.read().unwrap().len();
    //     let render_pass = self.render_passes.get_mut(name).unwrap();
    //     if render_pass.framebuffer_size != self.cmd_state.render_area.extent
    //         || render_pass.framebuffers.len() < img_cnt
    //     {
    //         render_pass.recreate_framebuffer(width, height, img_views, img_cnt);
    //     }
    //     self.cmd_state.render_pass = render_pass.clone();
    //     let render_pass = &self.cmd_state.render_pass;
    //     unsafe {
    //         gpu().cmd_begin_render_pass(
    //             self.cmd(),
    //             &vk::RenderPassBeginInfo::default()
    //                 .render_area(self.cmd_state.render_area)
    //                 .clear_values(&[vk::ClearValue {
    //                     color: vk::ClearColorValue {
    //                         float32: [0.0, 0.0, 0.0, 0.0],
    //                     },
    //                 }])
    //                 .render_pass(render_pass.render_pass)
    //                 .framebuffer(render_pass.framebuffers[swap_img_idx()]),
    //             vk::SubpassContents::INLINE,
    //         );
    //     }
    // }

    pub fn bind_pipeline(&mut self, name: &str) {
        let pipeline_data = self.pipelines[name].clone();
        self.cmd_state.pipeline_data = pipeline_data;

        unsafe {
            let dyn_states = &self.cmd_state.pipeline_data.info.dynamic_states;
            let extent = self.cmd_state.render_area.extent;
            if dyn_states.contains(&vk::DynamicState::VIEWPORT) {
                gpu().cmd_set_viewport(
                    self.cmd(),
                    0,
                    &[vk::Viewport {
                        x: 0.0,
                        y: 0.0,
                        width: extent.width as f32,
                        height: extent.height as f32,
                        min_depth: 0.0,
                        max_depth: 1.0,
                    }],
                );
            }
            if dyn_states.contains(&vk::DynamicState::SCISSOR) {
                gpu().cmd_set_scissor(
                    self.cmd(),
                    0,
                    &[vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent,
                    }],
                );
            }
            gpu().cmd_bind_pipeline(
                self.cmd(),
                self.cmd_state.pipeline_data.bind_point,
                self.cmd_state.pipeline_data.pipeline,
            )
        }
    }

    pub fn bind_desc_set(&mut self, name: &str) {
        self.cmd_state.desc_sets = vec![self.desc_sets[name]];
        unsafe {
            gpu().cmd_bind_descriptor_sets(
                self.cmd(),
                self.cmd_state.pipeline_data.bind_point,
                self.cmd_state.pipeline_data.info.layout,
                0,
                &self.cmd_state.desc_sets,
                &[],
            );
        }
    }

    pub fn bind_vbo(&self, name: &str) {
        unsafe {
            gpu().cmd_bind_vertex_buffers(self.cmd(), 0, &[self.buffers[name]], &[0]);
        }
    }

    pub fn bind_ebo(&self, name: &str) {
        unsafe {
            gpu().cmd_bind_index_buffer(self.cmd(), self.buffers[name], 0, vk::IndexType::UINT32);
        }
    }

    pub fn bind_vao(&self, name: &str, index_buffer_offset: vk::DeviceSize) {
        unsafe {
            gpu().cmd_bind_vertex_buffers(self.cmd(), 0, &[self.buffers[name]], &[0]);
            gpu().cmd_bind_index_buffer(
                self.cmd(),
                self.buffers[name],
                index_buffer_offset,
                vk::IndexType::UINT32,
            );
        }
    }

    pub fn draw(&self, vertices: u32, instances: u32) {
        unsafe {
            gpu().cmd_draw(self.cmd(), vertices, instances, 0, 0);
        }
    }

    pub fn draw_indexed(&self, indices: u32, instances: u32) {
        unsafe {
            gpu().cmd_draw_indexed(self.cmd(), indices, instances, 0, 0, 0);
        }
    }

    pub fn transition_image_layout(
        &self,
        image: vk::Image,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
    ) {
        let (src_stage, src_access_mask, dst_stage, dst_access_mask) =
            match (old_layout, new_layout) {
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
            gpu().cmd_pipeline_barrier2(
                self.cmd(),
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

    pub fn copy_buffer_off(
        &mut self,
        src_buffer: vk::Buffer,
        dst_buffer: vk::Buffer,
        src_off: vk::DeviceSize,
        dst_off: vk::DeviceSize,
    ) {
        let cmd = self.begin_cmd("init");
        unsafe {
            // let buffer_usage = self.buffer_alloc.get_usage(dst_buffer);
            // if buffer_usage.contains(vk::BufferUsageFlags::VERTEX_BUFFER)
            //     || buffer_usage.contains(vk::BufferUsageFlags::INDEX_BUFFER)
            // {
            //     gpu().cmd_pipeline_barrier(
            //         cmd,
            //         vk::PipelineStageFlags::VERTEX_INPUT,
            //         vk::PipelineStageFlags::TRANSFER,
            //         vk::DependencyFlags::empty(),
            //         &[],
            //         &[vk::BufferMemoryBarrier::default()
            //             .src_access_mask(vk::AccessFlags::VERTEX_ATTRIBUTE_READ)
            //             .dst_access_mask(vk::AccessFlags::TRANSFER_READ)
            //             .buffer(dst_buffer)
            //             .size(vk::WHOLE_SIZE)],
            //         &[],
            //     );
            // }
            let buf_size = self.buffer_alloc.get_size(src_buffer);
            let buf_size = buf_size.min(self.buffer_alloc.get_size(dst_buffer));
            let copy_region = vk::BufferCopy::default()
                .size(buf_size)
                .src_offset(src_off)
                .dst_offset(dst_off);
            gpu().cmd_copy_buffer(cmd, src_buffer, dst_buffer, &[copy_region]);
        }
        self.wait_cmd();
    }

    pub fn copy_buffer(&mut self, src_buffer: vk::Buffer, dst_buffer: vk::Buffer) {
        self.copy_buffer_off(src_buffer, dst_buffer, 0, 0);
    }

    pub fn write_buffer_off<T: ?Sized>(&mut self, name: &str, data: &T, off: vk::DeviceSize) {
        let buffer = self.buffer(name);
        if self.buffer_alloc.is_mappable(buffer) {
            self.buffer_alloc.write_mapped_off(buffer, data, off);
        } else {
            let staging = self.buffer("staging");
            self.buffer_alloc.write_mapped(staging, data);
            self.copy_buffer_off(staging, buffer, 0, off);
        }
    }

    pub fn read_buffer_off<T: ?Sized>(&mut self, name: &str, data: &mut T, off: vk::DeviceSize) {
        let buffer = self.buffer(name);
        if self.buffer_alloc.is_mappable(buffer) {
            self.buffer_alloc.read_mapped_off(buffer, data, off);
        } else {
            let staging_buffer = self.buffer("staging");
            self.copy_buffer_off(buffer, staging_buffer, off, 0);
            self.buffer_alloc.read_mapped(staging_buffer, data);
        }
    }

    pub fn write_buffer<T: ?Sized>(&mut self, name: &str, data: &T) {
        self.write_buffer_off(name, data, 0);
    }

    pub fn read_buffer<T: ?Sized>(&mut self, name: &str, data: &mut T) {
        self.read_buffer_off(name, data, 0);
    }

    pub fn clear(&self, image: vk::Image, color: [f32; 4]) {
        unsafe {
            gpu().cmd_clear_color_image(
                self.cmd(),
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

    pub fn recreate_swapchain(&mut self) {
        let surface_capabilities = self.surface_capabilities();
        let size = self.swapchain_size;
        let surface_resolution = match surface_capabilities.current_extent.width {
            u32::MAX => vk::Extent2D {
                width: size.width,
                height: size.height,
            },
            _ => surface_capabilities.current_extent,
        };
        if surface_resolution.width == 0
            || surface_resolution.height == 0
            || surface_resolution == size
        {
            return;
        }
        self.swapchain_size = surface_resolution;
        scope_time!(
            "resize {}x{}",
            surface_resolution.width,
            surface_resolution.height
        );
        let pre_transform = if surface_capabilities
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surface_capabilities.current_transform
        };
        let present_mode = self
            .surface_present_modes
            .iter()
            .find(|&mode| *mode == vk::PresentModeKHR::MAILBOX)
            .copied()
            .unwrap_or(vk::PresentModeKHR::FIFO);
        let mut desired_image_count = surface_capabilities.min_image_count + 1;
        if surface_capabilities.max_image_count > 0 {
            desired_image_count = surface_capabilities
                .max_image_count
                .min(desired_image_count);
        }
        // Destroy old swap chain images
        let old_swapchain = self.swapchain;
        self.swapchain = unsafe {
            self.swapchain_loader
                .create_swapchain(
                    &vk::SwapchainCreateInfoKHR::default()
                        .surface(self.surface)
                        .min_image_count(desired_image_count)
                        .image_color_space(self.surface_format.color_space)
                        .image_format(self.surface_format.format)
                        .image_extent(surface_resolution)
                        .image_array_layers(1)
                        .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
                        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                        .pre_transform(pre_transform)
                        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                        .present_mode(present_mode)
                        .old_swapchain(old_swapchain)
                        .clipped(true),
                    None,
                )
                .unwrap()
        };

        if old_swapchain != Default::default() {
            self.swapchain_images.clear();
            unsafe {
                self.swapchain_img_views
                    .drain(..)
                    .for_each(|image_view| gpu().destroy_image_view(image_view, alloc_callbacks()));
            }
            unsafe {
                self.swapchain_loader
                    .destroy_swapchain(old_swapchain, alloc_callbacks())
            };
        }

        self.swapchain_images = unsafe {
            self.swapchain_loader
                .get_swapchain_images(self.swapchain)
                .unwrap()
        };
        self.swapchain_img_views = self
            .swapchain_images
            .iter()
            .enumerate()
            .map(|(i, &swapchain_image)| unsafe {
                let img_view = gpu()
                    .create_image_view(
                        &vk::ImageViewCreateInfo::default()
                            .view_type(vk::ImageViewType::TYPE_2D)
                            .format(self.surface_format.format)
                            .components(vk::ComponentMapping {
                                r: vk::ComponentSwizzle::IDENTITY,
                                g: vk::ComponentSwizzle::IDENTITY,
                                b: vk::ComponentSwizzle::IDENTITY,
                                a: vk::ComponentSwizzle::IDENTITY,
                            })
                            .subresource_range(
                                vk::ImageSubresourceRange::default()
                                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                                    .layer_count(1)
                                    .level_count(1),
                            )
                            .image(swapchain_image),
                        None,
                    )
                    .unwrap();
                debug_name(&format!("swapchain img {i}"), swapchain_image);
                debug_name(&format!("swapchain img view {i}"), img_view);
                img_view
            })
            .collect();

        gpu_idle();
    }

    pub fn acquire_img(&mut self, signal: vk::Semaphore) {
        if self.swapchain == vk::SwapchainKHR::null() {
            self.recreate_swapchain();
        }
        unsafe {
            self.swapchain_img_idx = self
                .swapchain_loader
                .acquire_next_image(self.swapchain, u64::MAX, signal, vk::Fence::null())
                .unwrap()
                .0 as usize;
        }
    }

    pub fn present(&mut self, wait: &[vk::Semaphore]) {
        unsafe {
            self.swapchain_loader
                .queue_present(
                    queue(),
                    &vk::PresentInfoKHR::default()
                        .wait_semaphores(wait)
                        .swapchains(&[self.swapchain])
                        .image_indices(&[self.swapchain_img_idx as u32]),
                )
                .unwrap_or_else(|_| {
                    self.recreate_swapchain();
                    false
                })
        };
    }

    pub fn cur_img(&self) -> vk::Image {
        self.swapchain_images[self.swapchain_img_idx]
    }

    pub fn cur_img_view(&self) -> vk::ImageView {
        self.swapchain_img_views[self.swapchain_img_idx]
    }

    fn surface_capabilities(&self) -> vk::SurfaceCapabilitiesKHR {
        let mut surface_caps = vk::SurfaceCapabilities2KHR::default();
        unsafe {
            self.surface_caps2_loader
                .get_physical_device_surface_capabilities2(
                    physical_gpu(),
                    &vk::PhysicalDeviceSurfaceInfo2KHR::default().surface(self.surface),
                    &mut surface_caps,
                )
                .unwrap()
        };
        surface_caps.surface_capabilities
    }
}

pub struct DebugScope<'a> {
    render_ctx: &'a RenderContext,
}

impl<'a> DebugScope<'a> {
    fn new(render_ctx: &'a RenderContext, name: &str) -> Self {
        render_ctx.debug_begin(name);
        Self { render_ctx }
    }
    fn new_colored(render_ctx: &'a RenderContext, name: &str, color: [f32; 4]) -> Self {
        render_ctx.debug_begin_colored(name, color);
        Self { render_ctx }
    }
}

impl Drop for DebugScope<'_> {
    fn drop(&mut self) {
        self.render_ctx.debug_end();
    }
}

#[cfg(debug_assertions)]
impl RenderContext {
    pub fn debug_begin(&self, label: &str) {
        unsafe {
            DEBUG_UTILS_LOADER.cmd_begin_debug_utils_label(
                self.cmd(),
                &vk::DebugUtilsLabelEXT::default()
                    .label_name(&std::ffi::CString::new(label).unwrap())
                    .color([1.0, 1.0, 1.0, 1.0]),
            )
        }
    }

    pub fn debug_begin_colored(&self, label: &str, color: [f32; 4]) {
        unsafe {
            DEBUG_UTILS_LOADER.cmd_begin_debug_utils_label(
                self.cmd(),
                &vk::DebugUtilsLabelEXT::default()
                    .label_name(&std::ffi::CString::new(label).unwrap())
                    .color(color),
            )
        }
    }

    pub fn debug_end(&self) {
        unsafe { DEBUG_UTILS_LOADER.cmd_end_debug_utils_label(self.cmd()) }
    }

    pub fn debug_insert(&self, label: &str) {
        unsafe {
            DEBUG_UTILS_LOADER.cmd_insert_debug_utils_label(
                self.cmd(),
                &vk::DebugUtilsLabelEXT::default()
                    .label_name(&std::ffi::CString::new(label).unwrap())
                    .color([1.0, 1.0, 1.0, 1.0]),
            )
        }
    }

    pub fn debug_insert_colored(&self, label: &str, color: [f32; 4]) {
        unsafe {
            DEBUG_UTILS_LOADER.cmd_insert_debug_utils_label(
                self.cmd(),
                &vk::DebugUtilsLabelEXT::default()
                    .label_name(&std::ffi::CString::new(label).unwrap())
                    .color(color),
            )
        }
    }

    pub fn debug_scope<'a>(&'a self, name: &str) -> DebugScope<'a> {
        DebugScope::new(self, name)
    }

    pub fn debug_scope_colored<'a>(&'a self, name: &str, color: [f32; 4]) -> DebugScope<'a> {
        DebugScope::new_colored(self, name, color)
    }
}

#[cfg(not(debug_assertions))]
impl RenderContext {
    pub fn debug_begin_colored(&self, _label: &str, _color: [f32; 4]) {}
    pub fn debug_begin(&self, _label: &str) {}
    pub fn debug_end(&self) {}
    pub fn debug_insert(&self, _label: &str) {}
    pub fn debug_insert_colored(&self, _label: &str, _color: [f32; 4]) {}
    pub fn debug_scope<'a>(&'a self, name: &str) -> DebugScope<'a> {
        DebugScope::new(self, name)
    }
    pub fn debug_scope_colored<'a>(&'a self, name: &str, color: [f32; 4]) -> DebugScope<'a> {
        DebugScope::new_colored(self, name, color)
    }
}

#[cfg(debug_assertions)]
pub fn debug_name<T: vk::Handle>(name: &str, obj: T) {
    unsafe {
        DEBUG_UTILS_LOADER
            .set_debug_utils_object_name(
                &vk::DebugUtilsObjectNameInfoEXT::default()
                    .object_name(&std::ffi::CString::new(name).unwrap())
                    .object_handle(obj),
            )
            .unwrap()
    }
}

#[cfg(debug_assertions)]
pub fn debug_tag<T: vk::Handle>(name: u64, tag: &[u8], obj: T) {
    unsafe {
        DEBUG_UTILS_LOADER
            .set_debug_utils_object_tag(
                &vk::DebugUtilsObjectTagInfoEXT::default()
                    .tag_name(name)
                    .tag(tag)
                    .object_handle(obj),
            )
            .unwrap()
    }
}

#[cfg(not(debug_assertions))]
pub fn debug_name<T: vk::Handle>(_name: &str, _obj: T) {}
#[cfg(not(debug_assertions))]
pub fn debug_tag<T: vk::Handle>(_name: u64, _tag: &[u8], _obj: T) {}

pub fn write_desc_set(
    desc_set: vk::DescriptorSet,
    desc_type: vk::DescriptorType,
    buffer: vk::Buffer,
    range: std::ops::Range<vk::DeviceSize>,
    binding: u32,
) {
    unsafe {
        gpu().update_descriptor_sets(
            &[vk::WriteDescriptorSet::default()
                .buffer_info(&[vk::DescriptorBufferInfo::default()
                    .buffer(buffer)
                    .offset(range.start)
                    .range(if range.end == vk::WHOLE_SIZE {
                        vk::WHOLE_SIZE
                    } else {
                        range.end - range.start
                    })])
                .descriptor_count(1)
                .descriptor_type(desc_type)
                .dst_binding(binding)
                .dst_set(desc_set)],
            &[],
        )
    }
}

pub fn write_desc_set_uniform_buffer(
    desc_set: vk::DescriptorSet,
    buffer: vk::Buffer,
    range: std::ops::Range<vk::DeviceSize>,
    binding: u32,
) {
    write_desc_set(
        desc_set,
        vk::DescriptorType::UNIFORM_BUFFER,
        buffer,
        range,
        binding,
    )
}

pub fn write_desc_set_uniform_buffer_whole(
    desc_set: vk::DescriptorSet,
    buffer: vk::Buffer,
    binding: u32,
) {
    write_desc_set_uniform_buffer(desc_set, buffer, 0..vk::WHOLE_SIZE, binding)
}

pub fn write_desc_set_storage_buffer(
    desc_set: vk::DescriptorSet,
    buffer: vk::Buffer,
    range: std::ops::Range<vk::DeviceSize>,
    binding: u32,
) {
    write_desc_set(
        desc_set,
        vk::DescriptorType::STORAGE_BUFFER,
        buffer,
        range,
        binding,
    )
}

pub fn write_desc_set_storage_buffer_whole(
    desc_set: vk::DescriptorSet,
    buffer: vk::Buffer,
    binding: u32,
) {
    write_desc_set_storage_buffer(desc_set, buffer, 0..vk::WHOLE_SIZE, binding)
}

impl Drop for RenderContext {
    fn drop(&mut self) {}
}
