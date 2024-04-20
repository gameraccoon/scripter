use iced::{
    keyboard,
    widget::{button, text},
    Element,
};

use crate::config;
use crate::custom_keybinds;
use crate::key_mapping;
use crate::main_window;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeybindAssociatedData {
    AppAction(config::AppAction),
    _Script(config::Guid),
}

#[derive(Debug, Clone)]
pub struct KeybindEditData {
    pub edited_keybind: Option<KeybindAssociatedData>,
    pub edited_keybind_error: Option<(KeybindAssociatedData, String)>,
}

impl KeybindEditData {
    pub fn new() -> Self {
        Self {
            edited_keybind: None,
            edited_keybind_error: None,
        }
    }
}

pub fn process_key_press(
    app: &mut main_window::MainWindow,
    iced_key: keyboard::KeyCode,
    iced_modifiers: keyboard::Modifiers,
) -> bool {
    // check keybind editing first
    let edited_keybind = if let Some(window_edit_data) = &mut app.edit_data.window_edit_data {
        let edited_action = window_edit_data.keybind_editing.edited_keybind.clone();
        window_edit_data.keybind_editing.edited_keybind = None;
        window_edit_data.keybind_editing.edited_keybind_error = None;
        edited_action
    } else {
        None
    };

    if let Some(keybind) = edited_keybind {
        if iced_key == keyboard::KeyCode::Escape {
            match keybind {
                KeybindAssociatedData::AppAction(app_action) => {
                    clear_app_action_keybind(app, &app_action);
                }
                KeybindAssociatedData::_Script(_uuid) => {
                    panic!("Script keybind editing not implemented");
                }
            }
            app.edit_data.is_dirty = true;
        } else {
            let key = key_mapping::get_custom_key_code_from_iced_key_code(iced_key);
            let modifiers = key_mapping::get_custom_modifiers_from_iced_modifiers(iced_modifiers);

            match keybind {
                KeybindAssociatedData::AppAction(app_action) => {
                    if let Some(old_keybind) = app.keybinds.get_keybind(iced_key, iced_modifiers) {
                        if *old_keybind != app_action {
                            if let Some(window_edit_data) = &mut app.edit_data.window_edit_data {
                                window_edit_data.keybind_editing.edited_keybind_error = Some((
                                    KeybindAssociatedData::AppAction(app_action),
                                    "Error: Keybind already in use".to_string(),
                                ));
                            }
                            return true;
                        }
                    }

                    set_app_action_keybind(
                        app,
                        &app_action,
                        config::CustomKeybind { key, modifiers },
                    );
                }
                KeybindAssociatedData::_Script(_uuid) => {
                    panic!("Script keybind editing not implemented");
                }
            }

            app.edit_data.is_dirty = true;
        }
        update_keybinds(app);
        return true;
    }

    return false;
}

fn clear_app_action_keybind(app: &mut main_window::MainWindow, app_action: &config::AppAction) {
    let rewritable_config = main_window::get_rewritable_config_mut(
        &mut app.app_config,
        &app.edit_data.window_edit_data,
    );
    // remove all keybinds with the same action
    rewritable_config
        .app_actions_keybinds
        .retain(|x| x.action != *app_action);

    update_keybinds(app);
}

fn set_app_action_keybind(
    app: &mut main_window::MainWindow,
    app_action: &config::AppAction,
    keybind: config::CustomKeybind,
) {
    let rewritable_config = main_window::get_rewritable_config_mut(
        &mut app.app_config,
        &app.edit_data.window_edit_data,
    );
    // remove all keybinds with the same action
    rewritable_config
        .app_actions_keybinds
        .retain(|x| x.action != *app_action);
    // add new keybind
    rewritable_config
        .app_actions_keybinds
        .push(config::AppActionKeybind {
            action: *app_action,
            keybind,
        });

    update_keybinds(app);
}

pub fn update_keybinds(app: &mut main_window::MainWindow) {
    app.keybinds = custom_keybinds::CustomKeybinds::new();
    app.visual_caches.keybind_hints.clear();
    let rewritable_config = config::get_current_rewritable_config(&app.app_config);
    for app_action_bind in &rewritable_config.app_actions_keybinds {
        let key = key_mapping::get_iced_key_code_from_custom_key_code(app_action_bind.keybind.key);
        let modifiers = key_mapping::get_iced_modifiers_from_custom_modifiers(
            app_action_bind.keybind.modifiers,
        );
        if app.keybinds.has_keybind(key, modifiers) {
            eprintln!(
                "Keybind is used for multiple actions, skipping: {}",
                key_mapping::get_readable_keybind_name(
                    app_action_bind.keybind.key,
                    app_action_bind.keybind.modifiers
                )
            );
            continue;
        }

        app.keybinds
            .add_keybind(key, modifiers, app_action_bind.action);

        app.visual_caches.keybind_hints.insert(
            KeybindAssociatedData::AppAction(app_action_bind.action),
            format!(
                "{}",
                key_mapping::get_readable_keybind_name(
                    app_action_bind.keybind.key,
                    app_action_bind.keybind.modifiers
                ),
            ),
        );
    }
}

pub fn populate_keybind_editing_content(
    edit_content: &mut Vec<Element<'_, main_window::WindowMessage, iced::Renderer>>,
    window_edit_data: &main_window::WindowEditData,
    visual_caches: &main_window::VisualCaches,
    caption: &str,
    data: KeybindAssociatedData,
) {
    edit_content.push(text(caption).into());

    if window_edit_data.is_editing_config {
        if let Some(edited_keybind) = &window_edit_data.keybind_editing.edited_keybind {
            if *edited_keybind == data {
                edit_content.push(
                    button("<recording> Esc to clear")
                        .on_press(main_window::WindowMessage::StopRecordingKeybind)
                        .into(),
                );
                return;
            }
        }
    }

    if let Some((action, error)) = &window_edit_data.keybind_editing.edited_keybind_error {
        if *action == data {
            edit_content.push(
                button(text(error))
                    .on_press(main_window::WindowMessage::StartRecordingKeybind(data))
                    .into(),
            );
            return;
        }
    }

    if let Some(keybind_hint) = visual_caches.keybind_hints.get(&data) {
        edit_content.push(
            button(text(keybind_hint))
                .on_press(main_window::WindowMessage::StartRecordingKeybind(data))
                .into(),
        );
        return;
    }

    edit_content.push(
        button("Not set")
            .on_press(main_window::WindowMessage::StartRecordingKeybind(data))
            .into(),
    );
}
