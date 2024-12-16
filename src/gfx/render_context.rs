use std::collections::HashMap;

use ash::vk;

use crate::{
    scope_time, swap_img_idx, DebugMarker, BUFFER_ALLOC, DESC_ALLOC, DEVICE, SWAPCHAIN_IMAGES,
};

use super::shader::Shader;
use super::vulkan::pipeline::{GraphicsPipeline, PipelineStageInfo};
use super::CMD_ALLOC;
use super::{RenderPass, PIPELINE_LAYOUT_MANAGER};

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
        let shader_data = self.shaders.entry(name.to_string()).or_insert_with(|| {
            let shader = Shader::new(name);
            let dsls = shader.create_dsls();
            let module = shader.create_module();
            DebugMarker::name(name, module);
            let pipeline_layout = PIPELINE_LAYOUT_MANAGER.write().unwrap().get(&dsls);
            let pipeline_stages = shader.get_pipeline_stages(module);
            ShaderData {
                shader,
                dsls,
                pipeline_layout,
                pipeline_stages,
            }
        });
        &shader_data.shader
    }

    pub fn add_render_pass(&mut self, name: &str, mut render_pass: RenderPass) -> vk::RenderPass {
        let rp = render_pass.build();
        let inserted = self
            .render_passes
            .insert(name.to_string(), render_pass)
            .is_none();
        assert!(inserted, "render pass already exists: {name}");
        DebugMarker::name(name, rp);
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
        DebugMarker::name(name, pipeline);
        pipeline
    }

    pub fn add_desc_set(
        &mut self,
        name: &str,
        shader_name: &str,
        group: usize,
    ) -> vk::DescriptorSet {
        let dsl = self.shaders[shader_name].dsls[group];
        let desc_set = DESC_ALLOC.write().unwrap().alloc_one(dsl);
        let inserted = self.desc_sets.insert(name.to_string(), desc_set).is_none();
        assert!(inserted, "desc set already exists: {name}");
        DebugMarker::name(name, desc_set);
        desc_set
    }

    pub fn add_desc_sets(&mut self, names: &[&str], shader_name: &str) -> Vec<vk::DescriptorSet> {
        let dsls = &self.shaders[shader_name].dsls;
        let desc_sets = DESC_ALLOC.write().unwrap().alloc(dsls);
        for (name, &desc_set) in names.iter().zip(desc_sets.iter()) {
            let inserted = self.desc_sets.insert(name.to_string(), desc_set).is_none();
            assert!(inserted, "desc set already exists: {name}");
            DebugMarker::name(name, desc_set);
        }
        desc_sets
    }

    pub fn desc_set(&self, name: &str) -> vk::DescriptorSet {
        self.desc_sets[name]
    }

    pub fn add_cmds(&mut self, names: &[&str]) -> Vec<vk::CommandBuffer> {
        let cmds = CMD_ALLOC.read().unwrap().alloc(names.len() as u32);
        for (&cmd, &name) in cmds.iter().zip(names.iter()) {
            let inserted = self.cmds.insert(name.to_string(), cmd).is_none();
            assert!(inserted, "cmd buf already exists: {name}");
            DebugMarker::name(name, cmd);
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
        CMD_ALLOC.read().unwrap().dealloc(&cmds);
    }

    pub fn remove_cmd(&mut self, name: &str) {
        self.remove_cmds(&[name])
    }

    pub fn reset_cmds(&mut self, names: &[&str]) {
        for name in names.iter() {
            CMD_ALLOC
                .read()
                .unwrap()
                .reset(self.cmds[&name.to_string()]);
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
        let buf = BUFFER_ALLOC.write().unwrap().alloc(size, usage, mem_props); // TODO: free
        let inserted = self.buffers.insert(name.to_string(), buf).is_none();
        assert!(inserted, "buffer already exists: {name}");
        DebugMarker::name(name, buf);
        buf
    }

    pub fn remove_buffer(&mut self, name: &str) {
        let buf = self.buffers.remove(name).unwrap();
        BUFFER_ALLOC.write().unwrap().dealloc(buf);
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
        assert!(
            self.cmd_state.cmd == Default::default(),
            "cmd has already begun: {name}"
        );
        self.cmd_state.cmd = self.get_cmd(name);
        self.cmd_state.cmd_name = name.to_string();
        unsafe {
            DEVICE
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
        assert!(
            self.cmd_state.cmd != Default::default(),
            "can't end cmd that has not begun"
        );
        if self.cmd_state.render_area != Default::default() {
            self.end_render();
        }
        unsafe {
            DEVICE.end_command_buffer(self.cmd()).unwrap();
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
            DEVICE
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
            DEVICE.cmd_begin_rendering(
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
                DEVICE.cmd_end_rendering(self.cmd());
            } else {
                DEVICE.cmd_end_render_pass(self.cmd());
                self.cmd_state.render_pass = Default::default();
            }
        }
    }

    pub fn begin_rp(&mut self, name: &str, width: u32, height: u32, img_views: &[vk::ImageView]) {
        self.cmd_state.render_area = vk::Rect2D {
            offset: vk::Offset2D { x: 0, y: 0 },
            extent: vk::Extent2D { width, height },
        };
        let img_cnt = SWAPCHAIN_IMAGES.read().unwrap().len();
        let render_pass = self.render_passes.get_mut(name).unwrap();
        if render_pass.framebuffer_size != self.cmd_state.render_area.extent
            || render_pass.framebuffers.len() < img_cnt
        {
            render_pass.recreate_framebuffer(width, height, img_views, img_cnt);
        }
        self.cmd_state.render_pass = render_pass.clone();
        let render_pass = &self.cmd_state.render_pass;
        unsafe {
            DEVICE.cmd_begin_render_pass(
                self.cmd(),
                &vk::RenderPassBeginInfo::default()
                    .render_area(self.cmd_state.render_area)
                    .clear_values(&[vk::ClearValue {
                        color: vk::ClearColorValue {
                            float32: [0.0, 0.0, 0.0, 0.0],
                        },
                    }])
                    .render_pass(render_pass.render_pass)
                    .framebuffer(render_pass.framebuffers[swap_img_idx()]),
                vk::SubpassContents::INLINE,
            );
        }
    }

    pub fn bind_pipeline(&mut self, name: &str) {
        let pipeline_data = self.pipelines[name].clone();
        self.cmd_state.pipeline_data = pipeline_data;

        unsafe {
            let dyn_states = &self.cmd_state.pipeline_data.info.dynamic_states;
            let extent = self.cmd_state.render_area.extent;
            if dyn_states.contains(&vk::DynamicState::VIEWPORT) {
                DEVICE.cmd_set_viewport(
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
                DEVICE.cmd_set_scissor(
                    self.cmd(),
                    0,
                    &[vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent,
                    }],
                );
            }
            DEVICE.cmd_bind_pipeline(
                self.cmd(),
                self.cmd_state.pipeline_data.bind_point,
                self.cmd_state.pipeline_data.pipeline,
            )
        }
    }

    pub fn bind_desc_set(&mut self, name: &str) {
        self.cmd_state.desc_sets = vec![self.desc_sets[name]];
        unsafe {
            DEVICE.cmd_bind_descriptor_sets(
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
            DEVICE.cmd_bind_vertex_buffers(self.cmd(), 0, &[self.buffers[name]], &[0]);
        }
    }

    pub fn draw(&self, vertices: u32, instances: u32) {
        unsafe {
            DEVICE.cmd_draw(self.cmd(), vertices, instances, 0, 0);
        }
    }
}
