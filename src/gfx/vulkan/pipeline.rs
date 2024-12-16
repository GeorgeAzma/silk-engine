use crate::*;
use gfx::shader::Shader;
use util::Mem;

const fn pipeline_cache_path() -> &'static str {
    "res/cache/pipeline_cache"
}

#[derive(Default, Clone)]
pub struct PipelineStageInfo {
    pub stage: vk::ShaderStageFlags,
    pub module: vk::ShaderModule,
    pub name: String,
    pub spec_map_entries: Vec<vk::SpecializationMapEntry>,
    pub spec_data: Vec<u8>,
}

impl<'a> From<&'a PipelineStageInfo> for vk::PipelineShaderStageCreateInfo<'a> {
    fn from(value: &'a PipelineStageInfo) -> Self {
        vk::PipelineShaderStageCreateInfo::default()
            .stage(value.stage)
            .module(value.module)
            .name(std::ffi::CStr::from_bytes_with_nul(value.name.as_bytes()).unwrap())
    }
}

#[derive(Clone)]
pub struct GraphicsPipeline {
    pub stages: Vec<PipelineStageInfo>,
    pub vertex_input_binding_descriptions: Vec<vk::VertexInputBindingDescription>,
    pub vertex_input_attribute_descriptions: Vec<vk::VertexInputAttributeDescription>,
    pub topology: vk::PrimitiveTopology,
    pub primitive_restart_enable: bool,
    pub viewports: Vec<vk::Viewport>,
    pub scissors: Vec<vk::Rect2D>,
    pub depth_clamp_enable: bool,
    pub rasterizer_discard_enable: bool,
    pub polygon_mode: vk::PolygonMode,
    pub cull_mode: vk::CullModeFlags,
    pub front_face: vk::FrontFace,
    pub depth_bias_enable: bool,
    pub depth_bias_constant_factor: f32,
    pub depth_bias_clamp: f32,
    pub depth_bias_slope_factor: f32,
    pub line_width: f32,
    pub rasterization_samples: vk::SampleCountFlags,
    pub sample_shading_enable: bool,
    pub min_sample_shading: f32,
    pub alpha_to_coverage_enable: bool,
    pub alpha_to_one_enable: bool,
    pub depth_test_enable: bool,
    pub depth_write_enable: bool,
    pub depth_compare_op: vk::CompareOp,
    pub depth_bounds_test_enable: bool,
    pub stencil_test_enable: bool,
    pub front: vk::StencilOpState,
    pub back: vk::StencilOpState,
    pub min_depth_bounds: f32,
    pub max_depth_bounds: f32,
    pub logic_op_enable: bool,
    pub logic_op: vk::LogicOp,
    pub attachments: Vec<vk::PipelineColorBlendAttachmentState>,
    pub blend_constants: [f32; 4],
    pub dynamic_states: Vec<vk::DynamicState>,
    pub view_mask: u32,
    pub color_attachment_formats: Vec<vk::Format>,
    pub depth_attachment_format: vk::Format,
    pub stencil_attachment_format: vk::Format,
    pub layout: vk::PipelineLayout,
    pub render_pass: vk::RenderPass,
    pub subpass: u32,
}

impl Default for GraphicsPipeline {
    fn default() -> Self {
        Self {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            polygon_mode: vk::PolygonMode::FILL,
            front_face: vk::FrontFace::COUNTER_CLOCKWISE,
            line_width: 1.0,
            rasterization_samples: vk::SampleCountFlags::TYPE_1,
            max_depth_bounds: 1.0,
            stages: Default::default(),
            vertex_input_binding_descriptions: Default::default(),
            vertex_input_attribute_descriptions: Default::default(),
            primitive_restart_enable: Default::default(),
            viewports: Default::default(),
            scissors: Default::default(),
            depth_clamp_enable: Default::default(),
            rasterizer_discard_enable: Default::default(),
            cull_mode: Default::default(),
            depth_bias_enable: Default::default(),
            depth_bias_constant_factor: Default::default(),
            depth_bias_clamp: Default::default(),
            depth_bias_slope_factor: Default::default(),
            sample_shading_enable: Default::default(),
            min_sample_shading: Default::default(),
            alpha_to_coverage_enable: Default::default(),
            alpha_to_one_enable: Default::default(),
            depth_test_enable: Default::default(),
            depth_write_enable: Default::default(),
            depth_compare_op: Default::default(),
            depth_bounds_test_enable: Default::default(),
            stencil_test_enable: Default::default(),
            front: Default::default(),
            back: Default::default(),
            min_depth_bounds: Default::default(),
            logic_op_enable: Default::default(),
            logic_op: Default::default(),
            attachments: Default::default(),
            blend_constants: Default::default(),
            dynamic_states: Default::default(),
            view_mask: Default::default(),
            color_attachment_formats: Default::default(),
            depth_attachment_format: Default::default(),
            stencil_attachment_format: Default::default(),
            layout: Default::default(),
            render_pass: Default::default(),
            subpass: Default::default(),
        }
    }
}

