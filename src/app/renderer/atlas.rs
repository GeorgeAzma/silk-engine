use crate::cooldown;

use super::assets;
use super::image::Image;
use std::{
    collections::HashMap,
    hash::{Hash, Hasher},
    num::NonZeroU32,
    ops::Deref,
    rc::Rc,
};

struct RcWrapper(Rc<Image>);

impl PartialEq for RcWrapper {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for RcWrapper {}

impl Hash for RcWrapper {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let ptr = Rc::as_ptr(&self.0);
        ptr.hash(state);
    }
}

pub struct Packer {
    width: u32,
    height: u32,
    empty_spaces: Vec<(u32, u32, u32, u32)>,
    allow_rotate: bool,
}

impl Packer {
    pub fn new(width: u32, height: u32, allow_rotate: bool) -> Self {
        Self {
            width,
            height,
            empty_spaces: vec![(0, 0, width, height)],
            allow_rotate,
        }
    }

    // Might rotate the rectangle, do (width == old_width) to check
    pub fn pack(&mut self, mut w: u32, mut h: u32) -> Option<(u32, u32, u32, u32)> {
        let mut start_i = 0;

        let space;
        if self.allow_rotate {
            space = self.empty_spaces.binary_search_by(|(_, _, ew, eh)| {
                let cmp_wh = ew.cmp(&w).then(eh.cmp(&h));
                if cmp_wh == std::cmp::Ordering::Equal {
                    return std::cmp::Ordering::Equal;
                }

                let cmp_hw = ew.cmp(&h).then(eh.cmp(&w));
                if cmp_hw == std::cmp::Ordering::Equal {
                    std::mem::swap(&mut w, &mut h);
                    return std::cmp::Ordering::Equal;
                }

                if cmp_wh == std::cmp::Ordering::Greater || cmp_hw == std::cmp::Ordering::Greater {
                    return std::cmp::Ordering::Greater;
                }

                std::cmp::Ordering::Less
            });
        } else {
            space = self
                .empty_spaces
                .binary_search_by(|(_, _, ew, eh)| ew.cmp(&w).then(eh.cmp(&h)));
        }

        if let Ok(space) = space {
            let (ex, ey, _, _) = self.empty_spaces[space];
            self.empty_spaces.remove(space);
            return Some((ex, ey, w, h));
        } else if let Err(space) = space {
            start_i = space;
        }

        let len = self.empty_spaces.len();
        let mut search = |start, end| {
            for i in start..end {
                let (ex, ey, ew, eh) = self.empty_spaces[i];

                if (w <= ew && h <= eh) || (self.allow_rotate && h <= ew && w <= eh) {
                    if self.allow_rotate && ((ew - w) * (eh - h) > (ew - h) * (eh - w)) {
                        std::mem::swap(&mut w, &mut h); // Rotate
                    }

                    return if w < ew && h < eh {
                        if ew - w < eh - h {
                            self.empty_spaces[i] = (ex + w, ey, ew - w, h);
                            self.empty_spaces.push((ex, ey + h, ew, eh - h));
                        } else {
                            self.empty_spaces[i] = (ex, ey + h, w, eh - h);
                            self.empty_spaces.push((ex + w, ey, ew - w, eh));
                        }
                        Some((ex, ey, w, h))
                    } else if h == eh {
                        self.empty_spaces[i] = (ex + w, ey, ew - w, eh);
                        Some((ex, ey, w, h))
                    } else if w == ew {
                        self.empty_spaces[i] = (ex, ey + h, ew, eh - h);
                        Some((ex, ey, w, h))
                    } else {
                        self.empty_spaces.swap_remove(i);
                        Some((ex, ey, w, h))
                    };
                }
            }
            None
        };

        if let Some(space) = search(start_i, len) {
            return Some(space);
        } else {
            return search(0, start_i);
        }
    }

    pub fn reset(&mut self) {
        self.empty_spaces = vec![(0, 0, self.width, self.height)];
    }

    // Note: not a perfect resize
    pub fn resize(&mut self, width: u32, height: u32) {
        assert!(width >= self.width && height >= self.height);
        if width == self.width && height == self.height {
            return;
        }
        let (big, small) = if width - self.width < height - self.height {
            (
                (0, self.height, width, height - self.height),
                (self.width, 0, width - self.width, self.height),
            )
        } else {
            (
                (self.width, 0, width - self.width, height),
                (0, self.height, self.width, height - self.height),
            )
        };
        self.empty_spaces.push(big);
        self.empty_spaces.push(small);
        self.width = width;
        self.height = height;
    }
}

pub struct Manager {
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    packer: Packer,
    atlas: Image,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    rotate_bind_group_layout: wgpu::BindGroupLayout,
    rotate_pipeline: wgpu::ComputePipeline,
    rotate_uniform_buffer: wgpu::Buffer,
    needs_resize: bool,
    textures: HashMap<RcWrapper, (u32, u32, u32, u32)>,
    scheduled_writes: Vec<(Rc<Image>, u32, u32, u32, u32)>,
    shrink_cooldown: cooldown::Cooldown,
}

