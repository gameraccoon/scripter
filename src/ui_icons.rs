// Copyright (C) Pavel Grebnev 2023
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use iced::widget::image::Handle;

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
        Self {
            idle: Handle::from_memory(include_bytes!("../res/icons/idle.png")),
            in_progress: Handle::from_memory(include_bytes!("../res/icons/in-progress.png")),
            succeeded: Handle::from_memory(include_bytes!("../res/icons/positive.png")),
            failed: Handle::from_memory(include_bytes!("../res/icons/negative.png")),
            skipped: Handle::from_memory(include_bytes!("../res/icons/skip.png")),

            bright: ThemedIcons {
                play: Handle::from_memory(include_bytes!("../res/icons/play-w.png")),
                stop: Handle::from_memory(include_bytes!("../res/icons/stop-w.png")),
                retry: Handle::from_memory(include_bytes!("../res/icons/retry-w.png")),
                remove: Handle::from_memory(include_bytes!("../res/icons/remove-w.png")),
                plus: Handle::from_memory(include_bytes!("../res/icons/plus-w.png")),
                settings: Handle::from_memory(include_bytes!("../res/icons/settings-w.png")),
                up: Handle::from_memory(include_bytes!("../res/icons/up-w.png")),
                down: Handle::from_memory(include_bytes!("../res/icons/down-w.png")),
                log: Handle::from_memory(include_bytes!("../res/icons/log-w.png")),
                edit: Handle::from_memory(include_bytes!("../res/icons/edit-w.png")),
                quick_launch: Handle::from_memory(include_bytes!(
                    "../res/icons/quick_launch-w.png"
                )),
            },
            dark: ThemedIcons {
                play: Handle::from_memory(include_bytes!("../res/icons/play-b.png")),
                stop: Handle::from_memory(include_bytes!("../res/icons/stop-b.png")),
                retry: Handle::from_memory(include_bytes!("../res/icons/retry-b.png")),
                remove: Handle::from_memory(include_bytes!("../res/icons/remove-b.png")),
                plus: Handle::from_memory(include_bytes!("../res/icons/plus-b.png")),
                settings: Handle::from_memory(include_bytes!("../res/icons/settings-b.png")),
                up: Handle::from_memory(include_bytes!("../res/icons/up-b.png")),
                down: Handle::from_memory(include_bytes!("../res/icons/down-b.png")),
                log: Handle::from_memory(include_bytes!("../res/icons/log-b.png")),
                edit: Handle::from_memory(include_bytes!("../res/icons/edit-b.png")),
                quick_launch: Handle::from_memory(include_bytes!(
                    "../res/icons/quick_launch-b.png"
                )),
            },

            themed: ThemedIcons {
                play: Handle::from_memory(include_bytes!("../res/icons/play-b.png")),
                stop: Handle::from_memory(include_bytes!("../res/icons/stop-b.png")),
                retry: Handle::from_memory(include_bytes!("../res/icons/retry-b.png")),
                remove: Handle::from_memory(include_bytes!("../res/icons/remove-b.png")),
                plus: Handle::from_memory(include_bytes!("../res/icons/plus-b.png")),
                settings: Handle::from_memory(include_bytes!("../res/icons/settings-b.png")),
                up: Handle::from_memory(include_bytes!("../res/icons/up-b.png")),
                down: Handle::from_memory(include_bytes!("../res/icons/down-b.png")),
                log: Handle::from_memory(include_bytes!("../res/icons/log-b.png")),
                edit: Handle::from_memory(include_bytes!("../res/icons/edit-b.png")),
                quick_launch: Handle::from_memory(include_bytes!(
                    "../res/icons/quick_launch-b.png"
                )),
            },
        }
    }
}
