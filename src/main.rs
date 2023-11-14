#![allow(non_snake_case)]

use iced::{Application, Settings, window, Font, font};

mod search;
mod ui;
mod icon;
mod config;

fn main() -> iced::Result {
    let config = config::load_config();

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
        flags: config,
        ..Default::default()
    })
}
