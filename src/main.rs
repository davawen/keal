#![allow(non_snake_case)]

use arguments::Arguments;
use iced::{Application, Settings, window, Font, font};
use ui::Flags;

mod ui;
mod icon;
mod config;
mod xdg_utils;
mod ini_parser;
mod plugin;

mod arguments;

fn main() -> anyhow::Result<()> {
    let arguments = match Arguments::parse() {
        Ok(a) => a,
        Err(arguments::Error::Exit) => return Ok(()),
        Err(arguments::Error::UnknownFlag(flag)) => {
            anyhow::bail!("error: unknown flag `{flag}`")
        }
    };

    let config = config::Config::load();
    let manager = plugin::PluginManager::new(&arguments);

    ui::Keal::run(Settings {
        window: window::Settings {
            size: (1920/3, 1080/2),
            position: window::Position::Centered,
            resizable: false,
            decorations: false,
            transparent: true,
            level: window::Level::AlwaysOnTop,
            ..Default::default()
        },
        default_font: Font {
            family: font::Family::Name(config.font.clone().leak()),
            weight: config.font_weight,
            stretch: config.font_stretch,
            ..Default::default()
        },
        flags: Flags(config, manager),
        ..Default::default()
    })?;

    Ok(())
}
