use std::sync::Arc;

use ash::vk;

use crate::{
    prelude::ResultAny,
    vulkan::{
        device::Device,
        format_size,
        shader::{Shader, VertexInput},
    },
};

pub(crate) struct PipelineConfig<'a> {
    pub(crate) stages: Vec<vk::PipelineShaderStageCreateInfo<'a>>,
    pub(crate) vertex_input: vk::PipelineVertexInputStateCreateInfo<'a>,
    pub(crate) input_assembly: vk::PipelineInputAssemblyStateCreateInfo<'a>,
    pub(crate) tesselation: vk::PipelineTessellationStateCreateInfo<'a>,
    pub(crate) viewport: vk::PipelineViewportStateCreateInfo<'a>,
    pub(crate) rasterization: vk::PipelineRasterizationStateCreateInfo<'a>,
    pub(crate) multisample: vk::PipelineMultisampleStateCreateInfo<'a>,
    pub(crate) depth_stencil: vk::PipelineDepthStencilStateCreateInfo<'a>,
    pub(crate) color_blend: vk::PipelineColorBlendStateCreateInfo<'a>,
    pub(crate) dynamic_state: vk::PipelineDynamicStateCreateInfo<'a>,
    pub(crate) color_blend_attachments: Vec<vk::PipelineColorBlendAttachmentState>,
    pub(crate) render_pass: vk::RenderPass,
    pub(crate) subpass: u32,
    pub(crate) vertex_inputs: Vec<VertexInput>,
    pub(crate) vertex_input_bindings: Vec<vk::VertexInputBindingDescription>,
    pub(crate) vertex_input_attributes: Vec<vk::VertexInputAttributeDescription>,
}

impl<'a> Default for PipelineConfig<'a> {
    fn default() -> Self {
        Self {
            stages: vec![],
            vertex_input: vk::PipelineVertexInputStateCreateInfo::default(),
            input_assembly: vk::PipelineInputAssemblyStateCreateInfo::default()
                .topology(vk::PrimitiveTopology::TRIANGLE_STRIP),
            tesselation: vk::PipelineTessellationStateCreateInfo::default(),
            viewport: vk::PipelineViewportStateCreateInfo::default()
                .viewport_count(1)
                .scissor_count(1),
            rasterization: vk::PipelineRasterizationStateCreateInfo::default()
                .polygon_mode(vk::PolygonMode::FILL)
                .line_width(1.0)
                .cull_mode(vk::CullModeFlags::BACK)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE),
            multisample: vk::PipelineMultisampleStateCreateInfo::default()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1),
            depth_stencil: vk::PipelineDepthStencilStateCreateInfo::default()
                .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
                .min_depth_bounds(0.0)
                .max_depth_bounds(1.0),
            color_blend_attachments: vec![],
            color_blend: vk::PipelineColorBlendStateCreateInfo::default(),
            dynamic_state: vk::PipelineDynamicStateCreateInfo::default()
                .dynamic_states(&[vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR]),
            render_pass: vk::RenderPass::null(),
            subpass: 0,
            vertex_inputs: vec![],
            vertex_input_bindings: vec![],
            vertex_input_attributes: vec![],
        }
    }
}

impl<'a> PipelineConfig<'a> {
    pub(crate) fn with_shader(
        &mut self,
        shader: &Shader,
        specialization_info: &'a vk::SpecializationInfo<'a>,
    ) -> ResultAny<&mut Self> {
        match shader.reflect_vertex_input() {
            Ok(vertex_inputs) => self.vertex_inputs = vertex_inputs,
            Err(err) => return Err(format!("failed to reflect shader vertex input: {err}").into()),
        }
        self.stages = shader.pipeline_shader_stage_infos(specialization_info);
        Ok(self)
    }

    fn with_vertex_input_attribute_(
        &mut self,
        location: u32,
        binding: u32,
        input_rate: vk::VertexInputRate,
    ) -> ResultAny<&mut Self> {
        let vertex_input = self
            .vertex_inputs
            .iter()
            .find(|input| input.location == location)
            .ok_or(format!(
                "vertex input attribute with location {location} does not exist"
            ))?;

        let binding_desc: &mut vk::VertexInputBindingDescription = if let Some(b) = self
            .vertex_input_bindings
            .iter_mut()
            .find(|b| b.binding == binding)
        {
            b
        } else {
            self.vertex_input_bindings
                .push(vk::VertexInputBindingDescription {
                    binding,
                    stride: 0,
                    input_rate,
                });
            self.vertex_input_bindings.last_mut().unwrap()
        };

        let offset = binding_desc.stride;
        let format_size = format_size(vertex_input.format);
        binding_desc.stride += format_size;

        self.vertex_input_attributes.push(
            vk::VertexInputAttributeDescription::default()
                .location(vertex_input.location)
                .binding(binding)
                .format(vertex_input.format)
                .offset(offset),
        );

        Ok(self)
    }

