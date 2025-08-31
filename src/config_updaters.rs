// Copyright (C) Pavel Grebnev 2023-2025
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use crate::config;
use crate::json_file_updater::{JsonFileUpdater, UpdateResult};
use serde_json::{json, Value as JsonValue};

static VERSION_FIELD_NAME: &str = "version";
pub static LATEST_CONFIG_VERSION: &str = "0.18.0";
pub static LATEST_LOCAL_CONFIG_VERSION: &str = "0.18.0";

pub fn update_config_to_the_latest_version(config_json: &mut JsonValue) -> UpdateResult {
    let version = config_json[VERSION_FIELD_NAME].as_str();
    if let Some(version) = version {
        if version == LATEST_CONFIG_VERSION {
            return UpdateResult::NoUpdateNeeded;
        }
    }

    let json_config_updater = register_config_updaters();
    json_config_updater.update_json(config_json)
}

pub fn update_local_config_to_the_latest_version(config_json: &mut JsonValue) -> UpdateResult {
    let version = config_json[VERSION_FIELD_NAME].as_str();
    if let Some(version) = version {
        if version == LATEST_LOCAL_CONFIG_VERSION {
            return UpdateResult::NoUpdateNeeded;
        }
    }

    let json_config_updater = register_local_config_updaters();
    json_config_updater.update_json(config_json)
}

