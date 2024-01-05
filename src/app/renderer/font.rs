use crate::assets;
use image::EncodableLayout;
use owned_ttf_parser::AsFaceRef;
use rand::Error;
use wgpu::util::*;

#[derive(Clone, Debug)]
struct Shape {
    bezier_points: Vec<[[f32; 2]; 3]>,
}

#[derive(Debug)]
struct OutlineGenerator {
    shapes: Vec<Shape>,
    curr_shape: Shape,
    curr_pos: [f32; 2],
}

impl OutlineGenerator {
    fn new() -> Self {
        let curr_shape = Shape {
            bezier_points: Vec::new(),
        };

        Self {
            shapes: vec![curr_shape.clone(); 128],
            curr_shape,
            curr_pos: [0.0, 0.0],
        }
    }

    fn gen(&mut self, c: char, font: &owned_ttf_parser::Face) {
        let gid = font.glyph_index(c).unwrap();
        let n = 1.0 / font.units_per_em() as f32;
        self.shapes[c as usize] = Shape {
            bezier_points: Vec::new(),
        };
        self.curr_shape = self.shapes[c as usize].clone();
        font.outline_glyph(gid, self);
        self.shapes[c as usize] = self.curr_shape.clone();
        for e in self.shapes[c as usize].bezier_points.iter_mut() {
            e[0][0] *= n;
            e[0][1] *= n;
            e[1][0] *= n;
            e[1][1] *= n;
            e[2][0] *= n;
            e[2][1] *= n;
        }
    }
}

impl owned_ttf_parser::OutlineBuilder for OutlineGenerator {
    fn move_to(&mut self, x: f32, y: f32) {
        self.curr_pos = [x, y];
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let mid_x = self.curr_pos[0] * 0.5 + x * 0.5;
        let mid_y = self.curr_pos[1] * 0.5 + y * 0.5;
        self.curr_shape
            .bezier_points
            .push([self.curr_pos, [mid_x, mid_y], [x, y]]);
        self.curr_pos = [x, y];
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.curr_shape
            .bezier_points
            .push([self.curr_pos, [x1, y1], [x, y]]);
        self.curr_pos = [x, y];
    }

    fn curve_to(&mut self, x2: f32, y2: f32, _x1: f32, _y1: f32, x: f32, y: f32) {
        self.curr_shape
            .bezier_points
            .push([self.curr_pos, [x2, y2], [x, y]]);
        self.curr_pos = [x, y];
    }

    fn close(&mut self) {
        self.curr_pos = [0.0, 0.0];
    }
}

pub struct Font {
    font: owned_ttf_parser::OwnedFace,
    max_glyph_size: i32,
    char_atlas_uvs: [[f32; 2]; 128],
    atlas_texture: wgpu::Texture,
    atlas_view: wgpu::TextureView,
    atlas_sampler: wgpu::Sampler,
}

