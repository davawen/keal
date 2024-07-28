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

    log_time("reading config");

    let config = config::Config::init();

    log_time("initilizing window");

    set_trace_log_level(TraceLogLevel::Fatal);
    set_config_flags(ConfigFlags::TRANSPARENT);
    let mut rl = &mut init_window(1920/3, 1080/2, "Keal", 60);
    set_window_state(rl, WindowFlags::UNDECORATED | WindowFlags::RESIZABLE);

    log_time("initilizing font");

    let iosevka = include_bytes!("../public/iosevka-regular.ttf");
    let iosevka = load_font_bytes(rl, &iosevka[..]);

    log_time("initializing keal");

    let mut keal = Keal::new(iosevka);

    log_time("entering drawing loop");

    keal.update_input(true);

    while !window_should_close(rl) {
        begin_drawing(rl, |rl| {
            clear_background(rl, config.theme.background);

            keal.render(rl);
        });
        keal.update(&mut rl);
    }

    Ok(())
}
