use raylib::math::color::Color;

use keal::{config::FrontendConfig, parse_fields};

#[derive(Debug, Default, Clone)]
pub struct Theme {
    pub background: Color,

    pub input_placeholder: Color,
    pub input_selection: Color,
    pub input_background: Color,

    pub text: Color,
    pub matched_text: Color,
    pub selected_matched_text: Color,
    pub comment: Color,

    pub choice_background: Color,
    pub selected_choice_background: Color,
    pub hovered_choice_background: Color,
    pub pressed_choice_background: Color,

    pub scrollbar_enabled: bool,
    pub scrollbar: Color,
    pub hovered_scrollbar: Color,
    pub scrollbar_border_radius: f32
}

impl FrontendConfig for Theme {
    fn sections(&self) -> &'static [&'static str] {
        &["colors"]
    }

    fn add_field(&mut self, field: (String, String)) {
        parse_fields!(self, field, (
                background,
                input_placeholder, input_selection, input_background,
                text, matched_text, selected_matched_text, comment,
                choice_background, selected_choice_background, hovered_choice_background, pressed_choice_background,
                scrollbar_enabled, scrollbar, hovered_scrollbar, scrollbar_border_radius
        ));
    }
}

trait MyFromStr<T> {
    fn my_parse(&self) -> Result<T, &str>;
}

impl MyFromStr<Color> for str {
    fn my_parse(&self) -> Result<Color, &'static str> {
        let Some(Ok(r)) = self.get(0..2).map(|r| u32::from_str_radix(r, 16)) else { Err("invalid color code, mistyped or missing red channel")? };
        let Some(Ok(g)) = self.get(2..4).map(|r| u32::from_str_radix(r, 16)) else { Err("invalid color code, mistyped or missing green channel")? };
        let Some(Ok(b)) = self.get(4..6).map(|r| u32::from_str_radix(r, 16)) else { Err("invalid color code, mistyped or missing blue channel")? };

        let a = if let Some(a) = self.get(6..8) {
            let Ok(a) = u32::from_str_radix(a, 16) else { Err("invalid color code, mistyped alpha channel")? };
            a
        } else { 255 };

        Ok(Color {
            r: r as u8,
            g: g as u8,
            b: b as u8,
            a: a as u8
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