impl GraphicsPipeline {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn depth(mut self) -> Self {
        self.depth_test_enable = true;
        self.depth_write_enable = true;
        self.depth_compare_op = vk::CompareOp::LESS;
        self
    }

    pub fn dyn_size(mut self) -> Self {
        self.dynamic_states.push(vk::DynamicState::VIEWPORT);
        self.dynamic_states.push(vk::DynamicState::SCISSOR);
        self
    }

    pub fn layout(mut self, layout: vk::PipelineLayout) -> Self {
        self.layout = layout;
        self
    }

    pub fn stage(mut self, stage: PipelineStageInfo) -> Self {
        self.stages.push(stage);
        self
    }
    pub fn stages(mut self, stages: &[PipelineStageInfo]) -> Self {
        self.stages.extend(stages.iter().cloned());
        self
    }

    pub fn vert_layout(
        mut self,
        shader: &Shader,
        vert_input_bindings: &[(bool, Vec<u32>)],
    ) -> Self {
        (
            self.vertex_input_binding_descriptions,
            self.vertex_input_attribute_descriptions,
        ) = shader.get_vert_layout(vert_input_bindings);
        self
    }

    pub fn blend_attachment_standard(mut self) -> Self {
        self.attachments.push(
            vk::PipelineColorBlendAttachmentState::default()
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_blend_op(vk::BlendOp::ADD)
                .blend_enable(true)
                .color_write_mask(vk::ColorComponentFlags::RGBA)
                .dst_alpha_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .src_alpha_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .src_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA),
        );
        self
    }

    pub fn blend_attachment_empty(mut self) -> Self {
        self.attachments.push(
            vk::PipelineColorBlendAttachmentState::default()
                .alpha_blend_op(vk::BlendOp::ADD)
                .color_blend_op(vk::BlendOp::ADD)
                .blend_enable(false)
                .color_write_mask(vk::ColorComponentFlags::RGBA)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .dst_color_blend_factor(vk::BlendFactor::ZERO)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .src_color_blend_factor(vk::BlendFactor::ONE),
        );
        self
    }

    pub fn color_attachment(mut self, format: vk::Format) -> Self {
        // TODO: assert valid color format (same for depth)
        self.color_attachment_formats.push(format);
        self
    }

    pub fn depth_attachment(mut self, format: vk::Format) -> Self {
        self.depth_attachment_format = format;
        self
    }

    pub fn render_pass(mut self, render_pass: vk::RenderPass) -> Self {
        self.render_pass = render_pass;
        self
    }

    pub fn subpass(mut self, subpass: u32) -> Self {
        self.subpass = subpass;
        self
    }

    pub fn build(&self) -> vk::Pipeline {
        let stages = self.stages.iter().map(|s| s.into()).collect::<Vec<_>>();
        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_attribute_descriptions(&self.vertex_input_attribute_descriptions)
            .vertex_binding_descriptions(&self.vertex_input_binding_descriptions);
        let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::default()
            .topology(self.topology)
            .primitive_restart_enable(self.primitive_restart_enable);
        let mut viewport_state = vk::PipelineViewportStateCreateInfo::default()
            .viewports(&self.viewports)
            .scissors(&self.scissors);
        viewport_state.viewport_count = viewport_state.viewport_count.max(1);
        viewport_state.scissor_count = viewport_state.scissor_count.max(1);
        let rasterization_state = vk::PipelineRasterizationStateCreateInfo::default()
            .depth_clamp_enable(self.depth_clamp_enable)
            .rasterizer_discard_enable(self.rasterizer_discard_enable)
            .polygon_mode(self.polygon_mode)
            .cull_mode(self.cull_mode)
            .front_face(self.front_face)
            .depth_bias_enable(self.depth_bias_enable)
            .depth_bias_constant_factor(self.depth_bias_constant_factor)
            .depth_bias_clamp(self.depth_bias_clamp)
            .depth_bias_slope_factor(self.depth_bias_slope_factor)
            .line_width(self.line_width);
        let multisample_state = vk::PipelineMultisampleStateCreateInfo::default()
            .rasterization_samples(self.rasterization_samples)
            .sample_shading_enable(self.sample_shading_enable)
            .min_sample_shading(self.min_sample_shading)
            .alpha_to_coverage_enable(self.alpha_to_coverage_enable)
            .alpha_to_one_enable(self.alpha_to_one_enable);
        let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(self.depth_test_enable)
            .depth_write_enable(self.depth_write_enable)
            .depth_compare_op(self.depth_compare_op)
            .depth_bounds_test_enable(self.depth_bounds_test_enable)
            .stencil_test_enable(self.stencil_test_enable)
            .front(self.front)
            .back(self.back)
            .min_depth_bounds(self.min_depth_bounds)
            .max_depth_bounds(self.max_depth_bounds);
        let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
            .attachments(&self.attachments)
            .logic_op_enable(self.logic_op_enable)
            .logic_op(self.logic_op)
            .blend_constants(self.blend_constants);
        let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
            .view_mask(self.view_mask)
            .color_attachment_formats(&self.color_attachment_formats)
            .depth_attachment_format(self.depth_attachment_format)
            .stencil_attachment_format(self.stencil_attachment_format);
        let dynamic_state =
            vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&self.dynamic_states);
        let mut info = vk::GraphicsPipelineCreateInfo::default()
            .stages(&stages)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&input_assembly_state)
            .viewport_state(&viewport_state)
            .rasterization_state(&rasterization_state)
            .multisample_state(&multisample_state)
            .depth_stencil_state(&depth_stencil_state)
            .color_blend_state(&color_blend_state)
            .dynamic_state(&dynamic_state)
            .layout(self.layout)
            .render_pass(self.render_pass)
            .subpass(self.subpass)
            .flags(if cfg!(debug_assertions) {
                vk::PipelineCreateFlags::CAPTURE_STATISTICS_KHR
            } else {
                vk::PipelineCreateFlags::empty()
            });
        if self.render_pass == Default::default() {
            info = info.push_next(&mut rendering_info);
        }
        let cache = std::fs::read(pipeline_cache_path()).unwrap_or_default();
        let pipeline_cache = unsafe {
            DEVICE
                .create_pipeline_cache(
                    &vk::PipelineCacheCreateInfo::default().initial_data(&cache),
                    None,
                )
                .unwrap_or_default()
        };
        let graphics_pipelines = unsafe {
            DEVICE
                .create_graphics_pipelines(pipeline_cache, &[info], None)
                .unwrap()
        };
        std::fs::write(pipeline_cache_path(), unsafe {
            DEVICE
                .get_pipeline_cache_data(pipeline_cache)
                .unwrap_or_default()
        })
        .unwrap_or_default();
        unsafe {
            DEVICE.destroy_pipeline_cache(pipeline_cache, None);
        }
        let gp = graphics_pipelines[0];
        #[cfg(debug_assertions)]
        unsafe {
            let exec_props = PIPELINE_EXEC_PROPS_LOADER
                .get_pipeline_executable_properties(&vk::PipelineInfoKHR::default().pipeline(gp))
                .unwrap();

            for (i, exec_prop) in exec_props.iter().enumerate() {
                log!(
                    "{}",
                    exec_prop.description_as_c_str().unwrap().to_string_lossy()
                );
                let stats = PIPELINE_EXEC_PROPS_LOADER
                    .get_pipeline_executable_statistics(
                        &vk::PipelineExecutableInfoKHR::default()
                            .pipeline(gp)
                            .executable_index(i as u32),
                    )
                    .unwrap();
                for stat in stats {
                    let val = match stat.format {
                        vk::PipelineExecutableStatisticFormatKHR::BOOL32 => {
                            stat.value.b32.to_string()
                        }
                        vk::PipelineExecutableStatisticFormatKHR::INT64 => {
                            stat.value.i64.to_string()
                        }
                        vk::PipelineExecutableStatisticFormatKHR::UINT64 => {
                            stat.value.u64.to_string()
                        }
                        vk::PipelineExecutableStatisticFormatKHR::FLOAT64 => {
                            stat.value.f64.to_string()
                        }
                        _ => "unknown".to_string(),
                    };
                    let name = stat
                        .name_as_c_str()
                        .unwrap()
                        .to_string_lossy()
                        .replace("Memory", "Mem")
                        .replace("Size", "")
                        .replace("Register", "Reg")
                        .replace(" Count", "s")
                        .replace("Color", "Col")
                        .replace("Output", "Out")
                        .replace("Input", "In")
                        .replace("Local", "Loc")
                        .trim()
                        .to_string();
                    if name == "Loc Mem" {
                        continue;
                    }
                    if name == "Binary" {
                        let val = Mem::str(&val);
                        log!("    {name:<8} {val}");
                    } else {
                        log!("    {name:<8} {val}");
                    }
                }
            }
        }
        gp
    }
}

