use std::ffi::CString;
use std::rc::Rc;

use ash::version::DeviceV1_0;
use ash::vk;

use super::{DescriptorSetLayout, Device};
use crate::utils;

pub struct ComputePipeline {
    pub pipeline: vk::Pipeline,
    pub layout: vk::PipelineLayout,
    _device: Rc<Device>,
}

impl ComputePipeline {
    pub fn new(descriptor_set_layouts: &[DescriptorSetLayout], device: Rc<Device>) -> Self {
        let (pipeline, layout) = Self::create_compute_pipeline(descriptor_set_layouts, &device);

        Self {
            pipeline,
            layout,
            _device: device,
        }
    }

    fn create_pipeline_layout(
        descriptor_set_layouts: &[DescriptorSetLayout],
        device: &Device,
    ) -> vk::PipelineLayout {
        let set_layouts = descriptor_set_layouts
            .iter()
            .map(|descriptor| descriptor.descriptor_set_layout)
            .collect::<Vec<_>>();

        let layout_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(&set_layouts);
        // .push_constant_ranges(push_constant_ranges);

        unsafe { device.device.create_pipeline_layout(&layout_info, None) }
            .expect("failed to create pipeline layout!")
    }

    fn create_compute_pipeline(
        descriptor_set_layouts: &[DescriptorSetLayout],
        device: &Device,
    ) -> (vk::Pipeline, vk::PipelineLayout) {
        let shader_code = utils::read_file("shaders/compute.spv");
        let module = utils::create_shader_module(&shader_code, device);

        let name = CString::new("main").unwrap();

        let stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(module)
            .name(&name)
            // .specialization_info(specialization_info)
            .build();

        let pipeline_layout = Self::create_pipeline_layout(descriptor_set_layouts, device);

        let pipeline_info = vk::ComputePipelineCreateInfo::builder()
            .stage(stage)
            .layout(pipeline_layout)
            .build();

        let pipeline = unsafe {
            device.device.create_compute_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_info],
                None,
            )
        }
        .expect("failed to create compute pipeline")[0];

        unsafe {
            device.device.destroy_shader_module(module, None);
        }

        (pipeline, pipeline_layout)
    }
}

impl Drop for ComputePipeline {
    fn drop(&mut self) {
        unsafe {
            self._device.device.destroy_pipeline(self.pipeline, None);
            self._device
                .device
                .destroy_pipeline_layout(self.layout, None);
        }
    }
}
