use std::collections::HashMap;
use std::rc::Rc;
use std::slice::SliceIndex;

use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::vk;

use super::{Instance, Queue, QueueCreateInfo, QueueFamily};
use crate::utils;

pub struct Device {
    pub physical_device: vk::PhysicalDevice,
    pub device: ash::Device,
    queue_families: Vec<QueueFamily>,
    instance: Rc<Instance>,
}

impl Device {
    pub fn new<F: FnMut(QueueFamily, &[(usize, QueueCreateInfo)]) -> Option<QueueCreateInfo>>(
        queue_finder: F,
        instance: Rc<Instance>,
    ) -> (Rc<Device>, Vec<Vec<Queue>>) {
        let devices = unsafe {
            instance
                .instance
                .enumerate_physical_devices()
                .expect("failed to enumerate physical devices")
        };

        let physical_device = Self::pick_physical_device(&instance, &devices);

        let (device, queue_datas) =
            Self::create_device_and_query_queue_datas(queue_finder, &instance, physical_device);

        let queue_families = unsafe {
            instance
                .instance
                .get_physical_device_queue_family_properties(physical_device)
        }
        .into_iter()
        .enumerate()
        .map(|(index, property)| QueueFamily {
            index,
            property,
            physical_device,
        })
        .collect();

        let device = Rc::new(Self {
            physical_device,
            device,
            queue_families,
            instance,
        });

        let mut queue_groups = Vec::with_capacity(queue_datas.len());

        for (queue_family_index, queue_create_info) in queue_datas.into_iter() {
            let len = queue_create_info.priorities().len();
            let mut queues = Vec::with_capacity(len);

            for queue_index in 0..len {
                let handle = unsafe {
                    device
                        .device
                        .get_device_queue(queue_family_index as _, queue_index as _)
                };

                let queue = Queue {
                    handle,
                    queue_family_index,
                    queue_index,
                    device: Rc::clone(&device),
                };

                queues.push(queue);
            }

            queue_groups.push(queues);
        }

        (device, queue_groups)
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

        let device_features = vk::PhysicalDeviceFeatures::builder();

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

    pub fn get_queue_families<'a, I>(
        self: &'a Rc<Self>,
        index: I,
    ) -> &'a <I as SliceIndex<[QueueFamily]>>::Output
    where
        I: SliceIndex<[QueueFamily]>,
    {
        &self.queue_families[index]
    }

    fn create_device_and_query_queue_datas<
        F: FnMut(QueueFamily, &[(usize, QueueCreateInfo)]) -> Option<QueueCreateInfo>,
    >(
        queue_finder: F,
        instance: &Instance,
        physical_device: vk::PhysicalDevice,
    ) -> (ash::Device, Vec<(usize, QueueCreateInfo)>) {
        let queue_create_infos =
            utils::find_queue_families2(queue_finder, instance, physical_device);

        let vk_create_infos_builder = queue_create_infos.iter().map(|(idx, queue_create_info)| {
            vk::DeviceQueueCreateInfo::builder()
                .queue_family_index(*idx as _)
                .queue_priorities(queue_create_info.priorities())
        });

        let vk_create_infos: Vec<_> = vk_create_infos_builder
            .map(|builder| builder.build())
            .collect();

        let device_features = vk::PhysicalDeviceFeatures::builder();

        let create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&vk_create_infos)
            .enabled_features(&device_features);

        let device = unsafe {
            instance
                .instance
                .create_device(physical_device, &create_info, None)
        }
        .expect("failed to create logical device!");

        (device, queue_create_infos)
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
