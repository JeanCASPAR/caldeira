use std::num::NonZeroU32;
use std::rc::Rc;

use ash::version::DeviceV1_0;
use ash::vk;

use super::Device;
pub struct DescriptorSetLayoutBuilder<'a> {
    layout_bindings: Vec<vk::DescriptorSetLayoutBindingBuilder<'a>>,
}

impl<'a> DescriptorSetLayoutBuilder<'a> {
    pub fn new() -> Self {
        Self {
            layout_bindings: vec![],
        }
    }

    pub fn with_binding(
        mut self,
        descriptor_type: vk::DescriptorType,
        descriptor_count: NonZeroU32,
        stage_flags: vk::ShaderStageFlags,
        immutable_samplers: Option<&'a [vk::Sampler]>,
    ) -> Self {
        let mut layout_binding = vk::DescriptorSetLayoutBinding::builder()
            .binding(self.layout_bindings.len() as _)
            .descriptor_type(descriptor_type)
            .descriptor_count(descriptor_count.get())
            .stage_flags(stage_flags);
        if let Some(immutable_samplers) = immutable_samplers {
            layout_binding = layout_binding.immutable_samplers(immutable_samplers);
        }
        self.layout_bindings.push(layout_binding);
        self
    }

    pub fn build(self, device: Rc<Device>) -> DescriptorSetLayout {
        let bindings = self
            .layout_bindings
            .into_iter()
            .map(|binding| binding.build())
            .collect::<Vec<_>>();

        let layout_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

        let descriptor_set_layout = unsafe {
            device
                .device
                .create_descriptor_set_layout(&layout_info, None)
        }
        .expect("failed to create descriptor set layout!");

        DescriptorSetLayout {
            descriptor_set_layout,
            device,
        }
    }
}

impl<'a> Default for DescriptorSetLayoutBuilder<'a> {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DescriptorSetLayout {
    pub descriptor_set_layout: vk::DescriptorSetLayout,
    device: Rc<Device>,
}

impl DescriptorSetLayout {
    pub fn allocate_descriptor_sets(
        &self,
        descriptor_set_count: u32,
        descriptor_pool: &DescriptorPool,
    ) -> Vec<vk::DescriptorSet> {
        let layouts = vec![self.descriptor_set_layout; descriptor_set_count as _];

        let alloc_info = vk::DescriptorSetAllocateInfo::builder()
            .descriptor_pool(descriptor_pool.descriptor_pool)
            .set_layouts(&layouts);

        unsafe { self.device.device.allocate_descriptor_sets(&alloc_info) }
            .expect("failed to allocate descriptor sets!")
    }
}

impl Drop for DescriptorSetLayout {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        }
    }
}

pub struct DescriptorPoolBuilder {
    pool_sizes: Vec<vk::DescriptorPoolSize>,
}

impl DescriptorPoolBuilder {
    pub fn new() -> Self {
        Self { pool_sizes: vec![] }
    }

    pub fn with(mut self, descriptor_type: vk::DescriptorType, descriptor_count: u32) -> Self {
        let pool_size = vk::DescriptorPoolSize::builder()
            .ty(descriptor_type)
            .descriptor_count(descriptor_count)
            .build();

        self.pool_sizes.push(pool_size);
        self
    }

    pub fn build(self, max_sets: u32, device: Rc<Device>) -> DescriptorPool {
        let pool_info = vk::DescriptorPoolCreateInfo::builder()
            .pool_sizes(&self.pool_sizes)
            .max_sets(max_sets);

        let descriptor_pool = unsafe { device.device.create_descriptor_pool(&pool_info, None) }
            .expect("failed to create descriptor pool");

        DescriptorPool {
            descriptor_pool,
            device,
        }
    }
}

impl Default for DescriptorPoolBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct DescriptorPool {
    pub descriptor_pool: vk::DescriptorPool,
    device: Rc<Device>,
}

impl Drop for DescriptorPool {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device
                .destroy_descriptor_pool(self.descriptor_pool, None)
        };
    }
}
