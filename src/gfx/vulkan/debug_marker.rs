#[cfg(debug_assertions)]
use super::cur_cmd;
use crate::{DEVICE, INSTANCE};
use ash::vk;
use lazy_static::lazy_static;

pub struct DebugScope;

impl DebugScope {
    pub fn new(name: &str) -> Self {
        DebugMarker::begin(name);
        Self
    }
    pub fn new_colored(name: &str, color: [f32; 4]) -> Self {
        DebugMarker::begin_colored(name, color);
        Self
    }
}

impl Drop for DebugScope {
    fn drop(&mut self) {
        DebugMarker::end();
    }
}

#[macro_export]
macro_rules! debug_scope {
    ($name:expr) => {
        let _d = DebugScope::new($name);
    };
    ($name:expr, [$r:literal, $g:literal, $b:literal, $a:literal]) => {
        let _d = DebugScope::new($name, [r, g, b, a]);
    };
}

lazy_static! {
    static ref DEBUG_UTILS_LOADER: ash::ext::debug_utils::Device =
        ash::ext::debug_utils::Device::new(&INSTANCE, &DEVICE);
}

pub struct DebugMarker;

#[cfg(debug_assertions)]
impl DebugMarker {
    pub fn name<T: vk::Handle>(name: &str, obj: T) {
        unsafe {
            DEBUG_UTILS_LOADER
                .set_debug_utils_object_name(
                    &vk::DebugUtilsObjectNameInfoEXT::default()
                        .object_name(&std::ffi::CString::new(name).unwrap())
                        .object_handle(obj),
                )
                .unwrap()
        }
    }

    pub fn tag<T: vk::Handle>(name: u64, tag: &[u8], obj: T) {
        unsafe {
            DEBUG_UTILS_LOADER
                .set_debug_utils_object_tag(
                    &vk::DebugUtilsObjectTagInfoEXT::default()
                        .tag_name(name)
                        .tag(tag)
                        .object_handle(obj),
                )
                .unwrap()
        }
    }

    pub fn begin(label: &str) {
        unsafe {
            DEBUG_UTILS_LOADER.cmd_begin_debug_utils_label(
                cur_cmd(),
                &vk::DebugUtilsLabelEXT::default()
                    .label_name(&std::ffi::CString::new(label).unwrap())
                    .color([1.0, 1.0, 1.0, 1.0]),
            )
        }
    }

    pub fn begin_colored(label: &str, color: [f32; 4]) {
        unsafe {
            DEBUG_UTILS_LOADER.cmd_begin_debug_utils_label(
                cur_cmd(),
                &vk::DebugUtilsLabelEXT::default()
                    .label_name(&std::ffi::CString::new(label).unwrap())
                    .color(color),
            )
        }
    }

    pub fn end() {
        unsafe { DEBUG_UTILS_LOADER.cmd_end_debug_utils_label(cur_cmd()) }
    }

    pub fn insert(label: &str) {
        unsafe {
            DEBUG_UTILS_LOADER.cmd_insert_debug_utils_label(
                cur_cmd(),
                &vk::DebugUtilsLabelEXT::default()
                    .label_name(&std::ffi::CString::new(label).unwrap())
                    .color([1.0, 1.0, 1.0, 1.0]),
            )
        }
    }

    pub fn insert_colored(label: &str, color: [f32; 4]) {
        unsafe {
            DEBUG_UTILS_LOADER.cmd_insert_debug_utils_label(
                cur_cmd(),
                &vk::DebugUtilsLabelEXT::default()
                    .label_name(&std::ffi::CString::new(label).unwrap())
                    .color(color),
            )
        }
    }
}

#[cfg(not(debug_assertions))]
impl DebugMarker {
    pub fn name<T: vk::Handle>(_name: &str, _obj: T) {}
    pub fn tag<T: vk::Handle>(_name: u64, _tag: &[u8], _obj: T) {}
    pub fn begin_colored(_label: &str, _color: [f32; 4]) {}
    pub fn begin(_label: &str) {}
    pub fn end() {}
    pub fn insert(_label: &str) {}
    pub fn insert_colored(_label: &str, _color: [f32; 4]) {}
}
