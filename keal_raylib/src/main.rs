#![allow(non_snake_case)]

use keal::{arguments::{Arguments, self}, start_log_time, log_time};
use ui::Keal;
use raylib::prelude::*;

mod ui;
mod config;



fn main() -> anyhow::Result<()> {
    start_log_time();
    match Arguments::init("raylib") {
        Ok(_) => (),
        Err(arguments::Error::Exit) => return Ok(()),
        Err(arguments::Error::UnknownFlag(flag)) => {
            anyhow::bail!("error: unknown flag `{flag}`")
        }
    };

    log_time("reading config");

    let mut theme = config::Theme::default();
    let _config = keal::config::Config::init(&mut theme);

    let font_loader = std::thread::spawn(|| {
        log_time("loading font");
        let iosevka = include_bytes!("../../public/iosevka-regular.ttf");
        let iosevka = TrueTypeFont::from_bytes(&iosevka[..]).unwrap();
        log_time("finished loading font");

        iosevka
    });

    log_time("initializing window");

    set_trace_log_level(TraceLogLevel::Fatal);
    set_config_flags(ConfigFlags::TRANSPARENT);
    let mut rl = &mut init_window(1920/3, 1080/2, "Keal", 60);
    set_window_state(rl, WindowFlags::UNDECORATED | WindowFlags::RESIZABLE);

    log_time("initializing keal");
    let iosevka = load_font_ex(rl, font_loader.join().unwrap(), FontParams::default());
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
