use iced::font;

use crate::search::xdg::config_dir;

pub struct Config {
    pub font: String,
    pub font_weight: font::Weight,
    pub font_stretch: font::Stretch,
    pub font_size: f32,
    pub icon_theme: String,
    pub placeholder_text: String,
    pub theme: crate::ui::Theme
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font: "Iosevka".to_owned(),
            font_size: 16.0,
            font_weight: font::Weight::Medium,
            font_stretch: font::Stretch::Normal,
            icon_theme: "hicolor".to_owned(),
            placeholder_text: "search your dreams".to_owned(),
            theme: Default::default()
        }
    }
}

pub fn load_config() -> Config {
    let mut config = Config::default();

    let Ok(mut config_path) = config_dir() else { return config };
    config_path.push("config.ini");

    let Ok(file) = tini::Ini::from_file(&config_path) else { return config };

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
        parse_fields!(config, field, (
            font, font_size, font_weight, font_stretch, icon_theme, placeholder_text
        ));
    }

    for color in file.section_iter("colors") {
        let theme = &mut config.theme;
        parse_fields!(theme, color, (
            background,
            input_placeholder, input_selection, input_background,
            text, matched_text, selected_matched_text, comment,
            choice_background, selected_choice_background, hovered_choice_background, pressed_choice_background,
            scrollbar_enabled, scrollbar, hovered_scrollbar, scrollbar_border_radius
        ));
    }

    config
}

trait MyFromStr<T> {
    fn my_parse(&self) -> Result<T, &str>;
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
        let Some(Ok(r)) = self.get(0..2).map(|r| u32::from_str_radix(r, 16)) else { Err("invalid color code")? };
        let Some(Ok(g)) = self.get(2..4).map(|r| u32::from_str_radix(r, 16)) else { Err("invalid color code")? };
        let Some(Ok(b)) = self.get(4..6).map(|r| u32::from_str_radix(r, 16)) else { Err("invalid color code")? };

        let a = if let Some(Ok(a)) = self.get(6..8).map(|r| r.parse()) {
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
