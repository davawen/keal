use iced::{
    widget::{button, text_input, text, container, scrollable, svg},
    Color, application,
};

use crate::config::Theme;

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

impl application::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _: &Self::Style) -> application::Appearance {
        application::Appearance {
            text_color: self.text,
            background_color: self.background 
        }
    }
}

#[derive(Default, Clone)]
pub enum TextStyle {
    #[default]
    Normal,
    Matched {
        selected: bool
    },
    Comment
}

impl text::StyleSheet for Theme {
    type Style = TextStyle;
    fn appearance(&self, style: Self::Style) -> text::Appearance {
        text::Appearance { 
            color: Some(match style {
                TextStyle::Normal => self.text,
                TextStyle::Matched { selected: false } => self.matched_text,
                TextStyle::Matched { selected: true } => self.selected_matched_text,
                TextStyle::Comment => self.comment
            })
        }
    }
}

impl text_input::StyleSheet for Theme {
    type Style = ();

    fn active(&self, _: &Self::Style) -> text_input::Appearance {
        text_input::Appearance {
            background: self.input_background.into(),
            border_radius: [5.0, 5.0, 0.0, 0.0].into(),
            icon_color: Color::TRANSPARENT, border_width: 0.0, border_color: Color::TRANSPARENT
        }
    }

    fn focused(&self, style: &Self::Style) -> text_input::Appearance { self.active(style) }
    fn disabled(&self, style: &Self::Style) -> text_input::Appearance { self.active(style) }

    fn placeholder_color(&self, _: &Self::Style) -> Color { self.input_placeholder }
    fn value_color(&self, _: &Self::Style) -> Color { self.text }
    fn disabled_color(&self, style: &Self::Style) -> Color { self.value_color(style) }
    fn selection_color(&self, _: &Self::Style) -> Color { self.input_selection }
}

#[derive(Default)]
pub enum ButtonStyle {
    #[default]
    Normal,
    Selected
}

impl button::StyleSheet for Theme {
    type Style = ButtonStyle;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(match style {
                ButtonStyle::Normal => self.choice_background,
                ButtonStyle::Selected => self.selected_choice_background
            }.into()),
            text_color: self.text,
            ..Default::default()
        }
    }

    fn hovered(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(self.hovered_choice_background.into()),
            text_color: self.text,
            ..Default::default()
        }
    }

    fn pressed(&self, _: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(self.pressed_choice_background.into()),
            text_color: self.text,
            ..Default::default()
        }
    }
}

impl container::StyleSheet for Theme {
    type Style = ();

    fn appearance(&self, _: &Self::Style) -> container::Appearance {
        container::Appearance {
            text_color: Some(self.text),
            ..Default::default()
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
                color: if self.scrollbar_enabled { self.scrollbar } else { Color::TRANSPARENT },
                border_color: Color::TRANSPARENT,
                border_radius: self.scrollbar_border_radius.into(),
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
        if is_mouse_over_scrollbar && self.scrollbar_enabled {
            normal.scroller.color = self.hovered_scrollbar;
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
