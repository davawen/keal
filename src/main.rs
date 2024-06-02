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

fn draw_rectangle_rounded(draw: &mut DrawHandle, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color) {
    let left = x + radius;
    let top = y + radius;

    let right = x + w - radius;
    let bot = y + h - radius;

    let width = w - radius*2.0;
    let height = h - radius*2.0;

    draw.rectangle(left, top, width, height, color);

    draw.rectangle(left, y, width, radius, color);
    draw.rectangle(left, bot, width, radius, color);

    draw.rectangle(x, top, radius, height, color);
    draw.rectangle(right, top, radius, height, color);

    draw.circle(left, top, radius, color);
    draw.circle(right, top, radius, color);
    draw.circle(left, bot, radius, color);
    draw.circle(right, bot, radius, color);
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
    rl.set_window_state(ConfigFlags::FLAG_WINDOW_UNDECORATED | ConfigFlags::FLAG_WINDOW_RESIZABLE);

    let iosevka = include_bytes!("../public/iosevka-regular.ttf");
    let iosevka = TrueTypeFont::from_bytes(&iosevka[..]).unwrap();
    let mut keal = Keal::new(&mut rl, &iosevka);

    log_time("entering drawing loop");

    keal.update_input(true);

    while !rl.window_should_close() {
        rl.begin_drawing(|rl, draw| {
            draw.clear_background(config.theme.background);
            // draw_rectangle_rounded(draw, 0.0, 0.0, rl.get_render_width(), rl.get_render_height(), 10.0, config.theme.background);

            keal.render(rl, draw);
            keal.update(rl, draw);
        });

        if keal.quit { break }
    }

    Ok(())
}
