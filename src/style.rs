// Copyright (C) Pavel Grebnev 2023-2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use iced::border::Radius;
use iced::theme::{self, Theme};
use iced::widget::container;
use iced::Border;

use crate::config;

pub fn title_bar_active(theme: &Theme) -> container::Appearance {
    let palette = theme.extended_palette();

    container::Appearance {
        text_color: Some(palette.background.strong.text),
        background: Some(palette.background.strong.color.into()),
        ..Default::default()
    }
}

pub fn title_bar_focused(theme: &Theme) -> container::Appearance {
    let palette = theme.extended_palette();

    container::Appearance {
        text_color: Some(palette.primary.strong.text),
        background: Some(palette.primary.strong.color.into()),
        ..Default::default()
    }
}

pub fn title_bar_focused_completed(theme: &Theme) -> container::Appearance {
    let palette = theme.extended_palette();

    container::Appearance {
        text_color: Some(palette.background.weak.text),
        background: Some(palette.success.strong.color.into()),
        ..Default::default()
    }
}

pub fn title_bar_focused_failed(theme: &Theme) -> container::Appearance {
    let palette = theme.extended_palette();

    container::Appearance {
        text_color: Some(palette.background.weak.text),
        background: Some(palette.danger.base.color.into()),
        ..Default::default()
    }
}

pub fn pane_active(theme: &Theme) -> container::Appearance {
    let palette = theme.extended_palette();

    container::Appearance {
        background: Some(palette.background.weak.color.into()),
        border: Border {
            color: palette.background.strong.color,
            width: 2.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn pane_focused(theme: &Theme) -> container::Appearance {
    let palette = theme.extended_palette();

    container::Appearance {
        background: Some(palette.background.weak.color.into()),
        border: Border {
            color: palette.primary.strong.color,
            width: 2.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn get_custom_theme(custom_config: config::CustomTheme) -> Theme {
    Theme::custom(
        "custom".to_string(),
        theme::Palette {
            background: iced::Color::from_rgb(
                custom_config.background[0],
                custom_config.background[1],
                custom_config.background[2],
            ),
            text: iced::Color::from_rgb(
                custom_config.text[0],
                custom_config.text[1],
                custom_config.text[2],
            ),
            primary: iced::Color::from_rgb(
                custom_config.primary[0],
                custom_config.primary[1],
                custom_config.primary[2],
            ),
            success: iced::Color::from_rgb(
                custom_config.success[0],
                custom_config.success[1],
                custom_config.success[2],
            ),
            danger: iced::Color::from_rgb(
                custom_config.danger[0],
                custom_config.danger[1],
                custom_config.danger[2],
            ),
        },
    )
}

pub struct InvalidInputStyleSheet;

impl iced::widget::text_input::StyleSheet for InvalidInputStyleSheet {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> iced::widget::text_input::Appearance {
        iced::widget::text_input::Appearance {
            background: iced::Background::Color(style.extended_palette().background.base.color),
            border: Border {
                color: style.extended_palette().danger.base.color,
                width: 1.0,
                radius: Radius::from(1.0),
            },
            icon_color: iced::Color::WHITE,
        }
    }

    fn focused(&self, style: &Self::Style) -> iced::widget::text_input::Appearance {
        iced::widget::text_input::Appearance {
            background: iced::Background::Color(style.extended_palette().background.base.color),
            border: Border {
                color: style.extended_palette().danger.strong.color,
                ..self.active(style).border
            },
            ..self.active(style)
        }
    }

    fn placeholder_color(&self, style: &Self::Style) -> iced::Color {
        style.extended_palette().background.strong.color
    }

    fn value_color(&self, style: &Self::Style) -> iced::Color {
        style.extended_palette().background.strong.text
    }

    fn disabled_color(&self, style: &Self::Style) -> iced::Color {
        style.extended_palette().background.weak.text
    }

    fn selection_color(&self, style: &Self::Style) -> iced::Color {
        style.extended_palette().background.strong.text
    }

    fn hovered(&self, style: &Self::Style) -> iced::widget::text_input::Appearance {
        iced::widget::text_input::Appearance {
            background: iced::Background::Color(style.extended_palette().background.base.color),
            border: Border {
                color: style.extended_palette().danger.strong.text,
                ..self.active(style).border
            },
            ..self.active(style)
        }
    }

    fn disabled(&self, style: &Self::Style) -> iced::widget::text_input::Appearance {
        iced::widget::text_input::Appearance {
            background: iced::Background::Color(style.extended_palette().background.weak.color),
            ..self.active(style)
        }
    }
}
