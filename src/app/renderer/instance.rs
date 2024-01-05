use crate::cooldown::{self, *};
use std::rc::Rc;
use wgpu::{self, util::DeviceExt};

pub struct Manager<T: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Default> {
    device: Rc<wgpu::Device>,
    queue: Rc<wgpu::Queue>,
    instances: Vec<T>,
    instance_buffer: wgpu::Buffer,
    index: usize,
    max_instances: usize,
    shrink_cooldown: cooldown::Cooldown,
}

impl<T: Copy + Clone + bytemuck::Pod + bytemuck::Zeroable + Default> Manager<T> {
    const MIN_INSTANCES: usize = 8192;
    pub fn new(device: &Rc<wgpu::Device>, queue: &Rc<wgpu::Queue>) -> Self {
        let mut instances = Vec::new();
        instances.resize(Self::MIN_INSTANCES, T::default());
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            contents: bytemuck::cast_slice(&instances),
        });
        let max_instances = instances.len();

        Self {
            device: device.clone(),
            queue: queue.clone(),
            instances,
            instance_buffer,
            index: 0,
            max_instances,
            shrink_cooldown: cooldown::Cooldown::new(std::time::Duration::from_secs_f32(5.0)),
        }
    }

    // Called before rendering
    pub fn add(&mut self, instance: T) {
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
        self.instances.resize(self.max_instances, T::default());
        println!("Instance vector grown");
    }

    fn try_shrink(&mut self) {
        if self.max_instances / 2 <= Self::MIN_INSTANCES || self.index >= self.max_instances / 2 {
            self.shrink_cooldown.reset();
            return;
        }

        if self.shrink_cooldown.ready() {
            self.shrink_cooldown.reset();
            self.max_instances /= 2;
            self.instances.resize(self.max_instances, T::default());
            println!("Instance vector shrunk");
        }
    }

    // Called after finishing adding the instances
    pub fn flush(&mut self) {
        self.try_shrink();
        if self.instance_buffer.size() as usize != self.max_instances * std::mem::size_of::<T>() {
            self.instance_buffer =
                self.device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: None,
                        usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                        contents: bytemuck::cast_slice(&self.instances),
                    });
            println!("Instance buffer resized");
        } else {
            self.queue.write_buffer(
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