pub fn create_compute_pipeline(shader_name: &str) -> vk::Pipeline {
    let shader = Shader::new(shader_name);
    let module = shader.create_module(); // TODO: destroy
    let cache = std::fs::read(pipeline_cache_path()).unwrap_or_default();
    let pipeline_cache = unsafe {
        DEVICE
            .create_pipeline_cache(
                &vk::PipelineCacheCreateInfo::default().initial_data(&cache),
                None,
            )
            .unwrap_or_default()
    };
    let compute_pipeline = unsafe {
        DEVICE
            .create_compute_pipelines(
                pipeline_cache,
                &[vk::ComputePipelineCreateInfo::default().stage(
                    vk::PipelineShaderStageCreateInfo::default()
                        .stage(vk::ShaderStageFlags::COMPUTE)
                        .name(std::ffi::CStr::from_bytes_with_nul_unchecked(
                            shader_name.as_bytes(),
                        ))
                        .module(module)
                        .specialization_info(&vk::SpecializationInfo::default()),
                )],
                None,
            )
            .unwrap_or_default()
    };
    std::fs::write(pipeline_cache_path(), unsafe {
        DEVICE
            .get_pipeline_cache_data(pipeline_cache)
            .unwrap_or_default()
    })
    .unwrap_or_default();
    unsafe {
        DEVICE.destroy_pipeline_cache(pipeline_cache, None);
    }
    compute_pipeline[0]
}