fn register_config_updaters() -> JsonFileUpdater {
    let mut json_config_updater = JsonFileUpdater::new(VERSION_FIELD_NAME);

    json_config_updater.add_update_function("0.6.0", |config_json| {
        // keep the old behavior of the "window_status_reactions" field since the default changed
        if config_json.get("window_status_reactions").is_none() {
            config_json["window_status_reactions"] = json!(false);
        }
        // also all values should be set explicitly now
    });
    json_config_updater.add_update_function("0.7.0", |config_json| {
        config_json["keep_window_size"] = json!(false);
    });
    json_config_updater.add_update_function("0.7.1", |config_json| {
        if let Some(script_definitions) = config_json["script_definitions"].as_array_mut() {
            for script in script_definitions {
                script["uid"] = json!(config::Guid::new());
            }
        }
    });
    json_config_updater.add_update_function("0.7.2", |config_json| {
        let mut rewritable = json!({});
        rewritable["always_on_top"] = config_json["always_on_top"].take();
        rewritable["window_status_reactions"] = config_json["window_status_reactions"].take();
        rewritable["icon_path_relative_to_scripter"] =
            config_json["icon_path_relative_to_scripter"].take();
        rewritable["keep_window_size"] = config_json["keep_window_size"].take();
        rewritable["custom_theme"] = config_json["custom_theme"].take();
        config_json["rewritable"] = rewritable;
    });
    json_config_updater.add_update_function("0.9.3", |config_json| {
        if let Some(script_definitions) = config_json["script_definitions"].as_array_mut() {
            for script in script_definitions {
                script["requires_arguments"] = json!(false);
            }
        }
    });
    json_config_updater.add_update_function("0.9.4", |config_json| {
        let was_icon_path_relative_to_scripter =
            if let Some(rewritable_config) = config_json["rewritable"].as_object_mut() {
                rewritable_config
                    .get("icon_path_relative_to_scripter")
                    .unwrap_or(&json!(false))
                    .as_bool()
                    .unwrap_or_default()
            } else {
                false
            };

        if let Some(script_definitions) = config_json["script_definitions"].as_array_mut() {
            for script in script_definitions {
                script["icon"] =
                    convert_path_0_9_4(script["icon"].take(), was_icon_path_relative_to_scripter);
                script["command"] = convert_path_0_9_4(
                    script["command"].take(),
                    script["path_relative_to_scripter"]
                        .as_bool()
                        .unwrap_or(false),
                );
            }
        }

        config_json["child_config_path"] =
            convert_path_0_9_4(config_json["child_config_path"].take(), true);
    });
    json_config_updater.add_update_function("0.9.5", |config_json| {
        if let Some(script_definitions) = config_json["script_definitions"].as_array_mut() {
            for script in script_definitions {
                script["arguments_hint"] = json!("\"arg1\" \"arg2\"");
            }
        }
    });
    json_config_updater.add_update_function("0.10.0", |config_json| {
        if let Some(script_definitions) = config_json["script_definitions"].as_array_mut() {
            for script in script_definitions {
                script["Original"] = script.take();
            }
        }
    });
    json_config_updater.add_update_function("0.10.4", v0_10_4_add_caption_and_error_text_colors);
    json_config_updater.add_update_function("0.10.5", v0_10_5_add_filter_option);
    json_config_updater.add_update_function("0.12.1", v0_12_1_add_enable_title_editing_option);
    json_config_updater
        .add_update_function("0.12.2", v0_12_2_rename_child_to_local_and_parent_to_shared);
    json_config_updater.add_update_function("0.13.0", v0_13_0_added_custom_working_directory);
    json_config_updater.add_update_function("0.14.0", v0_14_0_added_config_version_update_field);
    json_config_updater.add_update_function("0.14.1", v0_14_1_added_app_action_keybinds);
    json_config_updater.add_update_function("0.14.2", v0_14_2_added_script_keybinds);
    json_config_updater.add_update_function("0.14.3", v0_14_3_added_show_current_git_branch);
    json_config_updater.add_update_function("0.15.0", v0_15_0_remove_always_on_top);
    json_config_updater.add_update_function("0.15.1", v0_15_1_update_keybinds_for_iced_12);
    json_config_updater.add_update_function("0.16.0", v0_16_0_replace_run_scripts_keybind_id);
    json_config_updater
        .add_update_function("0.16.1", v0_16_1_add_alt_for_cursor_confirm_keybind_variant);
    json_config_updater.add_update_function("0.16.2", v0_16_2_add_quick_launch_scripts);
    json_config_updater.add_update_function("0.16.4", v0_16_4_add_is_hidden_field);
    json_config_updater.add_update_function("0.16.5", v0_16_5_add_autoclean_on_success_field);
    json_config_updater.add_update_function("0.16.6", v0_16_6_add_show_working_directory_field);
    json_config_updater.add_update_function("0.16.7", v0_16_7_add_ignore_output_field);
    json_config_updater.add_update_function("0.17.0", v0_17_0_add_custom_executor_field);
    json_config_updater.add_update_function(
        "0.17.2",
        v0_17_2_add_argument_placeholders_field_and_arg_requirement,
    );
    json_config_updater.add_update_function("0.18.0", v0_18_0_add_placeholders_to_presets);
    // add update functions above this line
    // don't forget to update LATEST_CONFIG_VERSION at the beginning of the file

    json_config_updater
}

