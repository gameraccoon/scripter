// Copyright (C) Pavel Grebnev 2023-2025
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use crate::main_window::*;
use crate::main_window_utils::*;
use crate::{config, keybind_editing};
use iced::advanced::image::Handle;
use iced::widget::text::LineHeight;
use iced::widget::{
    button, horizontal_rule, image, pick_list, row, text, text_input, tooltip, Button, Column,
    Space,
};
use iced::{alignment, theme, Alignment, Element, Length, Theme};

const SEPARATOR_HEIGHT: u16 = 8;

pub(crate) const PATH_TYPE_PICK_LIST: &[config::PathType] = &[
    config::PathType::WorkingDirRelative,
    config::PathType::ScripterExecutableRelative,
];

impl std::fmt::Display for config::PathType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                config::PathType::WorkingDirRelative => "Path relative to working directory",
                config::PathType::ScripterExecutableRelative =>
                    "Path relative to scripter executable",
            }
        )
    }
}

pub(crate) const ARGUMENT_REQUIREMENT_PICK_LIST: &[config::ArgumentRequirement] = &[
    config::ArgumentRequirement::Required,
    config::ArgumentRequirement::Optional,
    config::ArgumentRequirement::Hidden,
];

impl std::fmt::Display for config::ArgumentRequirement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                config::ArgumentRequirement::Required => "Required",
                config::ArgumentRequirement::Optional => "Optional",
                config::ArgumentRequirement::Hidden => "Hidden",
            }
        )
    }
}

pub fn edit_button(label: &str, message: WindowMessage) -> Button<WindowMessage> {
    button(
        text(label)
            .vertical_alignment(alignment::Vertical::Center)
            .size(16),
    )
    .padding(4)
    .on_press(message)
}

pub fn populate_quick_launch_edit_button<'a>(
    content: &mut Vec<Element<'a, WindowMessage, Theme, iced::Renderer>>,
    visual_caches: &VisualCaches,
    script_uid: &config::Guid,
    window_edit_data: &Option<WindowEditData>,
) {
    if let Some(window_edit_data) = window_edit_data {
        if window_edit_data.edit_type == ConfigEditType::Local {
            content.push(horizontal_rule(SEPARATOR_HEIGHT).into());
            if is_script_in_quick_launch_buttons(&visual_caches, &script_uid) {
                content.push(
                    edit_button(
                        "Remove from quick launch panel",
                        WindowMessage::RemoveFromQuickLaunchPanel(script_uid.clone()),
                    )
                    .into(),
                );
            } else {
                content.push(
                    edit_button(
                        "Add to quick launch panel",
                        WindowMessage::AddToQuickLaunchPanel(script_uid.clone()),
                    )
                    .into(),
                );
            }
        }
    }
}

pub fn populate_path_editing_content(
    caption: &str,
    hint: &str,
    path: &config::PathConfig,
    edit_content: &mut Vec<Element<'_, WindowMessage, Theme, iced::Renderer>>,
    on_path_changed: impl Fn(String) -> WindowMessage + 'static,
    on_path_type_changed: impl Fn(config::PathType) -> WindowMessage + 'static,
) {
    edit_content.push(text(caption).into());
    edit_content.push(
        text_input(hint, &path.path)
            .on_input(on_path_changed)
            .padding(5)
            .into(),
    );
    edit_content.push(
        pick_list(
            PATH_TYPE_PICK_LIST,
            Some(path.path_type),
            on_path_type_changed,
        )
        .into(),
    );
}

pub fn inline_icon_button<'a, Message>(
    icon_handle: Handle,
    message: Message,
) -> Button<'a, Message> {
    button(
        image(icon_handle)
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(14.0)),
    )
    .padding(4)
    .on_press(message)
}

pub fn quick_launch_button(button_description: &QuickLaunchButton) -> Element<WindowMessage> {
    tooltip(
        button(
            image(button_description.icon.clone())
                .width(Length::Fixed(22.0))
                .height(Length::Fixed(22.0)),
        )
        .style(theme::Button::Secondary)
        .on_press(WindowMessage::OnQuickLaunchButtonPressed(
            button_description.script_uid.clone(),
        ))
        .padding(4),
        button_description.label.as_str(),
        tooltip::Position::Top,
    )
    .style(theme::Container::Box)
    .into()
}

pub fn main_icon_button(
    icon_handle: Handle,
    label: &str,
    message: Option<WindowMessage>,
) -> Button<WindowMessage> {
    let new_button = button(
        row![
            image(icon_handle)
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0)),
            Space::with_width(4),
            text(label).width(Length::Shrink).size(16),
        ]
        .align_items(Alignment::Center),
    )
    .width(Length::Shrink)
    .padding(8);

    if let Some(message) = message {
        new_button.on_press(message)
    } else {
        new_button
    }
}

