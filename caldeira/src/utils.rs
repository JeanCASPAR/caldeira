use std::ffi::CString;
use std::fs::File;
use std::os::raw::c_char;
use std::path::Path;

use ash::version::{DeviceV1_0, InstanceV1_0};
use ash::{util, vk};

use crate::vulkan::{Device, Instance, QueueCreateInfo, QueueFamily};

/// Free an iterator of *const c_char allocated by a CString and getted by using CString::into_raw() method
/// # Safety
/// The pointers must be unique and satisfy the conditions of CString::from_raw() method
pub unsafe fn free_cstring<I: IntoIterator<Item = *const c_char>>(iter: I) {
    for ptr in iter {
        CString::from_raw(ptr as *mut _);
    }
}

#[derive(Default)]
pub struct QueueFamilyIndices {
    pub graphics_family: Option<u32>,
    pub compute_family: Option<u32>,
    pub present_family: Option<u32>,
}

impl QueueFamilyIndices {
    pub fn is_complete(&self) -> bool {
        self.compute_family.is_some()
    }
}

pub fn find_queue_families(
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
) -> QueueFamilyIndices {
    let mut queue_family_indices = QueueFamilyIndices::default();

    let queue_families = unsafe {
        instance
            .instance
            .get_physical_device_queue_family_properties(physical_device)
    };

    for (i, queue_family) in queue_families.iter().enumerate() {
        println!(
            "family {}: queue count = {}, flags = {:?}",
            i, queue_family.queue_count, queue_family.queue_flags,
        );
        if (queue_family.queue_flags & vk::QueueFlags::COMPUTE != vk::QueueFlags::empty())
            && queue_family_indices.compute_family.is_none()
        {
            queue_family_indices.compute_family = Some(i as u32);
        }
    }

    queue_family_indices
}

pub fn find_queue_families2<
    F: FnMut(QueueFamily, &[(usize, QueueCreateInfo)]) -> Option<QueueCreateInfo>,
>(
    mut queue_finder: F,
    instance: &Instance,
    physical_device: vk::PhysicalDevice,
) -> Vec<(usize, QueueCreateInfo)> {
    let mut queue_family_infos = Vec::new();

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
    });

    for queue_family in queue_families.into_iter() {
        let index = queue_family.index();
        if let Some(queue_create_info) = queue_finder(queue_family, &queue_family_infos) {
            queue_family_infos.push((index, queue_create_info));
        }
    }

    queue_family_infos
}

pub fn read_file<P: AsRef<Path>>(path: P) -> Vec<u32> {
    let mut file = File::open(path).expect("failed to open file!");
    util::read_spv(&mut file).expect("failed to read file!")
}

pub fn create_shader_module(shader_code: &[u32], device: &Device) -> vk::ShaderModule {
    let create_info = vk::ShaderModuleCreateInfo::builder().code(shader_code);

    unsafe { device.device.create_shader_module(&create_info, None) }
        .expect("failed to create_shader_module")
}

pub fn find_memory_type(
    type_filter: u32,
    properties: vk::MemoryPropertyFlags,
    device: &Device,
    instance: &Instance,
) -> u32 {
    let mem_properties = unsafe {
        instance
            .instance
            .get_physical_device_memory_properties(device.physical_device)
    };

    for i in 0..mem_properties.memory_type_count {
        if (type_filter & (1 << i) != 0)
            && ((mem_properties.memory_types[i as usize].property_flags & properties) == properties)
        {
            return i;
        }
    }

    panic!("failed to find suitable memory type!")
}

#[allow(dead_code, unused_variables)]
pub fn image(image: vk::Image, format: vk::Format, device: &Device, instance: &Instance) {
    let format_properties = unsafe {
        instance
            .instance
            .get_physical_device_image_format_properties(
                device.physical_device,
                format,
                vk::ImageType::TYPE_2D,
                vk::ImageTiling::OPTIMAL,
                vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC,
                vk::ImageCreateFlags::empty(),
            )
    };
}