fn register_local_config_updaters() -> JsonFileUpdater {
    let mut json_config_updater = JsonFileUpdater::new(VERSION_FIELD_NAME);

    json_config_updater.add_update_function("0.7.2", |_config_json| {
        // empty updater to have a name for the first version
    });
    json_config_updater.add_update_function("0.9.3", |config_json| {
        for_each_script_added_definition_pre_0_10_0(config_json, |script| {
            script["requires_arguments"] = json!(false);
        });
    });
    json_config_updater.add_update_function("0.9.4", |config_json| {
        let was_icon_path_relative_to_scripter =
            if let Some(rewritable_config) = config_json["rewritable"].as_object_mut() {
                rewritable_config["icon_path_relative_to_scripter"]
                    .as_bool()
                    .unwrap_or(false)
            } else {
                false
            };

        for_each_script_added_definition_pre_0_10_0(config_json, |script| {
            script["icon"] =
                convert_path_0_9_4(script["icon"].take(), was_icon_path_relative_to_scripter);
            script["command"] = convert_path_0_9_4(
                script["command"].take(),
                script["path_relative_to_scripter"]
                    .as_bool()
                    .unwrap_or(false),
            );
        });
    });
    json_config_updater.add_update_function("0.9.5", |config_json| {
        for_each_script_added_definition_pre_0_10_0(config_json, |script| {
            script["arguments_hint"] = json!("\"arg1\" \"arg2\"");
        });
    });
    json_config_updater.add_update_function("0.10.0", |config_json| {
        if let Some(script_definitions) = config_json["script_definitions"].as_array_mut() {
            for script in script_definitions {
                if !script["Parent"].is_null() {
                    *script = json!({
                        "ReferenceToParent": script["Parent"].take(),
                    });
                }
            }
        }
    });
    json_config_updater.add_update_function("0.10.4", v0_10_4_add_caption_and_error_text_colors);
    json_config_updater.add_update_function("0.10.5", v0_10_5_add_filter_option);
    json_config_updater.add_update_function("0.12.1", v0_12_1_add_enable_title_editing_option);
    json_config_updater
        .add_update_function("0.12.2", v0_12_2_rename_child_to_local_and_parent_to_shared);
    json_config_updater.add_update_function("0.13.0", v0_13_0_added_custom_working_directory);
    json_config_updater.add_update_function("0.14.0", v0_14_0_added_config_version_update_field);
    json_config_updater.add_update_function("0.14.1", v0_14_1_added_app_action_keybinds);
    json_config_updater.add_update_function("0.14.2", v0_14_2_added_script_keybinds);
    json_config_updater.add_update_function("0.14.3", v0_14_3_added_show_current_git_branch);
    json_config_updater.add_update_function("0.15.0", v0_15_0_remove_always_on_top);
    json_config_updater.add_update_function("0.15.1", v0_15_1_update_keybinds_for_iced_12);
    json_config_updater.add_update_function("0.16.0", v0_16_0_replace_run_scripts_keybind_id);
    json_config_updater
        .add_update_function("0.16.1", v0_16_1_add_alt_for_cursor_confirm_keybind_variant);
    json_config_updater.add_update_function("0.16.2", v0_16_2_add_quick_launch_scripts);
    json_config_updater.add_update_function("0.16.4", v0_16_4_add_is_hidden_field);
    json_config_updater.add_update_function("0.16.5", v0_16_5_add_autoclean_on_success_field);
    json_config_updater.add_update_function("0.16.6", v0_16_6_add_show_working_directory_field);
    json_config_updater.add_update_function("0.16.7", v0_16_7_add_ignore_output_field);
    json_config_updater.add_update_function("0.17.0", v0_17_0_add_custom_executor_field);
    json_config_updater.add_update_function(
        "0.17.2",
        v0_17_2_add_argument_placeholders_field_and_arg_requirement,
    );
    json_config_updater.add_update_function("0.18.0", v0_18_0_add_placeholders_to_presets);

    // add update functions above this line
    // don't forget to update LATEST_LOCAL_CONFIG_VERSION at the beginning of the file

    json_config_updater
}

fn convert_path_0_9_4(old_path: JsonValue, is_relative_to_scripter: bool) -> JsonValue {
    let path_type = if is_relative_to_scripter {
        "ScripterExecutableRelative"
    } else {
        "WorkingDirRelative"
    };

    if old_path.is_null() {
        json!({
            "path_type": path_type,
            "path": "",
        })
    } else {
        json!({
            "path_type": path_type,
            "path": old_path.as_str().unwrap_or(""),
        })
    }
}

fn for_each_script_added_definition_pre_0_10_0<F>(config_json: &mut JsonValue, mut f: F)
where
    F: FnMut(&mut JsonValue),
{
    if let Some(script_definitions) = config_json["script_definitions"].as_array_mut() {
        for script in script_definitions {
            if let Some(obj) = script.as_object_mut() {
                if let Some(value) = obj.get_mut("Added") {
                    f(value);
                }
            }
        }
    }
}

