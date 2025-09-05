// Copyright (C) Pavel Grebnev 2023
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use iced::widget::image::Handle;
use iced::Color;

#[derive(Clone)]
pub struct ThemedIcons {
    pub play: Handle,
    pub stop: Handle,
    pub retry: Handle,
    pub remove: Handle,
    pub plus: Handle,
    pub settings: Handle,
    pub up: Handle,
    pub down: Handle,
    pub log: Handle,
    pub edit: Handle,
    pub quick_launch: Handle,
}

pub struct IconCaches {
    pub idle: Handle,
    pub in_progress: Handle,
    pub succeeded: Handle,
    pub failed: Handle,
    pub skipped: Handle,

    pub bright: ThemedIcons,
    pub dark: ThemedIcons,

    pub themed: ThemedIcons,
}

impl IconCaches {
    pub fn new() -> Self {
        let bright = get_bright_themed_icons();
        let dark = get_dark_themed_icons();

        Self {
            idle: Handle::from_bytes(
                include_bytes!("../res/icons/idle.png")
                    .into_iter()
                    .as_slice(),
            ),
            in_progress: Handle::from_bytes(
                include_bytes!("../res/icons/in-progress.png")
                    .into_iter()
                    .as_slice(),
            ),
            succeeded: Handle::from_bytes(
                include_bytes!("../res/icons/positive.png")
                    .into_iter()
                    .as_slice(),
            ),
            failed: Handle::from_bytes(
                include_bytes!("../res/icons/negative.png")
                    .into_iter()
                    .as_slice(),
            ),
            skipped: Handle::from_bytes(
                include_bytes!("../res/icons/skip.png")
                    .into_iter()
                    .as_slice(),
            ),

            bright,
            dark: dark.clone(),

            themed: dark,
        }
    }

    pub fn get_theme_for_color(&self, color: Color) -> &ThemedIcons {
        if color.r > 0.5 {
            &self.bright
        } else {
            &self.dark
        }
    }
}

fn get_dark_themed_icons() -> ThemedIcons {
    ThemedIcons {
        play: Handle::from_bytes(
            include_bytes!("../res/icons/play-b.png")
                .into_iter()
                .as_slice(),
        ),
        stop: Handle::from_bytes(
            include_bytes!("../res/icons/stop-b.png")
                .into_iter()
                .as_slice(),
        ),
        retry: Handle::from_bytes(
            include_bytes!("../res/icons/retry-b.png")
                .into_iter()
                .as_slice(),
        ),
        remove: Handle::from_bytes(
            include_bytes!("../res/icons/remove-b.png")
                .into_iter()
                .as_slice(),
        ),
        plus: Handle::from_bytes(
            include_bytes!("../res/icons/plus-b.png")
                .into_iter()
                .as_slice(),
        ),
        settings: Handle::from_bytes(
            include_bytes!("../res/icons/settings-b.png")
                .into_iter()
                .as_slice(),
        ),
        up: Handle::from_bytes(
            include_bytes!("../res/icons/up-b.png")
                .into_iter()
                .as_slice(),
        ),
        down: Handle::from_bytes(
            include_bytes!("../res/icons/down-b.png")
                .into_iter()
                .as_slice(),
        ),
        log: Handle::from_bytes(
            include_bytes!("../res/icons/log-b.png")
                .into_iter()
                .as_slice(),
        ),
        edit: Handle::from_bytes(
            include_bytes!("../res/icons/edit-b.png")
                .into_iter()
                .as_slice(),
        ),
        quick_launch: Handle::from_bytes(
            include_bytes!("../res/icons/quick_launch-b.png")
                .into_iter()
                .as_slice(),
        ),
    }
}

fn get_bright_themed_icons() -> ThemedIcons {
    ThemedIcons {
        play: Handle::from_bytes(
            include_bytes!("../res/icons/play-w.png")
                .into_iter()
                .as_slice(),
        ),
        stop: Handle::from_bytes(
            include_bytes!("../res/icons/stop-w.png")
                .into_iter()
                .as_slice(),
        ),
        retry: Handle::from_bytes(
            include_bytes!("../res/icons/retry-w.png")
                .into_iter()
                .as_slice(),
        ),
        remove: Handle::from_bytes(
            include_bytes!("../res/icons/remove-w.png")
                .into_iter()
                .as_slice(),
        ),
        plus: Handle::from_bytes(
            include_bytes!("../res/icons/plus-w.png")
                .into_iter()
                .as_slice(),
        ),
        settings: Handle::from_bytes(
            include_bytes!("../res/icons/settings-w.png")
                .into_iter()
                .as_slice(),
        ),
        up: Handle::from_bytes(
            include_bytes!("../res/icons/up-w.png")
                .into_iter()
                .as_slice(),
        ),
        down: Handle::from_bytes(
            include_bytes!("../res/icons/down-w.png")
                .into_iter()
                .as_slice(),
        ),
        log: Handle::from_bytes(
            include_bytes!("../res/icons/log-w.png")
                .into_iter()
                .as_slice(),
        ),
        edit: Handle::from_bytes(
            include_bytes!("../res/icons/edit-w.png")
                .into_iter()
                .as_slice(),
        ),
        quick_launch: Handle::from_bytes(
            include_bytes!("../res/icons/quick_launch-w.png")
                .into_iter()
                .as_slice(),
        ),
    }
}
