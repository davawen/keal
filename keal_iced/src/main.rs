#![allow(non_snake_case)]

use keal::{arguments::{self, Arguments}, log_time, start_log_time};
use iced::{Application, Settings, window, Font, font};

mod ui;
mod config;

fn main() -> anyhow::Result<()> {
    start_log_time();
    match Arguments::init() {
        Ok(_) => (),
        Err(arguments::Error::Exit) => return Ok(()),
        Err(arguments::Error::UnknownFlag(flag)) => {
            anyhow::bail!("error: unknown flag `{flag}`")
        }
    };

    let mut theme = config::Theme::default();
    let config = keal::config::Config::init(&mut theme);

    log_time("read config");

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
            weight: theme.font_weight,
            stretch: theme.font_stretch,
            ..Default::default()
        },
        flags: theme,
        ..Default::default()
    })?;

    Ok(())
}