pub fn main_icon_button_string(
    icon_handle: Handle,
    label: String,
    message: Option<WindowMessage>,
) -> Button<'static, WindowMessage> {
    let new_button = button(
        row![
            image(icon_handle)
                .width(Length::Fixed(16.0))
                .height(Length::Fixed(16.0)),
            Space::with_width(4),
            text(label.to_string()).width(Length::Shrink).size(16),
        ]
        .align_items(Alignment::Center),
    )
    .width(Length::Shrink)
    .padding(8);

    if let Some(message) = message {
        new_button.on_press(message)
    } else {
        new_button
    }
}

pub fn main_button(label: &str, message: Option<WindowMessage>) -> Button<WindowMessage> {
    let new_button = button(row![text(label).width(Length::Shrink).size(16)])
        .width(Length::Shrink)
        .padding(8);

    if let Some(message) = message {
        new_button.on_press(message)
    } else {
        new_button
    }
}

pub fn edit_mode_button<'a>(
    icon_handle: Handle,
    message: WindowMessage,
    window_state: &WindowState,
    visual_caches: &VisualCaches,
) -> Button<'a, WindowMessage> {
    let icon = image(icon_handle)
        .width(Length::Fixed(12.0))
        .height(Length::Fixed(12.0));

    button(if window_state.is_command_key_down {
        row![
            text(format_keybind_hint(
                visual_caches,
                "Edit",
                config::AppAction::TrySwitchWindowEditMode
            ))
            .size(12)
            .line_height(LineHeight::Absolute(iced::Pixels(12.0))),
            Space::with_width(4),
            icon
        ]
        .align_items(Alignment::Center)
    } else {
        row![icon]
    })
    .style(theme::Button::Secondary)
    .width(Length::Shrink)
    .padding(4)
    .on_press(message)
}

pub fn get_pane_name_from_variant(variant: &PaneVariant) -> &str {
    match variant {
        PaneVariant::ScriptList => "Scripts",
        PaneVariant::ExecutionList => "Execution",
        PaneVariant::LogOutput => "Log",
        PaneVariant::Parameters => "Parameters",
    }
}
pub fn get_config_error_content<'a>(
    error: &config::ConfigReadError,
    _theme: &Theme,
) -> Column<'a, WindowMessage> {
    let mut content = Vec::new();
    content.push(text("Error:").into());
    match error {
        config::ConfigReadError::FileReadError { file_path, error } => {
            content.push(
                text(format!(
                    "Failed to read file '{}'",
                    file_path.to_string_lossy()
                ))
                .into(),
            );
            content.push(text("Make sure the file has correct access rights").into());
            content.push(text(format!("Details: {}", error)).into());
            add_open_file_location_button(&mut content, file_path);
        }
        config::ConfigReadError::DataParseJsonError { file_path, error } => {
            content.push(
                text(format!(
                    "Failed to parse JSON data from file '{}'",
                    file_path.to_string_lossy()
                ))
                .into(),
            );
            content.push(
                text("If you made any manual edits to the file, make sure they are correct").into(),
            );
            content.push(text(format!("Details: {}", error)).into());
            content.push(
                button("Open file")
                    .on_press(WindowMessage::OpenWithDefaultApplication(file_path.clone()))
                    .into(),
            );
            content.push(text("If you believe this is a bug in scripter, please report it").into());
            content.push(
                button("Report bug")
                    .on_press(WindowMessage::OpenUrl(
                        "https://github.com/gameraccoon/scripter/labels/bug".to_string(),
                    ))
                    .into(),
            );
        }
        config::ConfigReadError::UpdaterUnknownVersion {
            file_path,
            version,
            latest_version,
        } => {
            content.push(
                text(format!(
                    "Unknown version of the config file '{}'",
                    file_path.to_string_lossy()
                ))
                .into(),
            );
            content.push(
                text(format!(
                    "The version of the file is '{}', latest known version is '{}'",
                    version, latest_version
                ))
                .into(),
            );
            content.push(
                text("This version of Scripter might not be compatible with the file").into(),
            );
            content.push(text("Please update Scripter to the latest version").into());
            content.push(
                button("Open releases")
                    .on_press(WindowMessage::OpenUrl(
                        "https://github.com/gameraccoon/scripter/releases".to_string(),
                    ))
                    .into(),
            );
        }
        config::ConfigReadError::ConfigDeserializeError { file_path, error } => {
            content.push(
                text(format!(
                    "Failed to deserialize config file '{}'",
                    file_path.to_string_lossy()
                ))
                .into(),
            );
            content.push(
                text("If you made any manual edits to the file, make sure they are correct").into(),
            );
            content.push(text(format!("Details: {}", error)).into());
            content.push(
                button("Open file")
                    .on_press(WindowMessage::OpenWithDefaultApplication(file_path.clone()))
                    .into(),
            );
            content.push(text("If you believe this is a bug in scripter, please report it").into());
            content.push(
                button("Report bug")
                    .on_press(WindowMessage::OpenUrl(
                        "https://github.com/gameraccoon/scripter/labels/bug".to_string(),
                    ))
                    .into(),
            );
        }
        config::ConfigReadError::ConfigSerializeError { error } => {
            content.push(text("Failed to serialize a config file").into());
            content.push(
                text("This is likely a bug in Scripter, please report it to the developer").into(),
            );
            content.push(text(format!("Details: {}", error)).into());
            content.push(
                button("Report bug")
                    .on_press(WindowMessage::OpenUrl(
                        "https://github.com/gameraccoon/scripter/labels/bug".to_string(),
                    ))
                    .into(),
            );
        }
        config::ConfigReadError::FileWriteError { file_path, error } => {
            content.push(
                text(format!(
                    "Failed to write to file '{}'",
                    file_path.to_string_lossy()
                ))
                .into(),
            );
            content.push(
                text("Make sure the file has correct access rights and is not read-only").into(),
            );
            content.push(text(format!("Details: {}", error)).into());
            add_open_file_location_button(&mut content, file_path);
        }
    }

    content.push(text(format!("Application version {}", env!("CARGO_PKG_VERSION"))).into());
    Column::with_children(content).spacing(10)
}

