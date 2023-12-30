use crate::assets;
use easy_signed_distance_field as sdf;
use image::EncodableLayout;

pub struct Font {
    font: sdf::Font,
    max_glyph_size: i32,
    // Note: size of these arrays can be 96, but that would require extra subtraction for access
    char_metrics: [sdf::Metrics; 128],
    char_atlas_uvs: [[f32; 2]; 128],
    atlas_texture: wgpu::Texture,
    atlas_view: wgpu::TextureView,
    atlas_sampler: wgpu::Sampler,
}

impl Font {
    const MAX_GRAPHIC_CHARS: i32 = 96;

    /// Returns (width, height)
    fn gen_uvs(
        font: &sdf::Font,
        max_glyph_size: i32,
        char_metrics: &mut [sdf::Metrics; 128],
        char_atlas_uvs: &mut [[f32; 2]; 128],
    ) -> (i32, i32) {
        // Prediction of atlas width to match its height as closely as possible
        let atlas_width: i32 =
            (max_glyph_size as f32 * (Font::MAX_GRAPHIC_CHARS as f32).sqrt() * 0.64) as i32;

        let mut x: i32 = 0;
        let mut y: i32 = 0;
        let mut max_height: i32 = 0;

        for i in 0..=127u8 {
            let c = i as char;
            if !char::is_ascii_graphic(&c) {
                continue;
            }

            if let Some(metrics) = font.metrics(c, max_glyph_size as f32) {
                let idx = i as usize;
                char_metrics[idx] = metrics;

                max_height = max_height.max(metrics.height);
                if x + metrics.width >= atlas_width {
                    x = 0;
                    y += max_height;
                    max_height = 0;
                }

                char_atlas_uvs[idx][0] = x as f32;
                char_atlas_uvs[idx][1] = y as f32;
                x += metrics.width;
            }
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
        font: &sdf::Font,
        atlas_width: i32,
        atlas_height: i32,
        max_glyph_size: i32,
        char_atlas_uvs: &[[f32; 2]; 128],
    ) -> Vec<u8> {
        let mut atlas_buffer = Vec::new();
        atlas_buffer.resize((atlas_width * atlas_height) as usize, 0.0);

        for i in 0..=127u8 {
            let c = i as char;
            if !char::is_ascii_graphic(&c) {
                continue;
            }
            if let Some((metrics, sdf)) = font.sdf_generate(max_glyph_size as f32, 2, 3.0, c) {
                let idx = i as usize;
                let dst_idx = ((char_atlas_uvs[idx][0] * atlas_width as f32) as i32
                    + (char_atlas_uvs[idx][1] * atlas_height as f32) as i32 * atlas_width)
                    as usize;
                for j in 0..metrics.height {
                    for i in 0..metrics.width {
                        let src_idx = (i + j * metrics.width) as usize;
                        atlas_buffer[dst_idx + (i + j * atlas_width) as usize] =
                            sdf.buffer[src_idx];
                    }
                }
            }
        }

        let mut atlas_pixels = vec![0u8; atlas_buffer.len()];
        for i in 0..atlas_pixels.len() {
            atlas_pixels[i] = (atlas_buffer[i] * 255.0).round() as u8;
        }
        atlas_pixels
    }

    fn atlas_path(font_name: &str) -> String {
        assets::get(format!("fonts/{font_name}.png"))
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

    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, font_name: &str) -> Self {
        let max_glyph_size: i32 = 64;
        // Load TTF font
        let font_path = assets::get(format!("fonts/{font_name}.ttf"));
        let font_data = std::fs::read(&font_path)
            .expect(format!("Failed to read font file: {font_path}").as_str());
        let font = sdf::Font::from_bytes(font_data.as_slice(), Default::default())
            .expect("Failed to parse font file");

        // Calculate char info
        let mut char_metrics: [sdf::Metrics; 128] = [sdf::Metrics::default(); 128];
        let mut char_atlas_uvs: [[f32; 2]; 128] = [[0.0, 0.0]; 128];

        let (atlas_width, atlas_height) = Font::gen_uvs(
            &font,
            max_glyph_size,
            &mut char_metrics,
            &mut char_atlas_uvs,
        );

        // Load atlas if it exists, otherwise generate a new one
        let atlas_path_str = Font::atlas_path(font_name);
        let atlas_path = std::path::Path::new(&atlas_path_str);
        let atlas_buffer = if atlas_path.exists() {
            Font::load_atlas(font_name)
        } else {
            let atlas_buffer = Font::gen_atlas(
                &font,
                atlas_width,
                atlas_height,
                max_glyph_size,
                &char_atlas_uvs,
            );
            Font::save_atlas(font_name, &atlas_buffer, atlas_width, atlas_height);
            atlas_buffer
        };

        // Create atlas GPU texture
        let atlas_texture_size = wgpu::Extent3d {
            width: atlas_width as u32,
            height: atlas_height as u32,
            depth_or_array_layers: 1,
        };
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            size: atlas_texture_size,
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
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        queue.write_texture(
            atlas_texture.as_image_copy(),
            &atlas_buffer,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(atlas_width as u32),
                rows_per_image: Some(atlas_height as u32),
            },
            atlas_texture_size,
        );

        Self {
            font,
            max_glyph_size,
            char_metrics,
            char_atlas_uvs,
            atlas_texture,
            atlas_view,
            atlas_sampler,
        }
    }

    pub fn atlas_view(&self) -> &wgpu::TextureView {
        &self.atlas_view
    }

    pub fn atlas_sampler(&self) -> &wgpu::Sampler {
        &self.atlas_sampler
    }

    pub fn char_uv(&self, c: char) -> [f32; 4] {
        let uv = self.char_atlas_uvs[c as usize];
        let metrics = self.char_metrics[c as usize];
        [
            uv[0],
            uv[1],
            metrics.width as f32 / self.atlas_texture.size().width as f32,
            metrics.height as f32 / self.atlas_texture.size().height as f32,
        ]
    }

    pub fn char_metrics(&self, c: char) -> sdf::Metrics {
        self.char_metrics[c as usize]
    }

    pub fn max_glyph_size(&self) -> i32 {
        self.max_glyph_size
    }

    pub fn units_per_em(&self) -> f32 {
        self.font.units_per_em()
    }
}
