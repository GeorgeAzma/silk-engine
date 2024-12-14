use pipeline::{GraphicsPipelineInfo, PipelineStageInfo};
use shader::Shader;

use crate::*;

struct ShaderData {
    shader: Shader,
    module: vk::ShaderModule,
    dsls: Vec<vk::DescriptorSetLayout>,
    pipeline_layout: vk::PipelineLayout,
    pipeline_stages: Vec<PipelineStageInfo>,
    push_constant_ranges: Vec<vk::PushConstantRange>,
}

#[derive(Default, Clone)]
struct PipelineData {
    pipeline: vk::Pipeline,
    info: GraphicsPipelineInfo,
    shader_name: String,
    bind_point: vk::PipelineBindPoint,
}

#[derive(Default)]
struct CmdState {
    cmd: vk::CommandBuffer,
    pipeline_data: PipelineData,
    desc_sets: Vec<vk::DescriptorSet>,
    render_area: vk::Rect2D,
}

#[derive(Default)]
pub struct RenderContext {
    cmd_state: CmdState,
    shaders: HashMap<String, ShaderData>,
    pipelines: HashMap<String, PipelineData>,
    desc_sets: HashMap<String, vk::DescriptorSet>,
    cmds: HashMap<String, vk::CommandBuffer>,
    buffers: HashMap<String, vk::Buffer>,
}

impl RenderContext {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn get_shader(&self, name: &str) -> &Shader {
        &self.shaders[name].shader
    }

    pub fn add_shader(&mut self, shader_name: &str) -> &Shader {
        self.add_shader_pcr(shader_name, &[])
    }

    pub fn add_shader_pcr(
        &mut self,
        shader_name: &str,
        push_constant_ranges: &[vk::PushConstantRange],
    ) -> &Shader {
        let shader_data = self
            .shaders
            .entry(shader_name.to_string())
            .or_insert_with(|| {
                let shader = Shader::new(shader_name);
                let dsls = shader.create_dsls();
                let module = shader.create_module();
                let pipeline_layout = PIPELINE_LAYOUT_MANAGER
                    .lock()
                    .unwrap()
                    .get(&dsls, push_constant_ranges);
                let pipeline_stages = shader.get_pipeline_stages(module);
                ShaderData {
                    shader,
                    module,
                    dsls,
                    pipeline_layout,
                    pipeline_stages,
                    push_constant_ranges: push_constant_ranges.to_vec(),
                }
            });
        &shader_data.shader
    }

    pub fn add_pipeline(
        &mut self,
        pipeline_name: &str,
        shader_name: &str,
        pipeline_info: GraphicsPipelineInfo,
        vert_input_bindings: &[(bool, Vec<u32>)],
    ) -> vk::Pipeline {
        self.get_shader(shader_name);
        let shader_data = &self.shaders[shader_name];
        let pipeline_info = pipeline_info
            .layout(shader_data.pipeline_layout)
            .stages(&shader_data.pipeline_stages)
            .vert_layout(&shader_data.shader, vert_input_bindings);
        let pipeline = pipeline_info.build();
        let inserted = self
            .pipelines
            .insert(
                pipeline_name.to_string(),
                PipelineData {
                    pipeline,
                    info: pipeline_info,
                    shader_name: shader_name.to_string(),
                    bind_point: vk::PipelineBindPoint::GRAPHICS,
                },
            )
            .is_none();
        assert!(inserted, "pipeline already exists");
        pipeline
    }

    pub fn add_desc_set(
        &mut self,
        desc_name: &str,
        shader_name: &str,
        group: usize,
    ) -> vk::DescriptorSet {
        let dsl = self.shaders[shader_name].dsls[group];
        let desc_set = DESC_ALLOC.alloc_single(dsl);
        let inserted = self
            .desc_sets
            .insert(desc_name.to_string(), desc_set)
            .is_none();
        assert!(inserted, "desc set already exists");
        desc_set
    }

