use std::error::Error;
use std::fmt;
use std::marker::PhantomData;
use std::mem;
use std::ops::Range;
use std::rc::Rc;
use std::slice;

use ash::version::DeviceV1_0;
use ash::vk;

use super::{Buffer, ByteCopiable, ComputePipeline, Device, Image, QueueFamily};

pub struct CommandPool {
    command_pool: vk::CommandPool,
    queue_family_index: usize,
    device: Rc<Device>,
}

impl CommandPool {
    pub fn new(queue_family: &QueueFamily, device: Rc<Device>) -> Self {
        let command_pool = {
            let pool_info =
                vk::CommandPoolCreateInfo::builder().queue_family_index(queue_family.index() as _);

            unsafe { device.device.create_command_pool(&pool_info, None) }
                .expect("failed to create command pool")
        };

        Self {
            command_pool,
            queue_family_index: queue_family.index(),
            device,
        }
    }

    pub fn allocate_command_buffers(
        self: &mut Rc<Self>,
        level: vk::CommandBufferLevel,
        count: usize,
    ) -> Vec<CommandBuffer> {
        let alloc_info = vk::CommandBufferAllocateInfo::builder()
            .command_pool(self.command_pool)
            .level(level)
            .command_buffer_count(count as _);

        unsafe { self.device.device.allocate_command_buffers(&alloc_info) }
            .expect("failed to allocate command buffers")
            .into_iter()
            .map(|command_buffer| CommandBuffer {
                handle: command_buffer,
                level,
                state: CommandBufferState::Initial,
                usage: vk::CommandBufferUsageFlags::empty(),
                command_pool: Rc::clone(&self),
                device: Rc::clone(&self.device),
            })
            .collect()
    }

    pub fn support_graphics(&self) -> bool {
        self.device
            .get_queue_families(self.queue_family_index)
            .support_graphics()
    }

    pub fn support_compute(&self) -> bool {
        self.device
            .get_queue_families(self.queue_family_index)
            .support_compute()
    }

    /// Indicate whether this family supports transfer operation separately of graphics or compute operation,
    /// since queues which supports either graphics or compute operation implicitly supports transfer operations
    /// I couldn't find a better name but this family could support either sparse binding, protected or both operation kinds
    /// # Warning
    /// This is incompatible with graphics or compute families
    pub fn support_transfer_only(&self) -> bool {
        self.device
            .get_queue_families(self.queue_family_index)
            .support_transfer_only()
    }

    pub fn support_sparse_binding(&self) -> bool {
        self.device
            .get_queue_families(self.queue_family_index)
            .support_sparse_binding()
    }

    pub fn support_protected(&self) -> bool {
        self.device
            .get_queue_families(self.queue_family_index)
            .support_protected()
    }
}

impl Drop for CommandPool {
    fn drop(&mut self) {
        unsafe {
            self.device
                .device
                .destroy_command_pool(self.command_pool, None)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CommandBufferState {
    Initial,
    Recording,
    Executable,
    Pending,
    Invalid,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct UnsupportedOperation;

impl fmt::Display for UnsupportedOperation {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "An unsupported operation occured!")
    }
}

impl Error for UnsupportedOperation {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DrawError {
    Draw,
    Indexed,
    Indirect,
}

impl fmt::Display for DrawError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "A draw command failed to be registered: {:?}!", self)
    }
}

impl Error for DrawError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum DispatchError {
    Dispatch,
    Indirect,
}

impl fmt::Display for DispatchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "A dispatch command failed to be registered: {:?}!", self)
    }
}

impl Error for DispatchError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum CopyError {
    RegionsOverlapped,
}

impl fmt::Display for CopyError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "A copy command failed to be registered: {:?}!", self)
    }
}

impl Error for CopyError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ClearError {}

impl fmt::Display for ClearError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "A clear command failed to be registered: {:?}!", self)
    }
}

impl Error for ClearError {}

pub enum Subpass {
    Inline {
        callback: Box<
            dyn FnOnce(
                &mut InsideOfRenderpassScope<'_, '_>,
            ) -> Result<(), Box<dyn Error + Send + Sync>>,
        >,
    },
}

impl Subpass {
    fn contents(&self) -> vk::SubpassContents {
        match self {
            Self::Inline { .. } => vk::SubpassContents::INLINE,
        }
    }
}

