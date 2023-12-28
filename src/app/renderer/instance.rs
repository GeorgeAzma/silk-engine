use wgpu::{self, util::DeviceExt, VertexAttribute};

#[repr(C, align(16))]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Instance {
    position: [f32; 2],
    scale: [f32; 2],
    color: [f32; 4],
    stroke_color: [f32; 4],
    stroke_width: f32,
    roundness: f32,
    rotation: f32,
    sides: i32,
}

impl Instance {
    pub fn new(
        position: [f32; 2],
        scale: [f32; 2],
        color: [f32; 4],
        stroke_color: [f32; 4],
        stroke_width: f32,
        roundness: f32,
        rotation: f32,
        sides: i32,
    ) -> Self {
        Self {
            position,
            scale,
            color,
            stroke_color,
            stroke_width,
            roundness,
            rotation,
            sides,
        }
    }
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        static ATTRS: [VertexAttribute; 8] = wgpu::vertex_attr_array![
            0 => Float32x2,
            1 => Float32x2,
            2 => Float32x4,
            3 => Float32x4,
            4 => Float32,
            5 => Float32,
            6 => Float32,
            7 => Sint32,
        ];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Instance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &ATTRS,
        }
    }
}

pub struct Manager {
    instances: Vec<Instance>,
    instance_buffer: wgpu::Buffer,
    index: usize,
}

impl Manager {
    pub fn new(device: &wgpu::Device) -> Self {
        let mut instances = Vec::new();
        instances.resize(1_000_000, Instance::default());
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&instances),
        });

        Self {
            instances,
            instance_buffer,
            index: 0,
        }
    }

    // Called before rendering
    pub fn add(&mut self, instance: Instance) {
        self.instances[self.index] = instance;
        self.index = (self.index + 1) % self.instances.len();
    }

    // Called after finishing adding the instances
    pub fn flush(&mut self, queue: &wgpu::Queue) {
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&self.instances[0..self.index]),
        );
    }

    pub fn render<'a>(&'a mut self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        render_pass.draw(0..4, 0..self.index as _);
        self.index = 0;
    }
}
