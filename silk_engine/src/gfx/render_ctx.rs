use std::collections::HashMap;
use std::ptr::null;

use ash::vk::{self, Handle};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

use crate::{gpu, gpu_idle, scope_time, util::Mem};

use super::{
    alloc_callbacks, entry, instance, physical_gpu, queue,
    shader::Shader,
    vulkan::{
        pipeline::{GraphicsPipelineInfo, PipelineStageInfo},
        DSLBinding, ImageInfo, SamplerManager,
    },
    CmdAlloc, DSLManager, DescAlloc, GpuAlloc, PipelineLayoutManager, RenderPass,
};

#[cfg(debug_assertions)]
static DEBUG_UTILS_LOADER: std::sync::LazyLock<ash::ext::debug_utils::Device> =
    std::sync::LazyLock::new(|| ash::ext::debug_utils::Device::new(crate::instance(), gpu()));

struct ShaderData {
    shader: Shader,
    pipeline_layout: vk::PipelineLayout,
    pipeline_stages: Vec<PipelineStageInfo>,
}

#[derive(Default, Clone)]
struct PipelineData {
    pipeline: vk::Pipeline,
    info: GraphicsPipelineInfo,
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
    viewport: vk::Viewport,
    scissor: vk::Rect2D,
}

#[derive(Default)]
struct FenceData {
    fence: vk::Fence,
    signaled: bool,
}

struct DescSetData {
    desc_set: vk::DescriptorSet,
    binds: Vec<DSLBinding>,
}

pub struct RenderCtx {
    cmd_state: CmdState,
    // allocators
    desc_alloc: DescAlloc,
    cmd_alloc: CmdAlloc,
    pub gpu_alloc: GpuAlloc,
    // managers (hashmap cached)
    dsl_manager: DSLManager,
    pipeline_layout_manager: PipelineLayoutManager,
    sampler_manager: SamplerManager,
    // named cached objects
    shaders: HashMap<String, ShaderData>,
    pipelines: HashMap<String, PipelineData>,
    desc_sets: HashMap<String, DescSetData>,
    cmds: HashMap<String, vk::CommandBuffer>,
    buffers: HashMap<String, vk::Buffer>,
    render_passes: HashMap<String, RenderPass>,
    fences: HashMap<String, FenceData>,
    semaphores: HashMap<String, vk::Semaphore>,
    images: HashMap<String, (vk::Image, Vec<String>)>,
    img_views: HashMap<String, (vk::ImageView, String)>,
    samplers: HashMap<String, vk::Sampler>,
    // window context
    surface_caps2_loader: ash::khr::get_surface_capabilities2::Instance,
    pub surface: vk::SurfaceKHR,
    pub surface_format: vk::SurfaceFormatKHR,
    surface_present_modes: Vec<vk::PresentModeKHR>,
    swapchain_loader: ash::khr::swapchain::Device,
    pub swapchain: vk::SwapchainKHR,
    pub swapchain_size: vk::Extent2D,
    pub swapchain_img_idx: usize,
}

impl RenderCtx {
    pub fn new(window: &Window) -> Self {
        let surface_loader = ash::khr::surface::Instance::new(entry(), instance());
        let surface_caps2 = ash::khr::get_surface_capabilities2::Instance::new(entry(), instance());
        let surface = unsafe {
            ash_window::create_surface(
                entry(),
                instance(),
                window.display_handle().unwrap().as_raw(),
                window.window_handle().unwrap().as_raw(),
                alloc_callbacks(),
            )
            .expect("failed to create surface")
        };
        debug_name("surface", surface);
        let surface_formats = unsafe {
            surface_loader
                .get_physical_device_surface_formats(physical_gpu(), surface)
                .expect("failed to get surface formats")
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
            gpu_alloc: GpuAlloc::default(),
            dsl_manager: DSLManager::default(),
            pipeline_layout_manager: PipelineLayoutManager::default(),
            sampler_manager: SamplerManager::default(),
            shaders: Default::default(),
            pipelines: Default::default(),
            desc_sets: Default::default(),
            cmds: Default::default(),
            buffers: Default::default(),
            render_passes: Default::default(),
            fences: Default::default(),
            semaphores: Default::default(),
            images: Default::default(),
            img_views: Default::default(),
            samplers: Default::default(),
            surface_caps2_loader: surface_caps2,
            surface,
            surface_format,
            surface_present_modes,
            swapchain_loader,
            swapchain: Default::default(),
            swapchain_size: Default::default(),
            swapchain_img_idx: Default::default(),
        };
        {
            slf.add_cmd("render");
            slf.add_cmd("init");
            slf.add_buffer(
                "staging",
                *Mem::kb(256) as vk::DeviceSize,
                vk::BufferUsageFlags::TRANSFER_DST | vk::BufferUsageFlags::TRANSFER_SRC,
                vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            );
            slf.add_fence("cmd wait", false);
            slf.add_semaphore("img available");
            slf.add_semaphore("render finished");
            slf.add_fence("prev frame finished", true);
        }
        slf
    }

