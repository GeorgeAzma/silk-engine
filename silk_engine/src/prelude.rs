pub use crate::{
    engine::{EnginePlugin, Time, EngineConfig, EventLoop, WindowEvent, WinitEvent},
    gfx::{DrawContext, Gfx, TextureAtlas, Unit::*},
    input::{Input, InputPlugin, Key, Mouse},
    sfx::{AudioData, Sfx, SfxPlugin, Source},
    util::{print::Level, ema::Ema},
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

pub struct DefaultPlugins;
impl PluginGroup for DefaultPlugins {
    fn build(self) -> bevy_app::PluginGroupBuilder {
        bevy_app::PluginGroupBuilder::start::<Self>()
            .add(EnginePlugin)
            .add(InputPlugin)
            .add(SfxPlugin)
    }
}