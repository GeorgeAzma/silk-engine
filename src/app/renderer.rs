pub mod instance;

pub struct Renderer {
    instance_manager: instance::Manager,
    render_pipeline: wgpu::RenderPipeline,
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub color: [f32; 4],
    pub stroke_color: [f32; 4],
    pub stroke_width: f32,
    pub roundness: f32,
    pub rotation: f32,
}

impl Renderer {
    pub fn new(device: &wgpu::Device, render_format: wgpu::TextureFormat) -> Self {
        let shader =
            device.create_shader_module(wgpu::include_wgsl!("../shaders/primitive_2d.wgsl"));

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[instance::Instance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: render_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill, // Features::NON_FILL_POLYGON_MODE
                unclipped_depth: false,                // Features::DEPTH_CLIP_CONTROL
                conservative: false,                   // Features::CONSERVATIVE_RASTERIZATION
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            instance_manager: instance::Manager::new(device),
            render_pipeline,
            position: [0.0, 0.0],
            scale: [1.0, 1.0],
            color: [1.0, 1.0, 1.0, 1.0],
            stroke_color: [1.0, 1.0, 1.0, 1.0],
            stroke_width: 0.0,
            roundness: 0.0,
            rotation: 0.0,
        }
    }

    pub fn add(&mut self, instance: instance::Instance) {
        self.instance_manager.add(instance);
    }

    pub fn ngon(&mut self, x: f32, y: f32, width: f32, height: f32, sides: i32) {
        self.add(instance::Instance::new(
            [x + self.position[0], y + self.position[1]],
            [width * self.scale[0], height * self.scale[1]],
            self.color,
            self.stroke_color,
            self.stroke_width,
            self.roundness,
            self.rotation,
            sides,
        ));
    }

    pub fn tri(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.ngon(x, y, width, height, 3);
    }

    pub fn rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.ngon(x, y, width, height, 4);
    }

    pub fn round_rect(&mut self, x: f32, y: f32, width: f32, height: f32, roundness: f32) {
        let old_roundness = self.roundness;
        self.roundness = roundness;
        self.rect(x, y, width, height);
        self.roundness = old_roundness;
    }

    pub fn square(&mut self, x: f32, y: f32, size: f32) {
        self.rect(x, y, size, size)
    }

    pub fn round_square(&mut self, x: f32, y: f32, size: f32, roundness: f32) {
        self.round_rect(x, y, size, size, roundness)
    }

    pub fn circle(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.ngon(x, y, width, height, 8192)
    }

    pub fn flush(&mut self, queue: &wgpu::Queue, device: &wgpu::Device) {
        self.instance_manager.flush(queue, device)
    }

    pub fn render(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.0,
                        g: 0.0,
                        b: 0.0,
                        a: 0.0,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(&self.render_pipeline);
        self.instance_manager.render(&mut render_pass);
    }
}
