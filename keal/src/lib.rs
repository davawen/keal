use std::sync::OnceLock;

use arguments::arguments;

pub mod config;
pub mod arguments;
pub mod icon;
pub mod xdg_utils;
pub mod ini_parser;
pub mod plugin;

static START: OnceLock<std::time::Instant> = OnceLock::new();
pub fn start_log_time() {
    START.get_or_init(std::time::Instant::now);
}

pub fn log_time(s: impl ToString) {
    if !arguments().timings { return }

    let now = std::time::Instant::now();
    let duration = now.duration_since(*START.get().unwrap());

    eprintln!("[{}.{:03}]: {}", duration.as_secs(), duration.subsec_millis(), s.to_string());
}
