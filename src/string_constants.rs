// Copyright (C) Pavel Grebnev 2023-2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

// depending on whether it's macOS or not we need to name the key Ctrl or Command
#[cfg(target_os = "macos")]
pub const FILTER_COMMAND_HINT: &str = "Command+F to focus Filter";
#[cfg(not(target_os = "macos"))]
pub const FILTER_COMMAND_HINT: &str = "Ctrl+F to focus Filter";
#[cfg(target_os = "macos")]
pub const RUN_COMMAND_HINT: &str = "Command+R to Run";
#[cfg(not(target_os = "macos"))]
pub const RUN_COMMAND_HINT: &str = "Ctrl+R to Run";
#[cfg(target_os = "macos")]
pub const RESCHEDULE_COMMAND_HINT: &str = "Command+Shift+R to Reschedule";
#[cfg(not(target_os = "macos"))]
pub const RESCHEDULE_COMMAND_HINT: &str = "Ctrl+Shift+R to Reschedule";
#[cfg(target_os = "macos")]
pub const STOP_COMMAND_HINT: &str = "Command+Shift+C to Stop";
#[cfg(not(target_os = "macos"))]
pub const STOP_COMMAND_HINT: &str = "Ctrl+Shift+C to Stop";
#[cfg(target_os = "macos")]
pub const CLEAR_COMMAND_HINT: &str = "Command+C to Clear";
#[cfg(not(target_os = "macos"))]
pub const CLEAR_COMMAND_HINT: &str = "Ctrl+C to Clear";
#[cfg(target_os = "macos")]
pub const EDIT_COMMAND_HINT: &str = "Command+E to Edit";
#[cfg(not(target_os = "macos"))]
pub const EDIT_COMMAND_HINT: &str = "Ctrl+E to Edit";
#[cfg(target_os = "macos")]
pub const FOCUS_COMMAND_HINT: &str = "Command+Q to Focus";
#[cfg(not(target_os = "macos"))]
pub const FOCUS_COMMAND_HINT: &str = "Ctrl+Q to Focus";
#[cfg(target_os = "macos")]
pub const UNFOCUS_COMMAND_HINT: &str = "Command+Q to Restore full window";
#[cfg(not(target_os = "macos"))]
pub const UNFOCUS_COMMAND_HINT: &str = "Ctrl+Q to Restore full window";
