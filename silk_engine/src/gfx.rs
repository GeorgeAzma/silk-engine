pub(crate) mod draw_context;
pub(crate) mod texture_atlas;
pub(crate) mod vertex;

pub use draw_context::DrawContext;
pub use texture_atlas::TextureAtlas;
pub use vertex::Vertex;

use std::{collections::HashMap, sync::Arc};

use crate::{
    prelude::ResultAny,
    util::{dirty::Dirty, font::Font, image_loader::ImageLoader, packer::Rect},
    vulkan::{
        PhysicalDeviceUse, Vulkan,
        buffer::Buffer,
        command_manager::CommandManager,
        device::Device,
        physical_device::PhysicalDevice,
        pipeline::{Pipeline, PipelineConfig, PipelineLayout},
        shader::Shader,
        window::{Frame, Window},
    },
};
use ash::vk;
use winit::{event_loop::ActiveEventLoop, window::WindowAttributes};

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Unit {
    /// pixels
    Px(i32),
    /// 1.0 is min(width, height) pixels
    Mn(f32),
    /// 1.0 is max(width, height) pixels
    Mx(f32),
    /// screen is 0-1 range
    Pc(f32),
}

#[derive(Resource)]
pub struct Gfx {
    pub(crate) physical_device: Arc<PhysicalDevice>,
    pub(crate) device: Arc<Device>,
    queue: vk::Queue,
    surface_format: Option<vk::Format>,
    command_manager: Arc<CommandManager>,
    shader: Shader,
    pipeline: Option<Pipeline>,
    pipeline_layout: Option<PipelineLayout>,
    uniform: Arc<Buffer>,
    width: f32,
    height: f32,
    descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    descriptor_sets: Vec<vk::DescriptorSet>,

    pub draw: DrawContext,
    pub atlas: TextureAtlas,
}

impl Gfx {
    pub fn create_window(
        &mut self,
        event_loop: &ActiveEventLoop,
        attributes: WindowAttributes,
    ) -> Window {
        let window = event_loop.create_window(attributes).unwrap();
        Window::new(&self.device, window, vec![], vec![]).unwrap()
    }

    // Delegate drawing methods to DrawContext
    pub fn alpha(&mut self, a: u8) {
        self.draw.alpha(a);
    }
    pub fn rgb(&mut self, r: u8, g: u8, b: u8) {
        self.draw.rgb(r, g, b);
    }
    pub fn rgba(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.draw.rgba(r, g, b, a);
    }
    pub fn hex(&mut self, hex: u32) {
        self.draw.hex(hex);
    }
    pub fn glow(&mut self, glow: f32) {
        self.draw.glow(glow);
    }
    pub fn stroke_alpha(&mut self, a: u8) {
        self.draw.stroke_alpha(a);
    }
    pub fn stroke_rgb(&mut self, r: u8, g: u8, b: u8) {
        self.draw.stroke_rgb(r, g, b);
    }
    pub fn stroke_rgba(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.draw.stroke_rgba(r, g, b, a);
    }
    pub fn stroke_hex(&mut self, hex: u32) {
        self.draw.stroke_hex(hex);
    }
    pub fn gradient_alpha(&mut self, a: u8) {
        self.draw.gradient_alpha(a);
    }
    pub fn gradient_rgb(&mut self, r: u8, g: u8, b: u8) {
        self.draw.gradient_rgb(r, g, b);
    }
    pub fn gradient_rgba(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.draw.gradient_rgba(r, g, b, a);
    }
    pub fn gradient_hex(&mut self, hex: u32) {
        self.draw.gradient_hex(hex);
    }
    pub fn no_gradient(&mut self) {
        self.draw.no_gradient();
    }
    pub fn font(&mut self, font: &str) {
        self.draw.font(font);
    }

