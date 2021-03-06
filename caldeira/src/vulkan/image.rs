use std::rc::Rc;

use ash::version::DeviceV1_0;
use ash::vk;

use image::RgbaImage;

use super::{Buffer, Device, Instance};
use crate::utils;

pub struct Image {
    pub handle: vk::Image,
    pub memory: vk::DeviceMemory,
    pub extent: vk::Extent3D,
    pub layout: vk::ImageLayout,
    pub view: vk::ImageView,
    device: Rc<Device>,
}

impl Image {
    pub fn new(
        width: u32,
        height: u32,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        aspect_flags: vk::ImageAspectFlags,
        properties: vk::MemoryPropertyFlags,
        device: Rc<Device>,
        instance: &Instance,
    ) -> Self {
        let (handle, memory, extent) = Self::create_image(
            width, height, format, tiling, usage, properties, &device, instance,
        );
        let view = Self::create_image_view(handle, format, aspect_flags, &device);
        let layout = vk::ImageLayout::UNDEFINED;

        Self {
            handle,
            memory,
            extent,
            layout,
            view,
            device,
        }
    }

    pub fn new_texture(image: RgbaImage, device: Rc<Device>, instance: &Instance) -> Self {
        let (width, height) = image.dimensions();
        let size = width * height * 4;
        let pixels = image.into_raw();

        let texture_image = Self::new(
            width,
            height,
            vk::Format::R8G8B8A8_SRGB,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
            vk::ImageAspectFlags::COLOR,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            Rc::clone(&device),
            instance,
        );

        let mut staging_buffer = Buffer::new(
            size as _,
            vk::BufferUsageFlags::TRANSFER_SRC,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            Rc::clone(&device),
            instance,
        );
        staging_buffer.copy_data(&pixels[..], 0);

        texture_image
    }

    pub fn new_storage(width: u32, height: u32, device: Rc<Device>, instance: &Instance) -> Self {
        Self::new(
            width,
            height,
            vk::Format::R8G8B8A8_UINT,
            vk::ImageTiling::OPTIMAL,
            vk::ImageUsageFlags::STORAGE | vk::ImageUsageFlags::TRANSFER_SRC,
            vk::ImageAspectFlags::COLOR,
            vk::MemoryPropertyFlags::DEVICE_LOCAL,
            device,
            instance,
        )
    }

    pub fn new_staging(
        width: u32,
        height: u32,
        format: vk::Format,
        device: Rc<Device>,
        instance: &Instance,
    ) -> Self {
        let (handle, memory, extent) = Self::create_image(
            width,
            height,
            format,
            vk::ImageTiling::LINEAR,
            vk::ImageUsageFlags::TRANSFER_SRC | vk::ImageUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
            &device,
            instance,
        );
        let layout = vk::ImageLayout::UNDEFINED;
        let view = vk::ImageView::null();

        Self {
            handle,
            memory,
            extent,
            layout,
            view,
            device,
        }
    }

    fn create_image(
        width: u32,
        height: u32,
        format: vk::Format,
        tiling: vk::ImageTiling,
        usage: vk::ImageUsageFlags,
        properties: vk::MemoryPropertyFlags,
        device: &Device,
        instance: &Instance,
    ) -> (vk::Image, vk::DeviceMemory, vk::Extent3D) {
        let extent = vk::Extent3D::builder()
            .width(width)
            .height(height)
            .depth(1)
            .build();

        let image_info = vk::ImageCreateInfo::builder()
            .image_type(vk::ImageType::TYPE_2D)
            .extent(extent)
            .mip_levels(1)
            .array_layers(1)
            .format(format)
            .tiling(tiling)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .usage(usage)
            .samples(vk::SampleCountFlags::TYPE_1)
            .sharing_mode(vk::SharingMode::EXCLUSIVE);

        let image = unsafe { device.device.create_image(&image_info, None) }
            .expect("failed to create image!");

        let mem_requirements = unsafe { device.device.get_image_memory_requirements(image) };

        let alloc_info = vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(utils::find_memory_type(
                mem_requirements.memory_type_bits,
                properties,
                device,
                instance,
            ));

        let memory = unsafe { device.device.allocate_memory(&alloc_info, None) }
            .expect("failed to allocate image memory!");

        unsafe {
            device.device.bind_image_memory(image, memory, 0).unwrap();
        }

        (image, memory, extent)
    }

