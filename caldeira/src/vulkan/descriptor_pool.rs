use std::rc::Rc;
use std::slice;

use ash::version::DeviceV1_0;
use ash::vk;

use super::{Descriptor, Device};

pub struct DescriptorPool {
    descriptor_pool: vk::DescriptorPool,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    _device: Rc<Device>,
}

impl DescriptorPool {
    pub fn new(descriptor: &Descriptor, device: Rc<Device>) -> Self {
        let descriptor_pool = Self::create_descriptor_pool(descriptor, &device);
        let descriptor_sets =
            Self::create_descriptor_sets(descriptor_pool, slice::from_ref(descriptor), &device);

        Self {
            descriptor_pool,
            descriptor_sets,
            _device: device,
        }
    }

    fn create_descriptor_pool(descriptor: &Descriptor, device: &Device) -> vk::DescriptorPool {
        let pool_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&descriptor.descriptor_sizes)
            .max_sets(1);

        unsafe { device.device.create_descriptor_pool(&pool_info, None) }
            .expect("failed to create descriptor pool!")
    }

    fn create_descriptor_sets(
        descriptor_pool: vk::DescriptorPool,
        layouts: &[Descriptor],
        device: &Device,
    ) -> Vec<vk::DescriptorSet> {
        let descriptor_set_layouts = layouts
            .iter()
            .map(|descriptor| descriptor.descriptor_set_layout)
            .collect::<Vec<_>>();

        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool)
            .set_layouts(&descriptor_set_layouts);

        unsafe { device.device.allocate_descriptor_sets(&alloc_info) }
            .expect("failed to allocate descriptor sets")
    }
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe {
            self._device
                .device
                .destroy_descriptor_pool(self.descriptor_pool, None);
        }
    }
}
