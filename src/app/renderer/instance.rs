use crate::cooldown::{self, *};
use wgpu::{self, util::DeviceExt, VertexAttribute};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Instance {
    position: [f32; 3],
    scale: [f32; 2],
    color: [u8; 4],
    stroke_color: [u8; 4],
    stroke_width: f32,
    roundness: f32,
    rotation: f32,
    sides: i32,
}

impl Instance {
    pub fn new(
        position: [f32; 3],
        scale: [f32; 2],
        color: [u8; 4],
        stroke_color: [u8; 4],
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
    max_instances: usize,
    shrink_cooldown: cooldown::Cooldown,
}

impl Manager {
    const MIN_INSTANCES: usize = 8192;
    pub fn new(device: &wgpu::Device) -> Self {
        let mut instances = Vec::new();
        instances.resize(Manager::MIN_INSTANCES, Instance::default());
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&instances),
        });
        let max_instances = instances.len();

        Self {
            instances,
            instance_buffer,
            index: 0,
            max_instances,
            shrink_cooldown: cooldown::Cooldown::new(std::time::Duration::from_secs_f32(5.0)),
        }
    }

    // Called before rendering
    pub fn add(&mut self, instance: Instance) {
        self.instances[self.index] = instance;
        self.index += 1;
        self.try_resize();
    }

    fn try_resize(&mut self) {
        if self.index < self.max_instances {
            return;
        }
        while self.index >= self.max_instances {
            self.max_instances *= 2;
        }
        self.instances
            .resize(self.max_instances, Instance::default());
        println!("Instance vector grown");
    }

    fn try_shrink(&mut self) {
        if self.max_instances / 2 <= Manager::MIN_INSTANCES || self.index >= self.max_instances / 2
        {
            self.shrink_cooldown.reset();
            return;
        }

        if self.shrink_cooldown.ready() {
            self.shrink_cooldown.reset();
            self.max_instances /= 2;
            self.instances
                .resize(self.max_instances, Instance::default());
            println!("Instance vector shrunk");
        }
    }

    // Called after finishing adding the instances
    pub fn flush(&mut self, queue: &wgpu::Queue, device: &wgpu::Device) {
        self.try_shrink();
        if self.instance_buffer.size() as usize
            != self.max_instances * std::mem::size_of::<Instance>()
        {
            self.instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                contents: bytemuck::cast_slice(&self.instances),
            });
            println!("Instance buffer resized");
        } else {
            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&self.instances[0..self.index]),
            );
        }
    }

    pub fn render<'a>(&'a mut self, render_pass: &mut wgpu::RenderPass<'a>) {
        render_pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        render_pass.draw(0..4, 0..self.index as _);
        self.index = 0;
    }
}
