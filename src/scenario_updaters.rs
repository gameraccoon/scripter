// Copyright (C) Pavel Grebnev 2025
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use crate::json_file_updater::{JsonFileUpdater, UpdateResult};
use serde_json::Value as JsonValue;

static FORMAT_VERSION_FIELD_NAME: &str = "format_version";
pub static LATEST_SCENARIO_FORMAT_VERSION: &str = "2";

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
    let mut json_scenario_updater = JsonFileUpdater::new(FORMAT_VERSION_FIELD_NAME);

    json_scenario_updater.add_update_function("1", |_| {});
    json_scenario_updater.add_update_function_with_validator(
        "2",
        |_| {},
        v2_validate_no_only_schedule_field,
    );
    // add update functions above this line
    // don't forget to update LATEST_SCENARIO_FORMAT_VERSION at the beginning of the file

    json_scenario_updater
}

fn for_each_parallel_execution_validate(
    json: &JsonValue,
    loop_fn: fn(&JsonValue) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(parallel_executions) = json.get("parallel_executions") {
        if let Some(parallel_executions) = parallel_executions.as_array() {
            for parallel_execution in parallel_executions {
                loop_fn(parallel_execution)?
            }
        }
    }

    Ok(())
}

fn v2_validate_no_only_schedule_field(json: &JsonValue) -> Result<(), String> {
    for_each_parallel_execution_validate(json, |parallel_execution| {
        if let Some(_) = parallel_execution["only_schedule"].as_bool() {
            return Err(format!("'only_schedule' field introduced in format version '2', but earlier version of the format is used"));
        }

        Ok(())
    })
}
