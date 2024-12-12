use crate::shader::Shader;
use crate::*;

const fn pipeline_cache_path() -> &'static str {
    "res/cache/pipeline_cache"
}

pub struct GraphicsPipelineInfo<'a> {
    pub stages: Vec<vk::PipelineShaderStageCreateInfo<'a>>,
    pub dynamic_states: Vec<vk::DynamicState>,
    pub vertex_input_binding_descriptions: Vec<vk::VertexInputBindingDescription>,
    pub vertex_input_attribute_descriptions: Vec<vk::VertexInputAttributeDescription>,
    pub vertex_input_state: vk::PipelineVertexInputStateCreateInfo<'a>,
    pub input_assembly_state: vk::PipelineInputAssemblyStateCreateInfo<'a>,
    pub viewport_state: vk::PipelineViewportStateCreateInfo<'a>,
    pub rasterization_state: vk::PipelineRasterizationStateCreateInfo<'a>,
    pub multisample_state: vk::PipelineMultisampleStateCreateInfo<'a>,
    pub depth_stencil_state: vk::PipelineDepthStencilStateCreateInfo<'a>,
    pub color_blend_state: vk::PipelineColorBlendStateCreateInfo<'a>,
    pub dynamic_state: vk::PipelineDynamicStateCreateInfo<'a>,
    pub rendering_info: vk::PipelineRenderingCreateInfo<'a>,
    pub create_info: vk::GraphicsPipelineCreateInfo<'a>,
}

impl Default for GraphicsPipelineInfo<'_> {
    fn default() -> Self {
        Self {
            stages: Vec::new(),
            dynamic_states: Vec::new(),
            vertex_input_binding_descriptions: Vec::new(),
            vertex_input_attribute_descriptions: Vec::new(),
            vertex_input_state: vk::PipelineVertexInputStateCreateInfo::default(),
            input_assembly_state: vk::PipelineInputAssemblyStateCreateInfo::default()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST),
            viewport_state: vk::PipelineViewportStateCreateInfo::default(),
            rasterization_state: vk::PipelineRasterizationStateCreateInfo::default()
                .polygon_mode(vk::PolygonMode::FILL)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .line_width(1.0),
            multisample_state: vk::PipelineMultisampleStateCreateInfo::default()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1),
            depth_stencil_state: vk::PipelineDepthStencilStateCreateInfo::default()
                .max_depth_bounds(1.0),
            color_blend_state: vk::PipelineColorBlendStateCreateInfo::default(),
            dynamic_state: vk::PipelineDynamicStateCreateInfo::default(),
            rendering_info: vk::PipelineRenderingCreateInfo::default(),
            create_info: vk::GraphicsPipelineCreateInfo::default(),
        }
    }
}

impl<'a> GraphicsPipelineInfo<'a> {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn depth(&mut self) -> &mut Self {
        self.depth_stencil_state.depth_test_enable = vk::TRUE;
        self.depth_stencil_state.depth_write_enable = vk::TRUE;
        self.depth_stencil_state.depth_compare_op = vk::CompareOp::LESS;
        self
    }

    pub fn dyn_size(&mut self) -> &mut Self {
        self.dynamic_states.push(vk::DynamicState::VIEWPORT);
        self.dynamic_states.push(vk::DynamicState::SCISSOR);
        self.dynamic_state.p_dynamic_states = self.dynamic_states.as_ptr();
        self.dynamic_state.dynamic_state_count = self.dynamic_states.len() as u32;
        self.viewport_state.viewport_count = 1;
        self.viewport_state.scissor_count = 1;
        self
    }

    pub fn layout(&mut self, layout: vk::PipelineLayout) -> &mut Self {
        self.create_info.layout = layout;
        self
    }

    pub fn stages(&mut self, stages: &[vk::PipelineShaderStageCreateInfo<'a>]) -> &mut Self {
        self.stages.extend(stages);
        self
    }

    pub fn vert_layout(
        &mut self,
        shader: &Shader,
        vert_input_bindings: &[(bool, Vec<u32>)],
    ) -> &mut Self {
        (
            self.vertex_input_binding_descriptions,
            self.vertex_input_attribute_descriptions,
        ) = shader.get_vert_layout(vert_input_bindings);
        self.vertex_input_state.p_vertex_binding_descriptions =
            self.vertex_input_binding_descriptions.as_ptr();
        self.vertex_input_state.vertex_binding_description_count =
            self.vertex_input_binding_descriptions.len() as u32;

        self.vertex_input_state.p_vertex_attribute_descriptions =
            self.vertex_input_attribute_descriptions.as_ptr();
        self.vertex_input_state.vertex_attribute_description_count =
            self.vertex_input_attribute_descriptions.len() as u32;
        self
    }

    pub fn build(&mut self) -> vk::Pipeline {
        self.create_info.p_stages = self.stages.as_ptr();
        self.create_info.stage_count = self.stages.len() as u32;
        self.create_info.p_vertex_input_state = &self.vertex_input_state;
        self.create_info.p_input_assembly_state = &self.input_assembly_state;
        self.create_info.p_viewport_state = &self.viewport_state;
        self.create_info.p_rasterization_state = &self.rasterization_state;
        self.create_info.p_multisample_state = &self.multisample_state;
        self.create_info.p_depth_stencil_state = &self.depth_stencil_state;
        self.create_info.p_color_blend_state = &self.color_blend_state;
        self.create_info.p_dynamic_state = &self.dynamic_state;
        self.create_info.p_next = std::ptr::from_ref(&self.rendering_info) as *const _;

        let cache = std::fs::read(pipeline_cache_path()).unwrap_or_default();
        let pipeline_cache = unsafe {
            DEVICE
                .create_pipeline_cache(
                    &vk::PipelineCacheCreateInfo::default().initial_data(&cache),
                    None,
                )
                .unwrap_or_default()
        }; // TODO: destroy
        let graphics_pipelines = unsafe {
            DEVICE
                .create_graphics_pipelines(pipeline_cache, &[self.create_info], None)
                .unwrap()
        };
        std::fs::write(pipeline_cache_path(), unsafe {
            DEVICE
                .get_pipeline_cache_data(pipeline_cache)
                .unwrap_or_default()
        })
        .unwrap_or_default();
        graphics_pipelines[0]
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
    }; // TODO: destroy
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
    compute_pipeline[0]
}