    pub fn rectc(&mut self, x: Unit, y: Unit, w: Unit, h: Unit) {
        self.draw.rectc(x, y, w, h);
    }
    pub fn rect(&mut self, x: Unit, y: Unit, w: Unit, h: Unit) {
        self.draw.rect(x, y, w, h);
    }
    pub fn rrectc(&mut self, x: Unit, y: Unit, w: Unit, h: Unit, r: f32) {
        self.draw.rrectc(x, y, w, h, r);
    }
    pub fn rrect(&mut self, x: Unit, y: Unit, w: Unit, h: Unit, r: f32) {
        self.draw.rrect(x, y, w, h, r);
    }
    pub fn squarec(&mut self, x: Unit, y: Unit, w: Unit) {
        self.draw.squarec(x, y, w);
    }
    pub fn square(&mut self, x: Unit, y: Unit, w: Unit) {
        self.draw.square(x, y, w);
    }
    pub fn rsquare(&mut self, x: Unit, y: Unit, w: Unit, r: f32) {
        self.draw.rsquare(x, y, w, r);
    }
    pub fn rsquarec(&mut self, x: Unit, y: Unit, w: Unit, r: f32) {
        self.draw.rsquarec(x, y, w, r);
    }
    pub fn aabb(&mut self, x0: Unit, y0: Unit, x1: Unit, y1: Unit) {
        self.draw.aabb(x0, y0, x1, y1);
    }
    pub fn circle(&mut self, x: Unit, y: Unit, r: Unit) {
        self.draw.circle(x, y, r);
    }
    pub fn line(&mut self, x0: Unit, y0: Unit, x1: Unit, y1: Unit, w: Unit) {
        self.draw.line(x0, y0, x1, y1, w);
    }
    pub fn rline(&mut self, x0: Unit, y0: Unit, x1: Unit, y1: Unit, w: Unit) {
        self.draw.rline(x0, y0, x1, y1, w);
    }
    pub fn bezier(&mut self, x0: Unit, y0: Unit, x1: Unit, y1: Unit, x2: Unit, y2: Unit, w: Unit) {
        self.draw.bezier(x0, y0, x1, y1, x2, y2, w);
    }
    pub fn area(&mut self, x: Unit, y: Unit, w: Unit, h: Unit) {
        self.draw.area(x, y, w, h);
    }
    pub fn push_area(&mut self, x: Unit, y: Unit, w: Unit, h: Unit) {
        self.draw.push_area(x, y, w, h);
    }
    pub fn pop_area(&mut self) {
        self.draw.pop_area();
    }
    pub fn begin_temp(&mut self) {
        self.draw.begin_temp();
    }
    pub fn end_temp(&mut self) {
        self.draw.end_temp();
    }

    // Delegate atlas methods to TextureAtlas
    pub fn add_font(&mut self, name: &str) {
        self.atlas.add_font(name);
        self.draw.font = name.to_string();
    }

    pub fn add_img(
        &mut self,
        name: &str,
        width: u32,
        height: u32,
    ) -> (u64, &mut Dirty<&'static mut [u8]>, Rect) {
        self.atlas.add_img(name, width, height)
    }

