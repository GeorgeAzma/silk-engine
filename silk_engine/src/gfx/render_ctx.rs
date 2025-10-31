use std::collections::HashMap;

use ash::vk::{self, Handle};
use winit::raw_window_handle::{HasDisplayHandle, HasWindowHandle};
use winit::window::Window;

use super::HDR;
use crate::{scope_time, util::Mem};

use super::{
    BufUsage, CmdManager, DSLBinding, DSLManager, DescAlloc, GpuAlloc, GraphicsPipelineInfo,
    ImageInfo, ImgLayout, ImgUsage, MemProp, PipelineLayoutManager, PipelineStageInfo,
    SamplerManager, alloc_callbacks, create_compute, entry, gpu, gpu_idle, instance, physical_gpu,
    queue, shader::Shader,
};

#[cfg(debug_assertions)]
static DEBUG_UTILS_LOADER: std::sync::LazyLock<ash::ext::debug_utils::Device> =
    std::sync::LazyLock::new(|| ash::ext::debug_utils::Device::new(instance(), gpu()));

struct ShaderData {
    shader: Shader,
    pipeline_layout: vk::PipelineLayout,
    pipeline_stages: Vec<PipelineStageInfo>,
}

#[derive(Debug, Default, Clone)]
struct PipelineData {
    pipeline: vk::Pipeline,
    info: GraphicsPipelineInfo,
    bind_point: vk::PipelineBindPoint,
    shader_name: String,
}

