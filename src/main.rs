// Copyright (C) Pavel Grebnev 2023-2025
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

#![windows_subsystem = "windows"]

mod app_arguments;
mod color_utils;
mod config;
mod config_updaters;
mod custom_keybinds;
mod execution_thread;
mod file_utils;
mod git_support;
mod json_file_updater;
mod key_mapping;
mod keybind_editing;
mod main_window;
mod main_window_utils;
mod main_window_widgets;
mod parallel_execution_manager;
mod ring_buffer;
mod style;
mod ui_icons;

use iced::window::icon;
use iced::{Application, Settings};

pub fn main() -> iced::Result {
    if let Some(e) = config::get_arguments_read_error() {
        eprintln!("{}", e);
        return Ok(());
    }

    let mut settings = Settings::default();
    if let Ok(icon) = icon::from_rgba(include_bytes!("../res/icon.rgba").to_vec(), 128, 128) {
        settings.window.icon = Some(icon);
    }
    settings.window.position = iced::window::Position::Centered;
    main_window::MainWindow::run(settings)
}
