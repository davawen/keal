use iced::font;

use crate::{xdg_utils::config_dir, ini_parser::Ini};

// WARN: When adding fields to the config, remember to set them in `add_from_string`!

// This should probably use serde, but the `serde_ini` crate seems suboptimal for sections, and the current custom parser works well enough
#[derive(Debug)]
pub struct Config {
    pub font: String,
    pub font_weight: font::Weight,
    pub font_stretch: font::Stretch,
    pub font_size: f32,
    pub icon_theme: Vec<String>,
    pub usage_frequency: bool,
    pub terminal_path: String,
    pub placeholder_text: String,
    pub default_plugins: Vec<String>,
    pub theme: crate::ui::Theme
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font: String::new(),
            font_size: 0.0,
            font_weight: font::Weight::Normal,
            font_stretch: font::Stretch::Normal,
            icon_theme: vec![],
            terminal_path: String::new(),
            placeholder_text: String::new(),
            usage_frequency: false,
            default_plugins: Vec::new(),
            theme: Default::default()
        }
    }
}

impl Config {
    /// Loads the default included configuration (in public/default-config.ini)
    pub fn default_config() -> Self {
        // SAFETY: the default config needs to have every field filled in
        let mut config = Config::default();
        config.add_from_string(include_str!("../public/default-config.ini").to_owned());
        config
    }

    fn add_from_string(&mut self, content: String) {
        let file = Ini::from_string(content, &['#', ';']);

        // Since the name of the field in the ini is the same as in the `Config` struct, we can match it directly.
        // This is what `stringify!($name)` is doing.
        // The type checker can work backwards from `$config.$name = v` to find what type is to be parsed, and what implementation of `MyFromStr` should be called.
        // Pretty cool!
        macro_rules! parse_fields {
            ($config:expr, $field:expr, ($($name:ident),+)) => {
                match $field.0.as_str() {
                    $(
                        stringify!($name) => match $field.1.my_parse() {
                            Ok(v) => $config.$name = v,
                            Err(e) => eprintln!("error with field `{}`: {}: `{}`", stringify!($name), e, $field.1)
                        }
                    ),+
                    _ => ()
                }
            };
        }

        for field in file.section_iter("keal") {
            parse_fields!(self, field, (
                font, font_size, font_weight, font_stretch, icon_theme, usage_frequency, terminal_path, placeholder_text, default_plugins
            ));
        }

        for color in file.section_iter("colors") {
            let theme = &mut self.theme;
            parse_fields!(theme, color, (
                background,
                input_placeholder, input_selection, input_background,
                text, matched_text, selected_matched_text, comment,
                choice_background, selected_choice_background, hovered_choice_background, pressed_choice_background,
                scrollbar_enabled, scrollbar, hovered_scrollbar, scrollbar_border_radius
            ));
        }
    }

    pub fn load() -> Self {
        let mut config = Config::default_config();

        let Ok(mut config_path) = config_dir() else { return config };
        config_path.push("config.ini");

        let Ok(content) = std::fs::read_to_string(config_path) else { return config };

        config.add_from_string(content);
        config
    }
}

trait MyFromStr<T> {
    fn my_parse(&self) -> Result<T, &str>;
}

impl<T> MyFromStr<Vec<T>> for str where str: MyFromStr<T> {
    fn my_parse(&self) -> Result<Vec<T>, &str> {
        self.split(',').map(|x| x.my_parse()).collect::<Result<_, _>>()
    }
}

impl MyFromStr<font::Weight> for str {
    fn my_parse(&self) -> Result<font::Weight, &str> {
        use font::Weight as W;
        let v = match self {
            "extralight" => W::ExtraLight,
            "light" => W::Light,
            "thin" => W::Thin,
            "regular" => W::Normal,
            "medium" => W::Medium,
            "semibold" => W::Semibold,
            "bold" => W::Bold,
            "extrabold" => W::ExtraBold,
            "black" => W::Black,
            _ => Err("unknown font weight")?
        };
        Ok(v)
    }
}

impl MyFromStr<font::Stretch> for str {
    fn my_parse(&self) -> Result<iced::font::Stretch, &str> {
        use font::Stretch as S;
        let v = match self {
            "ultraexpanded" => S::UltraExpanded,
            "extraexpanded" => S::ExtraExpanded,
            "expanded" => S::Expanded,
            "semiexpanded" => S::SemiExpanded,
            "normal" => S::Normal,
            "semicondensed" => S::SemiCondensed,
            "condensed" => S::Condensed,
            "extracondensed" => S::ExtraCondensed,
            "ultracondensed" => S::UltraCondensed,
            _ => Err("unknown font stretch")?
        };
        Ok(v)
    }
}

impl MyFromStr<iced::Color> for str {
    fn my_parse(&self) -> Result<iced::Color, &'static str> {
        let Some(Ok(r)) = self.get(0..2).map(|r| u32::from_str_radix(r, 16)) else { Err("invalid color code, mistyped or missing red channel")? };
        let Some(Ok(g)) = self.get(2..4).map(|r| u32::from_str_radix(r, 16)) else { Err("invalid color code, mistyped or missing green channel")? };
        let Some(Ok(b)) = self.get(4..6).map(|r| u32::from_str_radix(r, 16)) else { Err("invalid color code, mistyped or missing blue channel")? };

        let a = if let Some(a) = self.get(6..8) {
            let Ok(a) = u32::from_str_radix(a, 16) else { Err("invalid color code, mistyped alpha channel")? };
            a
        } else { 255 };

        Ok(iced::Color {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0
        })
    }
}

impl MyFromStr<bool> for str {
    fn my_parse(&self) -> Result<bool, &'static str> {
        match self {
            "true" => Ok(true),
            "false" => Ok(false),
            _ => Err("invalid boolean")
        }
    }
}

impl MyFromStr<String> for str {
    fn my_parse(&self) -> Result<String, &'static str> {
        Ok(self.to_owned())
    }
}

impl MyFromStr<f32> for str {
    fn my_parse(&self) -> Result<f32, &'static str> {
        self.parse().map_err(|_| "couldn't parse number")
    }
}
