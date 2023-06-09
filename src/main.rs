#![windows_subsystem = "windows"]

mod config;
mod config_updaters;
mod execution;
mod json_config_updater;
mod main_window;
mod ring_buffer;
mod style;

use iced::window::icon;
use iced::{Application, Settings};

pub fn main() -> iced::Result {
    let mut settings = Settings::default();
    if let Ok(icon) = icon::from_rgba(include_bytes!("../res/icon.rgba").to_vec(), 128, 128) {
        settings.window.icon = Some(icon);
    }
    settings.window.position = iced::window::Position::Centered;
    settings.window.always_on_top = config::is_always_on_top();
    main_window::MainWindow::run(settings)
}
