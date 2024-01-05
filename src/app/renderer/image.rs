use std::rc::Rc;

pub struct Image {
    pub texture: Rc<wgpu::Texture>,
    pub view: Option<Rc<wgpu::TextureView>>,
    pub sampler: Option<Rc<wgpu::Sampler>>,
}

impl Image {
    pub fn new(device: &wgpu::Device, descriptor: &wgpu::TextureDescriptor) -> Self {
        Self {
            texture: Rc::new(device.create_texture(descriptor)),
            view: None,
            sampler: None,
        }
    }

    pub fn new_2d_extra(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        mip_level_count: u32,
        sample_count: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        view_formats: &[wgpu::TextureFormat],
    ) -> Self {
        Self::new(
            device,
            &wgpu::TextureDescriptor {
                label: None,
                size: wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count,
                sample_count,
                dimension: wgpu::TextureDimension::D2,
                format,
                usage,
                view_formats,
            },
        )
    }

    pub fn new_2d(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Self {
        Self::new_2d_extra(device, width, height, 1, 1, format, usage, &[])
    }

    pub fn from(device: &wgpu::Device, queue: &wgpu::Queue, path: &str) -> Self {
        let full_path = format!("assets/images/{path}");
        let img = image::open(full_path.as_str())
            .expect(format!("Failed to load texture: {full_path}").as_str());

        let image = Self::new_2d(
            device,
            img.width(),
            img.height(),
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::STORAGE_BINDING,
        );

        let img_data = img.into_rgba8().into_vec();
        queue.write_texture(
            image.texture.as_image_copy(),
            &img_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(image.texture.width() * 4),
                rows_per_image: Some(image.texture.height()),
            },
            wgpu::Extent3d {
                width: image.texture.width(),
                height: image.texture.height(),
                depth_or_array_layers: 1,
            },
        );

        image
    }

    pub fn create_view(&mut self, descriptor: &wgpu::TextureViewDescriptor) {
        self.view = Some(Rc::new(self.texture.create_view(descriptor)));
    }

    pub fn create_view_default(&mut self) {
        self.create_view(&wgpu::TextureViewDescriptor::default());
    }
}
