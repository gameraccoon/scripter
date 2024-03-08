// Copyright (C) Pavel Grebnev 2023-2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use crate::config_updaters::{
    update_local_config_to_the_latest_version, update_config_to_the_latest_version,
    LATEST_LOCAL_CONFIG_VERSION, LATEST_CONFIG_VERSION,
};
use crate::json_file_updater::UpdateResult;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::str::FromStr;

const DEFAULT_CONFIG_NAME: &str = "scripter_config.json";
const WORK_PATH_CONFIG_NAME: &str = ".scripter_config.json";
thread_local!(static GLOBAL_CONFIG: AppConfig = read_config());

#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub enum PathType {
    WorkingDirRelative,
    ScripterExecutableRelative,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PathConfig {
    pub path: String,
    pub path_type: PathType,
}

impl Default for PathConfig {
    fn default() -> PathConfig {
        PathConfig {
            path: String::new(),
            path_type: PathType::WorkingDirRelative,
        }
    }
}

// Part of the config that can be fully overridden by the local config
#[derive(Default, Clone, Deserialize, Serialize)]
pub struct RewritableConfig {
    pub always_on_top: bool,
    pub window_status_reactions: bool,
    pub keep_window_size: bool,
    pub enable_script_filtering: bool,
    pub enable_title_editing: bool,
    pub custom_theme: Option<CustomTheme>,
}

#[derive(Default, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub version: String,
    pub rewritable: RewritableConfig,
    pub script_definitions: Vec<ScriptDefinition>,
    pub local_config_path: PathConfig,
    #[serde(skip)]
    pub paths: PathCaches,
    #[serde(skip)]
    pub env_vars: Vec<(OsString, OsString)>,
    #[serde(skip)]
    pub custom_title: Option<String>,
    #[serde(skip)]
    pub config_read_error: Option<String>,
    #[serde(skip)]
    pub local_config_body: Option<Box<LocalConfig>>,
    #[serde(skip)]
    pub displayed_configs_list_cache: Vec<ScriptListCacheRecord>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ScriptListCacheRecord {
    pub name: String,
    pub full_icon_path: Option<PathBuf>,
    pub is_hidden: bool,
    pub original_script_uid: Guid,
}

