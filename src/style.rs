// Copyright (C) Pavel Grebnev 2023-2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use crate::config;
use iced::theme::{self, Theme};
use iced::widget::{container, text_input};
use iced::Border;

pub fn title_bar_active(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        text_color: Some(palette.background.strong.text),
        background: Some(palette.background.strong.color.into()),
        ..Default::default()
    }
}

pub fn title_bar_focused(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        text_color: Some(palette.primary.strong.text),
        background: Some(palette.primary.strong.color.into()),
        ..Default::default()
    }
}

pub fn title_bar_focused_completed(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        text_color: Some(palette.background.weak.text),
        background: Some(palette.success.strong.color.into()),
        ..Default::default()
    }
}

pub fn title_bar_focused_failed(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        text_color: Some(palette.background.weak.text),
        background: Some(palette.danger.base.color.into()),
        ..Default::default()
    }
}

pub fn pane_active(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
        background: Some(palette.background.weak.color.into()),
        border: Border {
            color: palette.background.strong.color,
            width: 2.0,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub fn pane_focused(theme: &Theme) -> container::Style {
    let palette = theme.extended_palette();

    container::Style {
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

pub(crate) fn invalid_text_input_style(
    theme: &Theme,
    status: text_input::Status,
) -> text_input::Style {
    let default_theme = text_input::default(theme, status);

    match status {
        text_input::Status::Active => text_input::Style {
            border: Border {
                color: theme.extended_palette().danger.base.color,
                ..default_theme.border
            },
            ..default_theme
        },
        text_input::Status::Hovered => default_theme,
        text_input::Status::Focused => text_input::Style {
            border: Border {
                color: theme.extended_palette().danger.strong.color,
                ..default_theme.border
            },
            ..default_theme
        },
        text_input::Status::Disabled => default_theme,
    }
}
