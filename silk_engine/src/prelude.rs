pub use crate::{
    engine::{EngineConfig, EnginePlugin, EventLoop, Time, WindowEvent, WinitEvent},
    gfx::{DrawContext, Gfx, GfxPlugin, TextureAtlas, Unit::*},
    input::{Input, InputPlugin, Key, Mouse},
    sfx::{AudioData, Sfx, SfxPlugin, Source},
    util::{ema::Ema, print::Level},
    vulkan::{Vulkan, VulkanConfig, VulkanPlugin, window::Window},
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
        let mut engine_config = EngineConfig::default();
        engine_config.logger.min_level = Level::Debug;

        bevy_app::PluginGroupBuilder::start::<Self>()
            .add(EnginePlugin { engine_config })
            .add(VulkanPlugin {
                vulkan_config: VulkanConfig::default(),
            })
            .add(GfxPlugin)
            .add(InputPlugin)
            .add(SfxPlugin)
    }
}
