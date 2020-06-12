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

/*
use std::cell::RefCell;
use std::mem;
use std::rc::{Rc, Weak};

use ash::version::DeviceV1_0;
use ash::vk;

use super::{Device, Instance};
use crate::utils;

#[must_use = "SingleTimeCommand should be used with .submit() method"]
pub struct SingleTimeCommand {
    index: usize,
    pub command_buffer: vk::CommandBuffer,
    fence: vk::Fence,
    command_pool: Weak<RefCell<SingleUsageCommandPool>>,
}

impl SingleTimeCommand {
    fn new(
        index: usize,
        command_buffer: vk::CommandBuffer,
        fence: vk::Fence,
        command_pool: Rc<RefCell<SingleUsageCommandPool>>,
    ) -> Self {
        Self {
            index,
            command_buffer,
            fence, // temporaire
            command_pool: Rc::downgrade(&command_pool),
        }
    }

    pub fn submit(self) {
        Self::end_single_time_command(
            self.command_buffer,
            &self.command_pool.upgrade().unwrap().borrow().device,
        );
    }

    fn begin_single_time_command(&self) -> vk::CommandBuffer {
        let begin_info = vk::CommandBufferBeginInfo::builder()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.command_pool
                .upgrade()
                .unwrap()
                .borrow()
                .device
                .device
                .begin_command_buffer(self.command_buffer, &begin_info)
                .unwrap();
        }

        self.command_buffer
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

impl Drop for SingleTimeCommand {
    fn drop(&mut self) {
        unsafe {
            let command_pool = self.command_pool.upgrade().unwrap();
            let command_buffer = SingleTimeCommand::new(
                self.index,
                self.command_buffer,
                self.fence,
                Rc::clone(&command_pool),
            );

            let mut command_pool_mut = command_pool.borrow_mut();

            command_pool_mut
                .device
                .device
                .reset_command_buffer(
                    self.command_buffer,
                    vk::CommandBufferResetFlags::RELEASE_RESOURCES,
                )
                .unwrap();

            command_pool_mut.command_buffers[self.index] = (Some(command_buffer), self.fence);
        }
    }
}

pub struct SingleUsageCommandPool {
    command_pool: vk::CommandPool,
    command_buffers: Vec<(Option<SingleTimeCommand>, vk::Fence)>,
    pub(crate) device: Rc<Device>,
}

impl SingleUsageCommandPool {
    pub fn new(instance: &Instance, device: Rc<Device>) -> Rc<RefCell<Self>> {
        let command_pool = Self::create_command_pool(instance, &device);

        const BUFFERS_NB: usize = 10;

        let mut command_pool = Rc::new(RefCell::new(Self {
            command_pool,
            command_buffers: Vec::with_capacity(BUFFERS_NB),
            device,
        }));

        Self::allocate_command_buffers(&mut command_pool);

        command_pool
    }

    fn create_command_pool(instance: &Instance, device: &Device) -> vk::CommandPool {
        let queue_family_indices = utils::find_queue_families(instance, device.physical_device);

        let pool_info = vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_family_indices.compute_family.unwrap())
            .flags(
                vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
                    | vk::CommandPoolCreateFlags::TRANSIENT,
            );

        unsafe { device.device.create_command_pool(&pool_info, None) }
            .expect("failed to create command pool!")
    }

    fn allocate_command_buffers(command_pool: &mut Rc<RefCell<Self>>) {
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(command_pool.borrow().command_pool)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_buffer_count(command_pool.borrow().command_buffers.capacity() as u32);

        unsafe {
            let command_buffers = command_pool
                .borrow_mut()
                .device
                .device
                .allocate_command_buffers(&alloc_info)
                .expect("failed to allocate command buffers!");

            let mut fences = Vec::with_capacity(command_buffers.capacity());

            let fence_info = vk::FenceCreateInfo::builder();

            for _ in 0..command_buffers.capacity() {
                let fence = command_pool
                    .borrow()
                    .device
                    .device
                    .create_fence(&fence_info, None)
                    .unwrap();

                fences.push(fence);
            }

            let command_buffers = command_buffers
                .into_iter()
                .enumerate()
                .map(|(index, command_buffer)| {
                    let fence = fences[index];
                    (
                        Some(SingleTimeCommand::new(
                            index,
                            command_buffer,
                            fence,
                            Rc::clone(&command_pool),
                        )),
                        fence,
                    )
                })
                .collect();

            command_pool.borrow_mut().command_buffers = command_buffers;
        }
    }

    pub fn single_command(&mut self) -> Option<SingleTimeCommand> {
        // TODO: ajouter une fence crée dans la pool et passer en param de la commande, pour savoir si c'est bon
        // ptet que submit devrait la retourner ?
        /*if self.command_buffers.len() == 0 {
            return None;
        }
        let command_buffer = self.command_buffers.swap_remove(0);*/
        let idx = self
            .command_buffers
            .iter()
            .enumerate()
            .filter(|(_, (command_buffer, fence))| unsafe {
                if command_buffer.is_none() {
                    return false;
                }
                if *fence == vk::Fence::null() {
                    return true;
                }
                let status = self.device.device.get_fence_status(*fence);
                println!("status: {:?}", status);
                status.is_ok()
            })
            .next()?
            .0;

        self.command_buffers[idx].0.take()
    }
}

impl Drop for SingleUsageCommandPool {
    fn drop(&mut self) {
        self.command_buffers
            .iter()
            .map(|command_buffer| command_buffer.1)
            .for_each(|fence| unsafe { self.device.device.destroy_fence(fence, None) });

        let command_buffers = mem::replace(&mut self.command_buffers, Vec::new())
            .into_iter()
            .map(|command_buffer| command_buffer.0.unwrap().command_buffer)
            .collect::<Vec<_>>();

        unsafe {
            self.device
                .device
                .free_command_buffers(self.command_pool, &command_buffers)
        }
    }
}
*/
