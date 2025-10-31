#![allow(non_snake_case)]

use keal::{arguments::{self, Arguments}, log_time, start_log_time};
use iced::{window, Font};
use ui::Keal;

mod ui;
mod config;

fn main() -> anyhow::Result<()> {
    start_log_time();
    match Arguments::init("iced") {
        Ok(_) => (),
        Err(arguments::Error::Exit) => return Ok(()),
        Err(arguments::Error::UnknownFlag(flag)) => {
            anyhow::bail!("error: unknown flag `{flag}`")
        }
    };

    let mut theme = config::Theme::default();
    let _config = keal::config::Config::init(&mut theme);

    log_time("read config");

    iced::application("Keal", Keal::update, Keal::view)
        .theme(Keal::theme)
        .subscription(Keal::subscription)
        .settings(iced::Settings {
            fonts: vec![include_bytes!("../../public/iosevka-regular.ttf").as_slice().into()],
            default_font: Font::with_name("Iosevka"),
            ..Default::default()
        })
        .window(window::Settings {
            size: iced::Size::new(1920.0/3.0, 1080.0/2.0),
            position: window::Position::Centered,
            resizable: false,
            decorations: false,
            transparent: true,
            level: window::Level::AlwaysOnTop,
            ..Default::default()
        })
        .run_with(move || Keal::new(theme))?;

    Ok(())
}