fn for_each_script_original_definition_post_0_10_0<F>(config_json: &mut JsonValue, mut f: F)
where
    F: FnMut(&mut JsonValue),
{
    if let Some(script_definitions) = config_json["script_definitions"].as_array_mut() {
        for script in script_definitions {
            if let Some(obj) = script.as_object_mut() {
                if let Some(value) = obj.get_mut("Original") {
                    f(value);
                }
            }
        }
    }
}

fn for_each_script_preset<F>(config_json: &mut JsonValue, mut f: F)
where
    F: FnMut(&mut JsonValue),
{
    if let Some(script_definitions) = config_json["script_definitions"].as_array_mut() {
        for script in script_definitions {
            if let Some(obj) = script.as_object_mut() {
                if let Some(value) = obj.get_mut("Preset") {
                    f(value);
                }
            }
        }
    }
}

fn v0_10_4_add_caption_and_error_text_colors(config_json: &mut JsonValue) {
    if let Some(rewritable) = config_json["rewritable"].as_object_mut() {
        if let Some(custom_theme) = rewritable["custom_theme"].as_object_mut() {
            let primary = custom_theme
                .get("primary")
                .unwrap_or(&json!([0.0, 0.0, 0.5]))
                .clone();
            let danger = custom_theme
                .get("danger")
                .unwrap_or(&json!([0.5, 0.0, 0.0]))
                .clone();
            custom_theme.entry("caption_text").or_insert(primary);
            custom_theme.entry("error_text").or_insert(danger);
        }
    }
}

fn v0_10_5_add_filter_option(config_json: &mut JsonValue) {
    if let Some(rewritable) = config_json["rewritable"].as_object_mut() {
        rewritable.insert("enable_script_filtering".to_string(), json!(true));
    }
}

fn v0_12_1_add_enable_title_editing_option(config_json: &mut JsonValue) {
    if let Some(rewritable) = config_json["rewritable"].as_object_mut() {
        rewritable.insert("enable_title_editing".to_string(), json!(true));
    }
}

fn v0_12_2_rename_child_to_local_and_parent_to_shared(config_json: &mut JsonValue) {
    if let Some(script_definitions) = config_json["script_definitions"].as_array_mut() {
        for script in script_definitions {
            if let Some(obj) = script.as_object_mut() {
                if obj.get("ReferenceToParent").is_some() {
                    obj.insert("ReferenceToShared".to_string(), serde_json::Value::Null);
                    obj["ReferenceToShared"] = obj["ReferenceToParent"].take();
                    obj.remove("ReferenceToParent");
                }
            }
        }
    }

    if config_json.get("child_config_path").is_some() {
        config_json["local_config_path"] = config_json["child_config_path"].take();
    }
}

fn v0_13_0_added_custom_working_directory(config_json: &mut JsonValue) {
    for_each_script_original_definition_post_0_10_0(config_json, |script| {
        script["working_directory"] = json!({
            "path_type": "WorkingDirRelative",
            "path": ".",
        });
    });
}

fn v0_14_0_added_config_version_update_field(config_json: &mut JsonValue) {
    if let Some(rewritable) = config_json["rewritable"].as_object_mut() {
        rewritable.insert(
            "config_version_update_behavior".to_string(),
            json!("OnStartup"),
        );
    }
}

