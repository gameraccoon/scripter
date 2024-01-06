// Copyright (C) Pavel Grebnev 2023-2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

#![windows_subsystem = "windows"]

mod color_utils;
mod config;
mod config_updaters;
mod execution;
mod execution_lists;
mod file_utils;
mod json_file_updater;
mod main_window;
mod ring_buffer;
mod string_constants;
mod style;
mod ui_icons;

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