pub struct CommandBuffer {
    pub(crate) handle: vk::CommandBuffer,
    level: vk::CommandBufferLevel,
    state: CommandBufferState,
    usage: vk::CommandBufferUsageFlags,
    command_pool: Rc<CommandPool>,
    device: Rc<Device>,
}

impl CommandBuffer {
    pub fn begin(mut self, usage: vk::CommandBufferUsageFlags) -> CommandBufferRecorder<'static> {
        self.state = CommandBufferState::Recording;

        let begin_info = vk::CommandBufferBeginInfo::builder().flags(usage);

        unsafe {
            self.device
                .device
                .begin_command_buffer(self.handle, &begin_info)
        }
        .expect("failed to begin command buffer!");

        CommandBufferRecorder {
            inner: self,
            generic_bindings: GenericBindings::default(),
            graphics_bindings: GraphicsBindings::default(),
            compute_bindings: ComputeBindings::default(),
            phantom: std::marker::PhantomData,
        }
    }
}

#[derive(Default)]
pub struct GenericBindings {}

#[derive(Default)]
pub struct GraphicsBindings<'a> {
    graphics_pipeline: Option<&'a GraphicsPipeline>,
    index_buffer: bool,
    vertex_buffers: bool,
    descriptors: bool,
}

#[derive(Default)]
pub struct ComputeBindings<'a> {
    compute_pipeline: Option<&'a ComputePipeline>,
    descriptors: bool,
}

// TODO: ajouter vérification des pipelines (struct intermédiaire avant la renderpass pour graphics)
pub struct CommandBufferRecorder<'a> {
    inner: CommandBuffer,
    generic_bindings: GenericBindings,
    graphics_bindings: GraphicsBindings<'a>,
    compute_bindings: ComputeBindings<'a>,
    phantom: std::marker::PhantomData<&'a ()>,
}

impl<'a> CommandBufferRecorder<'a> {
    pub fn as_graphics_command_buffer(
        &mut self,
    ) -> Result<GraphicsCommandBuffer<'_, 'a>, UnsupportedOperation> {
        if self.inner.command_pool.support_graphics() {
            Ok(GraphicsCommandBuffer(self))
        } else {
            Err(UnsupportedOperation)
        }
    }

    pub fn as_compute_command_buffer(
        &mut self,
    ) -> Result<DispatchCommands<'_, 'a>, UnsupportedOperation> {
        if self.inner.command_pool.support_compute() {
            Ok(DispatchCommands(self))
        } else {
            Err(UnsupportedOperation)
        }
    }

    pub fn as_transfer_command_buffer(
        &mut self,
    ) -> Result<TransferCommandBuffer<'_, 'a>, UnsupportedOperation> {
        if self.inner.command_pool.support_compute()
            || self.inner.command_pool.support_graphics()
            || self.inner.command_pool.support_transfer_only()
        {
            Ok(TransferCommandBuffer(self))
        } else {
            Err(UnsupportedOperation)
        }
    }

    pub fn as_generic(&mut self) -> GenericCommands<'_, 'a> {
        GenericCommands(self)
    }

    pub fn end(mut self) -> ExecutableCommandBuffer {
        self.inner.state = CommandBufferState::Executable;

        unsafe {
            self.inner
                .device
                .device
                .end_command_buffer(self.inner.handle)
        }
        .expect("failed to end command buffer");

        ExecutableCommandBuffer(self.inner)
    }
}

pub struct TransferCommandBuffer<'a, 'b: 'a>(&'a mut CommandBufferRecorder<'b>);

impl<'a, 'b: 'a> TransferCommandBuffer<'a, 'b> {
    pub fn as_copy(&mut self) -> CopyCommands<'_, 'b> {
        CopyCommands(self.0)
    }
}

pub struct GraphicsCommandBuffer<'a, 'b: 'a>(&'a mut CommandBufferRecorder<'b>);