    // might cause a swapchain resize so returns new size
    pub(crate) fn begin_frame(&mut self) -> vk::Extent2D {
        self.wait_fence("prev frame finished");
        self.cmd_alloc.reset();
        let swapchain_size = self.acquire_img(self.semaphore("img available"));
        self.begin_cmd("render", true);
        swapchain_size
    }

    // might cause swapchain resize so returns new optimal size
    pub(crate) fn end_frame(&mut self, window: &Window) -> vk::Extent2D {
        self.submit_cmd(
            "render",
            queue(),
            &[self.semaphore("img available")],
            &[self.semaphore("render finished")],
            &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
            self.fence("prev frame finished"),
        );

        window.pre_present_notify();
        self.present(&[self.semaphore("render finished")])
    }

    pub fn begin_render_swapchain(&mut self, resolve_img_view: vk::ImageView) {
        self.transition_img_layout(
            self.cur_img(),
            vk::ImageLayout::UNDEFINED,
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::PipelineStageFlags2::TOP_OF_PIPE,
            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            vk::AccessFlags2::NONE,
            vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
        );

        let width = self.swapchain_size.width;
        let height = self.swapchain_size.height;
        let img_view = self.cur_img_view();
        self.begin_render(width, height, img_view, resolve_img_view);
    }

    pub fn end_render_swapchain(&mut self) {
        self.end_render();

        self.transition_img_layout(
            self.cur_img(),
            vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
            vk::ImageLayout::PRESENT_SRC_KHR,
            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            vk::PipelineStageFlags2::BOTTOM_OF_PIPE,
            vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
            vk::AccessFlags2::NONE,
        );
    }

    pub fn shader(&self, name: &str) -> &Shader {
        &self
            .shaders
            .get(name)
            .unwrap_or_else(|| panic!("shader not found: {name}"))
            .shader
    }

    pub fn add_shader(&mut self, name: &str) -> &Shader {
        let shader_data = {
            let shader = Shader::new(name);
            let dsls = self.dsl_manager.gets(shader.dsl_infos());
            let pipeline_layout = self.pipeline_layout_manager.get(&dsls);
            debug_name(name, pipeline_layout);
            let module = shader.create_module();
            debug_name(name, module);
            let pipeline_stages = shader.get_pipeline_stages(module);
            ShaderData {
                shader,
                pipeline_layout,
                pipeline_stages,
            }
        };
        let shader_data = self.shaders.entry(name.to_string()).or_insert(shader_data);
        &shader_data.shader
    }

    // TODO: impl remove_* for all add_*
    // pub fn remove_shader(&mut self, name: &str) {
    //     let shader = self.shaders.remove(name).unwrap();
    // }

    pub fn add_render_pass(&mut self, name: &str, mut render_pass: RenderPass) -> vk::RenderPass {
        let rp = render_pass.build();
        assert!(
            self.render_passes
                .insert(name.to_string(), render_pass)
                .is_none(),
            "render pass already exists: {name}"
        );
        debug_name(name, rp);
        rp
    }

    pub fn remove_render_pass(&mut self, name: &str) {
        self.render_passes
            .remove(name)
            .unwrap_or_else(|| panic!("render pass not found: {name}"));
    }

    pub fn render_pass(&mut self, name: &str) -> vk::RenderPass {
        self.render_passes
            .get(name)
            .unwrap_or_else(|| panic!("render pass not found: {name}"))
            .render_pass
    }

    pub fn add_fence(&mut self, name: &str, signaled: bool) {
        let fence = unsafe {
            gpu()
                .create_fence(
                    &vk::FenceCreateInfo::default().flags(if signaled {
                        vk::FenceCreateFlags::SIGNALED
                    } else {
                        vk::FenceCreateFlags::empty()
                    }),
                    alloc_callbacks(),
                )
                .unwrap_or_else(|_| panic!("failed to create fence: {name}"))
        };
        debug_name(name, fence);
        assert!(
            self.fences
                .insert(
                    name.to_string(),
                    FenceData {
                        fence,
                        signaled: false
                    }
                )
                .is_none(),
            "fence already exists: {name}"
        );
    }

    pub fn remove_fence(&mut self, name: &str) {
        let FenceData { fence, signaled } = self
            .fences
            .remove(name)
            .unwrap_or_else(|| panic!("fence not found: {name}"));
        assert!(signaled, "trying to destroy fence that is not signaled");
        unsafe { gpu().destroy_fence(fence, alloc_callbacks()) }
    }