impl Manager {
    pub fn new(device: &Rc<wgpu::Device>, queue: &Rc<wgpu::Queue>) -> Self {
        let packer = Packer::new(64, 64, true);

        let mut atlas = Image::new_2d(
            device,
            packer.width,
            packer.height,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
        );

        atlas.create_view_default();
        atlas.sampler = Some(Rc::new(device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            min_filter: wgpu::FilterMode::Nearest,
            mag_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        })));

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Texture Manager Atlas"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(atlas.view.as_ref().unwrap()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(atlas.sampler.as_ref().unwrap()),
                },
            ],
        });

        let rotate_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Texture Manager Atlas Rotate"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::WriteOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::StorageTexture {
                            access: wgpu::StorageTextureAccess::ReadOnly,
                            format: wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let rotate_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Rotate"),
                bind_group_layouts: &[&rotate_bind_group_layout],
                push_constant_ranges: &[],
            });

        let rotate_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Rotate"),
            layout: Some(&rotate_pipeline_layout),
            module: &device.create_shader_module(assets::get_shader("rotate90.wgsl")),
            entry_point: "main",
        });

        let rotate_uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Rotate"),
            size: 4 * 2,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            device: device.clone(),
            queue: queue.clone(),
            packer,
            atlas,
            bind_group_layout,
            bind_group,
            rotate_bind_group_layout,
            rotate_pipeline,
            rotate_uniform_buffer,
            needs_resize: false,
            textures: HashMap::new(),
            scheduled_writes: Vec::new(),
            shrink_cooldown: cooldown::Cooldown::new(std::time::Duration::from_secs_f32(5.0)),
        }
    }

    /// Returns negated width/height if rotated
    pub fn add(&mut self, image: &Rc<Image>) -> (f32, f32, f32, f32) {
        assert!(
            image.texture.format() == wgpu::TextureFormat::Rgba8Unorm,
            "Texture format must be Rgba8Unorm"
        );
        if let Some((x, y, w, h)) = self.textures.get(&RcWrapper(image.clone())) {
            let mut s = 1.0;
            if *w != image.texture.width() {
                s = -1.0;
            }
            return (
                *x as f32 / self.packer.width as f32,
                *y as f32 / self.packer.height as f32,
                *w as f32 / self.packer.width as f32 * s,
                *h as f32 / self.packer.height as f32 * s,
            );
        }

        let space = self
            .packer
            .pack(image.texture.width(), image.texture.height());
        if let Some((x, y, w, h)) = space {
            self.write(image, x, y, w, h);
            return (
                x as f32 / self.packer.width as f32,
                y as f32 / self.packer.height as f32,
                w as f32 / self.packer.width as f32,
                h as f32 / self.packer.height as f32,
            );
        }

        self.needs_resize = true;
        return (0.0, 0.0, 0.0, 0.0);
    }

    fn write(&mut self, texture: &Rc<Image>, x: u32, y: u32, w: u32, h: u32) {
        self.textures
            .insert(RcWrapper(texture.clone()), (x, y, w, h));
        self.scheduled_writes.push((texture.clone(), x, y, w, h));
    }

    pub fn flush(&mut self, encoder: &mut wgpu::CommandEncoder) {
        if self.needs_resize {
            self.needs_resize = false;
            self.resize(self.packer.width * 2, self.packer.height * 2);
        }
        // TODO: shrink overtime
        for (image, x, y, w, h) in self.scheduled_writes.iter() {
            // Rotated
            if *w != image.texture.width() {
                let data: [u32; 2] = [*x, *y];
                self.queue.write_buffer(
                    &self.rotate_uniform_buffer,
                    0,
                    bytemuck::cast_slice(&data),
                );

                let view = if let Some(image_view) = image.view.as_ref() {
                    image_view.clone()
                } else {
                    Rc::new(
                        image
                            .texture
                            .create_view(&wgpu::TextureViewDescriptor::default()),
                    )
                };
                let rotate_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: None,
                    layout: &self.rotate_bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(
                                self.atlas.view.as_ref().unwrap(),
                            ),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(&view),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: self.rotate_uniform_buffer.as_entire_binding(),
                        },
                    ],
                });

                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Rotate"),
                    timestamp_writes: None,
                });

                compute_pass.set_pipeline(&self.rotate_pipeline);
                compute_pass.set_bind_group(0, &rotate_bind_group, &[]);

                compute_pass.dispatch_workgroups(
                    (image.texture.width() + 15) / 16,
                    (image.texture.height() + 15) / 16,
                    1,
                );
            } else {
                encoder.copy_texture_to_texture(
                    image.texture.as_image_copy(),
                    wgpu::ImageCopyTextureBase {
                        texture: &self.atlas.texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d { x: *x, y: *y, z: 0 },
                        aspect: wgpu::TextureAspect::All,
                    },
                    wgpu::Extent3d {
                        width: *w,
                        height: *h,
                        depth_or_array_layers: 1,
                    },
                );
            }
        }
        self.scheduled_writes.clear();
    }

    fn resize(&mut self, width: u32, height: u32) {
        // This will recalculate everything next frame
        // It's avoidable, but I was encountering bugs
        // So decided to leave it
        self.packer.resize(width, height);
        self.packer.reset();
        self.textures.clear();

        let sampler = self.atlas.sampler.as_ref().unwrap().clone();
        self.atlas = Image::new_2d(
            &self.device,
            self.packer.width,
            self.packer.height,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::STORAGE_BINDING
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST,
        );
        self.atlas.create_view_default();
        self.atlas.sampler = Some(sampler.clone());
        self.bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(self.atlas.view.as_ref().unwrap()),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(self.atlas.sampler.as_ref().unwrap()),
                },
            ],
        });
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
