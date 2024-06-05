#![allow(non_snake_case)]

use std::sync::OnceLock;

use arguments::{Arguments, arguments};
use ui::Keal;
use raylib::prelude::*;

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

    let mut rl = Raylib::init_window(1920/3, 1080/2, "Keal", 60);
    rl.set_window_state(ConfigFlags::FLAG_WINDOW_UNDECORATED | ConfigFlags::FLAG_WINDOW_TRANSPARENT | ConfigFlags::FLAG_WINDOW_RESIZABLE);

    let iosevka = include_bytes!("../public/iosevka-regular.ttf");
    let iosevka = TrueTypeFont::from_bytes(&iosevka[..]).unwrap();
    let mut keal = Keal::new(&mut rl, &iosevka);

    log_time("entering drawing loop");

    keal.update_input(true);

    while !rl.window_should_close() {
        rl.begin_drawing(|rl| {
            rl.clear_background(config.theme.background);

            keal.render(rl);
        });
        keal.update(&mut rl);

        if keal.quit { break }
    }

    Ok(())
}