    pub fn load_img(&mut self, name: &str) -> &mut Dirty<&'static mut [u8]> {
        self.atlas.load_img(name)
    }

    pub fn atlas(&mut self) {
        self.draw.tex_coord = self.atlas.atlas_tex_coord();
    }

    pub fn no_img(&mut self) {
        self.draw.tex_coord = self.atlas.no_img_tex_coord();
    }

    pub fn img(&mut self, name: &str) -> &mut Dirty<&'static mut [u8]> {
        self.draw.tex_coord = self.atlas.img_tex_coord(name);
        self.atlas.img(name)
    }

    pub fn img_data(&self, name: &str) -> (&[u8], u32, u32) {
        self.atlas.img_data(name)
    }

    /// Renders text. Returns bounding rect in pixels
    pub fn text(&mut self, text: &str, x: Unit, y: Unit, w: Unit) -> (i32, i32, i32, i32) {
        let old_tex_coord = self.draw.tex_coord;
        assert!(
            self.draw.font.as_str() != "",
            "failed to render text, no font is active"
        );
        let old_roundness = self.draw.roundness;
        self.draw.roundness = -(self.draw.bold + 1.0 + 1e-5);
        let (x, y) = (self.draw.pc_x(x), self.draw.pc_y(y));
        let (w, h) = (self.draw.pc_x(w), self.draw.pc_y(w));

        let self_ptr = self as *mut Self;
        let (font, char_rects) = unsafe { &mut *self_ptr }
            .atlas
            .fonts
            .entry(self.draw.font.clone())
            .or_insert_with(|| {
                if let Ok(true) = std::fs::exists(format!("res/fonts/{}.ttf", self.draw.font)) {
                    (Font::new(&self.draw.font), HashMap::new())
                } else {
                    panic!(
                        "failed to render text \"{text}\", font does not exist: {}",
                        self.draw.font
                    )
                }
            });

        const SDF_PX: u32 = 64;
        let px = SDF_PX as f32;
        let (mut ax, mut ay, mut bx, mut by) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        let layout = font.layout(text);
        for (i, c) in text.chars().enumerate() {
            if !font.is_char_graphic(c) {
                continue;
            }

            let rect = *char_rects.entry(c).or_insert_with(|| {
                let sdf_img = font.gen_char_sdf(c, SDF_PX);
                let name = format!("{}-{c}", font.name());
                let (_, img, rect) =
                    unsafe { &mut *self_ptr }
                        .atlas
                        .add_img(&name, sdf_img.width, sdf_img.height);
                img.copy_from_slice(&ImageLoader::make4(&sdf_img.img, 1));
                rect
            });

            let (lx, ly) = layout[i];
            let (rw, rh) = rect.wh();
            let (rw, rh) = (rw as f32 / px * w, rh as f32 / px * h);
            let r = rect.packed_whxy();
            self.draw.tex_coord = [(r >> 32) as u32, r as u32];
            let (x, y) = (x + lx * w, y + ly * h);
            let (w, h) = (rw, rh);
            let vertex = self.draw.vert(x + w, y + h, w, h);
            self.draw.instances.add(vertex).unwrap();
            ax = ax.min(x);
            ay = ay.min(y);
            bx = bx.max(x + w * 2.0);
            by = by.max(y + h * 2.0);
        }
        self.draw.roundness = old_roundness;
        self.draw.tex_coord = old_tex_coord;
        use Unit::*;
        let (ax, ay, bx, by) = (
            self.draw.px_x(Pc(ax)),
            self.draw.px_y(Pc(ay)),
            self.draw.px_x(Pc(bx)),
            self.draw.px_y(Pc(by)),
        );
        (ax as i32, ay as i32, bx as i32, by as i32)
    }

    pub(crate) fn flush(&mut self) -> ResultAny {
        let buf_copies = self.atlas.dirty_copies();
        if !buf_copies.is_empty() {
            let cmd = self.command_manager.begin()?;
            self.atlas
                .atlas_image()
                .transition(cmd, vk::ImageLayout::TRANSFER_DST_OPTIMAL);
            self.atlas.atlas_image().copy_from_buffer_cmd(
                cmd,
                self.atlas.atlas_staging().lock().unwrap().as_ref(),
                &buf_copies,
            );
            self.atlas
                .atlas_image()
                .transition(cmd, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);
            self.command_manager.submit(self.queue, cmd, &[], &[])?;
            self.command_manager.wait(cmd)?;
        }
        Ok(())
    }

    fn recreate_render_data(&mut self) -> ResultAny {
        let surface_format = self.surface_format.unwrap_or(vk::Format::UNDEFINED);
        let mut rendering_info = vk::PipelineRenderingCreateInfo::default()
            .color_attachment_formats(std::slice::from_ref(&surface_format));

        let pipeline_layout = PipelineLayout::new(&self.device, &self.descriptor_set_layouts, &[])?;
        self.pipeline_layout = Some(pipeline_layout);

        let spec_info = vk::SpecializationInfo::default();
        let mut pipeline_info = PipelineConfig::default();
        let pipeline_info = pipeline_info
            .with_shader(&self.shader, &spec_info)?
            .with_auto_vertex_inputs()?
            .add_color_blend_disabled_attachment()
            .build(self.pipeline_layout.as_ref().unwrap().handle())
            .push_next(&mut rendering_info);

        let pipeline = Pipeline::new(&self.device, &pipeline_info)?;
        self.pipeline = Some(pipeline);

        Ok(())
    }

    fn record_command_buffer(
        &mut self,
        window: &mut Window,
        frame: &Frame,
    ) -> ResultAny<vk::CommandBuffer> {
        let extent = window.extent();
        let cmd = self.command_manager.begin()?;
        {
            let swapchain_image = &mut window.swapchain().images()[frame.image_index as usize];
            let color_attachment = vk::RenderingAttachmentInfo::default()
                .image_view(swapchain_image.view())
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE)
                .clear_value(vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.1, 0.0, 0.3, 1.0],
                    },
                });
            let color_attachments = [color_attachment];
            let rendering_info = vk::RenderingInfo::default()
                .render_area(vk::Rect2D::default().extent(extent))
                .layer_count(1)
                .color_attachments(&color_attachments);

            swapchain_image.transition(cmd, vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

            self.atlas
                .atlas_image()
                .transition(cmd, vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL);

            unsafe { self.device().cmd_begin_rendering(cmd, &rendering_info) };
            {
                unsafe {
                    self.device().cmd_set_viewport(
                        cmd,
                        0,
                        &[vk::Viewport::default()
                            .y(extent.height as f32)
                            .width(extent.width as f32)
                            .height(-(extent.height as f32))],
                    )
                };
                unsafe {
                    self.device()
                        .cmd_set_scissor(cmd, 0, &[vk::Rect2D::default().extent(extent)])
                };
                unsafe {
                    self.device().cmd_bind_descriptor_sets(
                        cmd,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline_layout.as_ref().unwrap().handle(),
                        0,
                        &self.descriptor_sets,
                        &[],
                    )
                };
                unsafe {
                    self.device().cmd_bind_pipeline(
                        cmd,
                        vk::PipelineBindPoint::GRAPHICS,
                        self.pipeline.as_ref().unwrap().handle(),
                    )
                };
                unsafe {
                    self.device().cmd_bind_vertex_buffers(
                        cmd,
                        1,
                        &[self.draw.instances.vertex_buffer.handle()],
                        &[0],
                    );
                }
                unsafe {
                    self.device()
                        .cmd_draw(cmd, 4, self.draw.instances.inst_len as u32, 0, 0)
                };
            }
            unsafe { self.device().cmd_end_rendering(cmd) };

            swapchain_image.transition(cmd, vk::ImageLayout::PRESENT_SRC_KHR);
        }
        self.command_manager.end()?;

        Ok(cmd)
    }

    pub fn render(&mut self, window: &mut Window) {
        if self.width as u32 != window.width() || self.height as u32 != window.height() {
            self.width = window.width() as f32;
            self.height = window.height() as f32;
            self.uniform.write_mapped(&[self.width, self.height]);
            self.draw.set_size(self.width, self.height);
        }

        self.flush().unwrap();

        let Some(frame) = window.begin_frame(|cmd| {
            self.command_manager.wait(cmd).unwrap();
        }) else {
            return;
        };

        let surface_format = window.format();
        if surface_format != self.surface_format.unwrap_or(vk::Format::UNDEFINED) {
            self.surface_format = Some(surface_format);
            self.recreate_render_data().unwrap();
        }

        let cmd = self.record_command_buffer(window, &frame).unwrap();

        self.command_manager
            .submit(
                self.queue,
                cmd,
                std::slice::from_ref(
                    &vk::SemaphoreSubmitInfo::default()
                        .semaphore(frame.wait_semaphore)
                        .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT),
                ),
                std::slice::from_ref(
                    &vk::SemaphoreSubmitInfo::default()
                        .semaphore(frame.signal_semaphore)
                        .stage_mask(vk::PipelineStageFlags2::COLOR_ATTACHMENT_OUTPUT),
                ),
            )
            .unwrap();

        window.end_frame(self.queue, cmd);
        window.request_redraw();
        self.draw.reset();
    }

    pub fn device(&self) -> &ash::Device {
        &self.device.device
    }
}

