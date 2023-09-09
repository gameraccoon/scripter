use crate::config;
use crate::json_config_updater::{JsonConfigUpdater, UpdateResult};
use serde_json::{json, Value as JsonValue};

static VERSION_FIELD_NAME: &str = "version";
pub static LATEST_CONFIG_VERSION: &str = "0.10.5";
pub static LATEST_CHILD_CONFIG_VERSION: &str = "0.10.5";

pub fn update_config_to_the_latest_version(config_json: &mut JsonValue) -> UpdateResult {
    let version = config_json[VERSION_FIELD_NAME].as_str();
    if let Some(version) = version {
        if version == LATEST_CONFIG_VERSION {
            return UpdateResult::NoUpdateNeeded;
        }
    }

    let json_config_updater = register_config_updaters();
    return json_config_updater.update_json(config_json);
}

pub fn update_child_config_to_the_latest_version(config_json: &mut JsonValue) -> UpdateResult {
    let version = config_json[VERSION_FIELD_NAME].as_str();
    if let Some(version) = version {
        if version == LATEST_CHILD_CONFIG_VERSION {
            return UpdateResult::NoUpdateNeeded;
        }
    }

    let json_config_updater = register_child_config_updaters();
    return json_config_updater.update_json(config_json);
}

fn register_config_updaters() -> JsonConfigUpdater {
    let mut json_config_updater = JsonConfigUpdater::new(VERSION_FIELD_NAME);

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
    // add update functions here
    // don't forget to update LATEST_CONFIG_VERSION at the beginning of the file

    json_config_updater
}

fn register_child_config_updaters() -> JsonConfigUpdater {
    let mut json_config_updater = JsonConfigUpdater::new(VERSION_FIELD_NAME);

    json_config_updater.add_update_function("0.7.2", |_config_json| {
        // empty updater to have a name for the first version
    });
    json_config_updater.add_update_function("0.9.3", |config_json| {
        for_each_script_original_definition(config_json, |script| {
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

        for_each_script_original_definition(config_json, |script| {
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
        for_each_script_original_definition(config_json, |script| {
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
    // add update functions here
    // don't forget to update LATEST_CHILD_CONFIG_VERSION at the beginning of the file

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

fn for_each_script_original_definition<F>(config_json: &mut JsonValue, mut f: F)
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
