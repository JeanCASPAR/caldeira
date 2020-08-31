use ash::vk;

pub const REQUIRED_VERSION: u32 = vk::make_version(1, 2, 0);

pub const REQUIRED_MAJOR: u32 = vk::version_major(REQUIRED_VERSION);
pub const REQUIRED_MINOR: u32 = vk::version_minor(REQUIRED_VERSION);
pub const REQUIRED_PATCH: u32 = vk::version_patch(REQUIRED_VERSION);

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
