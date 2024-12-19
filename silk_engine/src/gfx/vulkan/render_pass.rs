use ash::vk;

use super::gpu;

#[derive(Default, Clone)]
pub struct RenderPass {
    attachment_descs: Vec<vk::AttachmentDescription>,
    attachment_refs: Vec<vk::AttachmentReference>,
    pub framebuffer_size: vk::Extent2D,
    pub framebuffers: Vec<vk::Framebuffer>,
    pub render_pass: vk::RenderPass,
}

impl RenderPass {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(
        mut self,
        format: vk::Format,
        initial_layout: vk::ImageLayout,
        final_layout: vk::ImageLayout,
        load_op: vk::AttachmentLoadOp,
        store_op: vk::AttachmentStoreOp,
    ) -> Self {
        let attachment_ref = vk::AttachmentReference::default()
            .attachment(self.attachment_descs.len() as u32)
            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);
        self.attachment_refs.push(attachment_ref);
        self.attachment_descs.push(
            vk::AttachmentDescription::default()
                .initial_layout(initial_layout)
                .final_layout(final_layout)
                .format(format)
                .load_op(load_op)
                .store_op(store_op)
                .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                .samples(vk::SampleCountFlags::TYPE_1),
        );
        self
    }

    pub fn recreate_framebuffer(
        &mut self,
        width: u32,
        height: u32,
        img_views: &[vk::ImageView],
        count: usize,
    ) {
        unsafe {
            for &fb in self.framebuffers.iter() {
                gpu().destroy_framebuffer(fb, None);
            }
            self.framebuffers.resize(count, vk::Framebuffer::null());
            for i in 0..count {
                self.framebuffers[i] = gpu()
                    .create_framebuffer(
                        &vk::FramebufferCreateInfo::default()
                            .attachment_count(self.attachment_descs.len() as u32)
                            .layers(1)
                            .width(width)
                            .height(height)
                            .attachments(img_views)
                            .render_pass(self.render_pass),
                        None,
                    )
                    .unwrap();
            }
            self.framebuffer_size = vk::Extent2D { width, height };
        }
    }

    pub fn build(&mut self) -> vk::RenderPass {
        unsafe {
            self.render_pass = gpu()
                .create_render_pass(
                    &vk::RenderPassCreateInfo::default()
                        .attachments(&self.attachment_descs)
                        .dependencies(&[
                            vk::SubpassDependency::default()
                                .src_subpass(vk::SUBPASS_EXTERNAL)
                                .dst_subpass(0)
                                .src_stage_mask(vk::PipelineStageFlags::BOTTOM_OF_PIPE)
                                .src_access_mask(vk::AccessFlags::MEMORY_READ)
                                .dst_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                                .dst_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE),
                            vk::SubpassDependency::default()
                                .src_subpass(0)
                                .dst_subpass(vk::SUBPASS_EXTERNAL)
                                .src_stage_mask(vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT)
                                .src_access_mask(vk::AccessFlags::COLOR_ATTACHMENT_WRITE)
                                .dst_stage_mask(vk::PipelineStageFlags::BOTTOM_OF_PIPE)
                                .dst_access_mask(vk::AccessFlags::MEMORY_READ),
                        ])
                        .subpasses(&[vk::SubpassDescription::default()
                            .color_attachments(&self.attachment_refs)
                            .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)]),
                    None,
                )
                .unwrap()
        };
        self.render_pass
    }

    pub fn destroy(self) {
        unsafe {
            for fb in self.framebuffers {
                gpu().destroy_framebuffer(fb, None);
            }
            gpu().destroy_render_pass(self.render_pass, None);
        }
    }
}
