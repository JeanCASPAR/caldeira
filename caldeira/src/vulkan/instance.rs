use std::collections::HashSet;
use std::ffi::{CStr, CString};
use std::os::raw::c_char;

use ash::version::{EntryV1_0, InstanceV1_0};
use ash::vk;

use crate::consts::{
    INSTANCE_EXTENSIONS, REQUIRED_MAJOR, REQUIRED_MINOR, REQUIRED_PATCH, REQUIRED_VERSION,
    VALIDATION_LAYERS,
};
use crate::utils;

pub struct Instance {
    pub entry: ash::Entry,
    pub instance: ash::Instance,
}

impl Instance {
    pub fn new() -> Self {
        let entry = ash::Entry::new().expect("failed to load vulkan");
        let version = entry
            .try_enumerate_instance_version()
            .unwrap()
            .unwrap_or(vk::make_version(1, 0, 0)); // If vulkan 1.1 is not supported, this functions is not present

        let major = vk::version_major(version);
        let minor = vk::version_minor(version);
        let patch = vk::version_patch(version);

        println!("Vulkan version: {}.{}.{}", major, minor, patch);

        if major < REQUIRED_MAJOR || minor < REQUIRED_MINOR || patch < REQUIRED_PATCH {
            panic!(
                "The minimum required version is {}.{}.{} and device version is {}.{}.{}",
                REQUIRED_MAJOR, REQUIRED_MINOR, REQUIRED_PATCH, major, minor, patch
            );
        }

        let application_name = CString::new("Test").unwrap();
        let engine_name = CString::new("Caldeira").unwrap();
        let app_info = vk::ApplicationInfo::builder()
            .application_name(&application_name)
            .application_version(vk::make_version(0, 1, 0))
            .engine_name(&engine_name)
            .engine_version(vk::make_version(0, 1, 0))
            .api_version(REQUIRED_VERSION);

        let extension_names = Self::check_instance_extensions(&entry)
            .expect("instance extensions requested, but not available!");

        let create_info = vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extension_names);

        #[cfg(feature = "validation-layers")]
        let validation_layers = Self::check_validation_layers(&entry)
            .expect("validation layers requested, but not available!");

        #[cfg(feature = "validation-layers")]
        let enabled = [
            // vk::ValidationFeatureEnableEXT::GPU_ASSISTED,
            // vk::ValidationFeatureEnableEXT::GPU_ASSISTED_RESERVE_BINDING_SLOT,
            vk::ValidationFeatureEnableEXT::BEST_PRACTICES,
            // vk::ValidationFeatureEnableEXT::DEBUG_PRINTF,
        ];
        #[cfg(feature = "validation-layers")]
        let mut validation_features =
            vk::ValidationFeaturesEXT::builder().enabled_validation_features(&enabled);

        #[cfg(feature = "validation-layers")]
        let create_info = create_info
            .enabled_layer_names(&validation_layers)
            .push_next(&mut validation_features);

        let instance = unsafe { entry.create_instance(&create_info, None) }
            .expect("failed to create instance!");

        unsafe {
            utils::free_cstring(extension_names);

            #[cfg(feature = "validation-layers")]
            utils::free_cstring(validation_layers);
        }

        Self { entry, instance }
    }

    fn check_instance_extensions(entry: &ash::Entry) -> Option<Vec<*const c_char>> {
        let extension_names = entry
            .enumerate_instance_extension_properties()
            .expect("failed to enumerate extensions")
            .into_iter()
            .map(|property| property.extension_name)
            .map(|name| unsafe { CStr::from_ptr(name.as_ptr()).to_owned() })
            .inspect(|name| println!("instance extension: {:?}", name))
            .filter(|name| INSTANCE_EXTENSIONS.contains(&name.to_str().unwrap()))
            .collect::<HashSet<_>>();

        if extension_names.len() == INSTANCE_EXTENSIONS.len() {
            let extensions = extension_names
                .into_iter()
                .map(|name| name.into_raw() as *const _)
                .collect::<Vec<_>>();

            Some(extensions)
        } else {
            None
        }
    }

    fn check_validation_layers(entry: &ash::Entry) -> Option<Vec<*const i8>> {
        let validation_layer_names = entry
            .enumerate_instance_layer_properties()
            .expect("failed to enumerate validation layers")
            .into_iter()
            .map(|property| property.layer_name)
            .map(|name| unsafe { CStr::from_ptr(name.as_ptr()).to_owned() })
            .inspect(|name| println!("validation layer: {:?}", name))
            .filter(|name| VALIDATION_LAYERS.contains(&name.to_str().unwrap()))
            .collect::<HashSet<_>>();

        if validation_layer_names.len() == VALIDATION_LAYERS.len() {
            let validation_layers = validation_layer_names
                .into_iter()
                .map(|name| name.into_raw() as *const _)
                .collect::<Vec<_>>();
            Some(validation_layers)
        } else {
            None
        }
    }
}

impl Default for Instance {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        unsafe {
            self.instance.destroy_instance(None);
        }
    }
}