impl<'a, 'b: 'a> GraphicsCommandBuffer<'a, 'b> {
    pub fn renderpass(
        &mut self,
        begin_info: &vk::RenderPassBeginInfo,
        mut subpasses: Vec<Subpass>,
    ) -> Result<&mut Self, Box<dyn Error + Send + Sync>> {
        if subpasses.is_empty() {
            return Ok(self);
        }

        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_begin_render_pass(
                command_buffer.handle,
                begin_info,
                subpasses[0].contents(),
            )
        }

        let mut inside = InsideOfRenderpassScope(self.0);

        match subpasses.remove(0) {
            Subpass::Inline { callback } => callback(&mut inside)?,
        }

        for subpass in subpasses {
            let command_buffer = &self.0.inner;

            unsafe {
                command_buffer
                    .device
                    .device
                    .cmd_next_subpass(command_buffer.handle, subpass.contents());
            }

            let mut inside = InsideOfRenderpassScope(self.0);

            match subpass {
                Subpass::Inline { callback } => callback(&mut inside)?,
            }
        }

        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer
                .device
                .device
                .cmd_end_render_pass(command_buffer.handle);
        }

        Ok(self)
    }

    pub fn as_generic(&mut self) -> GenericCommands<'_, 'b> {
        GenericCommands(self.0)
    }
}

pub struct InsideOfRenderpassScope<'a, 'b: 'a>(&'a mut CommandBufferRecorder<'b>);

impl<'a, 'b: 'a> InsideOfRenderpassScope<'a, 'b> {
    // pub fn as_graphics_commandbuffer(self) -> GraphicsCommandBuffer<'a> {
    //     GraphicsCommandBuffer {
    //         inner: self.0.inner,
    //         bindings: self.0.bindings,
    //     }
    // }

    pub fn as_draw(&mut self) -> DrawCommands<'_, 'b> {
        DrawCommands(self.0)
    }

    pub fn as_graphics_generic(&mut self) -> GraphicsGenericCommands<'_, 'b> {
        GraphicsGenericCommands(self.0)
    }

    pub fn pipeline_barrier(
        &mut self,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        dependency_flags: vk::DependencyFlags,
        memory_barriers: &'b [vk::MemoryBarrier],
        image_memory_barriers: &'b [vk::ImageMemoryBarrier],
    ) -> &mut Self {
        let command_buffer = &self.0.inner;

        for (index, image_barrier) in image_memory_barriers.iter().enumerate() {
            //TODO: check for image being an attachment of current subpass as input and (color or depth/stencil)

            if image_barrier.old_layout != image_barrier.new_layout {
                panic!(format!(
                    "old and new layout of image barrier {} must be equal",
                    index
                ));
            }

            if image_barrier.src_queue_family_index != image_barrier.dst_queue_family_index {
                panic!(format!(
                    "src and dst queue family of image barrier {} must be equal",
                    index
                ));
            }
        }

        // And add more checks

        unsafe {
            command_buffer.device.device.cmd_pipeline_barrier(
                command_buffer.handle,
                src_stage_mask,
                dst_stage_mask,
                dependency_flags,
                memory_barriers,
                &[],
                image_memory_barriers,
            )
        }

        self
    }
}

/// Base for operations that can be recorded either outside or inside a renderpass
pub struct GenericCommands<'a, 'b: 'a>(&'a mut CommandBufferRecorder<'b>);

impl<'a, 'b: 'a> GenericCommands<'a, 'b> {
    pub fn as_generic_graphics(
        &mut self,
    ) -> Result<GraphicsGenericCommands<'_, 'b>, UnsupportedOperation> {
        if self.0.inner.command_pool.support_graphics() {
            Ok(GraphicsGenericCommands(self.0))
        } else {
            Err(UnsupportedOperation)
        }
    }

    pub fn as_generic_compute(
        &mut self,
    ) -> Result<ComputeGenericCommands<'_, 'b>, UnsupportedOperation> {
        if self.0.inner.command_pool.support_compute() {
            Ok(ComputeGenericCommands(self.0))
        } else {
            Err(UnsupportedOperation)
        }
    }

    pub fn pipeline_barrier(
        &mut self,
        src_stage_mask: vk::PipelineStageFlags,
        dst_stage_mask: vk::PipelineStageFlags,
        dependency_flags: vk::DependencyFlags,
        memory_barriers: &'b [vk::MemoryBarrier],
        buffer_memory_barriers: &'b [vk::BufferMemoryBarrier],
        image_memory_barriers: &'b [vk::ImageMemoryBarrier],
    ) -> &mut Self {
        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_pipeline_barrier(
                command_buffer.handle,
                src_stage_mask,
                dst_stage_mask,
                dependency_flags,
                memory_barriers,
                buffer_memory_barriers,
                image_memory_barriers,
            )
        }

        self
    }
}