fn v0_14_1_added_app_action_keybinds(config_json: &mut JsonValue) {
    if let Some(rewritable) = config_json["rewritable"].as_object_mut() {
        rewritable.insert(
            "app_actions_keybinds".to_string(),
            json!([
                {
                    "action": "RequestCloseApp",
                    "keybind": {"key": "W", "modifiers": "Cmd"},
                },
                {
                    "action": "FocusFilter",
                    "keybind": {"key": "F", "modifiers": "Cmd"},
                },
                {
                    "action": "TrySwitchWindowEditMode",
                    "keybind": {"key": "E", "modifiers": "Cmd"},
                },
                {
                    "action": "RescheduleScripts",
                    "keybind": {"key": "R", "modifiers": "Cmd+Shift"},
                },
                {
                    "action": "RunScripts",
                    "keybind": {"key": "R", "modifiers": "Cmd"},
                },
                {
                    "action": "StopScripts",
                    "keybind": {"key": "C", "modifiers": "Cmd+Shift"},
                },
                {
                    "action": "ClearExecutionScripts",
                    "keybind": {"key": "C", "modifiers": "Cmd"},
                },
                {
                    "action": "MaximizeOrRestoreExecutionPane",
                    "keybind": {"key": "Q", "modifiers": "Cmd"},
                },
                {
                    "action": "CursorConfirm",
                    "keybind": {"key": "Enter", "modifiers": ""},
                },
                {
                    "action": "CursorConfirm",
                    "keybind": {"key": "Enter", "modifiers": "Cmd"},
                },
                {
                    "action": "MoveScriptDown",
                    "keybind": {"key": "Down", "modifiers": "Shift"},
                },
                {
                    "action": "MoveScriptUp",
                    "keybind": {"key": "Up", "modifiers": "Shift"},
                },
                {
                    "action": "SwitchPaneFocusBackwards",
                    "keybind": {"key": "Tab", "modifiers": "Shift"},
                },
                {
                    "action": "MoveCursorDown",
                    "keybind": {"key": "Down", "modifiers": ""},
                },
                {
                    "action": "MoveCursorUp",
                    "keybind": {"key": "Up", "modifiers": ""},
                },
                {
                    "action": "SwitchPaneFocusForward",
                    "keybind": {"key": "Tab", "modifiers": ""},
                },
                {
                    "action": "RemoveCursorScript",
                    "keybind": {"key": "Delete", "modifiers": ""},
                },
            ]),
        );
    }
}

fn v0_14_2_added_script_keybinds(config_json: &mut JsonValue) {
    if let Some(rewritable) = config_json["rewritable"].as_object_mut() {
        rewritable.insert("script_keybinds".to_string(), json!([]));
    }
}

fn v0_14_3_added_show_current_git_branch(config_json: &mut JsonValue) {
    if let Some(rewritable) = config_json["rewritable"].as_object_mut() {
        rewritable.insert("show_current_git_branch".to_string(), json!(false));
    }
}

fn v0_15_0_remove_always_on_top(config_json: &mut JsonValue) {
    if let Some(rewritable) = config_json["rewritable"].as_object_mut() {
        rewritable.remove("always_on_top");
    }
}

fn v0_15_1_update_keybinds_for_iced_12(config_json: &mut JsonValue) {
    let update_keybind_key = |key: &mut JsonValue| {
        // remove keys starting with Numpad
        if key
            .as_str()
            .map(|s| s.starts_with("Numpad"))
            .unwrap_or(false)
        {
            *key = json!("Unknown");
        }

        match key.as_str().unwrap_or_default() {
            // Rename for better names
            "Untitled" => *key = json!("Unknown"),
            "AbntC2" => *key = json!("Tilde"),
            "Snapshot" => *key = json!("PrintScreen"),
            "Capital" => *key = json!("CapsLock"),
            "Scroll" => *key = json!("ScrollLock"),
            // Reduce duplication
            "AbntC1" => *key = json!("Grave"),
            "Ax" => *key = json!("Grave"),
            // No Left/Right distinction anymore :'(
            "LAlt" => *key = json!("Alt"),
            "LControl" => *key = json!("Control"),
            "LShift" => *key = json!("Shift"),
            "LWin" => *key = json!("Win"),
            "RAlt" => *key = json!("Alt"),
            "RControl" => *key = json!("Control"),
            "RShift" => *key = json!("Shift"),
            "RWin" => *key = json!("Win"),
            // Not supported at all with iced 0.12
            "Calculator" => *key = json!("Unknown"),
            "MyComputer" => *key = json!("Unknown"),
            "OEM102" => *key = json!("Unknown"),
            "Sleep" => *key = json!("Unknown"),
            "Sysrq" => *key = json!("Unknown"),
            "Wake" => *key = json!("Unknown"),
            _ => {}
        }
    };

    if let Some(rewritable) = config_json["rewritable"].as_object_mut() {
        if let Some(app_actions_keybinds) = rewritable["app_actions_keybinds"].as_array_mut() {
            for keybind in app_actions_keybinds {
                update_keybind_key(&mut keybind["keybind"]["key"]);
            }
        }
        if let Some(script_keybinds) = rewritable["script_keybinds"].as_array_mut() {
            for keybind in script_keybinds {
                update_keybind_key(&mut keybind["keybind"]["key"]);
            }
        }
    }
}

