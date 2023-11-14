use iced::font;

use crate::search::xdg::config_dir;

pub struct Config {
    pub font: String,
    pub font_weight: font::Weight,
    pub font_stretch: font::Stretch,
    pub font_size: f32,
    pub icon_theme: String
}

impl Default for Config {
    fn default() -> Self {
        Self {
            font: "Iosevkt".to_owned(),
            font_size: 16.0,
            font_weight: font::Weight::Medium,
            font_stretch: font::Stretch::Normal,
            icon_theme: "hicolor".to_owned()
        }
    }
}

pub fn load_config() -> Config {
    let mut config = Config::default();

    let Ok(mut config_path) = config_dir() else { return config };
    config_path.push("config.ini");

    let Ok(file) = tini::Ini::from_file(&config_path) else { return config };

    for field in file.section_iter("keal") {
        use font::Weight as W;
        use font::Stretch as S;

        match field.0.as_str() {
            "font" => config.font = field.1.clone(),
            "font_size" => if let Ok(num) = field.1.parse() {
                config.font_size = num;
            }
            "font_weight" => config.font_weight = match field.1.as_str() {
                "extralight" => W::ExtraLight,
                "light" => W::Light,
                "thin" => W::Thin,
                "regular" => W::Normal,
                "medium" => W::Medium,
                "semibold" => W::Semibold,
                "bold" => W::Bold,
                "extrabold" => W::ExtraBold,
                "black" => W::Black,
                weight => {
                    eprintln!("unknown font_weight: `{weight}`");
                    continue;
                }
            },
            "font_stretch" => config.font_stretch = match field.1.as_str() {
                "ultraexpanded" => S::UltraExpanded,
                "extraexpanded" => S::ExtraExpanded,
                "expanded" => S::Expanded,
                "semiexpanded" => S::SemiExpanded,
                "normal" => S::Normal,
                "semicondensed" => S::SemiCondensed,
                "condensed" => S::Condensed,
                "extracondensed" => S::ExtraCondensed,
                "ultracondensed" => S::UltraCondensed,
                stretch => {
                    eprintln!("unknown font_stretch: `{stretch}`");
                    continue;
                }
            },
            "icon_theme" => config.icon_theme = field.1.clone(),
            name => eprintln!("unknown config field: `{name}`")
        }
    }

    config
}
