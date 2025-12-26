// Copyright (C) Pavel Grebnev 2025
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use crate::json_file_updater::{JsonFileUpdater, UpdateResult};
use serde_json::Value as JsonValue;

static FORMAT_VERSION_FIELD_NAME: &str = "format_version";
pub static LATEST_SCENARIO_FORMAT_VERSION: &str = "1";

pub fn update_scenario_to_the_latest_version(scenario_json: &mut JsonValue) -> UpdateResult {
    let version = scenario_json[FORMAT_VERSION_FIELD_NAME].as_str();
    if let Some(version) = version {
        if version == LATEST_SCENARIO_FORMAT_VERSION {
            return UpdateResult::NoUpdateNeeded;
        }
    }

    let json_scenario_updater = register_scenario_updaters();
    json_scenario_updater.update_json(scenario_json)
}

fn register_scenario_updaters() -> JsonFileUpdater {
    let json_scenario_updater = JsonFileUpdater::new(FORMAT_VERSION_FIELD_NAME);

    // json_scenario_updater.add_update_function("2", v2_my_update_fn_name);
    // add update functions above this line
    // don't forget to update LATEST_SCENARIO_FORMAT_VERSION at the beginning of the file

    json_scenario_updater
}
