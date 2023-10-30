#![allow(non_snake_case)]

use dioxus_desktop::{Config, WindowBuilder};

mod search;
mod ui;

fn main() {
    dioxus_desktop::launch_with_props(ui::App, (), Config::new()
        .with_window(WindowBuilder::new()
            .with_resizable(false)
            .with_always_on_top(true)
            .with_transparent(true)
            .with_decorations(false)
            .with_title("Keal")
        )
        .with_custom_head(r#"<link rel="stylesheet" href="public/style.css" />"#.to_owned())
    );
}