#[derive(Default, Clone, Deserialize, Serialize)]
pub struct LocalConfig {
    pub version: String,
    pub rewritable: RewritableConfig,
    pub script_definitions: Vec<ScriptDefinition>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OriginalScriptDefinition {
    pub uid: Guid,
    pub name: String,
    pub icon: PathConfig,
    pub command: PathConfig,
    pub working_directory: PathConfig,
    pub arguments: String,
    pub autorerun_count: usize,
    pub ignore_previous_failures: bool,
    pub requires_arguments: bool,
    pub arguments_hint: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PresetItem {
    pub uid: Guid,
    // possible overrides
    pub name: Option<String>,
    pub arguments: Option<String>,
    pub autorerun_count: Option<usize>,
    pub ignore_previous_failures: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScriptPreset {
    pub uid: Guid,
    pub name: String,
    pub icon: PathConfig,
    pub items: Vec<PresetItem>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum ScriptDefinition {
    // taken from the shared config, second bool is whether it's hidden
    ReferenceToShared(Guid, bool),
    // added in the current config
    Original(OriginalScriptDefinition),
    // preset of multiple scripts
    Preset(ScriptPreset),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Guid {
    pub data: u128,
}

impl Serialize for Guid {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // extra dashes at the end not to reallocate the string
        let mut string = format!("{:032x}----", self.data);
        string.truncate(32);
        string.insert(8, '-');
        string.insert(13, '-');
        string.insert(18, '-');
        string.insert(23, '-');
        serializer.serialize_str(&string)
    }
}

impl<'de> Deserialize<'de> for Guid {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut s = String::deserialize(deserializer)?;
        s.retain(|c| c != '-');
        let data = u128::from_str_radix(&s, 16).map_err(serde::de::Error::custom)?;
        Ok(Guid { data })
    }
}

impl Guid {
    pub fn new() -> Guid {
        // generate version 4 GUID
        let mut rng = rand::thread_rng();
        let mut bytes = [0u8; 16];
        rng.fill_bytes(&mut bytes);
        bytes[6] = (bytes[6] & 0x0F) | 0x40;
        bytes[8] = (bytes[8] & 0x3F) | 0x80;
        return Guid {
            data: u128::from_be_bytes(bytes),
        };
    }
}

#[derive(Default, Clone)]
pub struct PathCaches {
    pub logs_path: PathBuf,
    pub work_path: PathBuf,
    pub exe_folder_path: PathBuf,
    pub config_path: PathBuf,
}

#[derive(Default, Clone)]
struct AppArguments {
    custom_config_path: Option<String>,
    custom_logs_path: Option<String>,
    custom_work_path: Option<String>,
    env_vars: Vec<(OsString, OsString)>,
    custom_title: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CustomTheme {
    pub background: [f32; 3],
    pub text: [f32; 3],
    pub primary: [f32; 3],
    pub success: [f32; 3],
    pub danger: [f32; 3],
    pub caption_text: [f32; 3],
    pub error_text: [f32; 3],
}

impl Default for CustomTheme {
    fn default() -> Self {
        CustomTheme {
            background: [0.25, 0.26, 0.29],
            text: [0.0, 0.0, 0.0],
            primary: [0.45, 0.53, 0.855],
            success: [0.31, 0.5, 0.17],
            danger: [0.7, 0.3, 0.3],
            caption_text: [0.7, 0.7, 0.7],
            error_text: [0.9, 0.3, 0.3],
        }
    }
}

pub fn get_app_config_copy() -> AppConfig {
    GLOBAL_CONFIG.with(|config| config.clone())
}

pub fn is_always_on_top() -> bool {
    GLOBAL_CONFIG.with(|config| config.rewritable.always_on_top)
}

pub fn save_config_to_file(config: &AppConfig) {
    let data = serde_json::to_string_pretty(&config);
    let data = match data {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Can't serialize config file {}", err);
            return;
        }
    };
    let result = std::fs::write(&config.paths.config_path, data);
    if let Err(err) = result {
        eprintln!(
            "Can't write config file {}, error {}",
            config.paths.config_path.display(),
            err
        );
    }

    if let Some(local_config) = &config.local_config_body {
        let data = serde_json::to_string_pretty(&local_config);
        let data = match data {
            Ok(data) => data,
            Err(err) => {
                eprintln!("Can't serialize local config file. Error: {}", err);
                return;
            }
        };
        if !config.local_config_path.path.is_empty() {
            let full_config_path = get_full_path(&config.paths, &config.local_config_path);
            let result = std::fs::write(&full_config_path, data);
            if let Err(err) = result {
                eprintln!(
                    "Can't write local config file {}, error {}",
                    &full_config_path.to_str().unwrap_or_default(),
                    err
                );
            }
        }
    }
}

pub fn get_full_path(paths: &PathCaches, path_config: &PathConfig) -> PathBuf {
    match path_config.path_type {
        PathType::WorkingDirRelative => paths.work_path.join(&path_config.path),
        PathType::ScripterExecutableRelative => paths.exe_folder_path.join(&path_config.path),
    }
}

pub fn get_full_optional_path(paths: &PathCaches, path_config: &PathConfig) -> Option<PathBuf> {
    if path_config.path.is_empty() {
        return None;
    }

    Some(match path_config.path_type {
        PathType::WorkingDirRelative => paths.work_path.join(&path_config.path),
        PathType::ScripterExecutableRelative => paths.exe_folder_path.join(&path_config.path),
    })
}

fn get_default_config(app_arguments: AppArguments, config_path: PathBuf) -> AppConfig {
    AppConfig {
        version: LATEST_CONFIG_VERSION.to_string(),
        rewritable: RewritableConfig {
            always_on_top: false,
            window_status_reactions: true,
            keep_window_size: false,
            enable_script_filtering: true,
            enable_title_editing: true,
            custom_theme: Some(CustomTheme::default()),
        },
        script_definitions: Vec::new(),
        paths: PathCaches {
            logs_path: if let Some(custom_logs_path) = app_arguments.custom_logs_path.clone() {
                PathBuf::from(custom_logs_path)
            } else {
                get_default_logs_path()
            },
            work_path: if let Some(custom_work_path) = app_arguments.custom_work_path.clone() {
                PathBuf::from(custom_work_path)
            } else {
                get_default_work_path()
            },
            exe_folder_path: get_exe_folder_path(),
            config_path,
        },
        local_config_path: PathConfig::default(),
        env_vars: app_arguments.env_vars,
        custom_title: app_arguments.custom_title,
        config_read_error: None,
        local_config_body: None,
        displayed_configs_list_cache: Vec::new(),
    }
}

pub fn get_default_local_config(shared_config: &AppConfig) -> LocalConfig {
    LocalConfig {
        version: LATEST_LOCAL_CONFIG_VERSION.to_string(),
        rewritable: shared_config.rewritable.clone(),
        script_definitions: Vec::new(),
    }
}

fn get_config_path(app_arguments: &AppArguments) -> PathBuf {
    if let Some(config_path) = &app_arguments.custom_config_path {
        return PathBuf::from(config_path.clone());
    }

    let config_in_work_path = if let Some(custom_work_path) = &app_arguments.custom_work_path {
        PathBuf::from(custom_work_path).join(WORK_PATH_CONFIG_NAME)
    } else {
        get_default_work_path().join(WORK_PATH_CONFIG_NAME)
    };

    if Path::new(&config_in_work_path).exists() {
        return config_in_work_path;
    }

    return std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(Path::new(""))
        .join(DEFAULT_CONFIG_NAME);
}

fn default_config_with_error(config: &AppConfig, error: String) -> AppConfig {
    AppConfig {
        config_read_error: Some(error),
        ..config.clone()
    }
}

pub fn read_config() -> AppConfig {
    let app_arguments = get_app_arguments();

    let config_path = get_config_path(&app_arguments);

    // create default config with all the non-serializable fields set
    let default_config = get_default_config(app_arguments.clone(), config_path);
    // if config file doesn't exist, create it
    if !default_config.paths.config_path.exists() {
        let data = serde_json::to_string_pretty(&default_config);
        let data = match data {
            Ok(data) => data,
            Err(err) => {
                return default_config_with_error(
                    &default_config,
                    format!(
                        "Failed to serialize default config.\nNotify the developer about this error.\nError: {}",
                        err,
                    )
                )
            },
        };
        let result = std::fs::write(&default_config.paths.config_path, data);
        if result.is_err() {
            return default_config_with_error(
                &default_config,
                format!(
                    "Failed to write default config to '{}'.\nMake sure you have write rights to that folder",
                    default_config.paths.config_path.to_string_lossy()
                ),
            );
        }
    }

    // read the config file from the disk
    let data = std::fs::read_to_string(&default_config.paths.config_path);
    let data = match data {
        Ok(data) => data,
        Err(err) => {
            return default_config_with_error(
                &default_config,
                format!(
                    "Config file '{}' can't be read.\nMake sure you have read rights to that file.\nError: {}",
                    default_config.paths.config_path.to_string_lossy(),
                    err
                ),
            )
        }
    };
    let config_json = serde_json::from_str(&data);
    let mut config_json = match config_json {
        Ok(config_json) => config_json,
        Err(err) => {
            return default_config_with_error(
                &default_config,
                format!(
                    "Config file '{}' has incorrect json format:\n{}",
                    default_config.paths.config_path.to_string_lossy(),
                    err
                ),
            )
        }
    };

    let update_result = update_config_to_the_latest_version(&mut config_json);
    let config = serde_json::from_value(config_json);
    let mut config = match config {
        Ok(config) => config,
        Err(err) => {
            default_config_with_error(
                &default_config,
                format!(
                    "Config file '{}' can't be read.\nMake sure your manual edits were correct.\nError: {}",
                    default_config.paths.config_path.to_string_lossy(),
                    err
                ),
            )
        }
    };

    if update_result == UpdateResult::Updated {
        let data = serde_json::to_string_pretty(&config);
        let data = match data {
            Ok(data) => data,
            Err(err) => {
                return default_config_with_error(
                    &default_config,
                    format!(
                        "Failed to serialize the updated config.\nNotify the developer about this error.\nError: {}",
                        err
                    ),
                )
            }
        };
        let result = std::fs::write(&default_config.paths.config_path, data);
        if result.is_err() {
            return default_config_with_error(
                &default_config,
                format!(
                    "Failed to write the updated config to '{}'.\nMake sure you have write rights to that folder and file",
                    default_config.paths.config_path.to_string_lossy()
                ),
            );
        }
    } else if let UpdateResult::Error(error) = update_result {
        return default_config_with_error(
            &default_config,
            format!(
                "Failed to update config file '{}'.\nError: {}",
                default_config.paths.config_path.to_string_lossy(),
                error
            ),
        );
    }

    if !config.local_config_path.path.is_empty() {
        let full_local_config_path =
            get_full_path(&default_config.paths, &config.local_config_path);
        let local_config = match read_local_config(full_local_config_path.clone(), &mut config) {
            Ok(local_config) => local_config,
            Err(error) => {
                return default_config_with_error(
                    &default_config,
                    format!(
                        "Failed to read local config file '{}'.\nError: {}",
                        full_local_config_path.to_string_lossy(),
                        error
                    ),
                );
            }
        };
        config.local_config_body = Some(Box::new(local_config));
    }

    config.paths = default_config.paths;
    config.env_vars = app_arguments.env_vars;
    config.custom_title = app_arguments.custom_title;

    return config;
}

pub fn populate_shared_scripts_from_config(app_config: &mut AppConfig) {
    if let Some(mut local_config) = app_config.local_config_body.take() {
        populate_shared_scripts(&mut local_config, app_config);
        app_config.local_config_body = Some(local_config);
    }
}

pub fn get_original_script_definition_by_uid(
    app_config: &AppConfig,
    script_uid: Guid,
) -> Option<ScriptDefinition> {
    if let Some(local_config) = &app_config.local_config_body {
        for script_definition in &local_config.script_definitions {
            if original_script_definition_search_predicate(script_definition, &script_uid) {
                return Some(script_definition.clone());
            }
        }
    }

    for script_definition in &app_config.script_definitions {
        if original_script_definition_search_predicate(script_definition, &script_uid) {
            return Some(script_definition.clone());
        }
    }
    return None;
}

fn original_script_definition_search_predicate(
    script_definition: &ScriptDefinition,
    script_uid: &Guid,
) -> bool {
    match script_definition {
        ScriptDefinition::Original(script) => {
            if script.uid == *script_uid {
                return true;
            }
            return false;
        }
        ScriptDefinition::Preset(preset) => {
            if preset.uid == *script_uid {
                return true;
            }
            return false;
        }
        _ => false,
    }
}

fn read_local_config(
    config_path: PathBuf,
    shared_config: &mut AppConfig,
) -> Result<LocalConfig, String> {
    // if config file doesn't exist, create it
    if !config_path.exists() {
        // create default config with all the non-serializable fields set
        let default_config = get_default_local_config(shared_config);
        let data = serde_json::to_string_pretty(&default_config);
        let data = match data {
            Ok(data) => data,
            Err(err) => {
                return Err(format!(
                        "Failed to serialize default config.\nNotify the developer about this error.\nError: {}",
                        err,
                    )
                )
            },
        };
        let result = std::fs::write(&config_path, data);
        if result.is_err() {
            return Err(format!(
                    "Failed to write default config to the file.\nMake sure you have write rights to that folder",
                )
            );
        }
    }

    // read the config file from the disk
    let data = std::fs::read_to_string(&config_path);
    let data = match data {
        Ok(data) => data,
        Err(err) => {
            return Err(format!(
            "Config file can't be read.\nMake sure you have read rights to that file.\nError: {}",
            err
        ))
        }
    };
    let config_json = serde_json::from_str(&data);
    let mut config_json = match config_json {
        Ok(config_json) => config_json,
        Err(err) => return Err(format!("Config file has incorrect json format:\n{}", err)),
    };

    let update_result = update_local_config_to_the_latest_version(&mut config_json);
    let config = serde_json::from_value(config_json);
    let mut config: LocalConfig = match config {
        Ok(config) => config,
        Err(err) => {
            return Err(format!(
                "Config file can't be read.\nMake sure your manual edits were correct.\nError: {}",
                err
            ))
        }
    };

    if update_result == UpdateResult::Updated {
        let data = serde_json::to_string_pretty(&config);
        let data = match data {
            Ok(data) => data,
            Err(err) => {
                return Err(format!(
                        "Failed to serialize the updated config.\nNotify the developer about this error.\nError: {}",
                        err
                    )
                );
            }
        };
        let result = std::fs::write(&config_path, data);
        if result.is_err() {
            return Err(format!(
                    "Failed to write the updated config.\nMake sure you have write rights to that folder and file",
                ),
            );
        }
    } else if let UpdateResult::Error(error) = update_result {
        return Err(format!("Failed to update config file.\nError: {}", error));
    }

    populate_shared_scripts(&mut config, shared_config);

    return Ok(config);
}

fn get_app_arguments() -> AppArguments {
    let mut custom_config_path = None;
    let mut custom_logs_path = None;
    let mut custom_work_path = None;
    let mut env_vars = Vec::new();
    let mut custom_title = None;

    let args: Vec<String> = std::env::args().collect();
    for i in 1..args.len() {
        let arg = &args[i];
        if arg == "--config-path" {
            if i + 1 < args.len() {
                custom_config_path = Some(args[i + 1].clone());
            }
        } else if arg == "--logs-path" {
            if i + 1 < args.len() {
                custom_logs_path = Some(args[i + 1].clone());
            }
        } else if arg == "--work-path" {
            if i + 1 < args.len() {
                custom_work_path = Some(args[i + 1].clone());
            }
        } else if arg == "--env" {
            if i + 2 < args.len() {
                env_vars.push((
                    OsString::from_str(&args[i + 1]).unwrap_or_default(),
                    OsString::from_str(&args[i + 2]).unwrap_or_default(),
                ));
            }
        } else if arg == "--title" {
            if i + 1 < args.len() {
                custom_title = Some(args[i + 1].clone());
            }
        }
    }

    AppArguments {
        custom_config_path,
        custom_logs_path,
        custom_work_path,
        env_vars,
        custom_title,
    }
}

fn get_exe_folder_path() -> PathBuf {
    std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(&PathBuf::from(""))
        .to_str()
        .unwrap_or_default()
        .to_string()
        .trim_start_matches("\\\\?\\")
        .into()
}

fn get_default_logs_path() -> PathBuf {
    return get_exe_folder_path().join("scripter_logs");
}

fn get_default_work_path() -> PathBuf {
    return std::env::current_dir().unwrap_or_default();
}

fn populate_shared_scripts(local_config: &mut LocalConfig, shared_config: &mut AppConfig) {
    // find all the shared scripts that are missing from the local config, and populate them
    let mut previous_script_idx = None;
    let mut has_configs_to_remove = false;
    for script in &shared_config.script_definitions {
        let original_script_uid = match script {
            ScriptDefinition::ReferenceToShared(_, _) => {
                continue;
            }
            ScriptDefinition::Original(script) => script.uid.clone(),
            ScriptDefinition::Preset(preset) => preset.uid.clone(),
        };

        // find position of the script in the local config
        let script_idx =
            local_config
                .script_definitions
                .iter()
                .position(|local_script: &ScriptDefinition| match local_script {
                    ScriptDefinition::ReferenceToShared(shared_script_uid, _is_hidden) => {
                        *shared_script_uid == original_script_uid
                    }
                    _ => false,
                });

        match script_idx {
            Some(script_idx) => {
                previous_script_idx = Some(script_idx);
                has_configs_to_remove = true;
            }
            None => {
                match &mut previous_script_idx {
                    Some(previous_script_idx) => {
                        // insert the script after the previous script
                        local_config.script_definitions.insert(
                            *previous_script_idx + 1,
                            ScriptDefinition::ReferenceToShared(original_script_uid.clone(), false),
                        );
                        *previous_script_idx = *previous_script_idx + 1;
                    }
                    None => {
                        // insert the script at the beginning
                        local_config.script_definitions.insert(
                            0,
                            ScriptDefinition::ReferenceToShared(original_script_uid.clone(), false),
                        );
                        previous_script_idx = Some(0);
                    }
                }
            }
        }
    }

    if has_configs_to_remove {
        // remove all the scripts that are not in the shared config
        local_config.script_definitions.retain(
            |local_script: &ScriptDefinition| match local_script {
                ScriptDefinition::ReferenceToShared(shared_script_uid, _is_hidden) => shared_config
                    .script_definitions
                    .iter()
                    .any(|script| match script {
                        ScriptDefinition::ReferenceToShared(_, _) => false,
                        ScriptDefinition::Original(script) => *shared_script_uid == script.uid,
                        ScriptDefinition::Preset(preset) => *shared_script_uid == preset.uid,
                    }),
                _ => true,
            },
        );
    }
}