pub struct GraphicsGenericCommands<'a, 'b: 'a>(&'a mut CommandBufferRecorder<'b>);

impl<'a, 'b: 'a> GraphicsGenericCommands<'a, 'b> {
    fn vertex_buffer_check(vertex_buffer: &Buffer) -> bool {
        if !vertex_buffer
            .usage
            .contains(vk::BufferUsageFlags::VERTEX_BUFFER)
        {
            return false;
        }

        true
    }

    fn index_buffer_check(index_buffer: &Buffer) -> bool {
        if !index_buffer
            .usage
            .contains(vk::BufferUsageFlags::INDEX_BUFFER)
        {
            return false;
        }

        true
    }

    pub fn bind_vertex_buffers(
        &mut self,
        first_binding: u32,
        buffers_and_offsets: &'b [(Buffer, u64)],
    ) -> Result<&mut Self, DrawError> {
        for (buffer, _) in buffers_and_offsets {
            if Self::vertex_buffer_check(buffer) {
                return Err(DrawError::Draw);
            }
        }

        let (buffers, offsets): (Vec<_>, Vec<_>) = buffers_and_offsets
            .iter()
            .map(|(buffer, offset)| (buffer.handle, *offset))
            .unzip();

        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_bind_vertex_buffers(
                command_buffer.handle,
                first_binding,
                &buffers,
                &offsets,
            )
        }

        self.0.graphics_bindings.vertex_buffers = true;

        Ok(self)
    }

    pub fn bind_index_buffer(
        &mut self,
        index_buffer: &'b Buffer,
        offset: vk::DeviceSize,
        index_type: vk::IndexType,
    ) -> Result<&mut Self, DrawError> {
        if !Self::index_buffer_check(index_buffer) {
            return Err(DrawError::Indexed);
        }

        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_bind_index_buffer(
                command_buffer.handle,
                index_buffer.handle,
                offset,
                index_type,
            );
        }

        Ok(self)
    }

    pub fn bind_descriptor_sets(
        &mut self,
        descriptor_sets: &'b [vk::DescriptorSet],
        dynamic_offsets: Option<&'b [u32]>,
    ) -> Result<&mut Self, UnsupportedOperation> {
        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_bind_descriptor_sets(
                command_buffer.handle,
                vk::PipelineBindPoint::GRAPHICS,
                self.0
                    .graphics_bindings
                    .graphics_pipeline
                    .as_ref()
                    .ok_or(UnsupportedOperation)?
                    .layout,
                0,
                descriptor_sets,
                dynamic_offsets.unwrap_or(&[]),
            )
        }

        self.0.graphics_bindings.descriptors = true;

        Ok(self)
    }

    pub fn bind_pipeline(&mut self, pipeline: &'b GraphicsPipeline) -> &mut Self {
        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_bind_pipeline(
                command_buffer.handle,
                vk::PipelineBindPoint::GRAPHICS,
                pipeline.pipeline,
            )
        }

        self.0.graphics_bindings.graphics_pipeline = Some(pipeline);

        self
    }
}

pub struct ComputeGenericCommands<'a, 'b: 'a>(&'a mut CommandBufferRecorder<'b>);

impl<'a, 'b: 'a> ComputeGenericCommands<'a, 'b> {
    pub fn bind_descriptor_sets(
        &mut self,
        descriptor_sets: &'b [vk::DescriptorSet],
        dynamic_offsets: Option<&'b [u32]>,
    ) -> Result<&mut Self, UnsupportedOperation> {
        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_bind_descriptor_sets(
                command_buffer.handle,
                vk::PipelineBindPoint::COMPUTE,
                self.0
                    .compute_bindings
                    .compute_pipeline
                    .as_ref()
                    .ok_or(UnsupportedOperation)?
                    .layout,
                0,
                descriptor_sets,
                dynamic_offsets.unwrap_or(&[]),
            )
        }

        self.0.compute_bindings.descriptors = true;

        Ok(self)
    }

    pub fn bind_pipeline(&mut self, pipeline: &'b ComputePipeline) -> &mut Self {
        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_bind_pipeline(
                command_buffer.handle,
                vk::PipelineBindPoint::COMPUTE,
                pipeline.pipeline,
            )
        }

        self.0.compute_bindings.compute_pipeline = Some(&pipeline);

        self
    }
}

pub struct DrawCommands<'a, 'b: 'a>(&'a mut CommandBufferRecorder<'b>);

