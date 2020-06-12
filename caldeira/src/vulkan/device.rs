use std::collections::HashMap;
use std::rc::Rc;

use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;

use super::Instance;
use crate::utils;

pub struct Device {
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    pub compute_queue: vk::Queue,
    _instance: Rc<Instance>,
}

impl Device {
    pub fn new(instance: Rc<Instance>) -> Device {
        let devices = unsafe {
            instance
                .instance
                .enumerate_physical_devices()
                .expect("failed to enumerate physical devices")
        };

        let physical_device = Self::pick_physical_device(&instance, &devices);

        let (device, compute_queue) =
            Self::create_logical_device_and_queues(&instance, physical_device);

        Self {
            physical_device,
            device,
            compute_queue,
            _instance: instance,
        }
    }

    fn pick_physical_device(
        instance: &Instance,
        physical_devices: &[vk::PhysicalDevice],
    ) -> vk::PhysicalDevice {
        let mut candidates = HashMap::new();

        for device in physical_devices {
            let score = Self::rate_device_suitability(instance, *device);
            if score > 0 {
                candidates.insert(score, device);
            }
        }

        let (_, device) = candidates
            .into_iter()
            .max_by_key(|(score, _)| *score)
            .expect("failed to find a suitable GPU!");
        *device
    }

    fn rate_device_suitability(instance: &Instance, physical_device: vk::PhysicalDevice) -> u32 {
        let indices = utils::find_queue_families(instance, physical_device);

        if !indices.is_complete() {
            return 0;
        }

        let properties = unsafe {
            instance
                .instance
                .get_physical_device_properties(physical_device)
        };

        let features = unsafe {
            instance
                .instance
                .get_physical_device_features(physical_device)
        };

        let mut score = 0;

        if properties.device_type == vk::PhysicalDeviceType::DISCRETE_GPU {
            score += 1000;
        }

        score += properties.limits.max_image_dimension2_d;

        if features.geometry_shader == 0 {
            return 0;
        }

        if features.shader_storage_image_write_without_format == 0 {
            return 0;
        }

        score
    }

    fn create_logical_device_and_queues(
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
    ) -> (ash::Device, vk::Queue) {
        let indices = utils::find_queue_families(instance, physical_device);

        let priorities = [1.0];

        let queue_create_info = vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(indices.compute_family.unwrap())
            .queue_priorities(&priorities);
        let queue_create_infos = [queue_create_info.build()];

        let device_features = vk::PhysicalDeviceFeatures::builder(); //.shader_storage_image_write_without_format(true);

        let create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_features(&device_features);

        let device = unsafe {
            instance
                .instance
                .create_device(physical_device, &create_info, None)
        }
        .expect("failed to create logical device!");

        let compute_queue = unsafe { device.get_device_queue(indices.compute_family.unwrap(), 0) };

        (device, compute_queue)
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            self.device.device_wait_idle().unwrap();
            self.device.destroy_device(None);
        }
    }
}