    pub fn fence(&self, name: &str) -> vk::Fence {
        self.fences
            .get(name)
            .unwrap_or_else(|| panic!("fence not found: {name}"))
            .fence
    }

    pub fn reset_fence(&mut self, name: &str) -> vk::Fence {
        let fence = self
            .fences
            .get_mut(name)
            .unwrap_or_else(|| panic!("fence not found: {name}"));
        if fence.signaled {
            unsafe {
                gpu()
                    .reset_fences(&[fence.fence])
                    .unwrap_or_else(|e| panic!("failed to reset fence: {name}, {e}"))
            };
            fence.signaled = false;
        }
        fence.fence
    }

    pub fn wait_fence(&mut self, name: &str) {
        let fence = self
            .fences
            .get_mut(name)
            .unwrap_or_else(|| panic!("failed to wait fence: {name}"));
        if !fence.signaled {
            unsafe {
                gpu()
                    .wait_for_fences(&[fence.fence], false, u64::MAX)
                    .unwrap()
            };
            fence.signaled = true;
        }
        self.reset_fence(name);
    }

    pub fn add_semaphore(&mut self, name: &str) {
        let semaphore = unsafe {
            gpu()
                .create_semaphore(&vk::SemaphoreCreateInfo::default(), alloc_callbacks())
                .unwrap()
        };
        debug_name(name, semaphore);
        assert!(
            self.semaphores
                .insert(name.to_string(), semaphore)
                .is_none(),
            "semaphore already exists: {name}"
        );
    }

    pub fn remove_semaphore(&mut self, name: &str) {
        let semaphore = self
            .semaphores
            .remove(name)
            .unwrap_or_else(|| panic!("semaphore not found: {name}"));
        unsafe {
            gpu().destroy_semaphore(semaphore, alloc_callbacks());
        }
    }

    pub fn semaphore(&self, name: &str) -> vk::Semaphore {
        *self
            .semaphores
            .get(name)
            .unwrap_or_else(|| panic!("semaphore not found: {name}"))
    }

    pub fn add_img(
        &mut self,
        name: &str,
        img_info: &ImageInfo,
        mem_props: vk::MemoryPropertyFlags,
    ) -> vk::Image {
        let img = self.gpu_alloc.alloc_img(img_info, mem_props);
        debug_name(name, img);
        assert!(
            self.images
                .insert(name.to_string(), (img, vec![]))
                .is_none(),
            "image already exists: {name}"
        );
        img
    }

    pub fn try_remove_img(&mut self, name: &str) -> Result<(), String> {
        let (image, img_views) = self
            .images
            .remove(name)
            .ok_or(format!("image not found: {name}"))?;
        self.gpu_alloc.dealloc_img(image);
        for img_view in img_views {
            let (img_view, _) = self
                .img_views
                .remove(&img_view)
                .unwrap_or_else(|| panic!("img view({img_view}) not found, for img({name})"));
            unsafe {
                gpu().destroy_image_view(img_view, alloc_callbacks());
            }
        }
        Ok(())
    }

    pub fn remove_img(&mut self, name: &str) {
        self.try_remove_img(name).unwrap_or_else(|e| panic!("{e}"))
    }

    pub fn img(&self, name: &str) -> vk::Image {
        self.images
            .get(name)
            .unwrap_or_else(|| panic!("img not found: {name}"))
            .0
    }

