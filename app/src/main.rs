#![allow(unused_variables)]

use std::num::NonZeroU32;
use std::rc::Rc;

use ash::version::DeviceV1_0;
use ash::vk;

use winit::event::{Event, KeyboardInput, VirtualKeyCode, WindowEvent};
use winit::event_loop::ControlFlow;

use caldeira::vulkan;

fn main() {
    pretty_env_logger::formatted_timed_builder()
        .parse_filters("caldeira=trace")
        .init();

    let instance = Rc::new(vulkan::Instance::new());

    #[allow(unused_variables)]
    #[cfg(feature = "validation-layers")]
    let debug = vulkan::Debug::new(Rc::clone(&instance));

    let (device, mut queues) = vulkan::Device::new(
        |queue_family, _| {
            if queue_family.support_compute() {
                let create_info = vulkan::QueueCreateInfo::new(vec![1.0]);

                Some(create_info)
            } else {
                None
            }
        },
        Rc::clone(&instance),
    );

    let mut compute_queue = queues.swap_remove(0).swap_remove(0);

    let mut command_pool = Rc::new(vulkan::CommandPool::new(
        compute_queue.family(),
        Rc::clone(&device),
    ));

    // let command_buffer = vulkan::SingleTimeCommand::new(&device, &command_pool); // TODO: utiliser des command buffers alloués normalement et stockés

    let mut command_buffers = command_pool
        .allocate_command_buffers(vk::CommandBufferLevel::PRIMARY, 3)
        .into_iter()
        .map(|command_buffer| command_buffer.begin(vk::CommandBufferUsageFlags::empty()))
        .collect::<Vec<_>>();

    let command_buffer = &mut command_buffers[0];

    let descriptor_set_layout = vulkan::DescriptorSetLayoutBuilder::new()
        .with_binding(
            vk::DescriptorType::STORAGE_BUFFER,
            NonZeroU32::new(1).unwrap(),
            vk::ShaderStageFlags::COMPUTE,
            None,
        )
        .with_binding(
            vk::DescriptorType::STORAGE_IMAGE,
            NonZeroU32::new(1).unwrap(),
            vk::ShaderStageFlags::COMPUTE,
            None,
        )
        .with_binding(
            vk::DescriptorType::UNIFORM_BUFFER,
            NonZeroU32::new(1).unwrap(),
            vk::ShaderStageFlags::COMPUTE,
            None,
        )
        .build(Rc::clone(&device));
    let descriptor_set_layouts = [descriptor_set_layout];

    let compute_pipeline =
        vulkan::ComputePipeline::new(&descriptor_set_layouts, Rc::clone(&device));

    let mut buffer = vulkan::Buffer::new(
        4,
        vk::BufferUsageFlags::STORAGE_BUFFER
            | vk::BufferUsageFlags::TRANSFER_SRC
            | vk::BufferUsageFlags::TRANSFER_DST,
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        Rc::clone(&device),
        &instance,
    );

    buffer.copy_data(&0u32, 0);

    let mut output_image = vulkan::Image::new_storage(1_000, 1_000, Rc::clone(&device), &instance);

    let (src_stage_mask, dst_stage_mask, dependency_flags, barrier) =
        output_image.transition_layout(vk::ImageLayout::GENERAL);

    let image_memory_barriers = [barrier.build()];

    command_buffer.as_generic().pipeline_barrier(
        src_stage_mask,
        dst_stage_mask,
        dependency_flags,
        &[],
        &[],
        &image_memory_barriers,
    );

    let command_buffer = command_buffers.swap_remove(0).end();

    let submits = [vulkan::QueueSubmission::builder()
        .with_command_buffer(&command_buffer)
        .build()];

    compute_queue.submit(&submits, None);
    compute_queue.wait_idle();

    let descriptor_pool = vulkan::DescriptorPoolBuilder::new()
        .with(vk::DescriptorType::STORAGE_BUFFER, 1)
        .with(vk::DescriptorType::STORAGE_IMAGE, 1)
        .build(1, Rc::clone(&device));

    let descriptor_sets = descriptor_set_layouts[0].allocate_descriptor_sets(1, &descriptor_pool);

    {
        let buffer_info_1 = vk::DescriptorBufferInfo::builder()
            .buffer(buffer.handle)
            .offset(0)
            .range(4);
        let buffer_infos = [buffer_info_1.build()];

        let image_info = vk::DescriptorImageInfo::builder()
            .image_layout(output_image.layout) // Problème de synchronisation : le layout correspond pas encore vu que le command buffer est pas submit
            .image_view(output_image.view);
        let image_infos = [image_info.build()];

        let descriptor_write_1 = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_sets[0])
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&buffer_infos[0..1]);
        let descriptor_write_2 = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_sets[0])
            .dst_binding(1)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .image_info(&image_infos);
        let descriptor_writes = [descriptor_write_1.build(), descriptor_write_2.build()];

        unsafe {
            device
                .device
                .update_descriptor_sets(&descriptor_writes, &[]);
        }
    }

    let command_buffer = &mut command_buffers[0];

    command_buffer
        .as_generic()
        .as_generic_compute()
        .unwrap()
        .bind_pipeline(&compute_pipeline)
        .bind_descriptor_sets(&descriptor_sets, None)
        .unwrap();

    command_buffer
        .as_compute_command_buffer()
        .unwrap()
        .dispatch(100, 100, 1)
        .unwrap();

    // unsafe {
    //     device.device.cmd_bind_pipeline(
    //         command_buffer.command_buffer,
    //         vk::PipelineBindPoint::COMPUTE,
    //         compute_pipeline.pipeline,
    //     );

    //     device.device.cmd_bind_descriptor_sets(
    //         command_buffer.command_buffer,
    //         vk::PipelineBindPoint::COMPUTE,
    //         compute_pipeline.layout,
    //         0,
    //         &descriptor_sets,
    //         &[],
    //     );

    //     device
    //         .device
    //         .cmd_dispatch(command_buffer.command_buffer, 100, 100, 1);

    //     command_buffer.submit();
    // }

    let command_buffer = command_buffers.swap_remove(0).end();

    let submits = [vulkan::QueueSubmission::builder()
        .with_command_buffer(&command_buffer)
        .build()];

    compute_queue.submit(&submits, None);
    compute_queue.wait_idle();

    let output = {
        let mut output = 0;
        buffer.get_data(&mut output, 0);
        println!("output: {}", output);
        output
    };

    debug_assert!(output == output_image.extent.width * output_image.extent.height);

    let pixels = {
        let size = output_image.extent.width * output_image.extent.height * 4;

        let mut pixels = vec![0u8; size as _];

        let mut staging_buffer = vulkan::Buffer::new(
            size as _,
            vk::BufferUsageFlags::TRANSFER_DST,
            vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
            Rc::clone(&device),
            &instance,
        );

        let command_buffer = &mut command_buffers[0];

        let (src_stage_mask, dst_stage_mask, dependency_flags, barrier) =
            output_image.transition_layout(vk::ImageLayout::TRANSFER_SRC_OPTIMAL);
        let image_memory_barriers = [barrier.build()];

        command_buffer.as_generic().pipeline_barrier(
            src_stage_mask,
            dst_stage_mask,
            dependency_flags,
            &[],
            &[],
            &image_memory_barriers,
        );

        let regions = [{
            let image_subresource = vk::ImageSubresourceLayers::builder()
                .aspect_mask(vk::ImageAspectFlags::COLOR)
                .mip_level(0)
                .base_array_layer(0)
                .layer_count(1)
                .build();

            let image_offset = vk::Offset3D::builder().x(0).y(0).z(0).build();

            let buffer_image_copy = vk::BufferImageCopy::builder()
                .buffer_offset(0)
                .buffer_row_length(0)
                .buffer_image_height(0)
                .image_subresource(image_subresource)
                .image_offset(image_offset)
                .image_extent(output_image.extent);

            buffer_image_copy.build()
        }];

        command_buffer
            .as_transfer_command_buffer()
            .unwrap()
            .as_copy()
            .copy_image_to_buffer(&output_image, &mut staging_buffer, &regions)
            .unwrap();

        // output_image.copy_to_buffer(&mut staging_buffer, &command_pool);

        let command_buffer = command_buffers.swap_remove(0).end();

        let submits = [vulkan::QueueSubmission::builder()
            .with_command_buffer(&command_buffer)
            .build()];

        compute_queue.submit(&submits, None);
        compute_queue.wait_idle();

        staging_buffer.get_data(&mut pixels[..], 0);

        pixels
    };

    {
        let image = image::ImageBuffer::<image::Rgba<_>, _>::from_vec(
            output_image.extent.width,
            output_image.extent.height,
            pixels,
        )
        .unwrap();

        image
            .save_with_format("image.png", image::ImageFormat::Png)
            .unwrap();
    }

    let mut window = vulkan::Window::<()>::new();

    window
        .event_loop
        .take()
        .unwrap()
        .run(move |event, _, control_flow| {
            let instance = &instance;
            let debug = &debug;
            let device = &device;
            let command_pool = &command_pool;
            let window = &window;
            let descriptors = &descriptor_set_layouts;
            let descriptor_pool = &descriptor_pool;
            let descriptor_sets = &descriptor_sets;
            let buffer = &buffer;
            let output_image = &output_image;
            let compute_pipeline = &compute_pipeline;

            // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
            // dispatched any events. This is ideal for games and similar applications.
            *control_flow = ControlFlow::Poll;

            // ControlFlow::Wait pauses the event loop if no events are available to process.
            // This is ideal for non-game applications that only update in response to user
            // input, and uses significantly less power/CPU time than ControlFlow::Poll.
            // *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent { event, .. } => {
                    if matches!(
                        event,
                        WindowEvent::CloseRequested
                            | WindowEvent::KeyboardInput {
                                input: KeyboardInput {
                                    virtual_keycode: Some(VirtualKeyCode::Escape),
                                    ..
                                },
                                ..
                            }
                    ) {
                        println!("The close button was pressed; stopping");
                        *control_flow = ControlFlow::Exit
                    }
                }
                Event::MainEventsCleared => {
                    // Application update code.

                    // Queue a RedrawRequested event.
                    //
                    // You only need to call this if you've determined that you need to redraw, in
                    // applications which do not always need to. Applications that redraw continuously
                    // can just render here instead.
                    window.window.request_redraw();
                }
                Event::RedrawRequested(_) => {
                    // Redraw the application.
                    //
                    // It's preferable for applications that do not render continuously to render in
                    // this event rather than in MainEventsCleared, since rendering in here allows
                    // the program to gracefully handle redraws requested by the OS.
                }
                _ => (),
            }
        });
}
