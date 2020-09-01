use std::ffi::{c_void, CStr};
use std::rc::Rc;

use ash::extensions::ext::DebugUtils;
use ash::vk;

use super::Instance;

unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void,
) -> u32 {
    // if message_severity < vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
    //     return vk::FALSE;
    // }

    let message_type = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "GENERAL",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "VALIDATION",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "PERFORMANCE",
        _ => "UNKNOWN",
    };

    let message = CStr::from_ptr((*callback_data).p_message).to_str().unwrap();
    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            log::trace!("[VERBOSE][{}] Validation layer: {}", message_type, message);
            vk::FALSE
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            log::info!("[INFO][{}] Validation layer: {}", message_type, message);
            vk::FALSE
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            log::warn!("[INFO][{}] Validation layer: {}", message_type, message);
            vk::FALSE
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            log::error!("[INFO][{}] Validation layer: {}", message_type, message);
            vk::TRUE
        }
        _ => {
            log::error!("[UNKNOWN][{}] Validation layer: {}", message_type, message);
            vk::TRUE
        }
    }
}

pub struct Debug {
    pub debug_utils: DebugUtils,
    pub debug_utils_messenger: vk::DebugUtilsMessengerEXT,
    _instance: Rc<Instance>,
}

impl Debug {
    pub fn new(instance: Rc<Instance>) -> Self {
        let create_info = Self::populate_debug_messenger_create_info();

        let debug_utils = DebugUtils::new(&instance.entry, &instance.instance);

        let debug_utils_messenger =
            unsafe { debug_utils.create_debug_utils_messenger(&create_info, None) }
                .expect("failed to setup debug utils messenger!");

        Self {
            debug_utils,
            debug_utils_messenger,
            _instance: instance,
        }
    }

    fn populate_debug_messenger_create_info<'a>() -> vk::DebugUtilsMessengerCreateInfoEXTBuilder<'a>
    {
        vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    // | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    // | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
            )
            .message_type(
                vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE,
            )
            .pfn_user_callback(Some(debug_callback))
    }
}

impl Drop for Debug {
    fn drop(&mut self) {
        unsafe {
            self.debug_utils
                .destroy_debug_utils_messenger(self.debug_utils_messenger, None);
        }
    }
}