    /// Return all src_stage_mask, dst_stage_mask, depency_flags and the image memory barrier
    /// This functions set the new layout, and therefore the transition is considered done
    pub fn transition_layout(
        &mut self,
        new_layout: vk::ImageLayout,
    ) -> (
        vk::PipelineStageFlags,
        vk::PipelineStageFlags,
        vk::DependencyFlags,
        vk::ImageMemoryBarrierBuilder<'_>,
    ) {
        if self.layout == new_layout {
            return (
                vk::PipelineStageFlags::empty(),
                vk::PipelineStageFlags::empty(),
                vk::DependencyFlags::empty(),
                vk::ImageMemoryBarrier::builder(),
            );
        }

        let subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1)
            .build();

        let (src_access_mask, src_stage_mask) = match self.layout {
            vk::ImageLayout::UNDEFINED => (
                vk::AccessFlags::empty(),
                vk::PipelineStageFlags::TOP_OF_PIPE,
            ),
            vk::ImageLayout::TRANSFER_DST_OPTIMAL => (
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TRANSFER,
            ),
            vk::ImageLayout::GENERAL => {
                (vk::AccessFlags::all(), vk::PipelineStageFlags::ALL_COMMANDS)
            }

            _ => panic!("Unsupported layout transition"),
        };

        let (dst_access_mask, dst_stage_mask) = match new_layout {
            vk::ImageLayout::TRANSFER_DST_OPTIMAL => (
                vk::AccessFlags::TRANSFER_WRITE,
                vk::PipelineStageFlags::TRANSFER,
            ),
            vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL => (
                vk::AccessFlags::SHADER_READ,
                vk::PipelineStageFlags::FRAGMENT_SHADER,
            ),
            vk::ImageLayout::GENERAL => {
                (vk::AccessFlags::all(), vk::PipelineStageFlags::ALL_COMMANDS)
            }
            vk::ImageLayout::TRANSFER_SRC_OPTIMAL => (
                vk::AccessFlags::TRANSFER_READ,
                vk::PipelineStageFlags::TRANSFER,
            ),

            _ => panic!("Unsupported layout transition"),
        };

        let barrier = vk::ImageMemoryBarrier::builder()
            .old_layout(self.layout)
            .new_layout(new_layout)
            .src_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .dst_queue_family_index(vk::QUEUE_FAMILY_IGNORED)
            .image(self.handle)
            .subresource_range(subresource_range)
            .src_access_mask(src_access_mask)
            .dst_access_mask(dst_access_mask);

        // unsafe {
        //     let memory_barriers = [];
        //     let buffer_memory_barriers = [];
        //     let image_memory_barriers = [barrier.build()];

        //     self.device.device.cmd_pipeline_barrier(
        //         command_buffer.command_buffer,
        //         src_stage_mask,
        //         dst_stage_mask,
        //         vk::DependencyFlags::empty(),
        //         &memory_barriers,
        //         &buffer_memory_barriers,
        //         &image_memory_barriers,
        //     );
        // }

        self.layout = new_layout;

        (
            src_stage_mask,
            dst_stage_mask,
            vk::DependencyFlags::empty(),
            barrier,
        )
    }

    fn create_image_view(
        image: vk::Image,
        format: vk::Format,
        aspect_flags: vk::ImageAspectFlags,
        device: &Device,
    ) -> vk::ImageView {
        let subresource_range = vk::ImageSubresourceRange::builder()
            .aspect_mask(aspect_flags)
            .base_mip_level(0)
            .level_count(1)
            .base_array_layer(0)
            .layer_count(1)
            .build();

        let view_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(format)
            .subresource_range(subresource_range);

        unsafe { device.device.create_image_view(&view_info, None) }
            .expect("failed to create texture image view!")
    }
}

impl Drop for Image {
    fn drop(&mut self) {
        unsafe {
            self.device.device.destroy_image_view(self.view, None);
            self.device.device.destroy_image(self.handle, None);
            self.device.device.free_memory(self.memory, None);
        }
    }
}