impl<'a, 'b: 'a> DrawCommands<'a, 'b> {
    pub fn new(
        command_buffer: InsideOfRenderpassScope<'a, 'b>,
        first_binding: u32,
        buffers_and_offsets: &'b [(Buffer, u64)],
    ) -> Result<Self, DrawError> {
        for (buffer, _) in buffers_and_offsets {
            if GraphicsGenericCommands::vertex_buffer_check(buffer) {
                return Err(DrawError::Draw);
            }
        }

        let (buffers, offsets): (Vec<_>, Vec<_>) = buffers_and_offsets
            .iter()
            .map(|(buffer, offset)| (buffer.handle, *offset))
            .unzip();

        let commands = DrawCommands(command_buffer.0);

        let command_buffer = &commands.0.inner;

        unsafe {
            command_buffer.device.device.cmd_bind_vertex_buffers(
                command_buffer.handle,
                first_binding,
                &buffers,
                &offsets,
            )
        }

        commands.0.graphics_bindings.vertex_buffers = true;

        Ok(commands)
    }

    /// This function verify that draw preconditions are met
    /// TODO: complete it
    fn can_draw(&self) -> bool {
        self.0.graphics_bindings.vertex_buffers
    }

    fn can_draw_indexed(&self) -> bool {
        self.0.graphics_bindings.index_buffer
    }

    /// This function verify that indirect draw preconditions are met
    /// TODO: complete it
    fn indirect_buffer_check(&self, indirect_buffer: &Buffer) -> bool {
        if !indirect_buffer
            .usage
            .contains(vk::BufferUsageFlags::INDIRECT_BUFFER)
        {
            return false;
        }

        true
    }

    pub fn as_indexed(&mut self) -> Result<IndexedDrawCommands<'_, 'b>, DrawError> {
        if self.can_draw_indexed() {
            Ok(IndexedDrawCommands(self.0))
        } else {
            Err(DrawError::Indexed)
        }
    }

    pub fn draw(
        &mut self,
        vertexes: Range<u32>,
        instances: Range<u32>,
    ) -> Result<&mut Self, DrawError> {
        if !self.can_draw() {
            return Err(DrawError::Draw);
        }

        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_draw(
                command_buffer.handle,
                vertexes.len() as _,
                instances.len() as _,
                vertexes.start,
                instances.start,
            );
        }

        Ok(self)
    }

    pub fn draw_indirect(
        &mut self,
        indirect_buffer: &'b Buffer,
        offset: vk::DeviceSize,
        draw_count: u32,
        stride: u32,
    ) -> Result<&mut Self, DrawError> {
        if !self.can_draw() {
            return Err(DrawError::Draw);
        }

        if !self.indirect_buffer_check(&indirect_buffer) {
            return Err(DrawError::Indirect);
        }

        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_draw_indirect(
                command_buffer.handle,
                indirect_buffer.handle,
                offset,
                draw_count,
                stride,
            )
        }

        Ok(self)
    }

    ///////////////////////////////
    // vkCmdDrawIndirectCount    //
    // vkCmdDrawIndirectCountKHR //
    // vkCmdDrawIndirectCountAMD //
    ///////////////////////////////

    ///////////////////////////////////
    // vkCmdDrawIndirectByteCountEXT //
    ///////////////////////////////////

    ///////////////////////////////////////
    // vkCmdBeginConditionalRenderingEXT //
    // vkCmdEndConditionalRenderingEXT   //
    ///////////////////////////////////////

    //////////////////////////////////
    // vkCmdDrawMeshTasksNV         //
    // vkCmdDrawMeshTasksIndirectNV //
    //////////////////////////////////
}

pub struct IndexedDrawCommands<'a, 'b: 'a>(&'a mut CommandBufferRecorder<'b>);

