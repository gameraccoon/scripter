use iced::theme::{self, Theme};
use iced::widget::container;

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
        border_width: 2.0,
        border_color: palette.background.strong.color,
        ..Default::default()
    }
}

pub fn pane_focused(theme: &Theme) -> container::Appearance {
    let palette = theme.extended_palette();

    container::Appearance {
        background: Some(palette.background.weak.color.into()),
        border_width: 2.0,
        border_color: palette.primary.strong.color,
        ..Default::default()
    }
}

pub fn get_custom_theme(custom_config: config::CustomTheme) -> Theme {
    Theme::custom(theme::Palette {
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
    })
}
