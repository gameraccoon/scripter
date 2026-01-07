// Copyright (C) Pavel Grebnev 2025
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use crate::json_file_updater::{JsonFileUpdater, UpdateResult};
use serde_json::Value as JsonValue;

static FORMAT_VERSION_FIELD_NAME: &str = "format_version";
pub static LATEST_SCENARIO_FORMAT_VERSION: &str = "5";

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
        v2_validate_no_only_schedule_field_before,
    );
    json_scenario_updater.add_update_function_with_validator(
        "3",
        |_| {},
        v3_validate_no_arguments_or_placeholders_before,
    );
    json_scenario_updater.add_update_function_with_validator(
        "4",
        |_| {},
        v4_validate_no_name_before,
    );
    json_scenario_updater.add_update_function_with_validator(
        "5",
        |_| {},
        v5_validate_no_start_focused_before,
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

fn for_each_script_validate(
    json: &JsonValue,
    loop_fn: fn(&JsonValue) -> Result<(), String>,
) -> Result<(), String> {
    if let Some(parallel_executions) = json.get("parallel_executions") {
        if let Some(parallel_executions) = parallel_executions.as_array() {
            for parallel_execution in parallel_executions {
                if let Some(scripts) = parallel_execution.get("scripts") {
                    if let Some(scripts) = scripts.as_array() {
                        for script in scripts {
                            loop_fn(script)?
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

fn get_wrong_field_version_err(field: &str, version: &str) -> Result<(), String> {
    Err(format!("'{field}' field introduced in format version '{version}', but an earlier version of the format is used. Please make sure you set the right format version in the scenario file."))
}

fn v2_validate_no_only_schedule_field_before(json: &JsonValue) -> Result<(), String> {
    for_each_parallel_execution_validate(json, |parallel_execution: &JsonValue| {
        if let Some(_) = parallel_execution["only_schedule"].as_bool() {
            return get_wrong_field_version_err("only_schedule", "2");
        }

        Ok(())
    })
}

fn v3_validate_no_arguments_or_placeholders_before(json: &JsonValue) -> Result<(), String> {
    for_each_script_validate(json, |script| {
        if script["arguments"].is_string() {
            return get_wrong_field_version_err("arguments", "3");
        }

        if let Some(_) = script["placeholders"].as_array() {
            return get_wrong_field_version_err("placeholders", "3");
        }

        Ok(())
    })
}

fn v4_validate_no_name_before(json: &JsonValue) -> Result<(), String> {
    for_each_script_validate(json, |script| {
        if script["name"].is_string() {
            return get_wrong_field_version_err("name", "4");
        }

        Ok(())
    })
}

fn v5_validate_no_start_focused_before(json: &JsonValue) -> Result<(), String> {
    if json["start_focused"].is_boolean() {
        return get_wrong_field_version_err("start_focused", "5");
    }

    Ok(())
}
