use std::collections::HashMap;

use ash::vk;

use crate::{gpu, scope_time};

use super::shader::Shader;
use super::vulkan::pipeline::{GraphicsPipeline, PipelineStageInfo};
use super::{BufferAlloc, CmdAlloc, DSLManager, DescAlloc, PipelineLayoutManager, RenderPass};

#[cfg(debug_assertions)]
static DEBUG_UTILS_LOADER: std::sync::LazyLock<ash::ext::debug_utils::Device> =
    std::sync::LazyLock::new(|| ash::ext::debug_utils::Device::new(crate::instance(), gpu()));

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

#[derive(Default)]
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
}

impl RenderContext {
    pub fn new() -> Self {
        Default::default()
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
        scope_time!("create pipeline: {name}");
        self.shader(shader_name);
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
        let buf = self.buffer_alloc.alloc(size, usage, mem_props); // TODO: free
        let inserted = self.buffers.insert(name.to_string(), buf).is_none();
        assert!(inserted, "buffer already exists: {name}");
        debug_name(name, buf);
        buf
    }

    pub fn remove_buffer(&mut self, name: &str) {
        let buf = self.buffers.remove(name).unwrap();
        self.buffer_alloc.dealloc(buf);
    }

    pub fn buffer(&self, name: &str) -> vk::Buffer {
        self.buffers[name]
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
            "failed to begin cmd, other cmd is already begun: {name}"
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

    pub fn bind_vert(&self, name: &str) {
        unsafe {
            gpu().cmd_bind_vertex_buffers(self.cmd(), 0, &[self.buffers[name]], &[0]);
        }
    }

    pub fn draw(&self, vertices: u32, instances: u32) {
        unsafe {
            gpu().cmd_draw(self.cmd(), vertices, instances, 0, 0);
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

    pub fn copy_buffer(&mut self, src_buffer: vk::Buffer, dst_buffer: vk::Buffer) {
        let cmd = self.begin_cmd("init");
        let buf_size = self.buffer_alloc.get_size(src_buffer) as usize;
        assert_eq!(
            self.buffer_alloc.get_size(src_buffer),
            self.buffer_alloc.get_size(dst_buffer)
        );
        unsafe {
            let copy_region = vk::BufferCopy::default().size(buf_size as u64);
            gpu().cmd_copy_buffer(cmd, src_buffer, dst_buffer, &[copy_region]);
        }
        self.end_cmd();
    }

    pub fn write_buffer<T>(&mut self, buffer: vk::Buffer, data: &T) {
        let buf_size = self.buffer_alloc.get_size(buffer) as usize;
        assert_eq!(buf_size, size_of_val(data));
        if self.buffer_alloc.is_mappable(buffer) {
            self.buffer_alloc.write_mapped(buffer, data);
        } else {
            let staging_buffer = self.buffer_alloc.alloc_staging_src(data);
            self.copy_buffer(staging_buffer, buffer);
            self.buffer_alloc.dealloc(staging_buffer);
        }
    }

    pub fn read_buffer<T>(&mut self, buffer: vk::Buffer, data: &mut T) {
        let buf_size = self.buffer_alloc.get_size(buffer) as usize;
        assert_eq!(buf_size, size_of_val(data));
        if self.buffer_alloc.is_mappable(buffer) {
            self.buffer_alloc.read_mapped(buffer, data);
        } else {
            let staging_buffer = self.buffer_alloc.alloc_staging_dst(data);
            self.copy_buffer(buffer, staging_buffer);
            self.buffer_alloc.read_mapped(staging_buffer, data);
            self.buffer_alloc.dealloc(staging_buffer);
        }
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

#[macro_export]
macro_rules! debug_scope {
    ($name:expr) => {
        let _d = DebugScope::new($name);
    };
    ($name:expr, [$r:literal, $g:literal, $b:literal, $a:literal]) => {
        let _d = DebugScope::new($name, [r, g, b, a]);
    };
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
