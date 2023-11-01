use iced::{
    color,
    widget::{button, text_input},
    Theme, Color,
};
use lazy_static::lazy_static;

lazy_static! {
    pub static ref MATCHED_TEXT_COLOR: Color = color!(0xa6da95);
    pub static ref SELECTED_MATCHED_TEXT_COLOR: Color = color!(0xeed49f);
    pub static ref COMMENT_COLOR: Color = color!(0xa5adcb);
}

pub struct Input;
impl text_input::StyleSheet for Input {
    type Style = Theme;

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
    fn value_color(&self, style: &Self::Style) -> Color { style.palette().text }
    fn disabled_color(&self, style: &Self::Style) -> Color { self.value_color(style) }
    fn selection_color(&self, style: &Self::Style) -> Color { style.palette().primary }
}

pub struct Item;

impl button::StyleSheet for Item {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(color!(0x24273a).into()),
            text_color: style.palette().text,
            ..Default::default()
        }
    }

    fn hovered(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(color!(0x363a4f).into()),
            text_color: style.palette().text,
            ..Default::default()
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(color!(0x181926).into()),
            text_color: style.palette().text,
            ..Default::default()
        }
    }
}

pub struct SelectedItem;

impl button::StyleSheet for SelectedItem {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(color!(0x494d64).into()),
            text_color: style.palette().text,
            ..Default::default()
        }
    }

    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(color!(0x181926).into()),
            text_color: style.palette().text,
            ..Default::default()
        }
    }
}
