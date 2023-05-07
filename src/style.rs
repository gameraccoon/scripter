use iced::theme::{self, Theme};
use iced::widget::container;

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

pub fn get_dark_theme() -> Theme {
    Theme::custom(theme::Palette {
        background: iced::Color::from_rgb(0.25, 0.26, 0.29),
        text: iced::Color::BLACK,
        primary: iced::Color::from_rgb(0.44, 0.53, 0.855),
        success: iced::Color::from_rgb(0.31, 0.50, 0.17),
        danger: iced::Color::from_rgb(1.0, 0.0, 0.0),
    })
}