impl Drop for Gfx {
    fn drop(&mut self) {
        self.device.wait();
    }
}

impl std::ops::Deref for Gfx {
    type Target = DrawContext;
    fn deref(&self) -> &Self::Target {
        &self.draw
    }
}

impl std::ops::DerefMut for Gfx {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.draw
    }
}

pub fn setup(vulkan: &Vulkan) -> ResultAny<Gfx> {
    let physical_device = vulkan
        .best_physical_device_for(PhysicalDeviceUse::General)
        .ok_or("no suitable GPU found")?;

    let queue_family_index = vulkan
        .best_queue_family_for(
            &physical_device.queue_family_properties,
            vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE | vk::QueueFlags::TRANSFER,
        )
        .ok_or("no suitable Queue Family found")?;

    let queue_create_infos = [vk::DeviceQueueCreateInfo::default()
        .queue_family_index(queue_family_index)
        .queue_priorities(&[1.0])];

    let mut descriptor_indexing = vk::PhysicalDeviceDescriptorIndexingFeatures::default()
        .shader_sampled_image_array_non_uniform_indexing(true)
        .shader_storage_buffer_array_non_uniform_indexing(true)
        .shader_storage_image_array_non_uniform_indexing(true)
        .descriptor_binding_sampled_image_update_after_bind(true)
        .descriptor_binding_storage_buffer_update_after_bind(true)
        .descriptor_binding_storage_image_update_after_bind(true)
        .descriptor_binding_partially_bound(true)
        .descriptor_binding_variable_descriptor_count(true)
        .runtime_descriptor_array(true);

    let mut features13 = vk::PhysicalDeviceVulkan13Features::default()
        .synchronization2(true)
        .dynamic_rendering(true);

    let mut ray_tracing_pipeline_features =
        vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default().ray_tracing_pipeline(true);
    let mut acceleration_structure_features =
        vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default().acceleration_structure(true);
    let mut buffer_device_address_features =
        vk::PhysicalDeviceBufferDeviceAddressFeatures::default().buffer_device_address(true);

    let mut enabled_device_features = vk::PhysicalDeviceFeatures2::default()
        .push_next(&mut descriptor_indexing)
        .push_next(&mut features13)
        .push_next(&mut ray_tracing_pipeline_features)
        .push_next(&mut acceleration_structure_features)
        .push_next(&mut buffer_device_address_features);

    let enabled_device_extensions = [
        ash::khr::swapchain::NAME.as_ptr(),
        ash::khr::deferred_host_operations::NAME.as_ptr(),
        ash::khr::acceleration_structure::NAME.as_ptr(),
        ash::khr::ray_tracing_pipeline::NAME.as_ptr(),
    ];

    let device_info = vk::DeviceCreateInfo::default()
        .queue_create_infos(&queue_create_infos)
        .enabled_extension_names(&enabled_device_extensions)
        .push_next(&mut enabled_device_features);

    let device = Device::new(&physical_device, &device_info)?;

    let queue = device.get_queue(queue_family_index, 0);
    device.debug_name(queue, "gfx");

    let command_manager = device.command_manager(queue_family_index);

    let shader = Shader::new(&["test.vert", "test.frag"], &device)?;

    let uniform = Buffer::new(
        &device,
        (2 * size_of::<f32>()) as u64,
        vk::BufferUsageFlags::UNIFORM_BUFFER,
        &[queue_family_index],
        vk::SharingMode::EXCLUSIVE,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )?;

    let atlas = TextureAtlas::new(&device, queue_family_index)?;
    let atlas_view = atlas.atlas_image().create_view()?;
    let atlas_sampler = device.get_sampler(
        vk::SamplerAddressMode::REPEAT,
        vk::SamplerAddressMode::REPEAT,
        vk::Filter::LINEAR,
        vk::Filter::LINEAR,
        vk::SamplerMipmapMode::LINEAR,
    );

    let descriptor_set_layouts = shader
        .reflect_descriptor_set_layouts()?
        .into_values()
        .collect::<Vec<_>>();
    let descriptor_sets = device.alloc_ds(&descriptor_set_layouts);
    let uniform_info = vk::DescriptorBufferInfo::default()
        .buffer(uniform.handle())
        .offset(0)
        .range(vk::WHOLE_SIZE);
    let atlas_info = vk::DescriptorImageInfo::default()
        .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
        .image_view(atlas_view)
        .sampler(atlas_sampler);
    let ds_write_uniform = vk::WriteDescriptorSet::default()
        .dst_set(descriptor_sets[0])
        .dst_binding(0)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .buffer_info(std::slice::from_ref(&uniform_info));
    let ds_write_atlas = vk::WriteDescriptorSet::default()
        .dst_set(descriptor_sets[0])
        .dst_binding(1)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .image_info(std::slice::from_ref(&atlas_info));
    unsafe {
        device
            .device
            .update_descriptor_sets(&[ds_write_uniform, ds_write_atlas], &[])
    };

    let draw = DrawContext::new(&device, queue_family_index)?;

    Ok(Gfx {
        physical_device,
        device: device.clone(),
        queue,
        surface_format: None,
        command_manager,
        shader,
        pipeline: None,
        pipeline_layout: None,
        uniform,
        width: 0.0,
        height: 0.0,
        descriptor_set_layouts,
        descriptor_sets,
        draw,
        atlas,
    })
}

pub struct GfxPlugin;
impl Plugin for GfxPlugin {
    fn build(&self, app: &mut App) {
        let gfx = setup(app.world().resource::<Vulkan>()).unwrap();
        app.insert_resource(gfx);
    }
}
