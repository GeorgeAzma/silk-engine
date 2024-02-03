use std::rc::Rc;

pub mod atlas;
pub mod font;
pub mod image;
pub mod instance;
use crate::assets;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct PrimitiveInstance {
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub color: [u8; 4],
    pub stroke_color: [u8; 4],
    pub stroke_width: f32,
    pub roundness: f32,
    pub rotation: f32,
    pub sides: u32,
    pub uv: [f32; 4],
}

impl PrimitiveInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        static ATTRS: [wgpu::VertexAttribute; 9] = wgpu::vertex_attr_array![
            0 => Float32x2,
            1 => Float32x2,
            2 => Uint32,
            3 => Uint32,
            4 => Float32,
            5 => Float32,
            6 => Float32,
            7 => Uint32,
            8 => Float32x4,
        ];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRS,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct TextInstance {
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub color: [u8; 4],
    pub stroke_color: [u8; 4],
    pub stroke_width: f32,
    pub rotation: f32,
    pub uv: [f32; 4],
    pub bold: f32,
}

impl TextInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        static ATTRS: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![
            0 => Float32x2,
            1 => Float32x2,
            2 => Uint32,
            3 => Uint32,
            4 => Float32,
            5 => Float32,
            6 => Float32x4,
            7 => Float32,
        ];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRS,
        }
    }
}

pub struct Renderer {
    device: Rc<wgpu::Device>,
    _queue: Rc<wgpu::Queue>,
    primitive_instance_manager: instance::Manager<PrimitiveInstance>,
    primitive_pipeline: wgpu::RenderPipeline,
    text_instance_manager: instance::Manager<TextInstance>,
    text_pipeline: wgpu::RenderPipeline,
    text_bind_group: wgpu::BindGroup,
    atlas_manager: atlas::Manager,
    current_texture_uv: [f32; 4],
    width: u32,
    height: u32,
    pub font: font::Font,
    pub position: [f32; 2],
    pub scale: [f32; 2],
    pub color: [u8; 4],
    pub stroke_color: [u8; 4],
    pub stroke_width: f32,
    pub roundness: f32,
    pub rotation: f32,
    pub bold: f32,
}

