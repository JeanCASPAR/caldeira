use std::rc::Rc;

use ash::version::DeviceV1_0;
use ash::vk;

use super::{Device, QueueSubmission};

pub struct QueueCreateInfo {
    priorities: Vec<f32>,
}

impl QueueCreateInfo {
    pub fn new(priorities: Vec<f32>) -> Self {
        Self { priorities }
    }

    pub const fn priorities(&self) -> &Vec<f32> {
        &self.priorities
    }
}

pub struct Queue {
    pub(super) handle: vk::Queue,
    pub(super) queue_index: usize,
    pub(super) queue_family_index: usize,
    pub(super) device: Rc<Device>,
}

impl Queue {
    pub fn queue_family_index(&self) -> usize {
        self.queue_family_index
    }

    pub fn family(&self) -> &QueueFamily {
        &self.device.get_queue_families(self.queue_family_index)
    }

    pub fn wait_idle(&mut self) {
        unsafe { self.device.device.queue_wait_idle(self.handle) }.unwrap()
    }

    pub fn submit(&mut self, submits: &[QueueSubmission<'_>], fence: Option<vk::Fence>) {
        let mut submit_info_builders = Vec::with_capacity(submits.len());
        let fence = fence.unwrap_or_default();

        for submit in submits {
            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(&submit.wait_semaphores())
                .wait_dst_stage_mask(&submit.wait_dst_stage_masks())
                .command_buffers(&submit.command_buffers())
                .signal_semaphores(&submit.signal_semaphores());

            submit_info_builders.push(submit_info);
        }

        // We build after to ensure that lifetimes are still valid out of the scope of the loop
        let submit_infos = submit_info_builders
            .into_iter()
            .map(vk::SubmitInfoBuilder::build)
            .collect::<Vec<_>>();

        unsafe {
            self.device
                .device
                .queue_submit(self.handle, &submit_infos, fence)
                .expect("failed to submit queue")
        }
    }
}

#[derive(Clone, Copy)]
pub struct QueueFamily {
    pub(crate) property: vk::QueueFamilyProperties,
    pub(crate) index: usize,
    pub(crate) physical_device: vk::PhysicalDevice,
}

impl QueueFamily {
    pub fn support_graphics(&self) -> bool {
        self.property.queue_flags.contains(vk::QueueFlags::GRAPHICS)
    }

    pub fn support_compute(&self) -> bool {
        self.property.queue_flags.contains(vk::QueueFlags::COMPUTE)
    }

    /// Indicate whether this family supports transfer operation separately of graphics or compute operation,
    /// since queues which supports either graphics or compute operation implicitly supports transfer operations
    /// I couldn't find a better name but this family could support either sparse binding, protected or both operation kinds
    /// # Warning
    /// This is incompatible with graphics or compute families
    pub fn support_transfer_only(&self) -> bool {
        self.property.queue_flags.contains(vk::QueueFlags::TRANSFER)
            && !self
                .property
                .queue_flags
                .intersects(vk::QueueFlags::GRAPHICS | vk::QueueFlags::COMPUTE)
    }

    pub fn support_sparse_binding(&self) -> bool {
        self.property
            .queue_flags
            .contains(vk::QueueFlags::SPARSE_BINDING)
    }

    pub fn support_protected(&self) -> bool {
        self.property
            .queue_flags
            .contains(vk::QueueFlags::PROTECTED)
    }

    pub const fn queue_count(&self) -> usize {
        self.property.queue_count as _
    }

    pub const fn timestamp_valid_bits(&self) -> u32 {
        self.property.timestamp_valid_bits
    }

    pub const fn min_image_transfer_granularity(&self) -> vk::Extent3D {
        self.property.min_image_transfer_granularity
    }

    pub const fn index(&self) -> usize {
        self.index
    }
}
