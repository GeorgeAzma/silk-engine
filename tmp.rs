enum TaskType {
    Graphics,
    Compute,
    Transfer,
}

enum Resource {
    Buffer,
    Image,
}

struct Task {
    task_type: TaskType,
    inputs: Vec<Resource>,
    outputs: Vec<Resource>,
    execute: fn(VkCommandBuffer),
}

struct RenderGraph {

}

impl RenderGraph {
    fn render
}

/*
depth_img = CreateImage::default_2D()(width, height, D32, DEPTH_STENCIL_ATTACHMENT_USAGE)
depth_view = CreateView::default()+(depth_image, D32, ASPECT_DEPTH)

rg.add_resource("depth buffer", format = DEPTH, usage = DEPTH_ATTACHMENT)

rg.add_pass("depth pre-pass", in = [], out = ["depth buffer"], exec = |vk::CommandBuffer cmd| {
    cmd.begin_render_pass(render_pass = "depth only", framebuffer = ["depth buffer"]);
    cmd.bind_pipeline(depth_only_pipeline);
    cmd.draw(opaque);
    cmd.end_render_pass();
})

rg.add_pass("main", in = ["depth buffer"], out = ["color buffer"], exec = |vk::CommandBuffer cmd| {
    cmd.begin_render_pass(render_pass = "color with depth test", framebuffer = ["color buffer", "depth buffer"])
    cmd.bind_pipeline(main_render_pipeline)
    cmd.draw(opaque)
    cmd.end_render_pass()
})
*/