impl Renderer {
    pub fn new(
        device: &Rc<wgpu::Device>,
        queue: &Rc<wgpu::Queue>,
        surf_conf: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let atlas_manager = atlas::Manager::new(device, queue);

        let primitive_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Primitive"),
                bind_group_layouts: &[atlas_manager.bind_group_layout()],
                push_constant_ranges: &[],
            });

        let primitive_shader = device.create_shader_module(assets::get_shader("primitive.wgsl"));
        let primitive_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Primitive"),
            layout: Some(&primitive_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &primitive_shader,
                entry_point: "vs_main",
                buffers: &[PrimitiveInstance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &primitive_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surf_conf.format,
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

        let text_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("Text"),
            });

        let font = font::Font::new(&device, &queue, "roboto_medium");

        let text_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &text_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(font.atlas_view()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(font.atlas_sampler()),
                },
            ],
            label: Some("diffuse_bind_group"),
        });

        let text_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text"),
            bind_group_layouts: &[&text_bind_group_layout],
            push_constant_ranges: &[],
        });

        let text_shader = device.create_shader_module(assets::get_shader("text.wgsl"));
        let text_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Text"),
            layout: Some(&text_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &text_shader,
                entry_point: "vs_main",
                buffers: &[TextInstance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &text_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surf_conf.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
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
            device: device.clone(),
            _queue: queue.clone(),
            primitive_instance_manager: instance::Manager::new(device, queue),
            primitive_pipeline,
            text_instance_manager: instance::Manager::new(device, queue),
            text_pipeline,
            text_bind_group,
            atlas_manager,
            current_texture_uv: [0.0, 0.0, 0.0, 0.0],
            font,
            width: surf_conf.width,
            height: surf_conf.height,
            position: [0.0, 0.0],
            scale: [1.0, 1.0],
            color: [255, 255, 255, 255],
            stroke_color: [255, 255, 255, 255],
            stroke_width: 0.0,
            roundness: 0.0,
            rotation: 0.0,
            bold: 0.0,
        }
    }

    // Always resize before rendering, if surface is same this does nothing
    pub fn resize(&mut self, width: u32, height: u32) {
        if width == self.width && height == self.height || width == 0 || height == 0 {
            return;
        }
        self.width = width;
        self.height = height;
    }

    pub fn set_image(&mut self, image: &Rc<image::Image>) {
        let (x, y, w, h) = self.atlas_manager.add(image);
        self.current_texture_uv = [x, y, w, h];
    }

    pub fn clear_image(&mut self) {
        self.current_texture_uv = [0.0, 0.0, 0.0, 0.0];
    }

    pub fn add(&mut self, instance: PrimitiveInstance) {
        self.primitive_instance_manager.add(instance);
    }

    pub fn ngon(&mut self, x: f32, y: f32, width: f32, height: f32, sides: u32) {
        self.add(PrimitiveInstance {
            position: [x + self.position[0], y + self.position[1]],
            scale: [width * self.scale[0], height * self.scale[1]],
            color: self.color,
            stroke_color: self.stroke_color,
            stroke_width: self.stroke_width,
            roundness: self.roundness,
            rotation: self.rotation,
            sides,
            uv: self.current_texture_uv,
        });
    }

    pub fn tri(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.ngon(x, y, width, height, 3);
    }

    pub fn rect(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.ngon(x, y, width, height, 4);
    }

    pub fn round_rect(&mut self, x: f32, y: f32, width: f32, height: f32, roundness: f32) {
        let old_roundness = self.roundness;
        self.roundness += roundness;
        self.rect(x, y, width, height);
        self.roundness = old_roundness;
    }

    pub fn square(&mut self, x: f32, y: f32, size: f32) {
        self.rect(x, y, size, size)
    }

    pub fn round_square(&mut self, x: f32, y: f32, size: f32, roundness: f32) {
        self.round_rect(x, y, size, size, roundness)
    }

    pub fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, size: f32) {
        let dir_x = x2 - x1;
        let dir_y = y2 - y1;
        let length = (dir_x * dir_x + dir_y * dir_y).sqrt() * 0.5;
        let angle = dir_y.atan2(dir_x);
        self.rotation -= angle;
        self.round_rect((x1 + x2) * 0.5, (y1 + y2) * 0.5, length + size, size, 1.0);
        self.rotation += angle;
    }

    pub fn lines(&mut self, points: &[f32], size: f32) {
        assert!(points.len() % 2 == 0, "must have x and y for each point!");
        assert!(points.len() >= 4, "must specify 2+ points with x,y!");
        for i in 0..points.len() / 2 - 1 {
            let p1x = points[i * 2];
            let p1y = points[i * 2 + 1];
            let p2x = points[(i + 1) * 2];
            let p2y = points[(i + 1) * 2 + 1];
            self.line(p1x, p1y, p2x, p2y, size);
        }
    }

    pub fn circle(&mut self, x: f32, y: f32, radius: f32) {
        self.ngon(x, y, radius, radius, 8192)
    }

    pub fn ellipse(&mut self, x: f32, y: f32, width: f32, height: f32) {
        self.ngon(x, y, width, height, 8192)
    }

    pub fn text(&mut self, text: &str, x: f32, y: f32, size: f32) {
        let layout = self.font.calc_layout(text);
        for (i, c) in text.chars().enumerate() {
            let (cx, cy) = layout[i];
            if cx < 0.0 {
                continue;
            }
            let (cw, ch) = self.font.char_size(c);
            self.text_instance_manager.add(TextInstance {
                position: [
                    x + self.position[0] + cx * size * self.scale[0],
                    y + self.position[1] + cy * size * self.scale[1],
                ],
                scale: [size * self.scale[0] * cw, size * self.scale[1] * ch],
                color: self.color,
                stroke_color: self.stroke_color,
                stroke_width: self.stroke_width,
                rotation: self.rotation,
                uv: self.font.char_uv(c),
                bold: self.bold,
            });
        }
    }

    pub fn atlas(&mut self) {
        self.current_texture_uv = [0.0, 0.0, 1.0, 1.0];
    }

    pub fn reset(&mut self) {
        self.color = [255, 255, 255, 255];
        self.stroke_color = [255, 255, 255, 255];
        self.stroke_width = 0.0;
        self.bold = 0.0;
        self.current_texture_uv = [0.0, 0.0, 0.0, 0.0];
        self.roundness = 0.0;
        self.rotation = 0.0;
        self.position = [0.0, 0.0];
        self.scale = [1.0, 1.0];
    }

    pub fn render(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
        self.reset();
        self.atlas_manager.flush(encoder);
        self.primitive_instance_manager.flush();
        self.text_instance_manager.flush();
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Renderer"),
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

        render_pass.set_pipeline(&self.primitive_pipeline);
        render_pass.set_bind_group(0, &self.atlas_manager.bind_group(), &[]);
        self.primitive_instance_manager.render(&mut render_pass);

        render_pass.set_pipeline(&self.text_pipeline);
        render_pass.set_bind_group(0, &self.text_bind_group, &[]);
        self.text_instance_manager.render(&mut render_pass);
        drop(render_pass);
    }
}
