use winit::dpi::LogicalSize;
use winit::event_loop::EventLoop;
use winit::window::{Window as WinitWindow, WindowBuilder};

use crate::consts::{HEIGHT, WIDTH};

pub struct Window<T: 'static> {
    pub event_loop: Option<EventLoop<T>>,
    pub window: WinitWindow,
    pub frame_buffer_resized: bool,
}

impl<T: 'static> Window<T> {
    pub fn new() -> Self {
        Self::with_size(WIDTH, HEIGHT)
    }

    pub fn with_size(width: usize, height: usize) -> Self {
        let event_loop = EventLoop::with_user_event();
        Self::with_event_loop(event_loop, width, height)
    }

    pub fn with_event_loop(event_loop: EventLoop<T>, width: usize, height: usize) -> Self {
        let size: LogicalSize<f64> = (width as f64, height as f64).into();
        let window = WindowBuilder::new()
            .with_decorations(true)
            .with_inner_size(size)
            .with_resizable(true)
            .with_title("Vulkan tutorial")
            .build(&event_loop)
            .expect("Unable to create a winit Window");

        Self::with_window(event_loop, window)
    }

    pub fn with_window(event_loop: EventLoop<T>, window: WinitWindow) -> Self {
        Self {
            event_loop: Some(event_loop),
            window,
            frame_buffer_resized: false,
        }
    }
}

impl<T> Default for Window<T> {
    fn default() -> Self {
        Self::new()
    }
}
