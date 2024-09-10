#![allow(non_snake_case)]

use keal::{arguments::{Arguments, self}, start_log_time, log_time};
use ui::Keal;
use raylib::prelude::*;

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

    log_time("reading config");

    let mut theme = config::Theme::default();
    let _config = keal::config::Config::init(&mut theme);

    log_time("initilizing window");

    set_trace_log_level(TraceLogLevel::Fatal);
    set_config_flags(ConfigFlags::TRANSPARENT);
    let mut rl = &mut init_window(1920/3, 1080/2, "Keal", 60);
    set_window_state(rl, WindowFlags::UNDECORATED | WindowFlags::RESIZABLE);

    log_time("initilizing font");

    let iosevka = include_bytes!("../../public/iosevka-regular.ttf");
    let iosevka = load_font_bytes(rl, &iosevka[..]);

    log_time("initializing keal");

    let mut keal = Keal::new(iosevka);

    log_time("entering drawing loop");

    keal.update_input(true);

    while !window_should_close(rl) {
        begin_drawing(rl, |rl| {
            clear_background(rl, theme.background);

            keal.render(rl, &theme);
        });
        keal.update(&mut rl);
    }

    Ok(())
}
