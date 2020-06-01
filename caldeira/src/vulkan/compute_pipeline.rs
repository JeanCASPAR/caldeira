use std::ffi::CString;
use std::rc::Rc;

use ash::version::DeviceV1_0;
use ash::vk;

use super::{Descriptor, Device};
use crate::utils;

pub struct ComputePipeline {
    pub compute_pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
    _device: Rc<Device>,
}

impl ComputePipeline {
    pub fn new(descriptors: &[Descriptor], device: Rc<Device>) -> Self {
        let (compute_pipeline, pipeline_layout) =
            Self::create_compute_pipeline(descriptors, &device);

        Self {
            compute_pipeline,
            pipeline_layout,
            _device: device,
        }
    }

    fn create_pipeline_layout(descriptors: &[Descriptor], device: &Device) -> vk::PipelineLayout {
        let set_layouts = descriptors
            .into_iter()
            .map(|descriptor| descriptor.descriptor_set_layout)
            .collect::<Vec<_>>();
        let layout_info = vk::PipelineLayoutCreateInfo::builder().set_layouts(&set_layouts);
        // .push_constant_ranges(push_constant_ranges);

        unsafe { device.device.create_pipeline_layout(&layout_info, None) }
            .expect("failed to create pipeline layout!")
    }

    fn create_compute_pipeline(
        descriptors: &[Descriptor],
        device: &Device,
    ) -> (vk::Pipeline, vk::PipelineLayout) {
        let shader_code = utils::read_file("shaders/compute.spv");
        let module = utils::create_shader_module(&shader_code, device);

        let name = CString::new("main").unwrap();

        let stage = vk::PipelineShaderStageCreateInfo::builder()
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(module)
            .name(&name)
            //.specialization_info(specialization_info)
            .build();

        let layout = Self::create_pipeline_layout(descriptors, device);

        let pipeline_info = vk::ComputePipelineCreateInfo::builder()
            .stage(stage)
            .layout(layout)
            .build();

        let pipeline = unsafe {
            device.device.create_compute_pipelines(
                vk::PipelineCache::null(),
                &[pipeline_info],
                None,
            )
        }
        .expect("failed to create compute pipeline")[0];

        (pipeline, layout)
    }
}

impl Drop for ComputePipeline {
    fn drop(&mut self) {
        unsafe {
            self._device
                .device
                .destroy_pipeline(self.compute_pipeline, None);
            self._device
                .device
                .destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
