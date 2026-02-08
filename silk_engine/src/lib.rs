#![feature(box_as_ptr, macro_metavar_expr)]
#![allow(dead_code)]

pub mod prelude;

mod engine;
mod gfx;
mod input;
mod sfx;
mod util;
mod vulkan;

const OS: &str = if cfg!(target_os = "linux") {
    "linux"
} else if cfg!(target_os = "windows") {
    "windows"
} else if cfg!(target_os = "macos") {
    "macos"
} else {
    "unknown"
};