#[derive(Debug, Default)]
struct CmdInfo {
    pipeline_data: PipelineData,
    desc_sets: Vec<vk::DescriptorSet>,
    render_area: vk::Rect2D,
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

pub struct ImageData {
    pub img: vk::Image,
    pub views: Vec<String>,
    pub info: ImageInfo,
}

pub struct RenderCtx {
    cmd_info: CmdInfo,
    // allocators
    desc_alloc: DescAlloc,
    pub gpu_alloc: GpuAlloc,
    // managers (hashmap cached)
    dsl_manager: DSLManager,
    pipeline_layout_manager: PipelineLayoutManager,
    sampler_manager: SamplerManager,
    cmd_manager: CmdManager,
    // named cached objects
    shaders: HashMap<String, ShaderData>,
    pipelines: HashMap<String, PipelineData>,
    desc_sets: HashMap<String, DescSetData>,
    bufs: HashMap<String, vk::Buffer>,
    fences: HashMap<String, FenceData>,
    semaphores: HashMap<String, vk::Semaphore>,
    imgs: HashMap<String, ImageData>,
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
    frame_cmd: vk::CommandBuffer,
}

#[derive(Debug)]
pub struct BufferImageCopy {
    pub buf_off: vk::DeviceSize,
    pub img_off_x: u32,
    pub img_off_y: u32,
    pub buf_width: u32,
    pub buf_height: u32,
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
        let surface_format = if HDR {
            surface_formats
                .iter()
                .find(|&format| {
                    format.format == vk::Format::A2B10G10R10_UNORM_PACK32
                        && format.color_space == vk::ColorSpaceKHR::HDR10_ST2084_EXT
                })
                .or_else(|| {
                    surface_formats.iter().find(|&format| {
                        format.format == vk::Format::R16G16B16A16_SFLOAT
                            && format.color_space == vk::ColorSpaceKHR::EXTENDED_SRGB_LINEAR_EXT
                    })
                })
        } else {
            surface_formats
                .iter()
                .find(|&format| {
                    format.format == vk::Format::B8G8R8A8_UNORM
                        && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
                })
                .or_else(|| {
                    surface_formats.iter().find(|&format| {
                        format.format == vk::Format::B8G8R8A8_SRGB
                            && format.color_space == vk::ColorSpaceKHR::SRGB_NONLINEAR
                    })
                })
        }
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
            cmd_info: CmdInfo::default(),
            desc_alloc: DescAlloc::default(),
            gpu_alloc: GpuAlloc::default(),
            dsl_manager: DSLManager::default(),
            pipeline_layout_manager: PipelineLayoutManager::default(),
            sampler_manager: SamplerManager::default(),
            cmd_manager: CmdManager::new(),
            shaders: Default::default(),
            pipelines: Default::default(),
            desc_sets: Default::default(),
            bufs: Default::default(),
            fences: Default::default(),
            semaphores: Default::default(),
            imgs: Default::default(),
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
            frame_cmd: Default::default(),
        };
        {
            slf.add_buf(
                "staging",
                *Mem::kb(256) as vk::DeviceSize,
                BufUsage::DST | BufUsage::SRC,
                MemProp::CPU,
            );
            slf.add_semaphore("img available");
            for i in 0..3 {
                slf.add_semaphore(&format!("render finished {i}"));
            }
            slf.add_sampler(
                "linear",
                vk::SamplerAddressMode::REPEAT,
                vk::SamplerAddressMode::REPEAT,
                vk::Filter::LINEAR,
                vk::Filter::LINEAR,
                vk::SamplerMipmapMode::LINEAR,
            );
            slf.add_sampler(
                "nearest",
                vk::SamplerAddressMode::REPEAT,
                vk::SamplerAddressMode::REPEAT,
                vk::Filter::NEAREST,
                vk::Filter::NEAREST,
                vk::SamplerMipmapMode::NEAREST,
            );
        }
        slf
    }

    pub(crate) fn wait_prev_frame(&mut self) {
        if !self.frame_cmd.is_null() {
            self.cmd_manager.wait(self.frame_cmd);
        }
    }

    // might cause a swapchain resize so returns new size
    pub(crate) fn begin_frame(&mut self) -> vk::Extent2D {
        self.cmd_info = Default::default();
        self.cmd_manager.reset();
        let swapchain_size = self.acquire_img(self.semaphore("img available"));
        self.frame_cmd = self.begin_cmd();
        swapchain_size
    }

    // might cause swapchain resize so returns new optimal size
    pub(crate) fn end_frame(&mut self, window: &Window) -> vk::Extent2D {
        let cmd = self.cmd_manager.end();
        let render_finished = format!("render finished {}", self.swapchain_img_idx);
        self.submit_cmd(
            cmd,
            &[self.semaphore("img available")],
            &[self.semaphore(&render_finished)],
            &[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
        );

        window.pre_present_notify();
        self.present(&[self.semaphore(&render_finished)])
    }

    pub fn begin_render_swapchain(&mut self, resolve_img_view_name: &str) {
        self.set_img_layout(
            &self.cur_img(),
            ImgLayout::COLOR,
            vk::PipelineStageFlags2::TOP_OF_PIPE,
            vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT,
            vk::AccessFlags2::NONE,
            vk::AccessFlags2::COLOR_ATTACHMENT_WRITE,
        );
        let width = self.swapchain_size.width;
        let height = self.swapchain_size.height;
        let img_view = self.cur_img_view();
        self.begin_render(width, height, &img_view, resolve_img_view_name);
    }

    pub fn end_render_swapchain(&mut self) {
        self.end_render();
        self.set_img_layout(
            &self.cur_img(),
            ImgLayout::PRESENT,
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
        &self
            .shaders
            .entry(name.to_string())
            .or_insert_with(|| {
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
            })
            .shader
    }

    pub fn add_fence(&mut self, name: &str, signaled: bool) -> vk::Fence {
        self.fences
            .entry(name.to_string())
            .or_insert_with(|| unsafe {
                let fence = gpu()
                    .create_fence(
                        &vk::FenceCreateInfo::default().flags(if signaled {
                            vk::FenceCreateFlags::SIGNALED
                        } else {
                            vk::FenceCreateFlags::empty()
                        }),
                        alloc_callbacks(),
                    )
                    .unwrap_or_else(|_| panic!("failed to create fence: {name}"));
                debug_name(name, fence);
                FenceData {
                    fence,
                    signaled: false,
                }
            })
            .fence
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
            .unwrap_or_else(|| panic!("fence not found: {name}"));
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

    pub fn add_semaphore(&mut self, name: &str) -> vk::Semaphore {
        *self
            .semaphores
            .entry(name.to_string())
            .or_insert_with(|| unsafe {
                let semaphore = gpu()
                    .create_semaphore(&vk::SemaphoreCreateInfo::default(), alloc_callbacks())
                    .unwrap();
                debug_name(name, semaphore);
                semaphore
            })
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
        info: &ImageInfo,
        mem_props: vk::MemoryPropertyFlags,
    ) -> vk::Image {
        self.imgs
            .entry(name.to_string())
            .or_insert_with(|| {
                let img = self.gpu_alloc.alloc_img(info, mem_props);
                debug_name(name, img);
                ImageData {
                    img,
                    views: vec![],
                    info: info.clone(),
                }
            })
            .img
    }

    pub fn try_remove_img(&mut self, name: &str) -> bool {
        if let Some(ImageData {
            img,
            views,
            info: _,
        }) = self.imgs.remove(name)
        {
            self.gpu_alloc.dealloc_img(img);
            for img_view in views {
                let (img_view, _) = self
                    .img_views
                    .remove(&img_view)
                    .unwrap_or_else(|| panic!("img view({img_view}) not found, for img({name})"));
                unsafe {
                    gpu().destroy_image_view(img_view, alloc_callbacks());
                }
            }
            true
        } else {
            false
        }
    }

    pub fn remove_img(&mut self, name: &str) {
        if !self.try_remove_img(name) {
            panic!("img not found: {name}")
        }
    }

    pub fn img(&self, name: &str) -> &ImageData {
        self.imgs
            .get(name)
            .unwrap_or_else(|| panic!("img not found: {name}"))
    }

    pub fn add_img_view(&mut self, name: &str, img_name: &str) -> vk::ImageView {
        self.img_views
            .entry(name.to_string())
            .or_insert_with(|| {
                let ImageData { img, views, info } = self
                    .imgs
                    .get_mut(img_name)
                    .unwrap_or_else(|| panic!("img not found: {img_name}"));
                views.push(name.to_string());
                let img_view = unsafe {
                    gpu()
                        .create_image_view(
                            &vk::ImageViewCreateInfo::default()
                                .view_type(vk::ImageViewType::TYPE_2D)
                                .format(info.format)
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
                debug_name(name, img_view);
                (img_view, img_name.to_string())
            })
            .0
    }

    pub fn remove_img_view(&mut self, name: &str) {
        let (img_view, img_name) = self.img_views.remove(name).unwrap();
        let img_views = &mut self.imgs.get_mut(&img_name).unwrap().views;
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
        if name.is_empty() {
            return vk::ImageView::null();
        }
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
        *self.samplers.entry(name.to_string()).or_insert_with(|| {
            let sampler = self.sampler_manager.get(
                addr_mode_u,
                addr_mode_v,
                min_filter,
                mag_filter,
                mip_filter,
            );
            debug_name(name, sampler);
            sampler
        })
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
        self.pipelines
            .entry(name.to_string())
            .or_insert_with(|| {
                let shader_data = &self
                    .shaders
                    .get(shader_name)
                    .unwrap_or_else(|| panic!("no shader found: {shader_name}"));
                let pipeline_info = pipeline_info
                    .layout(shader_data.pipeline_layout)
                    .stages(&shader_data.pipeline_stages)
                    .vert_layout(&shader_data.shader, vert_input_bindings);
                let pipeline = pipeline_info.build();
                debug_name(name, pipeline);
                PipelineData {
                    pipeline,
                    info: pipeline_info,
                    bind_point: vk::PipelineBindPoint::GRAPHICS,
                    shader_name: shader_name.to_string(),
                }
            })
            .pipeline
    }

    pub fn add_compute(&mut self, name: &str) -> vk::Pipeline {
        self.add_shader(name);
        let shader = &self.shaders[name];
        let module = shader.pipeline_stages[0].module;
        let layout = shader.pipeline_layout;
        let entry_name = &shader.pipeline_stages[0].name;
        self.pipelines
            .entry(name.to_string())
            .or_insert_with(|| {
                let pipeline = create_compute(module, layout, entry_name);
                debug_name(name, pipeline);
                PipelineData {
                    pipeline,
                    info: GraphicsPipelineInfo::default().layout(layout),
                    bind_point: vk::PipelineBindPoint::COMPUTE,
                    shader_name: name.to_string(),
                }
            })
            .pipeline
    }

    /// note: x,y,z are total size, not work group size
    pub fn dispatch(&mut self, x: u32, y: u32, z: u32) {
        let [wx, wy, wz] = self
            .shader(&self.cmd_info.pipeline_data.shader_name)
            .workgroup_size();
        unsafe { gpu().cmd_dispatch(self.cmd(), x.div_ceil(wx), y.div_ceil(wy), z.div_ceil(wz)) };
    }

    pub fn add_desc_set(
        &mut self,
        name: &str,
        shader_name: &str,
        group: usize,
    ) -> vk::DescriptorSet {
        self.desc_sets
            .entry(name.to_string())
            .or_insert_with(|| {
                let binds = self
                    .shaders
                    .get(shader_name)
                    .unwrap_or_else(|| panic!("no shader found: {shader_name}"))
                    .shader
                    .dsl_infos()[group]
                    .clone();
                let dsl = self.dsl_manager.get(&binds);
                let desc_set = self.desc_alloc.alloc_one(dsl);
                debug_name(name, desc_set);
                DescSetData { desc_set, binds }
            })
            .desc_set
    }

    pub fn desc_set(&self, name: &str) -> vk::DescriptorSet {
        self.desc_sets
            .get(name)
            .unwrap_or_else(|| panic!("descriptor set not found: {name}"))
            .desc_set
    }

    /// if exists with smaller size, grows buf (which invalidates old bufs)
    pub fn add_buf(
        &mut self,
        name: &str,
        size: u64,
        usage: vk::BufferUsageFlags,
        mem_props: vk::MemoryPropertyFlags,
    ) -> vk::Buffer {
        if let Some(buf) = self.bufs.get(name) {
            if self.buf_size(name) < size {
                self.gpu_alloc.dealloc_buf(*buf);
                let new_buf = self.gpu_alloc.alloc_buf(size, usage, mem_props);
                let buf_mut = &mut unsafe { *std::ptr::from_ref(buf).cast_mut() };
                *buf_mut = new_buf;
            }
            *buf
        } else {
            let buf = self.gpu_alloc.alloc_buf(size, usage, mem_props);
            debug_name(name, buf);
            self.bufs.insert(name.to_string(), buf);
            buf
        }
    }

    pub fn remove_buf(&mut self, name: &str) {
        let buf = self.bufs.remove(name).unwrap();
        self.gpu_alloc.dealloc_buf(buf);
    }

    pub fn realloc_buf(&mut self, name: &str, size: u64) -> vk::Buffer {
        let buffer = self.bufs.get_mut(name).unwrap();
        *buffer = self.gpu_alloc.realloc_buf(*buffer, size);
        debug_name(name, *buffer);
        *buffer
    }

    pub fn resize_buf(&mut self, name: &str, size: u64) -> vk::Buffer {
        let buffer = self.bufs.get_mut(name).unwrap();
        if self.gpu_alloc.is_mappable(*buffer) {
            *buffer = self.gpu_alloc.resize_mappable_buf(*buffer, size);
            debug_name(name, *buffer);
            *buffer
        } else {
            let old_size = self.buf_size(name);
            let staging = self.staging_buf(old_size);
            self.copy_buf(name, &staging);
            let buffer = self.realloc_buf(name, size);
            self.copy_buf(&staging, name);
            buffer
        }
    }

    pub fn buf(&self, name: &str) -> vk::Buffer {
        if name.is_empty() {
            return vk::Buffer::null();
        }
        *self
            .bufs
            .get(name)
            .unwrap_or_else(|| panic!("buffer not found: {name}"))
    }

    pub fn buf_size(&self, name: &str) -> u64 {
        self.gpu_alloc.buf_size(self.buf(name))
    }

    pub fn cmd(&self) -> vk::CommandBuffer {
        self.cmd_manager.cmd()
    }

    pub fn begin_cmd(&mut self) -> vk::CommandBuffer {
        self.cmd_manager.begin()
    }

    pub fn end_cmd(&mut self) -> vk::CommandBuffer {
        if self.cmd_info.render_area != Default::default() {
            self.end_render();
        }
        self.cmd_info = Default::default();
        self.cmd_manager.end()
    }

    pub fn submit_cmd(
        &mut self,
        cmd: vk::CommandBuffer,
        waits: &[vk::Semaphore],
        signals: &[vk::Semaphore],
        wait_dst_stage_mask: &[vk::PipelineStageFlags],
    ) {
        self.cmd_manager
            .submit(cmd, waits, signals, wait_dst_stage_mask)
    }

    pub fn wait_cmd(&mut self, cmd: vk::CommandBuffer) {
        self.cmd_manager.wait(cmd);
    }

    pub fn finish_cmd(&mut self) {
        let cmd = self.cmd_manager.end();
        self.cmd_manager.submit(cmd, &[], &[], &[]);
        self.cmd_manager.wait(cmd);
    }

    pub fn begin_render(
        &mut self,
        width: u32,
        height: u32,
        img_view_name: &str,
        sampled_img_view_name: &str,
    ) {
        let sampled = !sampled_img_view_name.is_empty();
        let img_view = self.img_view(img_view_name);
        self.cmd_info.render_area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D { width, height },
        };
        self.debug_begin(&format!("Begin Render({width}x{height})"));
        unsafe {
            gpu().cmd_begin_rendering(
                self.cmd(),
                &vk::RenderingInfo::default()
                    .render_area(self.cmd_info.render_area)
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
                        .resolve_image_layout(ImgLayout::COLOR)
                        .image_layout(ImgLayout::COLOR)
                        .image_view(if sampled {
                            self.img_view(sampled_img_view_name)
                        } else {
                            img_view
                        })]),
            )
        };
    }

    pub fn end_render(&mut self) {
        assert!(
            self.cmd_info.render_area != Default::default(),
            "can't end rendering that has not begun"
        );
        self.cmd_info.render_area = Default::default();
        unsafe {
            gpu().cmd_end_rendering(self.cmd());
        }
        self.debug_end();
    }

    pub fn set_viewport(&mut self, viewport: vk::Viewport) {
        if self.cmd_info.viewport.width == viewport.width
            && self.cmd_info.viewport.height == viewport.height
            && self.cmd_info.viewport.x == viewport.x
            && self.cmd_info.viewport.y == viewport.y
            && self.cmd_info.viewport.min_depth == viewport.min_depth
            && self.cmd_info.viewport.max_depth == viewport.max_depth
        {
            return;
        }
        self.cmd_info.viewport = viewport;
        unsafe { gpu().cmd_set_viewport(self.cmd(), 0, &[viewport]) };
    }

    pub fn set_scissor(&mut self, scissor: vk::Rect2D) {
        if self.cmd_info.scissor.extent.width == scissor.extent.width
            && self.cmd_info.scissor.extent.height == scissor.extent.height
            && self.cmd_info.scissor.offset.x == scissor.offset.x
            && self.cmd_info.scissor.offset.y == scissor.offset.y
        {
            return;
        }
        self.cmd_info.scissor = scissor;
        unsafe { gpu().cmd_set_scissor(self.cmd(), 0, &[scissor]) };
    }

    pub fn bind_pipeline(&mut self, name: &str) {
        let pipeline_data = self
            .pipelines
            .get(name)
            .unwrap_or_else(|| panic!("pipeline not found: {name}"))
            .clone();
        if pipeline_data.pipeline == self.cmd_info.pipeline_data.pipeline {
            return;
        }
        self.cmd_info.pipeline_data = pipeline_data;

        unsafe {
            if self.cmd_info.pipeline_data.bind_point == vk::PipelineBindPoint::GRAPHICS {
                let dyn_states = self.cmd_info.pipeline_data.info.dynamic_states.clone();
                let extent = self.cmd_info.render_area.extent;
                if dyn_states.contains(&vk::DynamicState::VIEWPORT) {
                    self.set_viewport(vk::Viewport {
                        x: 0.0,
                        y: 0.0,
                        width: extent.width as f32,
                        height: extent.height as f32,
                        min_depth: 0.0,
                        max_depth: 1.0,
                    });
                } else {
                    self.cmd_info.viewport = Default::default();
                }
                if dyn_states.contains(&vk::DynamicState::SCISSOR) {
                    self.set_scissor(vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent,
                    });
                } else {
                    self.cmd_info.scissor = Default::default();
                }
            }
            gpu().cmd_bind_pipeline(
                self.cmd(),
                self.cmd_info.pipeline_data.bind_point,
                self.cmd_info.pipeline_data.pipeline,
            )
        }
    }

    pub fn bind_ds(&mut self, name: &str) {
        self.cmd_info.desc_sets = vec![self.desc_set(name)];
        unsafe {
            gpu().cmd_bind_descriptor_sets(
                self.cmd(),
                self.cmd_info.pipeline_data.bind_point,
                self.cmd_info.pipeline_data.info.layout,
                0,
                &self.cmd_info.desc_sets,
                &[],
            );
        }
    }

    pub fn bind_vbo(&self, name: &str) {
        unsafe {
            gpu().cmd_bind_vertex_buffers(self.cmd(), 0, &[self.buf(name)], &[0]);
        }
    }

    pub fn bind_ebo(&self, name: &str) {
        unsafe {
            gpu().cmd_bind_index_buffer(self.cmd(), self.buf(name), 0, vk::IndexType::UINT32);
        }
    }

    pub fn bind_vao(&self, name: &str, index_buffer_offset: vk::DeviceSize) {
        unsafe {
            gpu().cmd_bind_vertex_buffers(self.cmd(), 0, &[self.buf(name)], &[0]);
            gpu().cmd_bind_index_buffer(
                self.cmd(),
                self.buf(name),
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

    pub fn set_img_layout(
        &mut self,
        img_name: &str,
        new_layout: vk::ImageLayout,
        src_stage: vk::PipelineStageFlags2,
        dst_stage: vk::PipelineStageFlags2,
        src_access: vk::AccessFlags2,
        dst_access: vk::AccessFlags2,
    ) {
        let cmd = self.cmd();
        let ImageData {
            img,
            views: _,
            info,
        } = self
            .imgs
            .get_mut(img_name)
            .unwrap_or_else(|| panic!("img not found: {img_name}"));
        if info.layout == new_layout {
            crate::log!("img layout transition to same layout: {new_layout:?}");
            return;
        }
        unsafe {
            gpu().cmd_pipeline_barrier2(
                cmd,
                &vk::DependencyInfo::default().image_memory_barriers(&[
                    vk::ImageMemoryBarrier2::default()
                        .dst_access_mask(dst_access)
                        .src_access_mask(src_access)
                        .src_stage_mask(src_stage)
                        .dst_stage_mask(dst_stage)
                        .image(*img)
                        .subresource_range(
                            vk::ImageSubresourceRange::default()
                                .aspect_mask(vk::ImageAspectFlags::COLOR)
                                .layer_count(1)
                                .level_count(1),
                        )
                        .old_layout(info.layout)
                        .new_layout(new_layout),
                ]),
            );
        }
        info.layout = new_layout;
    }

    pub fn staging_buf(&mut self, size: vk::DeviceSize) -> String {
        if self.buf_size("staging") < size {
            self.realloc_buf("staging", (size + 1).next_power_of_two());
        }
        "staging".to_string()
    }

    // TODO: do not begin cmd if cur cmd ends at convenient time
    // TODO: automatic pipeline barrier system
    pub fn copy_buf_off(
        &mut self,
        src_buf_name: &str,
        dst_buf_name: &str,
        src_off: vk::DeviceSize,
        dst_off: vk::DeviceSize,
    ) {
        let src_buf = self.buf(src_buf_name);
        let dst_buf = self.buf(dst_buf_name);
        let cmd = self.begin_cmd();
        unsafe {
            let buf_size = self
                .gpu_alloc
                .buf_size(src_buf)
                .min(self.gpu_alloc.buf_size(dst_buf));
            let copy_region = vk::BufferCopy::default()
                .size(buf_size)
                .src_offset(src_off)
                .dst_offset(dst_off);
            gpu().cmd_copy_buffer(cmd, src_buf, dst_buf, &[copy_region]);
        }
        self.finish_cmd();
    }

    pub fn copy_buf(&mut self, src_buf_name: &str, dst_buf_name: &str) {
        self.copy_buf_off(src_buf_name, dst_buf_name, 0, 0);
    }

    pub fn write_buf_off<T: ?Sized>(&mut self, name: &str, data: &T, off: vk::DeviceSize) {
        let buffer = self.buf(name);
        if self.gpu_alloc.is_mappable(buffer) {
            self.gpu_alloc.write_mapped_off(buffer, data, off);
        } else {
            let staging = self.staging_buf(size_of_val(data) as vk::DeviceSize);
            let staging_buf = self.buf(&staging);
            self.gpu_alloc.write_mapped(staging_buf, data);
            self.copy_buf_off(&staging, name, 0, off);
        }
    }

    pub fn read_buf_off<T: ?Sized>(&mut self, name: &str, data: &mut T, off: vk::DeviceSize) {
        let buf = self.buf(name);
        if self.gpu_alloc.is_mappable(buf) {
            self.gpu_alloc.read_mapped_off(buf, data, off);
        } else {
            let staging = self.staging_buf(size_of_val(data) as vk::DeviceSize);
            let staging_buf = self.buf(&staging);
            self.copy_buf_off(name, &staging, off, 0);
            self.gpu_alloc.read_mapped(staging_buf, data);
        }
    }

    pub fn map_buf(&mut self, name: &str) -> *mut u8 {
        let buf = self.buf(name);
        if self.gpu_alloc.is_mappable(buf) {
            self.gpu_alloc.map(buf)
        } else {
            panic!("buffer({name}) is not mappable")
        }
    }

    pub fn write_buf<T: ?Sized>(&mut self, name: &str, data: &T) {
        self.write_buf_off(name, data, 0);
    }

    pub fn read_buf<T: ?Sized>(&mut self, name: &str, data: &mut T) {
        self.read_buf_off(name, data, 0);
    }

    pub fn copy_buf_to_img(
        &mut self,
        src_buf_name: &str,
        dst_img_name: &str,
        copies: &[BufferImageCopy],
    ) {
        let src_buf = self.buf(src_buf_name);
        let dst_img_data = self.img(dst_img_name);
        unsafe {
            gpu().cmd_copy_buffer_to_image(
                self.cmd(),
                src_buf,
                dst_img_data.img,
                dst_img_data.info.layout,
                &copies
                    .iter()
                    .map(|c| {
                        vk::BufferImageCopy::default()
                            .buffer_offset(c.buf_off)
                            .buffer_row_length(c.buf_width)
                            .buffer_image_height(c.buf_height)
                            .image_extent(vk::Extent3D {
                                width: c.buf_width,
                                height: c.buf_height,
                                depth: 1,
                            })
                            .image_offset(vk::Offset3D {
                                x: c.img_off_x as i32,
                                y: c.img_off_y as i32,
                                z: 0,
                            })
                            .image_subresource(
                                vk::ImageSubresourceLayers::default()
                                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                                    .layer_count(1),
                            )
                    })
                    .collect::<Vec<_>>(),
            );
        }
    }

    pub fn writes_ds(
        &self,
        name: &str,
        buf_range_binds: &[(&str, std::ops::Range<vk::DeviceSize>, u32)],
        img_view_img_layout_sampler_binds: &[(&str, vk::ImageLayout, vk::Sampler, u32)],
    ) {
        let DescSetData { desc_set, binds } = &self
            .desc_sets
            .get(name)
            .unwrap_or_else(|| panic!("descriptor not found: {name}"));
        let buf_infos = buf_range_binds
            .iter()
            .map(|(buf, rng, _bind)| {
                vk::DescriptorBufferInfo::default()
                    .buffer(self.buf(buf))
                    .offset(rng.start)
                    .range(if rng.end == vk::WHOLE_SIZE {
                        vk::WHOLE_SIZE
                    } else {
                        rng.end - rng.start
                    })
            })
            .collect::<Vec<_>>();
        let img_infos = img_view_img_layout_sampler_binds
            .iter()
            .map(|&(img_view, layout, sampler, _bind)| {
                vk::DescriptorImageInfo::default()
                    .image_view(self.img_view(img_view))
                    .image_layout(layout)
                    .sampler(sampler)
            })
            .collect::<Vec<_>>();
        let desc_buf_writes = buf_range_binds
            .iter()
            .enumerate()
            .map(|(i, (_buf, _rng, bind))| {
                vk::WriteDescriptorSet::default()
                    .buffer_info(&buf_infos[i..i + 1])
                    .descriptor_count(1)
                    .descriptor_type(binds[*bind as usize].desc_ty)
                    .dst_binding(*bind)
                    .dst_set(*desc_set)
            })
            .collect::<Vec<_>>();
        let mut desc_img_writes = img_view_img_layout_sampler_binds
            .iter()
            .enumerate()
            .map(|(i, (_img, _layout, _sampler, bind))| {
                vk::WriteDescriptorSet::default()
                    .image_info(&img_infos[i..i + 1])
                    .descriptor_count(1)
                    .descriptor_type(binds[*bind as usize].desc_ty)
                    .dst_binding(*bind)
                    .dst_set(*desc_set)
            })
            .collect::<Vec<_>>();
        let mut desc_writes = desc_buf_writes;
        desc_writes.append(&mut desc_img_writes);
        unsafe { gpu().update_descriptor_sets(&desc_writes, &[]) }
    }

    pub fn write_ds_buf_ranges(
        &self,
        name: &str,
        buf_range_binds: &[(&str, std::ops::Range<vk::DeviceSize>, u32)],
    ) {
        self.writes_ds(name, buf_range_binds, &[]);
    }

    pub fn write_ds_buf_range(
        &self,
        name: &str,
        buf_name: &str,
        buf_range: std::ops::Range<vk::DeviceSize>,
        binding: u32,
    ) {
        self.write_ds_buf_ranges(name, &[(buf_name, buf_range, binding)]);
    }

    pub fn write_ds_bufs(&self, name: &str, buf_binds: &[(&str, u32)]) {
        self.write_ds_buf_ranges(
            name,
            &buf_binds
                .iter()
                .map(|&(buf, bind)| (buf, 0..vk::WHOLE_SIZE, bind))
                .collect::<Vec<_>>(),
        );
    }

    pub fn write_ds_buf(&self, name: &str, buf_name: &str, binding: u32) {
        self.write_ds_buf_range(name, buf_name, 0..vk::WHOLE_SIZE, binding)
    }

    pub fn write_ds_img(
        &self,
        name: &str,
        img_view_name: &str,
        img_layout: vk::ImageLayout,
        binding: u32,
    ) {
        self.writes_ds(
            name,
            &[],
            &[(img_view_name, img_layout, vk::Sampler::null(), binding)],
        );
    }

    pub fn write_ds_sampler(&self, name: &str, sampler_name: &str, binding: u32) {
        self.writes_ds(
            name,
            &[],
            &[(
                "",
                ImgLayout::UNDEFINED,
                self.sampler(sampler_name),
                binding,
            )],
        );
    }

    pub fn clear(&self, img: vk::Image, color: [f32; 4]) {
        unsafe {
            gpu().cmd_clear_color_image(
                self.cmd(),
                img,
                ImgLayout::COLOR,
                &vk::ClearColorValue { float32: color },
                &[vk::ImageSubresourceRange::default()
                    .aspect_mask(vk::ImageAspectFlags::COLOR)
                    .layer_count(1)
                    .level_count(1)],
            );
        }
    }

    pub fn blit(&self, src_img_name: &str, dst_img_name: &str) {
        let ImageData {
            img: src,
            views: _,
            info: src_info,
        } = self.img(src_img_name);
        let ImageData {
            img: dst,
            views: _,
            info: dst_info,
        } = self.img(dst_img_name);
        assert_eq!(
            src_info.width == dst_info.width,
            src_info.height == dst_info.height,
            "blit src img size must equal dst size"
        );
        let min = vk::Offset3D::default().x(0).y(0).z(0);
        let max = min.x(src_info.width as i32).y(src_info.height as i32).z(1);
        let subres = vk::ImageSubresourceLayers::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .layer_count(1);
        unsafe {
            gpu().cmd_blit_image(
                self.cmd(),
                *src,
                src_info.layout,
                *dst,
                dst_info.layout,
                &[vk::ImageBlit::default()
                    .src_offsets([min, max])
                    .src_subresource(subres)
                    .dst_offsets([min, max])
                    .dst_subresource(subres)],
                vk::Filter::NEAREST,
            )
        };
    }

    pub fn recreate_swapchain(&mut self) -> vk::Extent2D {
        let surf_caps = self.surface_capabilities();
        let size = self.swapchain_size;
        let surf_res = match surf_caps.current_extent.width {
            u32::MAX => vk::Extent2D {
                width: size.width,
                height: size.height,
            },
            _ => surf_caps.current_extent,
        };
        if surf_res.width == 0 || surf_res.height == 0 || surf_res == size {
            return surf_res;
        }
        self.swapchain_size = surf_res;
        scope_time!("resize {}x{}", surf_res.width, surf_res.height);
        let pre_transform = if surf_caps
            .supported_transforms
            .contains(vk::SurfaceTransformFlagsKHR::IDENTITY)
        {
            vk::SurfaceTransformFlagsKHR::IDENTITY
        } else {
            surf_caps.current_transform
        };
        let present_mode = self
            .surface_present_modes
            .iter()
            .find(|&mode| *mode == vk::PresentModeKHR::MAILBOX)
            .copied()
            .unwrap_or(vk::PresentModeKHR::FIFO);
        let mut desired_img_cnt = surf_caps.min_image_count + 1;
        if surf_caps.max_image_count > 0 {
            desired_img_cnt = surf_caps.max_image_count.min(desired_img_cnt);
        }
        // Destroy old swap chain images
        let old_swapchain = self.swapchain;
        self.swapchain = unsafe {
            self.swapchain_loader
                .create_swapchain(
                    &vk::SwapchainCreateInfoKHR::default()
                        .surface(self.surface)
                        .min_image_count(desired_img_cnt)
                        .image_color_space(self.surface_format.color_space)
                        .image_format(self.surface_format.format)
                        .image_extent(surf_res)
                        .image_array_layers(1)
                        .image_usage(ImgUsage::COLOR | ImgUsage::DST)
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
            // NOTE: assumes swapchain image count is constant
            for i in 0..desired_img_cnt {
                let img_name = format!("swapchain image {i}");
                let img_views = self.imgs[&img_name].views.clone();
                for img_view in img_views {
                    self.remove_img_view(&img_view);
                }
                self.imgs.remove(&img_name).unwrap();
            }
            unsafe {
                self.swapchain_loader
                    .destroy_swapchain(old_swapchain, alloc_callbacks())
            };
        }

        let swapchain_imgs = unsafe {
            self.swapchain_loader
                .get_swapchain_images(self.swapchain)
                .unwrap()
        };
        for (i, swap_img) in swapchain_imgs.into_iter().enumerate() {
            let img_name = format!("swapchain image {i}");
            debug_name(&img_name, swap_img);
            let img_view_name = format!("swapchain image view {i}");
            self.imgs.insert(
                img_name.clone(),
                ImageData {
                    img: swap_img,
                    views: vec![],
                    info: ImageInfo::new()
                        .width(surf_res.width)
                        .height(surf_res.height)
                        .format(self.surface_format.format)
                        .usage(ImgUsage::COLOR | ImgUsage::DST),
                },
            );
            self.add_img_view(&img_view_name, &img_name);
        }

        gpu_idle();
        surf_res
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

    pub fn cur_img(&self) -> String {
        format!("swapchain image {}", self.swapchain_img_idx)
    }

    pub fn cur_img_view(&self) -> String {
        format!("swapchain image view {}", self.swapchain_img_idx)
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
