// Copyright (C) Pavel Grebnev 2023-2025
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

#![windows_subsystem = "windows"]

mod app_arguments;
mod color_utils;
mod config;
mod config_updaters;
mod custom_keybinds;
mod drag_and_drop_list;
mod events;
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

pub fn main() -> iced::Result {
    if let Some(e) = config::get_arguments_read_error() {
        eprintln!("{}", e);
        return Ok(());
    }

    let icon = icon::from_rgba(include_bytes!("../res/icon.rgba").to_vec(), 128, 128);
    let icon = if let Ok(icon) = icon {
        Some(icon)
    } else {
        None
    };
    let window_settings = iced::window::Settings {
        position: iced::window::Position::Centered,
        icon,
        ..Default::default()
    };
    iced::application(
        main_window::MainWindow::title,
        main_window::MainWindow::update,
        main_window::MainWindow::view,
    )
    .window(window_settings)
    .subscription(main_window::MainWindow::subscription)
    .theme(main_window::MainWindow::theme)
    .run_with(main_window::MainWindow::new)
}