impl<'a, 'b: 'a> IndexedDrawCommands<'a, 'b> {
    pub fn new(
        draw_commands: &'a mut DrawCommands<'_, 'b>,
        index_buffer: &'b Buffer,
        offset: vk::DeviceSize,
        index_type: vk::IndexType,
    ) -> Result<Self, DrawError> {
        if !GraphicsGenericCommands::index_buffer_check(index_buffer) {
            return Err(DrawError::Indexed);
        }

        let commands = IndexedDrawCommands(draw_commands.0);

        let command_buffer = &(commands.0).inner;

        unsafe {
            command_buffer.device.device.cmd_bind_index_buffer(
                command_buffer.handle,
                index_buffer.handle,
                offset,
                index_type,
            );
        }

        Ok(commands)
    }

    pub fn as_draw(&mut self) -> DrawCommands<'_, 'b> {
        DrawCommands(self.0)
    }

    pub fn draw_indexed(
        &mut self,
        index: Range<u32>,
        vertex_offset: i32,
        instances: Range<u32>,
    ) -> Result<&mut Self, DrawError> {
        if !self.as_draw().can_draw() {
            return Err(DrawError::Draw);
        }

        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_draw_indexed(
                command_buffer.handle,
                index.len() as _,
                instances.len() as _,
                index.start,
                vertex_offset,
                instances.start,
            );
        }

        Ok(self)
    }

    pub fn draw_indexed_indirect(
        &mut self,
        indirect_buffer: &'b Buffer,
        offset: vk::DeviceSize,
        draw_count: u32,
        stride: u32,
    ) -> Result<&mut Self, DrawError> {
        if !self.as_draw().can_draw() {
            return Err(DrawError::Draw);
        }

        if !self.as_draw().indirect_buffer_check(&indirect_buffer) {
            return Err(DrawError::Indirect);
        }

        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_draw_indexed_indirect(
                command_buffer.handle,
                indirect_buffer.handle,
                offset,
                draw_count,
                stride,
            )
        }

        Ok(self)
    }

    //////////////////////////////////////
    // vkCmdDrawIndexedIndirectCount    //
    // vkCmdDrawIndexedIndirectCountKHR //
    // vkCmdDrawIndexedIndirectCountAMD //
    //////////////////////////////////////
}

/// Add verifications to all functions
pub struct DispatchCommands<'a, 'b: 'a>(&'a mut CommandBufferRecorder<'b>);

impl<'a, 'b: 'a> DispatchCommands<'a, 'b> {
    pub fn dispatch(
        &mut self,
        group_count_x: u32,
        group_count_y: u32,
        group_count_z: u32,
    ) -> Result<&mut Self, DispatchError> {
        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_dispatch(
                command_buffer.handle,
                group_count_x,
                group_count_y,
                group_count_z,
            );
        }

        Ok(self)
    }

    pub fn dispatch_indirect(
        &mut self,
        buffer: &'b Buffer,
        offset: vk::DeviceSize,
    ) -> Result<&mut Self, DispatchError> {
        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_dispatch_indirect(
                command_buffer.handle,
                buffer.handle,
                offset,
            );
        }

        Ok(self)
    }

    //////////////////////////
    // vkCmdDispatchBase    //
    // vkCmdDispatchBaseKHR //
    //////////////////////////

    pub fn as_generic(&mut self) -> GenericCommands<'_, 'b> {
        GenericCommands(self.0)
    }
}

/// Transfer + Graphics + Compute + Primary + Secondary + Outside except blit_image/resolve_image (only Graphics, and for resolve: Both)
pub struct ClearCommands<'a, 'b: 'a>(&'a mut CommandBufferRecorder<'b>);

impl<'a, 'b: 'a> ClearCommands<'a, 'b> {
    /// Outside renderpass
    pub fn clear_color_image(
        &mut self,
        image: &mut Image,
        color: &vk::ClearColorValue,
        ranges: &[vk::ImageSubresourceRange],
    ) -> Result<&mut Self, ClearError> {
        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_clear_color_image(
                command_buffer.handle,
                image.handle,
                image.layout,
                color,
                ranges,
            )
        }

        Ok(self)
    }

    /// Inside renderpass
    pub fn clear_attachments(
        &mut self,
        attachments: &[vk::ClearAttachment],
        rects: &[vk::ClearRect],
    ) -> Result<&mut Self, ClearError> {
        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_clear_attachments(
                command_buffer.handle,
                attachments,
                rects,
            )
        }

        Ok(self)
    }

    /// Outside renderpass
    pub fn fill_buffer(
        &mut self,
        dst_buffer: &mut Buffer,
        dst_offset: vk::DeviceSize,
        size: vk::DeviceSize,
        data: u32,
    ) -> Result<&mut Self, ClearError> {
        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_fill_buffer(
                command_buffer.handle,
                dst_buffer.handle,
                dst_offset,
                size,
                data,
            )
        }

        Ok(self)
    }

    /// Outside renderpass
    pub fn update_buffer<T: ByteCopiable>(
        &mut self,
        dst_buffer: &mut Buffer,
        dst_offset: vk::DeviceSize,
        data: &T,
    ) -> Result<&mut Self, ClearError> {
        let data_size = mem::size_of_val(data);

        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_update_buffer(
                command_buffer.handle,
                dst_buffer.handle,
                dst_offset,
                slice::from_raw_parts(data as *const T as *const u8, data_size),
            )
        }

        Ok(self)
    }
}

