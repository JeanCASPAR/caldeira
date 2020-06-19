use std::rc::Rc;

use ash::version::DeviceV1_0;
use ash::vk;

use super::{Device, Instance};
use crate::utils;

pub struct CommandPool {
    command_pool: vk::CommandPool,
    device: Rc<Device>,
}

impl CommandPool {
    pub fn new(instance: &Instance, device: Rc<Device>) -> Self {
        let command_pool = Self::create_command_pool(instance, &device);

        Self {
            command_pool,
            device,
        }
    }

    fn create_command_pool(instance: &Instance, device: &Device) -> vk::CommandPool {
        let queue_family_indices = utils::find_queue_families(instance, device.physical_device);

        let pool_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family_indices.compute_family.unwrap());

        unsafe { device.device.create_command_pool(&pool_info, None) }
            .expect("failed to create command pool!")
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device
                .destroy_command_pool(self.command_pool, None);
        }
    }
}

#[must_use = "SingleTimeCommand should be used with .submit() method"]
pub struct SingleTimeCommand<'a> {
    pub command_buffer: vk::CommandBuffer,
    command_pool: &'a CommandPool,
    device: &'a Device,
}

impl<'a> SingleTimeCommand<'a> {
    pub fn new(device: &'a Device, command_pool: &'a CommandPool) -> Self {
        let command_buffer = Self::begin_single_time_command(&device, command_pool.command_pool);

        Self {
            command_buffer,
            command_pool,
            device,
        }
    }

    pub fn submit(self) {
        Self::end_single_time_command(self.command_buffer, &self.device);
    }

    fn begin_single_time_command(
        device: &Device,
        command_pool: vk::CommandPool,
    ) -> vk::CommandBuffer {
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(command_pool)
            .command_buffer_count(1);

        let command_buffer =
            unsafe { device.device.allocate_command_buffers(&alloc_info) }.unwrap()[0];

        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            device
                .device
                .begin_command_buffer(command_buffer, &begin_info)
                .unwrap();
        }

        command_buffer
    }

    fn end_single_time_command(command_buffer: vk::CommandBuffer, device: &Device) {
        unsafe {
            device.device.end_command_buffer(command_buffer).unwrap();
        }
        let command_buffers = [command_buffer];

        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&command_buffers)
            .build();
        let submits = [submit_info];

        unsafe {
            device
                .device
                .queue_submit(device.compute_queue, &submits, vk::Fence::null())
                .unwrap();
            device.device.queue_wait_idle(device.compute_queue).unwrap();
        }
    }
}

impl Drop for SingleTimeCommand<'_> {
    fn drop(&mut self) {
        unsafe {
            let command_buffers = [self.command_buffer];

            self.device
                .device
                .free_command_buffers(self.command_pool.command_pool, &command_buffers);
        }
    }
}