    pub fn add_img_view(&mut self, name: &str, img_name: &str) -> vk::ImageView {
        let (img, img_views) = self
            .images
            .get_mut(img_name)
            .unwrap_or_else(|| panic!("no image found: {img_name}"));
        img_views.push(name.to_string());
        let img_view = unsafe {
            gpu()
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
                        .image(*img),
                    alloc_callbacks(),
                )
                .unwrap_or_else(|_| panic!("failed to create img view: {name}"))
        };
        self.img_views
            .insert(name.to_string(), (img_view, img_name.to_string()));
        debug_name(name, img_view);
        img_view
    }

    pub fn remove_img_view(&mut self, name: &str) {
        let (img_view, img_name) = self.img_views.remove(name).unwrap();
        let img_views = &mut self.images.get_mut(&img_name).unwrap().1;
        img_views.remove(
            img_views
                .iter()
                .position(|s| s.as_str() == name)
                .unwrap_or_else(|| panic!("img view({name}) not found for img({img_name})")),
        );
        unsafe {
            gpu().destroy_image_view(img_view, alloc_callbacks());
        }
    }

    pub fn img_view(&self, name: &str) -> vk::ImageView {
        self.img_views
            .get(name)
            .unwrap_or_else(|| panic!("img view not found: {name}"))
            .0
    }

    pub fn add_sampler(
        &mut self,
        name: &str,
        addr_mode_u: vk::SamplerAddressMode,
        addr_mode_v: vk::SamplerAddressMode,
        min_filter: vk::Filter,
        mag_filter: vk::Filter,
        mip_filter: vk::SamplerMipmapMode,
    ) -> vk::Sampler {
        let sampler =
            self.sampler_manager
                .get(addr_mode_u, addr_mode_v, min_filter, mag_filter, mip_filter);
        assert!(
            self.samplers.insert(name.to_string(), sampler).is_none(),
            "sampler already exists: {name}"
        );
        debug_name(name, sampler);
        sampler
    }

    pub fn remove_sampler(&mut self, name: &str) -> vk::Sampler {
        self.samplers
            .remove(name)
            .unwrap_or_else(|| panic!("sampler not found: {name}"))
    }

    pub fn sampler(&self, name: &str) -> vk::Sampler {
        *self
            .samplers
            .get(name)
            .unwrap_or_else(|| panic!("sampler not found: {name}"))
    }

    pub fn add_pipeline(
        &mut self,
        name: &str,
        shader_name: &str,
        pipeline_info: GraphicsPipelineInfo,
        vert_input_bindings: &[(bool, Vec<u32>)],
    ) -> vk::Pipeline {
        scope_time!("Create pipeline({name})");
        let shader_data = &self
            .shaders
            .get(shader_name)
            .unwrap_or_else(|| panic!("no shader found: {shader_name}"));
        let pipeline_info = pipeline_info
            .layout(shader_data.pipeline_layout)
            .stages(&shader_data.pipeline_stages)
            .vert_layout(&shader_data.shader, vert_input_bindings);
        let pipeline = pipeline_info.build();
        assert!(
            self.pipelines
                .insert(
                    name.to_string(),
                    PipelineData {
                        pipeline,
                        info: pipeline_info,
                        bind_point: vk::PipelineBindPoint::GRAPHICS,
                    },
                )
                .is_none(),
            "pipeline already exists: {name}"
        );
        debug_name(name, pipeline);
        pipeline
    }

    pub fn add_desc_set(
        &mut self,
        name: &str,
        shader_name: &str,
        group: usize,
    ) -> vk::DescriptorSet {
        let binds = self
            .shaders
            .get(shader_name)
            .unwrap_or_else(|| panic!("no shader found: {shader_name}"))
            .shader
            .dsl_infos()[group]
            .clone();
        let dsl = self.dsl_manager.get(&binds);
        let desc_set = self.desc_alloc.alloc_one(dsl);
        assert!(
            self.desc_sets
                .insert(name.to_string(), DescSetData { desc_set, binds })
                .is_none(),
            "desc set already exists: {name}"
        );
        debug_name(name, desc_set);
        desc_set
    }

    pub fn add_desc_sets(&mut self, names: &[&str], shader_name: &str) -> Vec<vk::DescriptorSet> {
        let dsl_infos = self.shaders[shader_name].shader.dsl_infos();
        let dsls = self.dsl_manager.gets(dsl_infos);
        let desc_sets = self.desc_alloc.alloc(&dsls);
        for (name, (&desc_set, binds)) in names
            .iter()
            .zip(desc_sets.iter().zip(dsl_infos.iter().cloned()))
        {
            assert!(
                self.desc_sets
                    .insert(name.to_string(), DescSetData { desc_set, binds })
                    .is_none(),
                "desc set already exists: {name}"
            );
            debug_name(name, desc_set);
        }
        desc_sets
    }

    pub fn desc_set(&self, name: &str) -> vk::DescriptorSet {
        self.desc_sets
            .get(name)
            .unwrap_or_else(|| panic!("descriptor set not found: {name}"))
            .desc_set
    }

    pub fn add_cmds(&mut self, names: &[&str]) -> Vec<vk::CommandBuffer> {
        let cmds = self.cmd_alloc.alloc(names.len() as u32);
        for (&cmd, &name) in cmds.iter().zip(names.iter()) {
            assert!(
                self.cmds.insert(name.to_string(), cmd).is_none(),
                "cmd buf already exists: {name}"
            );
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

    // pub fn reset_cmds(&mut self, names: &[&str]) {
    //     for name in names.iter() {
    //         self.cmd_alloc.reset(self.cmds[&name.to_string()]);
    //     }
    // }

    // pub fn reset_cmd(&mut self, name: &str) {
    //     self.reset_cmds(&[name])
    // }

    pub fn add_buffer(
        &mut self,
        name: &str,
        size: u64,
        usage: vk::BufferUsageFlags,
        mem_props: vk::MemoryPropertyFlags,
    ) -> vk::Buffer {
        crate::log!(
            "Alloc({name}) {:?}, {:?}, {:?}",
            crate::Mem::b(size as usize),
            usage,
            mem_props
        );
        let buf = self.gpu_alloc.alloc_buf(size, usage, mem_props);
        assert!(
            self.buffers.insert(name.to_string(), buf).is_none(),
            "buffer already exists: {name}"
        );
        debug_name(name, buf);
        buf
    }

    pub fn remove_buffer(&mut self, name: &str) {
        let buf = self.buffers.remove(name).unwrap();
        self.gpu_alloc.dealloc_buf(buf);
    }

    pub fn recreate_buffer(&mut self, name: &str, size: u64) -> vk::Buffer {
        let buffer = self.buffers.get_mut(name).unwrap();
        *buffer = self.gpu_alloc.realloc_buf(*buffer, size);
        debug_name(name, *buffer);
        *buffer
    }

    pub fn buffer(&self, name: &str) -> vk::Buffer {
        *self
            .buffers
            .get(name)
            .unwrap_or_else(|| panic!("buffer not found: {name}"))
    }

    pub fn buffer_size(&self, name: &str) -> u64 {
        self.gpu_alloc.buf_size(self.buffer(name))
    }

    pub fn get_cmd(&self, name: &str) -> vk::CommandBuffer {
        *self
            .cmds
            .get(name)
            .unwrap_or_else(|| panic!("cmd buffer not found: {name}"))
    }

    pub fn cmd(&self) -> vk::CommandBuffer {
        let cmd = self.cmd_state.cmd;
        assert_ne!(cmd, Default::default(), "no active cmd");
        cmd
    }

    pub fn cmd_name(&self) -> &str {
        &self.cmd_state.cmd_name
    }

    pub fn begin_cmd(&mut self, name: &str, once: bool) -> vk::CommandBuffer {
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
                    &vk::CommandBufferBeginInfo::default().flags(if once {
                        vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT
                    } else {
                        vk::CommandBufferUsageFlags::empty()
                    }),
                )
                .unwrap();
        }
        self.debug_begin(&format!("Begin Cmd({name})"));
        self.cmd_state.cmd
    }

    pub fn end_cmd(&mut self) {
        if self.cmd_state.cmd == Default::default() {
            return;
        }
        if self.cmd_state.render_area != Default::default() {
            self.end_render();
        }
        self.debug_end();
        unsafe {
            gpu().end_command_buffer(self.cmd()).unwrap();
        }
        self.cmd_state = Default::default();
    }

    pub fn wait_cmd(&mut self) {
        let cmd = self.cmd_name().to_string();
        self.submit_cmd(&cmd, queue(), &[], &[], &[], self.fence("cmd wait"));
        self.wait_fence("cmd wait");
    }

    pub fn submit_cmd(
        &mut self,
        name: &str,
        queue: vk::Queue,
        wait: &[vk::Semaphore],
        signal: &[vk::Semaphore],
        wait_dst_stage_mask: &[vk::PipelineStageFlags],
        fence: vk::Fence,
    ) {
        self.submit_cmds(&[name], queue, wait, signal, wait_dst_stage_mask, fence);
    }

    pub fn submit_cmds(
        &mut self,
        names: &[&str],
        queue: vk::Queue,
        wait: &[vk::Semaphore],
        signal: &[vk::Semaphore],
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
                    &[vk::SubmitInfo {
                        signal_semaphore_count: signal.len() as u32,
                        wait_semaphore_count: wait.len() as u32,
                        p_signal_semaphores: if signal.is_empty() {
                            null()
                        } else {
                            signal.as_ptr()
                        },
                        p_wait_semaphores: if wait.is_empty() {
                            null()
                        } else {
                            wait.as_ptr()
                        },
                        ..Default::default()
                    }
                    .command_buffers(&cmds)
                    .wait_dst_stage_mask(wait_dst_stage_mask)],
                    fence,
                )
                .unwrap();
        }
    }

    pub fn begin_render(
        &mut self,
        width: u32,
        height: u32,
        img_view: vk::ImageView,
        sampled_img_view: vk::ImageView,
    ) {
        self.cmd_state.render_area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D { width, height },
        };
        self.debug_begin(&format!("Begin Render({width}x{height})"));
        let sampled = !sampled_img_view.is_null();
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
                        .resolve_mode(if sampled {
                            vk::ResolveModeFlags::AVERAGE
                        } else {
                            vk::ResolveModeFlags::NONE
                        })
                        .resolve_image_view(if sampled {
                            img_view
                        } else {
                            vk::ImageView::null()
                        })
                        .resolve_image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .image_view(if sampled { sampled_img_view } else { img_view })]),
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
        self.debug_end();
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

    pub fn set_viewport(&mut self, viewport: vk::Viewport) {
        if self.cmd_state.viewport.width == viewport.width
            && self.cmd_state.viewport.height == viewport.height
            && self.cmd_state.viewport.x == viewport.x
            && self.cmd_state.viewport.y == viewport.y
            && self.cmd_state.viewport.min_depth == viewport.min_depth
            && self.cmd_state.viewport.max_depth == viewport.max_depth
        {
            return;
        }
        self.cmd_state.viewport = viewport;
        unsafe { gpu().cmd_set_viewport(self.cmd(), 0, &[viewport]) };
    }

    pub fn set_scissor(&mut self, scissor: vk::Rect2D) {
        if self.cmd_state.scissor.extent.width == scissor.extent.width
            && self.cmd_state.scissor.extent.height == scissor.extent.height
            && self.cmd_state.scissor.offset.x == scissor.offset.x
            && self.cmd_state.scissor.offset.y == scissor.offset.y
        {
            return;
        }
        self.cmd_state.scissor = scissor;
        unsafe { gpu().cmd_set_scissor(self.cmd(), 0, &[scissor]) };
    }

    pub fn bind_pipeline(&mut self, name: &str) {
        let pipeline_data = self
            .pipelines
            .get(name)
            .unwrap_or_else(|| panic!("pipeline not found: {name}"))
            .clone();
        if pipeline_data.pipeline == self.cmd_state.pipeline_data.pipeline {
            return;
        }
        self.cmd_state.pipeline_data = pipeline_data;

        unsafe {
            let dyn_states = self.cmd_state.pipeline_data.info.dynamic_states.clone();
            let extent = self.cmd_state.render_area.extent;
            if dyn_states.contains(&vk::DynamicState::VIEWPORT) {
                self.set_viewport(vk::Viewport {
                    x: 0.0,
                    y: 0.0,
                    width: extent.width as f32,
                    height: extent.height as f32,
                    min_depth: 0.0,
                    max_depth: 1.0,
                });
            }
            if dyn_states.contains(&vk::DynamicState::SCISSOR) {
                self.set_scissor(vk::Rect2D {
                    offset: vk::Offset2D { x: 0, y: 0 },
                    extent,
                });
            }
            gpu().cmd_bind_pipeline(
                self.cmd(),
                self.cmd_state.pipeline_data.bind_point,
                self.cmd_state.pipeline_data.pipeline,
            )
        }
    }

    pub fn bind_desc_set(&mut self, name: &str) {
        self.cmd_state.desc_sets = vec![self.desc_set(name)];
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
            gpu().cmd_bind_vertex_buffers(self.cmd(), 0, &[self.buffer(name)], &[0]);
        }
    }

    pub fn bind_ebo(&self, name: &str) {
        unsafe {
            gpu().cmd_bind_index_buffer(self.cmd(), self.buffer(name), 0, vk::IndexType::UINT32);
        }
    }

    pub fn bind_vao(&self, name: &str, index_buffer_offset: vk::DeviceSize) {
        unsafe {
            gpu().cmd_bind_vertex_buffers(self.cmd(), 0, &[self.buffer(name)], &[0]);
            gpu().cmd_bind_index_buffer(
                self.cmd(),
                self.buffer(name),
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

    pub fn transition_img_layout(
        &self,
        image: vk::Image,
        old_layout: vk::ImageLayout,
        new_layout: vk::ImageLayout,
        src_stage: vk::PipelineStageFlags2,
        dst_stage: vk::PipelineStageFlags2,
        src_access: vk::AccessFlags2,
        dst_access: vk::AccessFlags2,
    ) {
        unsafe {
            gpu().cmd_pipeline_barrier2(
                self.cmd(),
                &vk::DependencyInfo::default().image_memory_barriers(&[
                    vk::ImageMemoryBarrier2::default()
                        .dst_access_mask(dst_access)
                        .src_access_mask(src_access)
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

    pub fn staging_buffer(&mut self, size: vk::DeviceSize) -> vk::Buffer {
        if self.buffer_size("staging") >= size {
            self.buffer("staging")
        } else {
            self.recreate_buffer("staging", (size + 1).next_power_of_two())
        }
    }

    // TODO: don't begin cmd unless have to
    // TODO: automatic barrier system
    pub fn copy_buffer_off(
        &mut self,
        src_buffer: vk::Buffer,
        dst_buffer: vk::Buffer,
        src_off: vk::DeviceSize,
        dst_off: vk::DeviceSize,
    ) {
        let cmd = self.begin_cmd("init", false);
        unsafe {
            // let buffer_usage = self.gpu_alloc.buf_usage(dst_buffer);
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
            let buf_size = self
                .gpu_alloc
                .buf_size(src_buffer)
                .min(self.gpu_alloc.buf_size(dst_buffer));
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
        if self.gpu_alloc.is_mappable(buffer) {
            self.gpu_alloc.write_mapped_off(buffer, data, off);
        } else {
            let staging = self.staging_buffer(size_of_val(data) as vk::DeviceSize);
            self.gpu_alloc.write_mapped(staging, data);
            self.copy_buffer_off(staging, buffer, 0, off);
        }
    }

    pub fn read_buffer_off<T: ?Sized>(&mut self, name: &str, data: &mut T, off: vk::DeviceSize) {
        let buffer = self.buffer(name);
        if self.gpu_alloc.is_mappable(buffer) {
            self.gpu_alloc.read_mapped_off(buffer, data, off);
        } else {
            let staging = self.staging_buffer(size_of_val(data) as vk::DeviceSize);
            self.copy_buffer_off(buffer, staging, off, 0);
            self.gpu_alloc.read_mapped(staging, data);
        }
    }

    pub fn write_buffer<T: ?Sized>(&mut self, name: &str, data: &T) {
        self.write_buffer_off(name, data, 0);
    }

    pub fn read_buffer<T: ?Sized>(&mut self, name: &str, data: &mut T) {
        self.read_buffer_off(name, data, 0);
    }

    pub fn write_ds_range(
        &self,
        name: &str,
        buffer_name: &str,
        range: std::ops::Range<vk::DeviceSize>,
        binding: u32,
    ) {
        let DescSetData { desc_set, binds } = &self
            .desc_sets
            .get(name)
            .unwrap_or_else(|| panic!("descriptor not found: {name}"));
        let buffer = self.buffer(buffer_name);
        let ds_type = binds[binding as usize].descriptor_type;
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
                    .descriptor_type(ds_type)
                    .dst_binding(binding)
                    .dst_set(*desc_set)],
                &[],
            )
        }
    }

    pub fn write_ds_img(
        &self,
        name: &str,
        img_view_name: &str,
        img_layout: vk::ImageLayout,
        binding: u32,
    ) {
        let DescSetData { desc_set, binds } = &self
            .desc_sets
            .get(name)
            .unwrap_or_else(|| panic!("descriptor not found: {name}"));
        let img_view = self.img_view(img_view_name);
        let ds_type = binds[binding as usize].descriptor_type;
        unsafe {
            gpu().update_descriptor_sets(
                &[vk::WriteDescriptorSet::default()
                    .image_info(&[vk::DescriptorImageInfo::default()
                        .image_layout(img_layout)
                        .image_view(img_view)])
                    .descriptor_count(1)
                    .descriptor_type(ds_type)
                    .dst_binding(binding)
                    .dst_set(*desc_set)],
                &[],
            )
        }
    }

    pub fn write_ds_sampler(&self, name: &str, sampler_name: &str, binding: u32) {
        let DescSetData { desc_set, .. } = &self
            .desc_sets
            .get(name)
            .unwrap_or_else(|| panic!("descriptor not found: {name}"));
        let sampler = self.sampler(sampler_name);
        unsafe {
            gpu().update_descriptor_sets(
                &[vk::WriteDescriptorSet::default()
                    .image_info(&[vk::DescriptorImageInfo::default().sampler(sampler)])
                    .descriptor_count(1)
                    .descriptor_type(vk::DescriptorType::SAMPLER)
                    .dst_binding(binding)
                    .dst_set(*desc_set)],
                &[],
            )
        }
    }

    pub fn write_ds(&self, name: &str, buffer_name: &str, binding: u32) {
        self.write_ds_range(name, buffer_name, 0..vk::WHOLE_SIZE, binding)
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

    pub fn recreate_swapchain(&mut self) -> vk::Extent2D {
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
            return surface_resolution;
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
                        .image_usage(
                            vk::ImageUsageFlags::COLOR_ATTACHMENT
                                | vk::ImageUsageFlags::TRANSFER_DST,
                        )
                        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
                        .pre_transform(pre_transform)
                        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
                        .present_mode(present_mode)
                        .old_swapchain(old_swapchain)
                        .clipped(true),
                    alloc_callbacks(),
                )
                .unwrap()
        };
        debug_name("swapchain", self.swapchain);

        if old_swapchain != Default::default() {
            // FIXME: assumes swapchain image count is constant
            for i in 0..desired_image_count {
                let img_name = format!("swapchain image {i}");
                let img_views = self.images[&img_name].1.clone();
                for img_view in img_views {
                    self.remove_img_view(&img_view);
                }
                self.images.remove(&img_name).unwrap();
            }
            unsafe {
                self.swapchain_loader
                    .destroy_swapchain(old_swapchain, alloc_callbacks())
            };
        }

        let swapchain_images = unsafe {
            self.swapchain_loader
                .get_swapchain_images(self.swapchain)
                .unwrap()
        };
        for (i, swap_img) in swapchain_images.into_iter().enumerate() {
            let img_name = format!("swapchain image {i}");
            debug_name(&img_name, swap_img);
            let img_view_name = format!("swapchain image view {i}");
            self.images.insert(img_name.clone(), (swap_img, vec![]));
            self.add_img_view(&img_view_name, &img_name);
        }

        gpu_idle();
        surface_resolution
    }

    // might cause resize so returns optimal swapchain size
    pub fn acquire_img(&mut self, signal: vk::Semaphore) -> vk::Extent2D {
        let extent = if self.swapchain == vk::SwapchainKHR::null() {
            self.recreate_swapchain()
        } else {
            self.swapchain_size
        };
        unsafe {
            self.swapchain_img_idx = self
                .swapchain_loader
                .acquire_next_image(self.swapchain, u64::MAX, signal, vk::Fence::null())
                .unwrap()
                .0 as usize;
        }
        extent
    }

    // might cause resize so returns optimal swapchain size
    pub fn present(&mut self, wait: &[vk::Semaphore]) -> vk::Extent2D {
        unsafe {
            self.swapchain_loader
                .queue_present(
                    queue(),
                    &vk::PresentInfoKHR::default()
                        .wait_semaphores(wait)
                        .swapchains(&[self.swapchain])
                        .image_indices(&[self.swapchain_img_idx as u32]),
                )
                .map(|_| self.swapchain_size)
                .unwrap_or_else(|_| self.recreate_swapchain())
        }
    }

    pub fn cur_img(&self) -> vk::Image {
        self.img(&format!("swapchain image {}", self.swapchain_img_idx))
    }

    pub fn cur_img_view(&self) -> vk::ImageView {
        self.img_view(&format!("swapchain image view {}", self.swapchain_img_idx))
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
    render_ctx: &'a RenderCtx,
}

impl<'a> DebugScope<'a> {
    fn new(render_ctx: &'a RenderCtx, name: &str) -> Self {
        render_ctx.debug_begin(name);
        Self { render_ctx }
    }
    fn new_colored(render_ctx: &'a RenderCtx, name: &str, color: [f32; 4]) -> Self {
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
impl RenderCtx {
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

impl Drop for RenderCtx {
    fn drop(&mut self) {
        gpu_idle();
        for pipeline in self.pipelines.values() {
            let pipeline = pipeline.pipeline;
            if !pipeline.is_null() {
                unsafe {
                    gpu().destroy_pipeline(pipeline, alloc_callbacks());
                }
            }
        }
        // TODO: shader modules are weird
        // implement system like:
        // ctx().add_shader_module("vert")
        // ctx().add_shader_module("frag")
        // ctx().add_shader(["vert", "frag"])
        // ctx().add_shader("wgsl")
        // where add_shader generates dsl bindings
        // and pipeline stages with modules
        // ctx().remove_shader_module("vert")
        // ctx().remove_shader_module("frag")
        // ctx().remove_shader_module("wgsl")
        // destroys shader modules
        // can also debug check for dangling shaders
        // for shader in self.shaders.values() {
        //     for ps in shader.pipeline_stages.iter() {
        //         let module = ps.module;
        //         if !module.is_null() {
        //             unsafe {
        //                 gpu().destroy_shader_module(module, alloc_callbacks());
        //             }
        //         }
        //     }
        // }
        for fence in self.fences.values() {
            let fence = fence.fence;
            if !fence.is_null() {
                unsafe {
                    gpu().destroy_fence(fence, alloc_callbacks());
                }
            }
        }
        for &semaphore in self.semaphores.values() {
            if !semaphore.is_null() {
                unsafe {
                    gpu().destroy_semaphore(semaphore, alloc_callbacks());
                }
            }
        }
        if !self.swapchain.is_null() {
            unsafe {
                self.swapchain_loader
                    .destroy_swapchain(self.swapchain, alloc_callbacks())
            };
        }
        for &(img_view, _) in self.img_views.values() {
            if !img_view.is_null() {
                unsafe {
                    gpu().destroy_image_view(img_view, alloc_callbacks());
                }
            }
        }
    }
}

#[cfg(not(debug_assertions))]
impl RenderCtx {
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
    let raw = obj.as_raw();
    unsafe {
        DEBUG_UTILS_LOADER
            .set_debug_utils_object_name(
                &vk::DebugUtilsObjectNameInfoEXT::default()
                    .object_name(&std::ffi::CString::new(name).unwrap())
                    .object_handle(T::from_raw(raw)),
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
