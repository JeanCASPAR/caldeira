use std::mem;
use std::rc::Rc;

use ash::version::DeviceV1_0;
use ash::vk;

use winit::event::{Event, WindowEvent};
use winit::event_loop::ControlFlow;

use caldeira::vulkan;

fn main() {
    pretty_env_logger::formatted_timed_builder()
        .parse_filters("caldeira=trace")
        .init();

    let instance = Rc::new(vulkan::Instance::new());
    #[cfg(feature = "validation-layers")]
    let debug = vulkan::Debug::new(Rc::clone(&instance));

    let device = Rc::new(vulkan::Device::new(Rc::clone(&instance)));

    let descriptor = vulkan::DescriptorBuilder::new()
        .with_binding(
            vk::DescriptorType::STORAGE_BUFFER,
            1,
            vk::ShaderStageFlags::COMPUTE,
            None,
        )
        .build(Rc::clone(&device));
    let descriptors = vec![descriptor];

    let compute_pipeline = vulkan::ComputePipeline::new(&descriptors, Rc::clone(&device));

    let buffer = vulkan::Buffer::new(
        4,
        vk::BufferUsageFlags::STORAGE_BUFFER
            | vk::BufferUsageFlags::TRANSFER_SRC
            | vk::BufferUsageFlags::TRANSFER_DST,
        vk::MemoryPropertyFlags::HOST_COHERENT | vk::MemoryPropertyFlags::HOST_VISIBLE,
        Rc::clone(&device),
        &instance,
    );

    unsafe {
        let data = device
            .device
            .map_memory(buffer.memory, 0, 4, vk::MemoryMapFlags::empty())
            .unwrap();
        (data as *mut u32).write(0);
        device.device.unmap_memory(buffer.memory);
    };

    let mut descriptor_pool = vulkan::DescriptorPool::new(&descriptors[0], Rc::clone(&device));
    let descriptor_sets = mem::replace(&mut descriptor_pool.descriptor_sets, Vec::new()); // Y en a qu'un pour l'instant

    {
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(buffer.buffer)
            .offset(0)
            .range(4);
        let buffer_infos = [buffer_info.build()];

        let descriptor_write = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_sets[0])
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .buffer_info(&buffer_infos);
        let descriptor_writes = [descriptor_write.build()];

        unsafe {
            device
                .device
                .update_descriptor_sets(&descriptor_writes, &[]);
        }
    }

    let command_pool = vulkan::CommandPool::new(&instance, Rc::clone(&device));

    let command_buffer = vulkan::SingleTimeCommand::new(&device, &command_pool); // TODO: utiliser des command buffers alloués normalement et stockés

    unsafe {
        device.device.cmd_bind_pipeline(
            command_buffer.command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            compute_pipeline.compute_pipeline,
        );

        device.device.cmd_bind_descriptor_sets(
            command_buffer.command_buffer,
            vk::PipelineBindPoint::COMPUTE,
            compute_pipeline.pipeline_layout,
            0,
            &descriptor_sets,
            &[],
        );

        device
            .device
            .cmd_dispatch(command_buffer.command_buffer, 10, 10, 10);

        command_buffer.submit();
    }

    unsafe {
        let data = device
            .device
            .map_memory(buffer.memory, 0, 4, vk::MemoryMapFlags::empty())
            .unwrap();
        let output = (data as *mut u32).read();
        println!("output: {}", output);
        device.device.unmap_memory(buffer.memory);
    };

    let mut window = vulkan::Window::<()>::new();

    window
        .event_loop
        .take()
        .unwrap()
        .run(move |event, _, control_flow| {
            // ControlFlow::Poll continuously runs the event loop, even if the OS hasn't
            // dispatched any events. This is ideal for games and similar applications.
            *control_flow = ControlFlow::Poll;

            // ControlFlow::Wait pauses the event loop if no events are available to process.
            // This is ideal for non-game applications that only update in response to user
            // input, and uses significantly less power/CPU time than ControlFlow::Poll.
            // *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    ..
                } => {
                    println!("The close button was pressed; stopping");
                    *control_flow = ControlFlow::Exit
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
