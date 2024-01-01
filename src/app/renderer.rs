use image::EncodableLayout;
use wgpu::{util::DeviceExt, ShaderStages, SurfaceConfiguration};

pub mod font;
pub mod instance;
use crate::assets;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct PrimitiveInstance {
    pub position: [f32; 3],
    pub scale: [f32; 2],
    pub color: [u8; 4],
    pub stroke_color: [u8; 4],
    pub stroke_width: f32,
    pub roundness: f32,
    pub rotation: f32,
    pub sides: i32,
}

impl PrimitiveInstance {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        static ATTRS: [wgpu::VertexAttribute; 8] = wgpu::vertex_attr_array![
            0 => Float32x3,
            1 => Float32x2,
            2 => Uint32,
            3 => Uint32,
            4 => Float32,
            5 => Float32,
            6 => Float32,
            7 => Sint32,
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
    pub position: [f32; 3],
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
            0 => Float32x3,
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
    // Note: it's possible to have a single instance manager, but this is cleaner
    primitive_instance_manager: instance::Manager<PrimitiveInstance>,
    primitive_pipeline: wgpu::RenderPipeline,
    text_instance_manager: instance::Manager<TextInstance>,
    text_pipeline: wgpu::RenderPipeline,
    text_bind_group: wgpu::BindGroup,
    depth_texture: wgpu::Texture,
    depth_view: wgpu::TextureView,
    width: u32,
    height: u32,
    pub font: font::Font,
    pub depth: f32,
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
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        surf_conf: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let primitive_shader = device.create_shader_module(assets::get_shader("primitive.wgsl"));
        let primitive_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Primitive"),
            layout: None,
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
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
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::LessEqual,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: surf_conf.width,
                height: surf_conf.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            primitive_instance_manager: instance::Manager::new(device),
            primitive_pipeline,
            text_instance_manager: instance::Manager::new(device),
            text_pipeline,
            text_bind_group,
            depth_texture,
            depth_view,
            font,
            width: surf_conf.width,
            height: surf_conf.height,
            depth: 1.0,
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
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }
        self.width = width;
        self.height = height;
        self.depth_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.depth_view = self
            .depth_texture
            .create_view(&wgpu::TextureViewDescriptor::default());
    }

    pub fn add(&mut self, instance: PrimitiveInstance) {
        self.primitive_instance_manager.add(instance);
        self.depth -= f32::EPSILON;
    }

    pub fn ngon(&mut self, x: f32, y: f32, width: f32, height: f32, sides: i32) {
        self.add(PrimitiveInstance {
            position: [x + self.position[0], y + self.position[1], self.depth],
            scale: [width * self.scale[0], height * self.scale[1]],
            color: self.color,
            stroke_color: self.stroke_color,
            stroke_width: self.stroke_width,
            roundness: self.roundness,
            rotation: self.rotation,
            sides,
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
                    x + cx * size * self.scale[0],
                    y + cy * size * self.scale[1],
                    self.depth,
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
        self.depth -= f32::EPSILON;
    }

    pub fn flush(&mut self, queue: &wgpu::Queue, device: &wgpu::Device) {
        self.depth = 1.0;
        self.primitive_instance_manager.flush(queue, device);
        self.text_instance_manager.flush(queue, device);
    }

    pub fn render(&mut self, encoder: &mut wgpu::CommandEncoder, view: &wgpu::TextureView) {
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
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_view,
                depth_ops: Some(wgpu::Operations::<f32> {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.primitive_pipeline);
        self.primitive_instance_manager.render(&mut render_pass);

        render_pass.set_pipeline(&self.text_pipeline);
        render_pass.set_bind_group(0, &self.text_bind_group, &[]);
        self.text_instance_manager.render(&mut render_pass);
        drop(render_pass);
    }
}
