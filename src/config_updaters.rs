use crate::json_config_updater::{JsonConfigUpdater, UpdateResult};
use serde_json::{json, Value as JsonValue};

static VERSION_FIELD_NAME: &str = "version";
pub static LATEST_CONFIG_VERSION: &str = "0.7.0";

pub fn update_config_to_the_latest_version(config_json: &mut JsonValue) -> UpdateResult {
    let version = config_json[VERSION_FIELD_NAME].as_str();
    if let Some(version) = version {
        if version == LATEST_CONFIG_VERSION {
            return UpdateResult::NoUpdateNeeded;
        }
    }

    let json_config_updater = register_updaters();
    return json_config_updater.update_json(config_json);
}

fn register_updaters() -> JsonConfigUpdater {
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
    // add update functions here
    // don't forget to update LATEST_CONFIG_VERSION at the beginning of the file

    json_config_updater
}
