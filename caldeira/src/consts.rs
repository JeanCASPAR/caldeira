pub const WIDTH: usize = 800;
pub const HEIGHT: usize = 600;

#[cfg(feature = "validation-layers")]
pub const DEBUG: bool = true;
#[cfg(not(feature = "validation-layers"))]
pub const DEBUG: bool = false;

pub const INSTANCE_EXTENSIONS: &[&str] = &[
    "VK_KHR_surface",
    "VK_KHR_win32_surface",
    #[cfg(feature = "validation-layers")]
    "VK_EXT_debug_utils",
];
pub const VALIDATION_LAYERS: &[&str] = &[
    // "VK_LAYER_LUNARG_api_dump",
    #[cfg(feature = "validation-layers")]
    "VK_LAYER_KHRONOS_validation",
    "VK_LAYER_NV_optimus",
];
pub const DEVICE_EXTENSIONS: &[&str] = &["VK_KHR_swapchain"];
