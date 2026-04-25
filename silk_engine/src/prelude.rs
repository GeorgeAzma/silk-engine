pub use crate::{
    engine::{Engine, EngineConfig, EventLoop, WindowEvent, WinitEvent},
    gfx::{Gfx, Unit::*},
    input::{Input, Key, Mouse},
    sfx::{AudioData, Sfx, Source},
    util::print::Level,
    vulkan::{Vulkan, VulkanConfig, window::Window},
};

#[cfg(feature = "midi")]
pub use crate::midi::{Midi, MidiEvent, MidiPlugin};

pub use std::{collections::HashMap, error::Error};

pub use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
    event_loop::ActiveEventLoop,
    window::{CustomCursor, CustomCursorSource, Theme, WindowAttributes, WindowId},
};

pub use bevy_app::prelude::*;
pub use bevy_ecs::prelude::*;

pub type ResultAny<T = ()> = Result<T, Box<dyn Error>>;
