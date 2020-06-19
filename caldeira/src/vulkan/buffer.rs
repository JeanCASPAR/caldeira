use std::mem;
use std::ptr;
use std::rc::Rc;

use ash::version::DeviceV1_0;
use ash::vk;

use super::{ByteCopiable, CommandPool, Device, Image, Instance, SingleTimeCommand};
use crate::utils;

pub struct Buffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    device: Rc<Device>,
}

impl Buffer {
    pub fn new(
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
        device: Rc<Device>,
        instance: &Instance,
    ) -> Self {
        let (buffer, memory) = Self::create_buffer(size, usage, properties, &device, instance);

        Self {
            buffer,
            memory,
            device,
        }
    }

    fn create_buffer(
        size: vk::DeviceSize,
        usage: vk::BufferUsageFlags,
        properties: vk::MemoryPropertyFlags,
        device: &Device,
        instance: &Instance,
    ) -> (vk::Buffer, vk::DeviceMemory) {
        let buffer_info = vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { device.device.create_buffer(&buffer_info, None) }
            .expect("failed to allocate command buffers!");

        let mem_requirements = unsafe { device.device.get_buffer_memory_requirements(buffer) };

        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(utils::find_memory_type(
                mem_requirements.memory_type_bits,
                properties,
                device,
                instance,
            ));

        let memory = unsafe { device.device.allocate_memory(&alloc_info, None) }
            .expect("failed to allocate buffer memory!");

        unsafe {
            device.device.bind_buffer_memory(buffer, memory, 0).unwrap();
        }

        (buffer, memory)
    }

    pub fn copy_data<T: ?Sized + ByteCopiable>(&mut self, data: &T, offset: usize) {
        let size = mem::size_of_val(data);
        let src = data as *const _ as *const u8;

        unsafe {
            let ptr = self
                .device
                .device
                .map_memory(
                    self.memory,
                    offset as _,
                    size as _,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap();
            ptr::copy_nonoverlapping(src, ptr.cast(), size);
            self.device.device.unmap_memory(self.memory);
        }
    }

    pub fn get_data<T: ?Sized + ByteCopiable>(&self, data: &mut T, offset: usize) {
        let dst = data as *mut _ as *mut u8;
        let size = mem::size_of_val(data);

        unsafe {
            let src = self
                .device
                .device
                .map_memory(
                    self.memory,
                    offset as _,
                    size as _,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap();
            ptr::copy_nonoverlapping(src.cast(), dst, size);
            self.device.device.unmap_memory(self.memory);
        }
    }

    pub fn copy_to_buffer(&self, dst: &mut Self, size: vk::DeviceSize, command_pool: &CommandPool) {
        let command_buffer = SingleTimeCommand::new(&self.device, command_pool);

        let buffer_copy = vk::BufferCopy::builder().size(size);
        let regions = [buffer_copy.build()];

        unsafe {
            self.device.device.cmd_copy_buffer(
                command_buffer.command_buffer,
                self.buffer,
                dst.buffer,
                &regions,
            );
        }

        command_buffer.submit();
    }

    pub fn copy_to_image(&self, dst: &mut Image, command_pool: &CommandPool) {
        let command_buffer = SingleTimeCommand::new(&self.device, command_pool);

        let image_subresource = vk::ImageSubresourceLayers::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .mip_level(0)
            .base_array_layer(0)
            .layer_count(1)
            .build();

        let image_offset = vk::Offset3D::builder().x(0).y(0).z(0).build();

        let region = vk::BufferImageCopy::builder()
            .buffer_offset(0)
            .buffer_row_length(0)
            .buffer_image_height(0)
            .image_subresource(image_subresource)
            .image_offset(image_offset)
            .image_extent(dst.extent)
            .build();
        let regions = [region];

        if dst.layout != vk::ImageLayout::TRANSFER_DST_OPTIMAL {
            dst.transition_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL, command_pool);
        }

        unsafe {
            self.device.device.cmd_copy_buffer_to_image(
                command_buffer.command_buffer,
                self.buffer,
                dst.image,
                vk::ImageLayout::TRANSFER_DST_OPTIMAL,
                &regions,
            );
        }

        command_buffer.submit();
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_buffer(self.buffer, None);
            self.device.device.free_memory(self.memory, None);
        }
    }
}