impl Font {
    const MAX_GRAPHIC_CHARS: u32 = 96;

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, font_name: &str) -> Self {
        let max_glyph_size: i32 = 64;

        // Load TTF font
        let font_path = format!("assets/fonts/{font_name}.ttf");
        let font_data = std::fs::read(&font_path)
            .expect(format!("Failed to read font file: {font_path}").as_str());
        let font =
            owned_ttf_parser::OwnedFace::from_vec(font_data, 0).expect("Failed to parse font data");

        // Calculate metrics and atlas
        let mut char_atlas_uvs: [[f32; 2]; 128] = [[0.0, 0.0]; 128];
        let (atlas_width, atlas_height) =
            Font::gen_uvs(font.as_face_ref(), max_glyph_size, &mut char_atlas_uvs);

        // Create atlas GPU texture
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: wgpu::Extent3d {
                width: atlas_width as u32,
                height: atlas_height as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            label: Some("Font Atlas"),
            view_formats: &[],
        });
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let atlas_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        // Load atlas if it exists, otherwise generate a new one
        let atlas_path_str = Font::atlas_path(font_name);
        let atlas_path = std::path::Path::new(&atlas_path_str);
        if atlas_path.exists() {
            let atlas_buffer = Font::load_atlas(font_name);

            queue.write_texture(
                atlas_texture.as_image_copy(),
                &atlas_buffer,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(atlas_width as u32),
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    width: atlas_width as u32,
                    height: atlas_height as u32,
                    depth_or_array_layers: 1,
                },
            );
        } else {
            let atlas_buffer = Font::gen_atlas(
                device,
                queue,
                font.as_face_ref(),
                max_glyph_size,
                &char_atlas_uvs,
                &atlas_texture,
            );
            Font::save_atlas(font_name, &atlas_buffer, atlas_width, atlas_height);
        };

        Self {
            font,
            max_glyph_size,
            char_atlas_uvs,
            atlas_texture,
            atlas_view,
            atlas_sampler,
        }
    }

    /// Returns (width, height)
    fn gen_uvs(
        font: &owned_ttf_parser::Face,
        max_glyph_size: i32,
        char_atlas_uvs: &mut [[f32; 2]; 128],
    ) -> (i32, i32) {
        // Prediction of atlas width to match its height as closely as possible
        let padding = (max_glyph_size as f32 * 0.4) as i32;
        let mut atlas_width: i32 = ((max_glyph_size + padding) as f32
            * (Font::MAX_GRAPHIC_CHARS as f32).sqrt()
            * 0.64) as i32
            + padding;

        // For meeting row alignement requirements
        atlas_width = if atlas_width < 128 {
            256
        } else {
            ((atlas_width as f32 / 256.0).round() * 256.0) as i32
        };

        let mut x: i32 = padding;
        let mut y: i32 = padding * 2;
        let mut max_height: i32 = 0;

        for i in 0..=127u8 {
            let c = i as char;
            if !char::is_ascii_graphic(&c) {
                continue;
            }

            let gid = font.glyph_index(c).unwrap();
            let n = 1.0 / font.units_per_em() as f32;
            let bb = font.glyph_bounding_box(gid).unwrap();
            let px = padding;
            let py = padding;
            let width = (bb.x_max as f32 * n * max_glyph_size as f32) as i32 + px;
            let height = (bb.y_max as f32 * n * max_glyph_size as f32 + py as f32 * 1.75) as i32;

            let idx = i as usize;
            max_height = max_height.max(height);
            if x + width >= atlas_width {
                x = px;
                y += max_height;
                max_height = 0;
            }

            char_atlas_uvs[idx][0] = x as f32;
            char_atlas_uvs[idx][1] = y as f32;
            x += width;
        }

        let atlas_height: i32 = y + max_height;

        for i in 0..=127u8 {
            let c = i as char;
            if !char::is_ascii_graphic(&c) {
                continue;
            }
            let idx = i as usize;
            char_atlas_uvs[idx][0] /= atlas_width as f32;
            char_atlas_uvs[idx][1] /= atlas_height as f32;
        }

        (atlas_width, atlas_height)
    }

    fn gen_atlas(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        font: &owned_ttf_parser::Face,
        max_glyph_size: i32,
        char_atlas_uvs: &[[f32; 2]; 128],
        atlas_texture: &wgpu::Texture,
    ) -> Vec<u8> {
        let start = std::time::Instant::now();
        let sdf_gen_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let sdf_gen_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: None,
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: None,
                    bind_group_layouts: &[&sdf_gen_bind_group_layout],
                    push_constant_ranges: &[],
                }),
            ),
            module: &device.create_shader_module(assets::get_shader("sdf_gen.wgsl")),
            entry_point: "main",
        });

        let mut outline_gen = OutlineGenerator::new();
        for i in 0..=127u8 {
            let c = i as char;
            if !char::is_ascii_graphic(&c) {
                continue;
            }
            outline_gen.gen(c, font);
        }

        let sdf_buffer_atlas = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SDF Atlas"),
            size: (atlas_texture.width() * atlas_texture.height()) as u64,
            usage: wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
        struct Glyph {
            offset: u32,
            size: u32,
            res: [u32; 2],
            uv: [f32; 4],
        }
        let mut glyphs: Vec<Glyph> = Vec::new();
        let mut all_points: Vec<[[f32; 2]; 3]> = Vec::new();
        let mut offset: u32 = 0;
        for i in 0..=127u8 {
            let c = i as char;
            if !char::is_ascii_graphic(&c) {
                continue;
            }
            let idx = i as usize;

            let points = &outline_gen.shapes[idx].bezier_points;
            all_points.extend(points);
            glyphs.push(Glyph {
                offset,
                size: points.len() as u32,
                res: [atlas_texture.width(), atlas_texture.height()],
                uv: [
                    char_atlas_uvs[idx][0],
                    char_atlas_uvs[idx][1],
                    max_glyph_size as f32 / atlas_texture.width() as f32,
                    max_glyph_size as f32 / atlas_texture.height() as f32,
                ],
            });
            offset += points.len() as u32;
        }

        let glyph_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Glyph"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::cast_slice(&glyphs),
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        let curve_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Curve"),
            usage: wgpu::BufferUsages::STORAGE,
            contents: bytemuck::cast_slice(&all_points),
        });

        let sdf_gen_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &sdf_gen_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer(
                        sdf_buffer_atlas.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Buffer(
                        curve_buffer.as_entire_buffer_binding(),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Buffer(
                        glyph_buffer.as_entire_buffer_binding(),
                    ),
                },
            ],
        });

        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });
        compute_pass.set_pipeline(&sdf_gen_pipeline);
        compute_pass.set_bind_group(0, &sdf_gen_bind_group, &[]);
        compute_pass.dispatch_workgroups(
            atlas_texture.width() * atlas_texture.height() / 256, // This is correct, because atlas_texture.width() has 256 byte alignment
            Self::MAX_GRAPHIC_CHARS,
            1,
        );
        drop(compute_pass);

        encoder.copy_buffer_to_texture(
            wgpu::ImageCopyBuffer {
                buffer: &sdf_buffer_atlas,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(atlas_texture.width()),
                    rows_per_image: None,
                },
            },
            atlas_texture.as_image_copy(),
            wgpu::Extent3d {
                width: atlas_texture.width(),
                height: atlas_texture.height(),
                depth_or_array_layers: 1,
            },
        );
        queue.submit(std::iter::once(encoder.finish()));
        sdf_buffer_atlas
            .slice(..)
            .map_async(wgpu::MapMode::Read, |_| {});
        device.poll(wgpu::MaintainBase::Wait);
        let atlas_bytes = sdf_buffer_atlas.slice(..).get_mapped_range();
        let atlas_pixels = atlas_bytes.as_bytes().to_owned();
        drop(atlas_bytes);

        println!(
            "SDF Gen: {:?}",
            std::time::Instant::now().duration_since(start)
        );

        atlas_pixels
    }

    fn atlas_path(font_name: &str) -> String {
        format!("assets/fonts/{font_name}.png")
    }

    fn save_atlas(font_name: &str, atlas: &Vec<u8>, atlas_width: i32, atlas_height: i32) {
        image::save_buffer(
            Font::atlas_path(font_name),
            &atlas,
            atlas_width as u32,
            atlas_height as u32,
            image::ColorType::L8,
        )
        .expect("Couldn't save atlas");
    }

    fn load_atlas(font_name: &str) -> Vec<u8> {
        let img = image::open(Font::atlas_path(font_name)).expect("Failed to load font atlas");
        img.to_luma8().into_vec()
    }

    /// # Returns
    /// (x: f32, y: f32) locations of chars in em units, x or y is negative if char is not renderable
    pub fn calc_layout(&self, text: &str) -> Vec<(f32, f32)> {
        let mut x: i32 = 0;
        let mut y: i32 = 0;
        let font = self.font.as_face_ref();
        let x_space = (font.global_bounding_box().width() as f32 * 0.5) as i32;
        let y_space = (font.global_bounding_box().height() as f32 * 1.5) as i32;
        let mut layout = Vec::with_capacity(text.len());
        let gap = (0.1 * font.units_per_em() as f32) as i32;
        let mut prev_c: char = '\0';
        for c in text.chars() {
            match c {
                ' ' => {
                    x += x_space;
                    layout.push((-1.0, -1.0));
                }
                '\n' => {
                    y -= y_space;
                    x = 0;
                    layout.push((-1.0, -1.0));
                }
                '\t' => {
                    x += x_space * 4;
                    layout.push((-1.0, -1.0));
                }
                _ if c.is_ascii_graphic() => {
                    let em = 1.0 / font.units_per_em() as f32;
                    let gid = font.glyph_index(c).unwrap();
                    let bb = font.glyph_bounding_box(gid).unwrap();
                    let prev_gid = font.glyph_index(prev_c);
                    if let Some(prev_gid) = prev_gid {
                        let prev_bb = font.glyph_bounding_box(prev_gid);
                        if let Some(prev_bb) = prev_bb {
                            x += prev_bb.width() as i32 + gap;
                        }
                    }
                    x += bb.width() as i32 + gap;
                    let xoff = bb.x_min as i32;
                    let yoff = (bb.y_min + bb.y_max) as i32;
                    layout.push(((x + xoff) as f32 * em, (y + yoff) as f32 * em));
                }
                _ => {
                    layout.push((-1.0, -1.0));
                }
            }
            prev_c = c;
        }
        layout
    }

    fn glyph2uv(&self, x: i16) -> f32 {
        x as f32 * self.max_glyph_size as f32 / self.font.as_face_ref().units_per_em() as f32
    }

    pub fn char_uv(&self, c: char) -> [f32; 4] {
        let pos = self.char_atlas_uvs[c as usize];
        let font = self.font.as_face_ref();
        let gid = font.glyph_index(c).unwrap();
        let bb = font.glyph_bounding_box(gid).unwrap();
        let w = self.atlas_texture.width() as f32;
        let h = self.atlas_texture.height() as f32;
        let x = self.glyph2uv(bb.x_min) / w;
        let y = self.glyph2uv(bb.y_min) / h;
        let width = self.glyph2uv(bb.width()) / w;
        let height = self.glyph2uv(bb.height()) / h;
        [pos[0] + x, 1.0 - pos[1] - y, width, height]
    }

    pub fn char_size(&self, c: char) -> (f32, f32) {
        let font = self.font.as_face_ref();
        let gid = font.glyph_index(c).unwrap();
        let bb = font.glyph_bounding_box(gid).unwrap();
        let n = 1.0 / font.units_per_em() as f32;
        (bb.width() as f32 * n, bb.height() as f32 * n)
    }

    pub fn atlas_view(&self) -> &wgpu::TextureView {
        &self.atlas_view
    }

    pub fn atlas_sampler(&self) -> &wgpu::Sampler {
        &self.atlas_sampler
    }
}