fn add_open_file_location_button(
    content: &mut Vec<Element<WindowMessage, Theme, iced::Renderer>>,
    file_path: &std::path::PathBuf,
) {
    if let Some(file_path) = file_path.parent() {
        content.push(
            button("Open file location")
                .on_press(WindowMessage::OpenWithDefaultApplication(
                    file_path.to_path_buf(),
                ))
                .into(),
        );
    }
}

pub fn format_keybind_hint(caches: &VisualCaches, hint: &str, action: config::AppAction) -> String {
    if let Some(keybind_hint) = caches
        .keybind_hints
        .get(&keybind_editing::KeybindAssociatedData::AppAction(action))
    {
        return format!("{} ({})", hint, keybind_hint);
    }
    hint.to_string()
}

pub fn populate_argument_placeholders_config_content<'a>(
    content: &mut Vec<Element<'a, WindowMessage, Theme, iced::Renderer>>,
    argument_placeholders: &Vec<config::ArgumentPlaceholder>,
) {
    for i in 0..argument_placeholders.len() {
        let argument_placeholder = &argument_placeholders[i];
        content.push(
            row![
                text_input("Name", &argument_placeholder.name,)
                    .on_input(move |new_value| {
                        WindowMessage::EditArgumentPlaceholderName(i, new_value)
                    })
                    .padding(5),
                text_input("Placeholder", &argument_placeholder.placeholder,)
                    .on_input(move |new_value| {
                        WindowMessage::EditArgumentPlaceholderPlaceholder(i, new_value)
                    })
                    .padding(5),
            ]
            .into(),
        );
        content.push(
            text_input("Default value", &argument_placeholder.value)
                .on_input(move |new_value| {
                    WindowMessage::EditArgumentPlaceholderValueForConfig(i, new_value)
                })
                .padding(5)
                .into(),
        );
        content.push(
            button("Remove Placeholder")
                .on_press(WindowMessage::RemoveArgumentPlaceholder(i))
                .into(),
        );
        if i + 1 < argument_placeholders.len() {
            content.push(horizontal_rule(1).into());
        }
    }

    content.push(
        button("+")
            .on_press(WindowMessage::AddArgumentPlaceholder)
            .into(),
    );
}

pub fn populate_argument_placeholders_content<'a>(
    content: &mut Vec<Element<'a, WindowMessage, Theme, iced::Renderer>>,
    argument_placeholders: &Vec<config::ArgumentPlaceholder>,
) {
    if !argument_placeholders.is_empty() {
        content.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    }

    for i in 0..argument_placeholders.len() {
        let argument_placeholder = &argument_placeholders[i];
        content.push(
            text(format!("{}:", argument_placeholder.name))
                .size(16)
                .horizontal_alignment(alignment::Horizontal::Left)
                .into(),
        );
        content.push(
            text_input("Value", &argument_placeholder.value)
                .on_input(move |new_value| {
                    WindowMessage::EditArgumentPlaceholderValueForScriptExecution(i, new_value)
                })
                .padding(5)
                .into(),
        );
    }
}
