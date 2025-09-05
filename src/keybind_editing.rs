// Copyright (C) Pavel Grebnev 2023-2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

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
    Script(config::Guid),
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
    iced_key: keyboard::Key,
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
        if iced_key == keyboard::Key::Named(keyboard::key::Named::Escape) {
            match keybind {
                KeybindAssociatedData::AppAction(app_action) => {
                    clear_app_action_keybind(app, &app_action);
                }
                KeybindAssociatedData::Script(guid) => {
                    clear_script_keybind(app, &guid);
                }
            }
            app.edit_data.is_dirty = true;
        } else {
            if let Some(old_keybind) = app.keybinds.get_keybind(iced_key.clone(), iced_modifiers) {
                if *old_keybind != keybind {
                    if let Some(window_edit_data) = &mut app.edit_data.window_edit_data {
                        window_edit_data.keybind_editing.edited_keybind_error =
                            Some((keybind, "Error: Keybind already in use".to_string()));
                    }
                    return true;
                }
            }

            let key = key_mapping::get_custom_key_code_from_iced_key_code(iced_key);
            let modifiers = key_mapping::get_custom_modifiers_from_iced_modifiers(iced_modifiers);

            match keybind {
                KeybindAssociatedData::AppAction(app_action) => {
                    set_app_action_keybind(
                        app,
                        &app_action,
                        config::CustomKeybind { key, modifiers },
                    );
                }
                KeybindAssociatedData::Script(guid) => {
                    set_script_keybind(app, &guid, config::CustomKeybind { key, modifiers });
                }
            }

            app.edit_data.is_dirty = true;
        }
        return true;
    }

    false
}

fn clear_app_action_keybind(app: &mut main_window::MainWindow, app_action: &config::AppAction) {
    let edit_mode = if let Some(window_edit_data) = &app.edit_data.window_edit_data {
        if let Some(edit_mode) = window_edit_data.settings_edit_mode {
            edit_mode
        } else {
            return;
        }
    } else {
        return;
    };

    let rewritable_config = config::get_rewritable_config_mut(&mut app.app_config, edit_mode);
    // remove all keybinds with the same action
    rewritable_config
        .app_actions_keybinds
        .retain(|x| x.action != *app_action);

    update_keybinds(app);
    update_keybind_visual_caches(app, edit_mode);
}

fn set_app_action_keybind(
    app: &mut main_window::MainWindow,
    app_action: &config::AppAction,
    keybind: config::CustomKeybind,
) {
    let edit_mode = if let Some(window_edit_data) = &app.edit_data.window_edit_data {
        if let Some(edit_mode) = window_edit_data.settings_edit_mode {
            edit_mode
        } else {
            return;
        }
    } else {
        return;
    };

    let rewritable_config = config::get_rewritable_config_mut(&mut app.app_config, edit_mode);
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
    update_keybind_visual_caches(app, edit_mode);
}

fn clear_script_keybind(app: &mut main_window::MainWindow, guid: &config::Guid) {
    let rewritable_config = config::get_main_rewritable_config_mut(&mut app.app_config);
    // remove all keybinds with the same action
    rewritable_config
        .script_keybinds
        .retain(|x| x.script_uid != *guid);

    update_keybinds(app);
    update_keybind_visual_caches(app, config::get_main_edit_mode(&app.app_config));
}

fn set_script_keybind(
    app: &mut main_window::MainWindow,
    guid: &config::Guid,
    keybind: config::CustomKeybind,
) {
    let rewritable_config = config::get_main_rewritable_config_mut(&mut app.app_config);
    // remove all keybinds with the same action
    rewritable_config
        .script_keybinds
        .retain(|x| x.script_uid != *guid);
    // add new keybind
    rewritable_config
        .script_keybinds
        .push(config::ScriptKeybind {
            script_uid: guid.clone(),
            keybind,
        });

    update_keybinds(app);
    update_keybind_visual_caches(app, config::get_main_edit_mode(&app.app_config));
}

pub fn update_keybinds(app: &mut main_window::MainWindow) {
    app.keybinds = custom_keybinds::CustomKeybinds::new();
    let rewritable_config = config::get_main_rewritable_config(&app.app_config);
    for app_action_bind in &rewritable_config.app_actions_keybinds {
        let key = key_mapping::get_iced_key_code_from_custom_key_code(app_action_bind.keybind.key);
        let modifiers = key_mapping::get_iced_modifiers_from_custom_modifiers(
            app_action_bind.keybind.modifiers,
        );
        if app.keybinds.has_keybind(key.clone(), modifiers) {
            eprintln!(
                "Keybind is used for multiple actions, skipping: {}",
                key_mapping::get_readable_keybind_name(
                    app_action_bind.keybind.key,
                    app_action_bind.keybind.modifiers
                )
            );
            continue;
        }

        app.keybinds.add_keybind(
            key,
            modifiers,
            KeybindAssociatedData::AppAction(app_action_bind.action),
        );
    }

    for script_bind in &rewritable_config.script_keybinds {
        let key = key_mapping::get_iced_key_code_from_custom_key_code(script_bind.keybind.key);
        let modifiers =
            key_mapping::get_iced_modifiers_from_custom_modifiers(script_bind.keybind.modifiers);
        if app.keybinds.has_keybind(key.clone(), modifiers) {
            eprintln!(
                "Keybind is used for multiple actions, skipping: {}",
                key_mapping::get_readable_keybind_name(
                    script_bind.keybind.key,
                    script_bind.keybind.modifiers
                )
            );
            continue;
        }

        app.keybinds.add_keybind(
            key,
            modifiers,
            KeybindAssociatedData::Script(script_bind.script_uid.clone()),
        );
    }
}

pub fn update_keybind_visual_caches(
    app: &mut main_window::MainWindow,
    edit_mode: config::ConfigEditMode,
) {
    app.visual_caches.keybind_hints.clear();
    let rewritable_config = config::get_rewritable_config(&app.app_config, edit_mode);
    for app_action_bind in &rewritable_config.app_actions_keybinds {
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

    for script_bind in &rewritable_config.script_keybinds {
        app.visual_caches.keybind_hints.insert(
            KeybindAssociatedData::Script(script_bind.script_uid.clone()),
            format!(
                "{}",
                key_mapping::get_readable_keybind_name(
                    script_bind.keybind.key,
                    script_bind.keybind.modifiers
                ),
            ),
        );
    }
}

pub fn prune_unused_keybinds(app: &mut main_window::MainWindow) {
    // collect uids used in keybinds
    let mut used_script_uids = std::collections::HashSet::new();
    let rewritable_config = config::get_current_rewritable_config(&app.app_config);
    for script_bind in &rewritable_config.script_keybinds {
        used_script_uids.insert(script_bind.script_uid.clone());
    }

    // find those that are not used
    used_script_uids.retain(|uid| {
        config::get_original_script_definition_by_uid(&app.app_config, uid).is_some()
    });

    // remove those that are not used
    let rewritable_config = config::get_main_rewritable_config_mut(&mut app.app_config);
    rewritable_config
        .script_keybinds
        .retain(|x| used_script_uids.contains(&x.script_uid));
}

pub fn populate_keybind_editing_content(
    edit_content: &mut Vec<Element<'_, main_window::WindowMessage, iced::Theme, iced::Renderer>>,
    window_edit_data: &main_window::WindowEditData,
    visual_caches: &main_window::VisualCaches,
    caption: &'static str,
    data: KeybindAssociatedData,
) {
    edit_content.push(text(caption).into());

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