/// Outside render pass except vkCmdWriteBufferMarkerAMD (both)
pub struct CopyCommands<'a, 'b: 'a>(&'a mut CommandBufferRecorder<'b>);

impl<'a, 'b: 'a> CopyCommands<'a, 'b> {
    /// dst_buffer should be taken with &mut but src_buffer and dst_buffer can be aliases
    /// but copy regions shouldn't aliased
    /// add checks
    pub fn copy_buffer(
        &mut self,
        src_buffer: &'b Buffer,
        dst_buffer: &'b Buffer,
        regions: &[vk::BufferCopy],
    ) -> Result<&mut Self, CopyError> {
        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_copy_buffer(
                command_buffer.handle,
                src_buffer.handle,
                dst_buffer.handle,
                regions,
            )
        }

        Ok(self)
    }

    /// Should return an error if layout doesn't fit
    /// Same for aliasing as before
    pub fn copy_image(
        &mut self,
        src_image: &'b Image,
        dst_image: &'b Image,
        regions: &'b [vk::ImageCopy],
    ) -> Result<&mut Self, CopyError> {
        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_copy_image(
                command_buffer.handle,
                src_image.handle,
                src_image.layout,
                dst_image.handle,
                dst_image.layout,
                regions,
            )
        }

        Ok(self)
    }

    pub fn copy_buffer_to_image(
        &mut self,
        src_buffer: &'b Buffer,
        dst_image: &'b mut Image,
        regions: &'b [vk::BufferImageCopy],
    ) -> Result<&mut Self, CopyError> {
        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_copy_buffer_to_image(
                command_buffer.handle,
                src_buffer.handle,
                dst_image.handle,
                dst_image.layout,
                regions,
            )
        }

        Ok(self)
    }

    pub fn copy_image_to_buffer(
        &mut self,
        src_image: &'b Image,
        dst_buffer: &'b mut Buffer,
        regions: &'b [vk::BufferImageCopy],
    ) -> Result<&mut Self, CopyError> {
        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_copy_image_to_buffer(
                command_buffer.handle,
                src_image.handle,
                src_image.layout,
                dst_buffer.handle,
                regions,
            )
        }

        Ok(self)
    }

    pub fn as_graphics_copy(&mut self) -> GraphicsCopyCommands<'_, 'b> {
        if !self.0.inner.command_pool.support_graphics() {
            panic!("Can't use graphics copy command in a command buffer that doesn't supports graphics operation");
        }

        GraphicsCopyCommands(self.0)
    }

    ///////////////////////////////
    // vkCmdWriteBufferMarkerAMD //
    ///////////////////////////////
}

pub struct GraphicsCopyCommands<'a, 'b: 'a>(&'a mut CommandBufferRecorder<'b>);

impl<'a, 'b: 'a> GraphicsCopyCommands<'a, 'b> {
    pub fn blit_image(
        &mut self,
        src_image: &'b Image,
        dst_image: &'b mut Image,
        regions: &'b [vk::ImageBlit],
        filter: vk::Filter,
    ) -> Result<&mut Self, CopyError> {
        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_blit_image(
                command_buffer.handle,
                src_image.handle,
                src_image.layout,
                dst_image.handle,
                dst_image.layout,
                regions,
                filter,
            )
        }

        Ok(self)
    }

    pub fn resolve_image(
        &mut self,
        src_image: &'b Image,
        dst_image: &'b mut Image,
        regions: &'b [vk::ImageResolve],
    ) -> Result<&mut Self, CopyError> {
        let command_buffer = &self.0.inner;

        unsafe {
            command_buffer.device.device.cmd_resolve_image(
                command_buffer.handle,
                src_image.handle,
                src_image.layout,
                dst_image.handle,
                dst_image.layout,
                regions,
            )
        }

        Ok(self)
    }
}