fn v0_16_0_replace_run_scripts_keybind_id(config_json: &mut JsonValue) {
    if let Some(rewritable) = config_json["rewritable"].as_object_mut() {
        if let Some(app_actions_keybinds) = rewritable["app_actions_keybinds"].as_array_mut() {
            for keybind in app_actions_keybinds {
                if keybind["action"] == "RunScripts" {
                    keybind["action"] = json!("RunScriptsAfterExecution");
                }
            }
        }
    }
}

fn v0_16_1_add_alt_for_cursor_confirm_keybind_variant(config_json: &mut JsonValue) {
    if let Some(rewritable) = config_json["rewritable"].as_object_mut() {
        if let Some(app_actions_keybinds) = rewritable["app_actions_keybinds"].as_array_mut() {
            app_actions_keybinds.push(json!({
                "action": "CursorConfirm",
                "keybind": {"key": "Enter", "modifiers": "Cmd+Alt"},
            }));
        }
    }
}

fn v0_16_2_add_quick_launch_scripts(config_json: &mut JsonValue) {
    if let Some(rewritable) = config_json["rewritable"].as_object_mut() {
        rewritable.insert("quick_launch_scripts".to_string(), json!([]));
    }
}

fn v0_16_4_add_is_hidden_field(config_json: &mut JsonValue) {
    for_each_script_original_definition_post_0_10_0(config_json, |script| {
        script["is_hidden"] = json!(false);
    });
}

fn v0_16_5_add_autoclean_on_success_field(config_json: &mut JsonValue) {
    for_each_script_original_definition_post_0_10_0(config_json, |script| {
        script["autoclean_on_success"] = json!(false);
    });
}

fn v0_16_6_add_show_working_directory_field(config_json: &mut JsonValue) {
    if let Some(rewritable) = config_json["rewritable"].as_object_mut() {
        rewritable.insert("show_working_directory".to_string(), json!(true));
    }
}

fn v0_16_7_add_ignore_output_field(config_json: &mut JsonValue) {
    for_each_script_original_definition_post_0_10_0(config_json, |script| {
        script["ignore_output"] = json!(false);
    });
}

fn v0_17_0_add_custom_executor_field(config_json: &mut JsonValue) {
    for_each_script_original_definition_post_0_10_0(config_json, |script| {
        // there were two preview versions that had this field but were missing the updater
        // so keep the value if it was set
        if script["custom_executor"].is_null() {
            script["custom_executor"] = json!(None::<bool>);
        }
    });
}

fn v0_17_2_add_argument_placeholders_field_and_arg_requirement(config_json: &mut JsonValue) {
    for_each_script_original_definition_post_0_10_0(config_json, |script| {
        if script["argument_placeholders"].is_null() {
            script["argument_placeholders"] = json!([]);
        }

        let are_arguments_required = script["requires_arguments"]
            .take()
            .as_bool()
            .unwrap_or(false);
        script["arguments_requirement"] = if are_arguments_required {
            json!("Required")
        } else {
            json!("Optional")
        };
    });
}

fn v0_18_0_add_placeholders_to_presets(config_json: &mut JsonValue) {
    for_each_script_preset(config_json, |preset| {
        if let Some(items) = preset["items"].as_array_mut() {
            for item in items {
                item["overridden_placeholder_values"] = json!({});
            }
        }
    });
}
