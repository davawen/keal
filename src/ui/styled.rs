use iced::{
    widget::{button, text_input, text, container, scrollable, svg},
    Color, application,
};

/// Workaround for the iced `color!` macro not supporting const contexts
#[macro_export]
macro_rules! color {
    [$r:literal, $g:literal, $b:literal] => {
        Color { r: $r, g: $g, b: $b, a: 1.0 }
    };
    [$r:literal, $g:literal, $b:literal, $a:literal] => {
        Color { r: $r, g: $g, b: $b, a: $a }
    };
    ($hex:literal, $alpha:literal) => {
        Color {
            r: (($hex >> 16) & 0xFF) as f32 / 255.0,
            g: (($hex >> 8) & 0xFF) as f32 / 255.0,
            b: ($hex & 0xFF) as f32 / 255.0,
            a: $alpha
        }
    };
    ($hex:literal) => {
        color!($hex, 1.0)
    };
}

#[derive(Default)]
pub struct Theme {
    pub text_color: Color
}

impl application::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _: &Self::Style) -> application::Appearance {
        application::Appearance {
            text_color: self.text_color,
            background_color: color!(0x24273a) 
        }
    }
}

pub const MATCHED_TEXT_COLOR: Color = color!(0xa6da95);
pub const SELECTED_MATCHED_TEXT_COLOR: Color = color!(0xeed49f);
pub const COMMENT_COLOR: Color = color!(0xa5adcb);

impl text::StyleSheet for Theme {
    type Style = Option<Color>;
    fn appearance(&self, style: Self::Style) -> text::Appearance {
        text::Appearance { 
            color: Some(style.unwrap_or(self.text_color))
        }
    }
}

impl text_input::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: color!(0x363a4f).into(),
            border_radius: [5.0, 5.0, 0.0, 0.0].into(),
            icon_color: Color::TRANSPARENT, border_width: 0.0, border_color: Color::TRANSPARENT
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance { self.active(style) }
    fn disabled(&self, style: &Self::Style) -> text_input::Appearance { self.active(style) }

    fn placeholder_color(&self, _: &Self::Style) -> Color { color!(0xa5adcb) }
    fn value_color(&self, _: &Self::Style) -> Color { self.text_color }
    fn disabled_color(&self, style: &Self::Style) -> Color { self.value_color(style) }
    fn selection_color(&self, _: &Self::Style) -> Color { color!(0xb4d5ff) }
}

#[derive(Default)]
pub enum ButtonState {
    #[default]
    Normal,
    Selected
}

impl button::StyleSheet for Theme {
    type Style = ButtonState;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(match style {
                ButtonState::Normal => color!(0x24273a),
                ButtonState::Selected => color!(0x494d64)
            }.into()),
            text_color: self.text_color,
            ..Default::default()
        }
    }

    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(color!(0x363a4f).into()),
            text_color: self.text_color,
            ..Default::default()
        }
    }

    fn pressed(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(color!(0x181926).into()),
            text_color: self.text_color,
            ..Default::default()
        }
    }
}

impl container::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(self.text_color),
            ..Default::default()
            // background: (),
            // border_radius: (), 
            // border_width: (),
            // border_color: ()
        }
    }
}

impl scrollable::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _: &Self::Style) -> scrollable::Scrollbar {
        scrollable::Scrollbar {
            background: None,
            border_radius: 0.0.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
            scroller: scrollable::Scroller {
                color: color!(0xdddddd),
                border_color: Color::TRANSPARENT,
                border_radius: 0.0.into(),
                border_width: 0.0
            }
        }
    }

    fn hovered(
            &self,
            style: &Self::Style,
            is_mouse_over_scrollbar: bool,
        ) -> scrollable::Scrollbar {
        let mut normal = self.active(style);
        if is_mouse_over_scrollbar {
            normal.scroller.color = color!(0xeeeeee);
        }
        normal
    }
}

impl svg::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _: &Self::Style) -> svg::Appearance {
        svg::Appearance { color: None }
    }
}