/// TODO: créer une vraie graphics pipeline
pub struct GraphicsPipeline {
    pub layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
}

pub struct ExecutableCommandBuffer(pub(crate) CommandBuffer);

impl ExecutableCommandBuffer {
    pub unsafe fn as_record(self) -> CommandBufferRecorder<'static> {
        let usage = self.0.usage;

        self.0.begin(usage)
    }

    pub unsafe fn as_executable(self) -> Self {
        self
    }
}

#[derive(Default)]
pub struct QueueSubmission<'a> {
    wait_semaphores: Vec<vk::Semaphore>,
    wait_dst_stage_masks: Vec<vk::PipelineStageFlags>,
    command_buffers: Vec<vk::CommandBuffer>,
    signal_semaphores: Vec<vk::Semaphore>,
    phantom_data: PhantomData<&'a ()>,
}

impl QueueSubmission<'static> {
    pub fn builder() -> QueueSubmissionBuilder<'static> {
        QueueSubmissionBuilder::default()
    }
}

impl<'a> QueueSubmission<'a> {
    pub(crate) fn wait_semaphores(&self) -> &[vk::Semaphore] {
        &self.wait_semaphores
    }

    pub(crate) fn wait_dst_stage_masks(&self) -> &[vk::PipelineStageFlags] {
        &self.wait_dst_stage_masks
    }

    pub(crate) fn command_buffers(&self) -> &[vk::CommandBuffer] {
        &self.command_buffers
    }

    pub(crate) fn signal_semaphores(&self) -> &[vk::Semaphore] {
        &self.signal_semaphores
    }
}

#[derive(Default)]
pub struct QueueSubmissionBuilder<'a>(QueueSubmission<'a>);

impl<'a> QueueSubmissionBuilder<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_wait_semaphore(
        mut self,
        semaphore: vk::Semaphore,
        pipeline_stage: vk::PipelineStageFlags,
    ) -> Self {
        self.0.wait_semaphores.push(semaphore);
        self.0.wait_dst_stage_masks.push(pipeline_stage);
        self
    }

    pub fn with_command_buffer(mut self, command_buffer: &'a ExecutableCommandBuffer) -> Self {
        self.0.command_buffers.push(command_buffer.0.handle);
        self
    }

    pub fn with_signal_semaphore(&mut self, signal_semaphore: vk::Semaphore) -> &mut Self {
        self.0.signal_semaphores.push(signal_semaphore);
        self
    }

    pub fn with_wait_semaphores<I: IntoIterator<Item = (vk::Semaphore, vk::PipelineStageFlags)>>(
        mut self,
        iter: I,
    ) -> Self {
        for (semaphore, pipeline_stage) in iter {
            self = self.with_wait_semaphore(semaphore, pipeline_stage);
        }
        self
    }

    pub fn with_command_buffers<I: IntoIterator<Item = &'a ExecutableCommandBuffer> + 'a>(
        mut self,
        iter: I,
    ) -> Self {
        self.0.command_buffers.extend(
            iter.into_iter()
                .map(|command_buffer| command_buffer.0.handle),
        );
        self
    }

    pub fn with_signal_semaphores<I: IntoIterator<Item = vk::Semaphore>>(
        mut self,
        iter: I,
    ) -> Self {
        self.0.signal_semaphores.extend(iter);
        self
    }

    pub fn build(self) -> QueueSubmission<'a> {
        self.0
    }
}

#[cfg(test)]
mod test {
    use std::mem::MaybeUninit;

    use ash::vk;

    use super::*;

    #[test]
    #[should_panic]
    #[allow(invalid_value, dead_code, unreachable_code)]
    pub fn test() {
        panic!("compile time type check for help, don't run it");

        let mut graphics_command_buffer =
            unsafe { MaybeUninit::<GraphicsCommandBuffer>::uninit().assume_init() };

        graphics_command_buffer
            .renderpass(
                &vk::RenderPassBeginInfo::builder(),
                vec![Subpass::Inline {
                    callback: Box::new(|inside_of_render_pass_scope| {
                        let mut a = inside_of_render_pass_scope.as_draw();
                        a.draw(0..4, 0..1)?
                            .as_indexed()?
                            .draw_indexed(0..4, 0, 0..1)?;
                        a.draw(0..4, 0..1)?;

                        Ok(())
                    }),
                }],
            )
            .unwrap();
    }
}
