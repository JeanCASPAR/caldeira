use std::ffi::{c_void, CStr};
use std::rc::Rc;

use ash::extensions::ext::DebugUtils;
use ash::vk;

use super::Instance;

// use ash::prelude::VkResult;
// use ash::version::{EntryV1_0, InstanceV1_0};
// use std::ffi::CString;
// use std::mem::transmute;
// use std::ptr::null;
// use std::rc::Rc;

unsafe extern "system" fn debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT,
    _user_data: *mut c_void,
) -> u32 {
    /*if message_severity < vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        return vk::FALSE;
    }*/

    let message_type = match message_type {
        vk::DebugUtilsMessageTypeFlagsEXT::GENERAL => "GENERAL",
        vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION => "VALIDATION",
        vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE => "PERFORMANCE",
        _ => "UNKNOWN",
    };

    let message = CStr::from_ptr((*callback_data).p_message).to_str().unwrap();
    match message_severity {
        vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE => {
            log::trace!("[VERBOSE][{}] Validation layer: {}", message_type, message)
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::INFO => {
            log::info!("[INFO][{}] Validation layer: {}", message_type, message)
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::WARNING => {
            log::warn!("[INFO][{}] Validation layer: {}", message_type, message)
        }
        vk::DebugUtilsMessageSeverityFlagsEXT::ERROR => {
            log::error!("[INFO][{}] Validation layer: {}", message_type, message)
        }
        _ => log::error!("[UNKNOWN][{}] Validation layer: {}", message_type, message),
    }

    vk::FALSE
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

    // fn create_debug_utils_messenger_ext(
    //     instance: &Instance,
    //     create_info: &vk::DebugUtilsMessengerCreateInfoEXT,
    //     allocator: Option<&vk::AllocationCallbacks>,
    // ) -> VkResult<vk::DebugUtilsMessengerEXT> {
    //     let allocator = allocator
    //         .map(|alloc| alloc as *const vk::AllocationCallbacks)
    //         .unwrap_or(null());

    //     let name = CString::new("vkCreateDebugUtilsMessengerEXT").unwrap();

    //     let func = unsafe {
    //         instance
    //             .entry
    //             .get_instance_proc_addr(instance.instance.handle(), name.as_ptr())
    //             .map(|func| transmute::<_, vk::PFN_vkCreateDebugUtilsMessengerEXT>(func))
    //     };

    //     let mut debug_messenger = vk::DebugUtilsMessengerEXT::null();

    //     if let Some(func) = func {
    //         let result = func(
    //             instance.instance.handle(),
    //             create_info,
    //             allocator,
    //             &mut debug_messenger,
    //         );

    //         if result == vk::Result::SUCCESS {
    //             Ok(debug_messenger)
    //         } else {
    //             Err(result)
    //         }
    //     } else {
    //         Err(vk::Result::ERROR_EXTENSION_NOT_PRESENT)
    //     }
    // }

    // fn destroy_debug_utils_messenger_ext(&mut self, allocator: Option<&vk::AllocationCallbacks>) {
    //     let allocator = allocator
    //         .map(|alloc| alloc as *const vk::AllocationCallbacks)
    //         .unwrap_or(null());

    //     let name = CString::new("vkDestroyDebugUtilsMessengerEXT").unwrap();

    //     let func = unsafe {
    //         self.instance
    //             .entry
    //             .get_instance_proc_addr(self.instance.instance.handle(), name.as_ptr())
    //             .map(|func| transmute::<_, vk::PFN_vkDestroyDebugUtilsMessengerEXT>(func))
    //     };

    //     if let Some(func) = func {
    //         func(
    //             self.instance.instance.handle(),
    //             self.debug_utils_messenger,
    //             allocator,
    //         );
    //     }
    // }

    fn populate_debug_messenger_create_info<'a>() -> vk::DebugUtilsMessengerCreateInfoEXTBuilder<'a>
    {
        vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
                    // | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                    | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    | vk::DebugUtilsMessageSeverityFlagsEXT::ERROR,
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
        // self.destroy_debug_utils_messenger_ext(None);
        unsafe {
            self.debug_utils
                .destroy_debug_utils_messenger(self.debug_utils_messenger, None);
        }
    }
}