    pub(crate) fn with_vertex_input_attribute(
        &mut self,
        location: u32,
        binding: u32,
    ) -> ResultAny<&mut Self> {
        self.with_vertex_input_attribute_(location, binding, vk::VertexInputRate::VERTEX)
    }

    pub(crate) fn with_vertex_input_attribute_instanced(
        &mut self,
        location: u32,
        binding: u32,
    ) -> ResultAny<&mut Self> {
        self.with_vertex_input_attribute_(location, binding, vk::VertexInputRate::INSTANCE)
    }

    pub(crate) fn with_vertex_input_attribute_name(
        &mut self,
        name: &str,
        binding: u32,
    ) -> ResultAny<&mut Self> {
        let location = self
            .vertex_inputs
            .iter()
            .find(|input| input.name == name)
            .ok_or(format!(
                "vertex input attribute with name \"{name}\" does not exist"
            ))?
            .location;
        self.with_vertex_input_attribute_(location, binding, vk::VertexInputRate::VERTEX)
    }

    pub(crate) fn with_vertex_input_attribute_name_instanced(
        &mut self,
        name: &str,
        binding: u32,
    ) -> ResultAny<&mut Self> {
        let location = self
            .vertex_inputs
            .iter()
            .find(|input| input.name == name)
            .ok_or(format!(
                "vertex input attribute with name \"{name}\" does not exist"
            ))?
            .location;
        self.with_vertex_input_attribute_(location, binding, vk::VertexInputRate::INSTANCE)
    }

    /// auto assigns vertex input attributes to binding 0, and instanced vertex input attributes to binding 1
    /// whether vertex input attribute is instanced or not depends on its prefix "i_" means instanced "v_" means non-instanced (default: non-instanced)
    pub(crate) fn with_auto_vertex_inputs(&mut self) -> ResultAny<&mut Self> {
        let vertex_inputs = self.vertex_inputs.clone();
        for input in vertex_inputs {
            let is_instanced = input.name.starts_with("i_");
            if is_instanced {
                self.with_vertex_input_attribute_(
                    input.location,
                    1,
                    vk::VertexInputRate::INSTANCE,
                )?;
            } else {
                self.with_vertex_input_attribute_(input.location, 0, vk::VertexInputRate::VERTEX)?;
            }
        }
        Ok(self)
    }

    pub(crate) fn with_render_pass(&mut self, render_pass: vk::RenderPass) -> &mut Self {
        self.render_pass = render_pass;
        self
    }

    pub(crate) fn with_subpass(&mut self, subpass: u32) -> &mut Self {
        self.subpass = subpass;
        self
    }

    pub(crate) fn with_default_depth(&mut self) -> &mut Self {
        self.depth_stencil = vk::PipelineDepthStencilStateCreateInfo::default()
            .depth_test_enable(true)
            .depth_write_enable(true)
            .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
            .min_depth_bounds(0.0)
            .max_depth_bounds(1.0);
        self
    }

    pub(crate) fn with_color_blend_attachments(
        &mut self,
        color_blend_attachments: &[vk::PipelineColorBlendAttachmentState],
    ) -> &mut Self {
        self.color_blend_attachments = color_blend_attachments.to_vec();
        self
    }

    pub(crate) fn add_color_blend_attachment(
        &mut self,
        color_blend_attachment: &vk::PipelineColorBlendAttachmentState,
    ) -> &mut Self {
        self.color_blend_attachments.push(*color_blend_attachment);
        self
    }

