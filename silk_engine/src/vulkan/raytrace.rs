use std::sync::Arc;

use ash::vk::{self, ImageCreateInfo};

use crate::{
    prelude::ResultAny,
    vulkan::{buffer::Buffer, device::Device, image::Image},
};

pub(crate) struct Raytrace {}

impl Raytrace {
    pub(crate) fn new(
        device: &Arc<Device>,
        queue_family_index: u32,
        queue: vk::Queue,
    ) -> ResultAny<Self> {
        #[repr(C)]
        #[derive(Default, Debug, Clone, Copy)]
        struct VertexRT {
            position: [f32; 3],
        }

        let vertices = [
            VertexRT {
                position: [-1.0, -1.0, 0.0],
            },
            VertexRT {
                position: [1.0, -1.0, 0.0],
            },
            VertexRT {
                position: [0.0, 1.0, 0.0],
            },
        ];
        let indices: [u32; _] = [0, 1, 2];

        let rt_vertex_buffer = Buffer::new(
            &device,
            size_of_val(&vertices) as u64,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            &[queue_family_index],
            vk::SharingMode::EXCLUSIVE,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        rt_vertex_buffer.write(&vertices, 0)?;

        let rt_index_buffer = Buffer::new(
            &device,
            size_of_val(&indices) as u64,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR,
            &[queue_family_index],
            vk::SharingMode::EXCLUSIVE,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        rt_index_buffer.write(&indices, 0)?;

        let rt_vertex_buffer_address = unsafe {
            device.device.get_buffer_device_address(
                &vk::BufferDeviceAddressInfo::default().buffer(rt_vertex_buffer.handle()),
            )
        };
        let rt_index_buffer_address = unsafe {
            device.device.get_buffer_device_address(
                &vk::BufferDeviceAddressInfo::default().buffer(rt_index_buffer.handle()),
            )
        };

        let triangles = vk::AccelerationStructureGeometryTrianglesDataKHR::default()
            .vertex_format(vk::Format::R32G32B32_SFLOAT)
            .vertex_data(vk::DeviceOrHostAddressConstKHR {
                device_address: rt_vertex_buffer_address,
            })
            .vertex_stride(size_of::<VertexRT>() as u64)
            .max_vertex(vertices.len() as u32 - 1) // max vertex index, or vertex count - 1 if not using index buffer
            .index_type(vk::IndexType::UINT32)
            .index_data(vk::DeviceOrHostAddressConstKHR {
                device_address: rt_index_buffer_address,
            });

        let accel = ash::khr::acceleration_structure::Device::new(
            &device.physical_device().vulkan().instance,
            &device.device,
        );

        let geometry = vk::AccelerationStructureGeometryKHR::default()
            .geometry_type(vk::GeometryTypeKHR::TRIANGLES)
            .geometry(vk::AccelerationStructureGeometryDataKHR { triangles })
            .flags(vk::GeometryFlagsKHR::OPAQUE);

        let mut build_geometry = vk::AccelerationStructureBuildGeometryInfoKHR::default()
            .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .geometries(std::slice::from_ref(&geometry))
            .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE);

        let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
        unsafe {
            accel.get_acceleration_structure_build_sizes(
                vk::AccelerationStructureBuildTypeKHR::DEVICE,
                &build_geometry,
                &[1],
                &mut size_info,
            )
        };

        let blas_buffer = Buffer::new(
            &device,
            size_info.acceleration_structure_size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR,
            &[queue_family_index],
            vk::SharingMode::EXCLUSIVE,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let scratch_buffer = Buffer::new(
            &device,
            size_info.build_scratch_size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            &[queue_family_index],
            vk::SharingMode::EXCLUSIVE,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        let scratch_buffer_address = unsafe {
            device.device.get_buffer_device_address(
                &vk::BufferDeviceAddressInfo::default().buffer(scratch_buffer.handle()),
            )
        };
        build_geometry.scratch_data = vk::DeviceOrHostAddressKHR {
            device_address: scratch_buffer_address,
        };

        let blas = unsafe {
            accel.create_acceleration_structure(
                &vk::AccelerationStructureCreateInfoKHR::default()
                    .ty(vk::AccelerationStructureTypeKHR::BOTTOM_LEVEL)
                    .buffer(blas_buffer.handle())
                    .size(size_info.acceleration_structure_size),
                device.physical_device().vulkan().allocation_callbacks(),
            )?
        };
        let blas_address = unsafe {
            accel.get_acceleration_structure_device_address(
                &vk::AccelerationStructureDeviceAddressInfoKHR::default()
                    .acceleration_structure(blas),
            )
        };
        build_geometry.dst_acceleration_structure = blas;

        let range = vk::AccelerationStructureBuildRangeInfoKHR {
            primitive_count: 1,
            primitive_offset: 0,
            first_vertex: 0,
            transform_offset: 0,
        };

        let cmd_manager = device.command_manager(queue_family_index);
        let cmd = cmd_manager.begin()?;
        unsafe { accel.cmd_build_acceleration_structures(cmd, &[build_geometry], &[&[range]]) };
        cmd_manager.submit(queue, cmd, &[], &[])?;
        cmd_manager.wait(cmd)?;

        let instance = vk::AccelerationStructureInstanceKHR {
            transform: vk::TransformMatrixKHR {
                matrix: [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0],
            },
            instance_custom_index_and_mask: vk::Packed24_8::new(0, 0xff),
            instance_shader_binding_table_record_offset_and_flags: vk::Packed24_8::new(0, 0),
            acceleration_structure_reference: vk::AccelerationStructureReferenceKHR {
                device_handle: blas_address,
            },
        };
        let instances = [instance];

        let instance_buffer = Buffer::new(
            &device,
            size_of_val(&instances) as u64,
            vk::BufferUsageFlags::ACCELERATION_STRUCTURE_BUILD_INPUT_READ_ONLY_KHR
                | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            &[queue_family_index],
            vk::SharingMode::EXCLUSIVE,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        let instance_buffer_address = unsafe {
            device.device.get_buffer_device_address(
                &vk::BufferDeviceAddressInfo::default().buffer(instance_buffer.handle()),
            )
        };

        let instances_data = vk::AccelerationStructureGeometryInstancesDataKHR::default()
            .array_of_pointers(false)
            .data(vk::DeviceOrHostAddressConstKHR {
                device_address: instance_buffer_address,
            });

        let geometry = vk::AccelerationStructureGeometryKHR::default()
            .geometry_type(vk::GeometryTypeKHR::INSTANCES)
            .geometry(vk::AccelerationStructureGeometryDataKHR {
                instances: instances_data,
            });

        let mut build_geometry = vk::AccelerationStructureBuildGeometryInfoKHR::default()
            .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
            .mode(vk::BuildAccelerationStructureModeKHR::BUILD)
            .geometries(std::slice::from_ref(&geometry))
            .flags(vk::BuildAccelerationStructureFlagsKHR::PREFER_FAST_TRACE);

        let mut size_info = vk::AccelerationStructureBuildSizesInfoKHR::default();
        unsafe {
            accel.get_acceleration_structure_build_sizes(
                vk::AccelerationStructureBuildTypeKHR::DEVICE,
                &build_geometry,
                &[1], // 1 instance
                &mut size_info,
            )
        };

        let tlas_buffer = Buffer::new(
            &device,
            size_info.acceleration_structure_size,
            vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                | vk::BufferUsageFlags::ACCELERATION_STRUCTURE_STORAGE_KHR,
            &[queue_family_index],
            vk::SharingMode::EXCLUSIVE,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let scratch_buffer = Buffer::new(
            &device,
            size_info.build_scratch_size,
            vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
            &[queue_family_index],
            vk::SharingMode::EXCLUSIVE,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;
        let scratch_buffer_address = unsafe {
            device.device.get_buffer_device_address(
                &vk::BufferDeviceAddressInfo::default().buffer(scratch_buffer.handle()),
            )
        };
        build_geometry.scratch_data = vk::DeviceOrHostAddressKHR {
            device_address: scratch_buffer_address,
        };

        let tlas = unsafe {
            accel.create_acceleration_structure(
                &vk::AccelerationStructureCreateInfoKHR::default()
                    .ty(vk::AccelerationStructureTypeKHR::TOP_LEVEL)
                    .buffer(tlas_buffer.handle())
                    .size(size_info.acceleration_structure_size),
                device.physical_device().vulkan().allocation_callbacks(),
            )
        };

        let range = vk::AccelerationStructureBuildRangeInfoKHR {
            primitive_count: 1, // instances
            primitive_offset: 0,
            first_vertex: 0,
            transform_offset: 0,
        };

        let cmd_manager = device.command_manager(queue_family_index);
        let cmd = cmd_manager.begin()?;
        unsafe { accel.cmd_build_acceleration_structures(cmd, &[build_geometry], &[&[range]]) };
        cmd_manager.submit(queue, cmd, &[], &[])?;
        cmd_manager.wait(cmd)?;

        let extent = vk::Extent3D {
            width: 1280,
            height: 720,
            depth: 1,
        };

        let raytrace_image = Image::new(
            &device,
            &ImageCreateInfo::default()
                .image_type(vk::ImageType::TYPE_2D)
                .extent(extent)
                .format(vk::Format::B8G8R8A8_UNORM)
                .usage(
                    vk::ImageUsageFlags::STORAGE
                        | vk::ImageUsageFlags::TRANSFER_SRC
                        | vk::ImageUsageFlags::TRANSFER_DST,
                ),
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
        )?;

        let rt_pipeline = ash::khr::ray_tracing_pipeline::Device::new(
            &device.physical_device().vulkan().instance,
            &device.device,
        );

        Ok(Self {})
    }
}
