#![allow(non_snake_case)]

use std::sync::OnceLock;

use arguments::{Arguments, arguments};
// use iced::{Application, Settings, window, Font, font};
use macroquad::{miniquad::window::{order_quit, quit, request_quit}, prelude::*, ui::{hash, root_ui, widgets::Window}};
use ui::Keal;

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

fn window_conf() -> Conf {
    Conf {
        window_title: "Keal".to_owned(),
        window_width: 1920/3,
        window_height: 1080/2,
        platform: miniquad::conf::Platform {
            linux_backend: miniquad::conf::LinuxBackend::X11WithWaylandFallback,
            wayland_use_fallback_decorations: false,
            framebuffer_alpha: true,
            ..Default::default()
        },
        ..Default::default()
    }
}

fn draw_rectangle_rounded(x: f32, y: f32, w: f32, h: f32, radius: f32, color: Color) {
    let left = x + radius;
    let top = y + radius;

    let right = x + w - radius;
    let bot = y + h - radius;

    let width = w - radius*2.0;
    let height = h - radius*2.0;

    draw_rectangle(left, top, width, height, color);

    draw_rectangle(left, y, width, radius, color);
    draw_rectangle(left, bot, width, radius, color);

    draw_rectangle(x, top, radius, height, color);
    draw_rectangle(right, top, radius, height, color);

    draw_circle(left, top, radius, color);
    draw_circle(right, top, radius, color);
    draw_circle(left, bot, radius, color);
    draw_circle(right, bot, radius, color);
}

#[macroquad::main(window_conf)]
async fn main() -> anyhow::Result<()> {
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

    let mut keal = Keal::new();

    log_time("entering drawing loop");

    keal.update_input(String::new(), true);

    loop {
        clear_background(BLANK);
        draw_rectangle_rounded(0.0, 0.0, screen_width(), screen_height(), 10.0, config.theme.background);

        keal.render();

        next_frame().await;

        keal.update();
    }
}