    pub(crate) fn add_color_blended_attachment(&mut self) -> &mut Self {
        self.add_color_blend_attachment(
            &vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(true)
                .color_write_mask(vk::ColorComponentFlags::RGBA)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD),
        )
    }

    pub(crate) fn add_color_blend_disabled_attachment(&mut self) -> &mut Self {
        self.add_color_blend_attachment(
            &vk::PipelineColorBlendAttachmentState::default()
                .blend_enable(true)
                .color_write_mask(vk::ColorComponentFlags::RGBA)
                .src_color_blend_factor(vk::BlendFactor::SRC_ALPHA)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .color_blend_op(vk::BlendOp::ADD)
                .src_alpha_blend_factor(vk::BlendFactor::ONE)
                .dst_alpha_blend_factor(vk::BlendFactor::ZERO)
                .alpha_blend_op(vk::BlendOp::ADD),
        )
    }

    pub(crate) fn build(
        &'a mut self,
        layout: vk::PipelineLayout,
    ) -> vk::GraphicsPipelineCreateInfo<'a> {
        self.vertex_input = vk::PipelineVertexInputStateCreateInfo::default()
            .vertex_binding_descriptions(&self.vertex_input_bindings)
            .vertex_attribute_descriptions(&self.vertex_input_attributes);
        self.color_blend.attachment_count = self.color_blend_attachments.len() as u32;
        self.color_blend.p_attachments = self.color_blend_attachments.as_ptr();
        vk::GraphicsPipelineCreateInfo::default()
            .stages(&self.stages)
            .vertex_input_state(&self.vertex_input)
            .input_assembly_state(&self.input_assembly)
            .tessellation_state(&self.tesselation)
            .viewport_state(&self.viewport)
            .rasterization_state(&self.rasterization)
            .multisample_state(&self.multisample)
            .depth_stencil_state(&self.depth_stencil)
            .color_blend_state(&self.color_blend)
            .dynamic_state(&self.dynamic_state)
            .layout(layout)
            .render_pass(self.render_pass)
            .subpass(self.subpass)
    }
}

pub(crate) struct Pipeline {
    handle: vk::Pipeline,
    device: Arc<Device>,
}

impl Pipeline {
    pub(crate) fn create_graphics_pipelines(
        device: &Arc<Device>,
        create_infos: &[vk::GraphicsPipelineCreateInfo],
    ) -> ResultAny<Vec<Self>> {
        let pipelines = unsafe {
            device.device.create_graphics_pipelines(
                device.pipeline_cache().handle(),
                create_infos,
                device.allocation_callbacks().as_ref(),
            )
        }
        .map_err(|(_created, result)| result)?;
        Ok(pipelines
            .into_iter()
            .map(|handle| Self::new_from_handle(handle, device))
            .collect())
    }

    pub(crate) fn new(
        device: &Arc<Device>,
        create_info: &vk::GraphicsPipelineCreateInfo,
    ) -> ResultAny<Self> {
        let pipelines = Self::create_graphics_pipelines(device, std::slice::from_ref(create_info))?;
        Ok(pipelines.into_iter().next().unwrap())
    }

    fn new_from_handle(handle: vk::Pipeline, device: &Arc<Device>) -> Self {
        Self {
            handle,
            device: Arc::clone(device),
        }
    }

    pub(crate) fn handle(&self) -> vk::Pipeline {
        self.handle
    }
}

impl Drop for Pipeline {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device
                .destroy_pipeline(self.handle, self.device.allocation_callbacks().as_ref());
        }
    }
}

pub(crate) struct PipelineLayout {
    handle: vk::PipelineLayout,
    device: Arc<Device>,
}

impl PipelineLayout {
    pub(crate) fn new(
        device: &Arc<Device>,
        descriptor_set_layouts: &[vk::DescriptorSetLayout],
        push_constant_ranges: &[vk::PushConstantRange],
    ) -> ResultAny<Self> {
        let pipeline_layout_info = vk::PipelineLayoutCreateInfo::default()
            .set_layouts(descriptor_set_layouts)
            .push_constant_ranges(push_constant_ranges);
        let handle = unsafe {
            device.device.create_pipeline_layout(
                &pipeline_layout_info,
                device.allocation_callbacks().as_ref(),
            )
        }?;
        Ok(Self::new_from_handle(handle, device))
    }

    fn new_from_handle(handle: vk::PipelineLayout, device: &Arc<Device>) -> Self {
        Self {
            handle,
            device: Arc::clone(device),
        }
    }

    pub(crate) fn handle(&self) -> vk::PipelineLayout {
        self.handle
    }
}

impl Drop for PipelineLayout {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device
                .destroy_pipeline_layout(self.handle, self.device.allocation_callbacks().as_ref());
        }
    }
}
