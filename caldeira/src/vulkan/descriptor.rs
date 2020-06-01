use std::marker::PhantomData;
use std::rc::Rc;

use ash::version::DeviceV1_0;
use ash::vk;

use super::Device;

pub struct Descriptor {
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    pub descriptor_sizes: Vec<vk::DescriptorPoolSize>,
    _device: Rc<Device>,
}

impl Descriptor {
    fn new(bindings: Vec<vk::DescriptorSetLayoutBinding>, device: Rc<Device>) -> Self {
        let descriptor_sizes = bindings
            .iter()
            .map(|binding| {
                vk::DescriptorPoolSize::builder()
                    .ty(binding.descriptor_type)
                    .descriptor_count(1)
                    .build()
            })
            .collect();
        let descriptor_set_layout = Self::create_descriptor_set_layout(bindings, &device);

        Self {
            descriptor_set_layout,
            descriptor_sizes,
            _device: device,
        }
    }

    fn create_descriptor_set_layout(
        bindings: Vec<vk::DescriptorSetLayoutBinding>,
        device: &Device,
    ) -> vk::DescriptorSetLayout {
        let layout_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

        unsafe {
            device
                .device
                .create_descriptor_set_layout(&layout_info, None)
        }
        .expect("failed to create descriptor set layout!")
    }
}

impl Drop for Descriptor {
    fn drop(&mut self) {
        unsafe {
            self._device
                .device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}

pub struct DescriptorBuilder<'a> {
    current_binding: u32,
    bindings: Vec<vk::DescriptorSetLayoutBinding>,
    _phantom: PhantomData<&'a ()>,
}

impl<'a> DescriptorBuilder<'a> {
    pub fn new() -> Self {
        Self {
            current_binding: 0,
            bindings: vec![],
            _phantom: PhantomData,
        }
    }

    pub fn with_binding(
        mut self,
        descriptor_type: vk::DescriptorType,
        count: u32,
        stage_flags: vk::ShaderStageFlags,
        immutable_samplers: Option<&'a [vk::Sampler]>,
    ) -> Self {
        let mut layout_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(self.current_binding)
            .descriptor_type(descriptor_type)
            .descriptor_count(count)
            .stage_flags(stage_flags);

        if let Some(immutable_samplers) = immutable_samplers {
            layout_binding = layout_binding.immutable_samplers(immutable_samplers);
        }

        self.bindings.push(layout_binding.build());

        self.current_binding += 1;
        self
    }

    pub fn build(self, device: Rc<Device>) -> Descriptor {
        Descriptor::new(self.bindings, device)
    }
}