    pub fn add_desc_sets(
        &mut self,
        desc_names: &[&str],
        shader_name: &str,
    ) -> Vec<vk::DescriptorSet> {
        let dsls = &self.shaders[shader_name].dsls;
        let desc_sets = DESC_ALLOC.alloc(dsls);
        for (desc_name, desc_set) in desc_names.iter().zip(desc_sets.iter()) {
            let inserted = self
                .desc_sets
                .insert(desc_name.to_string(), *desc_set)
                .is_none();
            assert!(inserted, "desc set already exists");
        }
        desc_sets
    }

    pub fn get_desc_set(&self, name: &str) -> vk::DescriptorSet {
        self.desc_sets[name]
    }

    pub fn add_cmd(&mut self, name: &str) -> vk::CommandBuffer {
        let cmd_buf = CMD_ALLOC.alloc();
        let inserted = self.cmds.insert(name.to_string(), cmd_buf).is_none();
        assert!(inserted, "cmd buf already exists");
        cmd_buf
    }

    pub fn add_buffer(
        &mut self,
        name: &str,
        size: u64,
        usage: vk::BufferUsageFlags,
        mem_props: vk::MemoryPropertyFlags,
    ) -> vk::Buffer {
        let buf = BUFFER_ALLOC.lock().unwrap().alloc(size, usage, mem_props); // TODO: free
        let inserted = self.buffers.insert(name.to_string(), buf).is_none();
        assert!(inserted, "buffer already exists");
        buf
    }

    pub fn remove_buffer(&mut self, name: &str) {
        let buf = self.buffers.remove(name).unwrap();
        BUFFER_ALLOC.lock().unwrap().dealloc(buf);
    }

    pub fn get_cmd(&self, name: &str) -> vk::CommandBuffer {
        self.cmds[name]
    }

    pub fn cmd(&self) -> vk::CommandBuffer {
        self.cmd_state.cmd
    }

    pub fn begin_cmd(&mut self, name: &str) -> vk::CommandBuffer {
        assert!(
            self.cmd_state.cmd == Default::default(),
            "cmd has already begun"
        );
        self.cmd_state.cmd = self.get_cmd(name);
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
            "cmd has not begun"
        );
        if self.cmd_state.render_area != Default::default() {
            self.end_render();
        }
        unsafe {
            DEVICE.end_command_buffer(self.cmd_state.cmd).unwrap();
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
                self.cmd_state.cmd,
                &vk::RenderingInfo::default()
                    .render_area(self.cmd_state.render_area)
                    .layer_count(1)
                    .color_attachments(&[vk::RenderingAttachmentInfo::default()
                        .load_op(vk::AttachmentLoadOp::CLEAR)
                        .store_op(vk::AttachmentStoreOp::STORE)
                        .clear_value(vk::ClearValue {
                            color: vk::ClearColorValue {
                                float32: [1.0, 0.0, 1.0, 1.0],
                            },
                        })
                        .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                        .image_view(image_view)]),
            )
        };
    }

    pub fn end_render(&mut self) {
        self.cmd_state.render_area = Default::default();
        unsafe {
            DEVICE.cmd_end_rendering(self.cmd_state.cmd);
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
                    self.cmd_state.cmd,
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
                    self.cmd_state.cmd,
                    0,
                    &[vk::Rect2D {
                        offset: vk::Offset2D { x: 0, y: 0 },
                        extent,
                    }],
                );
            }
            DEVICE.cmd_bind_pipeline(
                self.cmd_state.cmd,
                self.cmd_state.pipeline_data.bind_point,
                self.cmd_state.pipeline_data.pipeline,
            )
        }
    }

    pub fn bind_desc_set(&mut self, name: &str) {
        self.cmd_state.desc_sets = vec![self.desc_sets[name]];
        unsafe {
            DEVICE.cmd_bind_descriptor_sets(
                self.cmd_state.cmd,
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
            DEVICE.cmd_bind_vertex_buffers(self.cmd_state.cmd, 0, &[self.buffers[name]], &[0]);
        }
    }
}
