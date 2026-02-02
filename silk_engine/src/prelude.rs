pub use crate::{
    engine::{App, Engine, EngineConfig},
    gfx::{Gfx, Unit::*},
    sfx::{AudioData, Sfx, Source},
    vulkan::{VulkanConfig, window::Window},
};

pub use std::{collections::HashMap, error::Error};

pub use winit::{
    dpi::{LogicalPosition, LogicalSize, PhysicalPosition, PhysicalSize},
    event::WindowEvent,
    event_loop::ActiveEventLoop,
    window::{CustomCursor, CustomCursorSource, Theme, WindowAttributes, WindowId},
};

pub type ResultAny<T = ()> = Result<T, Box<dyn Error>>;
