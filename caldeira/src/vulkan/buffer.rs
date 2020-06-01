use std::rc::Rc;

use ash::version::DeviceV1_0;
use ash::vk;

use super::{Device, Instance};
use crate::utils;

pub struct Buffer {
    pub buffer: vk::Buffer,
    pub memory: vk::DeviceMemory,
    _device: Rc<Device>,
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
            _device: device,
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
}

impl Drop for Buffer {
    fn drop(&mut self) {
        unsafe {
            self._device.device.destroy_buffer(self.buffer, None);
            self._device.device.free_memory(self.memory, None);
        }
    }
}
