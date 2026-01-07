// Copyright (C) Pavel Grebnev 2025
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use crate::app_arguments;
use crate::config::Guid;
use crate::json_file_updater::{JsonFileUpdaterError, UpdateResult};
use crate::scenario_updaters::{
    update_scenario_to_the_latest_version, LATEST_SCENARIO_FORMAT_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

thread_local!(static GLOBAL_SCENARIO: Option<Result<Scenario, String>> = read_scenario());

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    pub format_version: String,
    pub parallel_executions: Vec<Execution>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Execution {
    pub scripts: Vec<Script>,
    pub only_schedule: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Script {
    pub uid: Guid,
    pub name: Option<String>,
    pub arguments: Option<String>,
    pub placeholders: Option<HashMap<String, String>>,
}

impl Script {
    fn with_uid(uid: Guid) -> Script {
        Self {
            uid,
            name: None,
            arguments: None,
            placeholders: None,
        }
    }
}

fn read_scenario() -> Option<Result<Scenario, String>> {
    let app_arguments = app_arguments::get_app_arguments();

    if app_arguments.scenario.is_some() && app_arguments.run_script.is_some() {
        return Some(Err(
            "Both '--scenario' and '--run' arguments are provided. Can only use one at a time."
                .to_string(),
        ));
    }

    let scenario = if let Some(scenario_path) = app_arguments.scenario {
        let scenario_path = PathBuf::from(scenario_path);
        if !scenario_path.exists() {
            return Some(Err(format!(
                "Scenario file '{}' does not exist",
                scenario_path.to_str().unwrap_or("")
            )));
        }

        let data = std::fs::read_to_string(&scenario_path);
        let data = match data {
            Ok(data) => data,
            Err(err) => {
                return Some(Err(format!(
                    "Scenario file '{}' could not be read: {}",
                    scenario_path.to_str().unwrap_or(""),
                    err
                )));
            }
        };
        let scenario_json = serde_json::from_str(&data);
        let mut scenario_json = match scenario_json {
            Ok(scenario_json) => scenario_json,
            Err(err) => {
                return Some(Err(format!(
                    "Scenario file '{}' did not contain correct json: {}",
                    scenario_path.to_str().unwrap_or(""),
                    err
                )));
            }
        };

        let update_result = update_scenario_to_the_latest_version(&mut scenario_json);
        let scenario = serde_json::from_value(scenario_json);
        let scenario: Scenario = match scenario {
            Ok(scenario) => scenario,
            Err(err) => {
                return Some(Err(format!(
                    "Scenario loaded from file '{}' could not be read after being updated: {}",
                    scenario_path.to_str().unwrap_or(""),
                    err
                )));
            }
        };

        if let UpdateResult::Error(error) = update_result {
            return match error {
                JsonFileUpdaterError::UnknownVersion {
                    version,
                    latest_version,
                } => Some(Err(format!(
                    "Scenario loaded from file '{}' has unexpected format version {}. Latest known version is {}. You may need to update scripter.",
                    scenario_path.to_str().unwrap_or(""),
                    version,
                    latest_version,
                ))),
                JsonFileUpdaterError::ValidatorError{
                    version,
                    error,
                } => Some(Err(format!(
                    "Scenario loaded from file '{}' encountered validation error (version={}). Error: {}",
                    scenario_path.to_str().unwrap_or(""),
                    version,
                    error,
                )))
            };
        }

        scenario
    } else {
        let Some(run_script) = app_arguments.run_script else {
            // this is the most likely case, when we are running without scenario-related arguments
            return None;
        };

        let uid = match Guid::from_string(run_script) {
            Ok(uid) => uid,
            Err(err) => {
                return Some(Err(format!(
                    "Could not read script uid with error: '{}'",
                    err.to_string()
                )));
            }
        };

        Scenario {
            format_version: LATEST_SCENARIO_FORMAT_VERSION.to_string(),
            parallel_executions: vec![Execution {
                scripts: vec![Script::with_uid(uid)],
                only_schedule: None,
            }],
        }
    };

    Some(Ok(scenario))
}

pub fn get_scenario_copy() -> Option<Result<Scenario, String>> {
    GLOBAL_SCENARIO.with(|scenario| scenario.clone())
}
