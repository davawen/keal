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

impl application::DefaultStyle for Theme {
    fn default_style(&self) -> application::Appearance {
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

impl text::Catalog for Theme {
    type Class<'a> = TextStyle;

    fn style(&self, style: &Self::Class<'_>) -> text::Style {
        text::Style { 
            color: Some(match style {
                TextStyle::Normal => self.text,
                TextStyle::Matched { selected: false } => self.matched_text,
                TextStyle::Matched { selected: true } => self.selected_matched_text,
                TextStyle::Comment => self.comment
            })
        }
    }

    fn default<'a>() -> Self::Class<'a> {
        TextStyle::default()
    }
}

impl text_input::Catalog for Theme {
    type Class<'a> = ();

    fn default<'a>() -> Self::Class<'a> { () }

    fn style(&self, class: &Self::Class<'_>, _status: text_input::Status) -> text_input::Style {
        text_input::Style {
            background: self.input_background.into(),
            border: iced::Border {
                color: Color::TRANSPARENT,
                width: 0.0,
                radius: iced::border::top(5.0)
            },
            icon: Color::TRANSPARENT,
            value: self.text,
            placeholder: self.input_placeholder,
            selection: self.input_selection
        }
    }
}

#[derive(Default)]
pub enum ButtonStyle {
    #[default]
    Normal,
    Selected
}

impl button::Catalog for Theme {
    type Class<'a> = ButtonStyle;

    fn default<'a>() -> Self::Class<'a> { ButtonStyle::default() }

    fn style(&self, style: &Self::Class<'_>, status: button::Status) -> button::Style {
        button::Style {
            background: Some(match status {
                button::Status::Active => match style {
                    ButtonStyle::Normal => self.choice_background,
                    ButtonStyle::Selected => self.selected_choice_background
                }
                button::Status::Hovered => self.hovered_choice_background,
                button::Status::Pressed => self.pressed_choice_background,
                button::Status::Disabled => color!(0xdddddd)
            }.into()),
            text_color: self.text,
            ..Default::default()
        }
    }
}

impl container::Catalog for Theme {
    type Class<'a> = ();

    fn default<'a>() -> Self::Class<'a> { () }

    fn style(&self, class: &Self::Class<'_>) -> container::Style {
        container::Style { text_color: Some(self.text), ..Default::default() }
    }
}

impl scrollable::Catalog for Theme {
    type Class<'a> = ();

    fn default<'a>() -> Self::Class<'a> { () }

    fn style(&self, class: &Self::Class<'_>, status: scrollable::Status) -> scrollable::Style {
        let mut style = scrollable::Style {
            container: container::Style::default(),
            gap: None,
            vertical_rail: scrollable::Rail {
                background: None,
                scroller: scrollable::Scroller {
                    color: if self.scrollbar_enabled { self.scrollbar } else { Color::TRANSPARENT },
                    border: iced::Border::default(),
                },
                border: iced::Border::default()
            },
            horizontal_rail: scrollable::Rail {
                border: iced::Border::default(),
                background: None,
                scroller: scrollable::Scroller {
                    color: Color::TRANSPARENT,
                    border: iced::Border::default()
                }
            }
        };

        match status {
            scrollable::Status::Hovered { .. } | scrollable::Status::Dragged { .. } if self.scrollbar_enabled => {
                style.vertical_rail.scroller.color = self.hovered_scrollbar;
            }
            _ => ()
        }

        style
    }
}

impl svg::Catalog for Theme {
    type Class<'a> = ();

    fn style(&self, _: &Self::Class<'_>, _status: svg::Status) -> svg::Style {
        svg::Style::default()
    }

    fn default<'a>() -> Self::Class<'a> { () }
}
