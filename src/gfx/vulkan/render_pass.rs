use ash::vk;

pub struct RenderPass {
    attachment_descs: Vec<vk::AttachmentDescription>,
}

impl RenderPass {
    pub fn add(
        mut self,
        format: vk::Format,
        initial_layout: vk::ImageLayout,
        final_layout: vk::ImageLayout,
        load_op: vk::AttachmentLoadOp,
        store_op: vk::AttachmentStoreOp,
    ) {
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
    }

    //     pub fn build(&self) -> vk::RenderPass {
    //         vk::RenderPassCreateInfo::default()
    //             .attachments(&self.attachment_descs)
    //             .dependencies(vk::SubpassDependency::default())
    //             .subpasses(
    //                 vk::SubpassDescription::default()
    //                     .color_attachments(color_attachments)
    //                     .depth_stencil_attachment(depth_stencil_attachment)
    //                     .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS),
    //             );
    //     }
}
