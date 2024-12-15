use std::ffi::CStr;
use std::ffi::CString;

use super::config::*;
use super::ENTRY;
use crate::{fatal, log, print, warn};
use ash::vk;
use lazy_static::lazy_static;

lazy_static! {
    static ref ERROR_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
}

#[cfg(debug_assertions)]
unsafe extern "system" fn vulkan_debug_callback(
    message_severity: vk::DebugUtilsMessageSeverityFlagsEXT,
    message_type: vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const vk::DebugUtilsMessengerCallbackDataEXT<'_>,
    _user_data: *mut std::os::raw::c_void,
) -> vk::Bool32 {
    use std::process::abort;

    if message_severity == vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE
        || (message_severity == vk::DebugUtilsMessageSeverityFlagsEXT::INFO
            && message_type == vk::DebugUtilsMessageTypeFlagsEXT::GENERAL)
    {
        return vk::FALSE;
    }
    let callback_data = *p_callback_data;
    let mut message = callback_data
        .message_as_c_str()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let full_message = message.clone();
    if let Some(i) = message.find(" (http") {
        message.truncate(i);
    }
    let match_str = format!("MessageID = 0x{:x} | ", callback_data.message_id_number);
    if let Some(i) = message.find(&match_str) {
        message = message[i + match_str.len()..].to_string();
    }
    message = message
        .replace("The Vulkan spec states: ", "")
        .replace("VK_", "")
        .replace("the ", "");

    type Severity = vk::DebugUtilsMessageSeverityFlagsEXT;
    let ansi_message = match message_severity {
        Severity::ERROR => print::err(&message),
        Severity::WARNING => print::warn(&message),
        Severity::INFO => print::info(&message),
        _ => message,
    };

    let mut backtrace = print::backtrace_callers();
    backtrace.pop();
    let backtrace = backtrace.join(" > ");
    log!("{full_message}\n|> {backtrace}\n");
    let ansi_backtrace = print::trace(&["|> ", &backtrace].concat());
    let print_str = format!("{ansi_message}\n{ansi_backtrace}");
    match message_severity {
        Severity::ERROR => {
            eprintln!("{print_str}");
            let err_cnt = ERROR_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if err_cnt > 8 {
                eprintln!("{}", print::fatal("too many vulkan errors"));
                abort();
            }
        }
        Severity::WARNING => {
            println!("{print_str}");
            ERROR_COUNT.store(0, std::sync::atomic::Ordering::SeqCst);
        }
        _ => ERROR_COUNT.store(0, std::sync::atomic::Ordering::SeqCst),
    }

    vk::FALSE
}

lazy_static! {
    pub static ref INSTANCE_EXTENSIONS: Vec<CString> = unsafe {
        ENTRY
            .enumerate_instance_extension_properties(None)
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.extension_name_as_c_str().unwrap().to_owned())
            .collect()
    };
    pub static ref INSTANCE: ash::Instance = {
        let app_info = vk::ApplicationInfo::default()
            .api_version(vk::API_VERSION_1_3)
            .application_name(CStr::from_bytes_with_nul(b"silky\0").unwrap())
            .engine_name(CStr::from_bytes_with_nul(b"silk-engine\0").unwrap())
            .application_version(0)
            .engine_version(0);

        let required_instance_extensions: Vec<CString> = required_vulkan_instance_extensions()
            .into_iter()
            .filter(|re| {
                INSTANCE_EXTENSIONS
                    .contains(re)
                    .then_some(true)
                    .unwrap_or_else(|| fatal!("Unsupported vulkan instance extension: {re:?}"))
            })
            .collect();

        let preferred_instance_extensions: Vec<CString> = preferred_vulkan_instance_extensions()
            .into_iter()
            .filter(|pe| {
                INSTANCE_EXTENSIONS
                    .contains(pe)
                    .then_some(true)
                    .unwrap_or_else(|| {
                        warn!("Unsupported vulkan instance extension: {pe:?}");
                        false
                    })
            })
            .collect();
        let enabled_extensions =
            [required_instance_extensions, preferred_instance_extensions].concat();

        let enabled_exts = enabled_extensions
            .iter()
            .map(|e| e.as_ptr())
            .collect::<Vec<_>>();
        let info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_extension_names(&enabled_exts);

        let layers: Vec<CString> = unsafe {
            ENTRY
                .enumerate_instance_layer_properties()
                .unwrap_or_default()
                .into_iter()
                .map(|e| e.layer_name_as_c_str().unwrap().to_owned())
                .collect()
        };
        let mut enabled_layers = enabled_layers();
        enabled_layers.retain(|e| layers.contains(e));
        let enabled_layers = enabled_layers
            .iter()
            .map(|e| e.as_ptr())
            .collect::<Vec<_>>();
        let info = info.enabled_layer_names(&enabled_layers);

        let instance = unsafe {
            ENTRY
                .create_instance(&info, None)
                .expect("Failed to init VkInstance")
        };

        #[cfg(debug_assertions)]
        unsafe {
            ash::ext::debug_utils::Instance::new(&ENTRY, &instance)
                .create_debug_utils_messenger(
                    &vk::DebugUtilsMessengerCreateInfoEXT::default()
                        .message_severity(
                            vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                                | vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                                | vk::DebugUtilsMessageSeverityFlagsEXT::INFO
                                | vk::DebugUtilsMessageSeverityFlagsEXT::VERBOSE,
                        )
                        .message_type(
                            vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                                | vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                                | vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
                                | vk::DebugUtilsMessageTypeFlagsEXT::DEVICE_ADDRESS_BINDING,
                        )
                        .pfn_user_callback(Some(vulkan_debug_callback)),
                    None,
                )
                .unwrap()
        };

        instance
    };
}
