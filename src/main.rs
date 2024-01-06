#![allow(non_snake_case)]

use std::sync::OnceLock;

use arguments::{Arguments, arguments};
use iced::{Application, Settings, window, Font, font};

mod ui;
mod icon;
mod config;
mod xdg_utils;
mod ini_parser;
mod plugin;

mod arguments;

static START: OnceLock<std::time::Instant> = OnceLock::new();
fn log_time(s: impl ToString) {
    if !arguments().timings { return }

    let now = std::time::Instant::now();
    let duration = now.duration_since(*START.get().unwrap());

    eprintln!("[{}.{:03}]: {}", duration.as_secs(), duration.subsec_millis(), s.to_string());
}

fn main() -> anyhow::Result<()> {
    START.get_or_init(std::time::Instant::now);
    match Arguments::init() {
        Ok(_) => (),
        Err(arguments::Error::Exit) => return Ok(()),
        Err(arguments::Error::UnknownFlag(flag)) => {
            anyhow::bail!("error: unknown flag `{flag}`")
        }
    };

    let config = config::Config::init();

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
            weight: config.font_weight,
            stretch: config.font_stretch,
            ..Default::default()
        },
        ..Default::default()
    })?;

    Ok(())
}
