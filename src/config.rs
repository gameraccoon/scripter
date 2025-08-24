// Copyright (C) Pavel Grebnev 2023-2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use crate::app_arguments::{self, AppArguments};
use crate::config_updaters::{
    update_config_to_the_latest_version, update_local_config_to_the_latest_version,
    LATEST_CONFIG_VERSION, LATEST_LOCAL_CONFIG_VERSION,
};
use crate::json_file_updater::{JsonFileUpdaterError, UpdateResult};
use crate::key_mapping::{CustomKeyCode, CustomModifiers};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::{Path, PathBuf};

const DEFAULT_CONFIG_NAME: &str = "scripter_config.json";
const WORK_PATH_CONFIG_NAME: &str = ".scripter_config.json";
thread_local!(static GLOBAL_CONFIG: AppConfig = read_config());

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
pub enum PathType {
    WorkingDirRelative,
    ScripterExecutableRelative,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
pub enum ConfigUpdateBehavior {
    OnStartup,
    OnManualSave,
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
#[derive(Clone, Deserialize, Serialize)]
pub struct RewritableConfig {
    pub window_status_reactions: bool,
    pub keep_window_size: bool,
    pub enable_script_filtering: bool,
    pub show_working_directory: bool,
    pub enable_title_editing: bool,
    pub config_version_update_behavior: ConfigUpdateBehavior,
    pub custom_theme: Option<CustomTheme>,
    pub app_actions_keybinds: Vec<AppActionKeybind>,
    pub script_keybinds: Vec<ScriptKeybind>,
    pub show_current_git_branch: bool,
    pub quick_launch_scripts: Vec<Guid>,
}

#[derive(Clone)]
pub enum ConfigReadError {
    FileReadError {
        file_path: PathBuf,
        error: String,
    },
    DataParseJsonError {
        file_path: PathBuf,
        error: String,
    },
    UpdaterUnknownVersion {
        file_path: PathBuf,
        version: String,
        latest_version: String,
    },
    ConfigDeserializeError {
        file_path: PathBuf,
        error: String,
    },
    ConfigSerializeError {
        error: String,
    },
    FileWriteError {
        file_path: PathBuf,
        error: String,
    },
}

#[derive(Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub version: String,
    pub rewritable: RewritableConfig,
    pub script_definitions: Vec<ScriptDefinition>,
    pub local_config_path: PathConfig,
    #[serde(skip)]
    pub is_read_only: bool,
    #[serde(skip)]
    pub paths: PathCaches,
    #[serde(skip)]
    pub env_vars: Vec<(OsString, OsString)>,
    #[serde(skip)]
    pub custom_title: Option<String>,
    #[serde(skip)]
    pub config_read_error: Option<ConfigReadError>,
    #[serde(skip)]
    pub local_config_body: Option<Box<LocalConfig>>,
    #[serde(skip)]
    pub arguments_read_error: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct LocalConfig {
    pub version: String,
    pub rewritable: RewritableConfig,
    pub script_definitions: Vec<ScriptDefinition>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ReferenceToSharedScript {
    pub uid: Guid,
    pub is_hidden: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct ArgumentPlaceholder {
    pub placeholder: String,
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum ArgumentRequirement {
    Required,
    Optional,
    Hidden,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OriginalScriptDefinition {
    pub uid: Guid,
    pub name: String,
    pub icon: PathConfig,
    pub command: PathConfig,
    pub working_directory: PathConfig,
    pub arguments: String,
    pub argument_placeholders: Vec<ArgumentPlaceholder>,
    pub arguments_requirement: ArgumentRequirement,
    pub autorerun_count: usize,
    pub ignore_previous_failures: bool,
    pub arguments_hint: String,
    pub custom_executor: Option<Vec<String>>,
    pub is_hidden: bool,
    pub autoclean_on_success: bool,
    pub ignore_output: bool,
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
    ReferenceToShared(ReferenceToSharedScript),
    // added in the current config
    Original(OriginalScriptDefinition),
    // preset of multiple scripts
    Preset(ScriptPreset),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
        Guid {
            data: u128::from_be_bytes(bytes),
        }
    }
}

pub const GUID_NULL: Guid = Guid { data: 0 };

#[derive(Default, Clone)]
pub struct PathCaches {
    pub logs_path: PathBuf,
    pub work_path: PathBuf,
    pub exe_folder_path: PathBuf,
    pub config_path: PathBuf,
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

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum AppAction {
    RequestCloseApp,
    FocusFilter,
    TrySwitchWindowEditMode,
    RescheduleScripts,
    RunScriptsInParallel,
    RunScriptsAfterExecution,
    StopScripts,
    ClearExecutionScripts,
    MaximizeOrRestoreExecutionPane,
    CursorConfirm,
    MoveScriptDown,
    MoveScriptUp,
    SwitchPaneFocusForward,
    SwitchPaneFocusBackwards,
    MoveCursorDown,
    MoveCursorUp,
    RemoveCursorScript,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomKeybind {
    pub key: CustomKeyCode,
    pub modifiers: CustomModifiers,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppActionKeybind {
    pub action: AppAction,
    pub keybind: CustomKeybind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptKeybind {
    pub script_uid: Guid,
    pub keybind: CustomKeybind,
}

pub fn get_app_config_copy() -> AppConfig {
    GLOBAL_CONFIG.with(|config| config.clone())
}

pub fn get_arguments_read_error() -> Option<String> {
    GLOBAL_CONFIG.with(|config| config.arguments_read_error.clone())
}

pub fn save_config_to_file(config: &AppConfig) -> bool {
    let data = serde_json::to_string_pretty(&config);
    let data = match data {
        Ok(data) => data,
        Err(err) => {
            eprintln!("Can't serialize config file {}", err);
            return false;
        }
    };
    let result = std::fs::write(&config.paths.config_path, data);
    if let Err(err) = result {
        eprintln!(
            "Can't write config file {}, error {}",
            config.paths.config_path.display(),
            err
        );
        return false;
    }

    if let Some(local_config) = &config.local_config_body {
        let data = serde_json::to_string_pretty(&local_config);
        let data = match data {
            Ok(data) => data,
            Err(err) => {
                eprintln!("Can't serialize local config file. Error: {}", err);
                return false;
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
                return false;
            }
        }
    }

    true
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
            window_status_reactions: true,
            keep_window_size: false,
            enable_script_filtering: true,
            show_working_directory: true,
            enable_title_editing: true,
            config_version_update_behavior: ConfigUpdateBehavior::OnStartup,
            custom_theme: Some(CustomTheme::default()),
            app_actions_keybinds: get_default_app_action_keybinds(),
            script_keybinds: Vec::new(),
            show_current_git_branch: false,
            quick_launch_scripts: Vec::new(),
        },
        script_definitions: Vec::new(),
        is_read_only: !has_write_permission(&config_path),
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
        arguments_read_error: app_arguments.read_error,
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

    std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(Path::new(""))
        .join(DEFAULT_CONFIG_NAME)
}

fn default_config_with_error(config: &AppConfig, error: ConfigReadError) -> AppConfig {
    AppConfig {
        config_read_error: Some(error),
        ..config.clone()
    }
}

pub fn read_config() -> AppConfig {
    let app_arguments = app_arguments::get_app_arguments();

    let config_path = get_config_path(&app_arguments);

    // create default config with all the non-serializable fields set
    let default_config = get_default_config(app_arguments.clone(), config_path);
    // if config file doesn't exist, create it
    if !default_config.paths.config_path.exists() && !default_config.is_read_only {
        let data = serde_json::to_string_pretty(&default_config);
        let data = match data {
            Ok(data) => data,
            Err(err) => {
                return default_config_with_error(
                    &default_config,
                    ConfigReadError::ConfigSerializeError {
                        error: format!("Failed to serialize default config: {}", err,),
                    },
                )
            }
        };
        let result = std::fs::write(&default_config.paths.config_path, data);
        if let Err(err) = result {
            return default_config_with_error(
                &default_config,
                ConfigReadError::FileWriteError {
                    file_path: default_config.paths.config_path.clone(),
                    error: err.to_string(),
                },
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
                ConfigReadError::FileReadError {
                    file_path: default_config.paths.config_path.clone(),
                    error: err.to_string(),
                },
            )
        }
    };
    let config_json = serde_json::from_str(&data);
    let mut config_json = match config_json {
        Ok(config_json) => config_json,
        Err(err) => {
            return default_config_with_error(
                &default_config,
                ConfigReadError::DataParseJsonError {
                    file_path: default_config.paths.config_path.clone(),
                    error: err.to_string(),
                },
            )
        }
    };

    let update_result = update_config_to_the_latest_version(&mut config_json);
    let config = serde_json::from_value(config_json);
    let mut config: AppConfig = match config {
        Ok(config) => config,
        Err(err) => {
            return default_config_with_error(
                &default_config,
                ConfigReadError::ConfigDeserializeError {
                    file_path: default_config.paths.config_path.clone(),
                    error: err.to_string(),
                },
            )
        }
    };

    if update_result == UpdateResult::Updated {
        if !config.is_read_only
            && config.rewritable.config_version_update_behavior == ConfigUpdateBehavior::OnStartup
        {
            let data = serde_json::to_string_pretty(&config);
            let data = match data {
                Ok(data) => data,
                Err(err) => {
                    return default_config_with_error(
                        &default_config,
                        ConfigReadError::ConfigSerializeError {
                            error: format!("Failed to serialize the updated config: {}", err),
                        },
                    )
                }
            };
            let result = std::fs::write(&default_config.paths.config_path, data);
            if let Err(err) = result {
                return default_config_with_error(
                    &default_config,
                    ConfigReadError::FileWriteError {
                        file_path: default_config.paths.config_path.clone(),
                        error: err.to_string(),
                    },
                );
            }
        }
    } else if let UpdateResult::Error(error) = update_result {
        let file_path = default_config.paths.config_path.clone();
        return match error {
            JsonFileUpdaterError::UnknownVersion {
                version,
                latest_version,
            } => default_config_with_error(
                &default_config,
                ConfigReadError::UpdaterUnknownVersion {
                    file_path,
                    version,
                    latest_version,
                },
            ),
        };
    }

    config.paths = default_config.paths;
    config.is_read_only = default_config.is_read_only;
    config.env_vars = default_config.env_vars;
    config.custom_title = default_config.custom_title;
    config.arguments_read_error = default_config.arguments_read_error;

    if !config.local_config_path.path.is_empty() {
        let full_local_config_path = get_full_path(&config.paths, &config.local_config_path);

        if !config.is_read_only && !has_write_permission(&full_local_config_path) {
            // if at lease one of the configs is read-only, both should be read-only
            config.is_read_only = true;
        }

        let local_config = match read_local_config(full_local_config_path.clone(), &mut config) {
            Ok(local_config) => local_config,
            Err(error) => {
                return default_config_with_error(&config, error);
            }
        };
        config.local_config_body = Some(Box::new(local_config));
    }

    config
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
        if let Some(result) =
            find_original_script_definition_by_uid(&local_config.script_definitions, &script_uid)
        {
            return Some(result);
        }
    }

    find_original_script_definition_by_uid(&app_config.script_definitions, &script_uid)
}

fn find_original_script_definition_by_uid(
    script_definitions: &Vec<ScriptDefinition>,
    script_uid: &Guid,
) -> Option<ScriptDefinition> {
    for script_definition in script_definitions {
        if original_script_definition_search_predicate(script_definition, &script_uid) {
            return Some(script_definition.clone());
        }
    }
    None
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
            false
        }
        ScriptDefinition::Preset(preset) => {
            if preset.uid == *script_uid {
                return true;
            }
            false
        }
        _ => false,
    }
}

pub fn get_current_rewritable_config(app_config: &AppConfig) -> &RewritableConfig {
    if let Some(local_config) = &app_config.local_config_body {
        return &local_config.rewritable;
    }

    &app_config.rewritable
}

fn read_local_config(
    config_path: PathBuf,
    shared_config: &mut AppConfig,
) -> Result<LocalConfig, ConfigReadError> {
    // if config file doesn't exist, create it
    if !config_path.exists() {
        // create default config with all the non-serializable fields set
        let default_config = get_default_local_config(shared_config);
        let data = serde_json::to_string_pretty(&default_config);
        let data = match data {
            Ok(data) => data,
            Err(err) => {
                return Err(ConfigReadError::ConfigSerializeError {
                    error: format!("Failed to serialize default local config: {}", err,),
                })
            }
        };
        let result = std::fs::write(&config_path, data);
        if let Err(err) = result {
            return Err(ConfigReadError::FileWriteError {
                file_path: config_path,
                error: err.to_string(),
            });
        }
    }

    // read the config file from the disk
    let data = std::fs::read_to_string(&config_path);
    let data = match data {
        Ok(data) => data,
        Err(err) => {
            return Err(ConfigReadError::FileReadError {
                file_path: config_path,
                error: err.to_string(),
            })
        }
    };
    let config_json = serde_json::from_str(&data);
    let mut config_json = match config_json {
        Ok(config_json) => config_json,
        Err(err) => {
            return Err(ConfigReadError::DataParseJsonError {
                file_path: config_path,
                error: err.to_string(),
            })
        }
    };

    let update_result = update_local_config_to_the_latest_version(&mut config_json);
    let config = serde_json::from_value(config_json);
    let mut config: LocalConfig = match config {
        Ok(config) => config,
        Err(err) => {
            return Err(ConfigReadError::ConfigDeserializeError {
                file_path: config_path,
                error: err.to_string(),
            })
        }
    };

    if update_result == UpdateResult::Updated {
        if !shared_config.is_read_only
            && config.rewritable.config_version_update_behavior == ConfigUpdateBehavior::OnStartup
        {
            let data = serde_json::to_string_pretty(&config);
            let data = match data {
                Ok(data) => data,
                Err(err) => {
                    return Err(ConfigReadError::ConfigSerializeError {
                        error: format!("Failed to serialize the updated local config: {}", err),
                    });
                }
            };
            let result = std::fs::write(&config_path, data);
            if let Err(err) = result {
                return Err(ConfigReadError::FileWriteError {
                    file_path: config_path,
                    error: err.to_string(),
                });
            }
        }
    } else if let UpdateResult::Error(error) = update_result {
        return match error {
            JsonFileUpdaterError::UnknownVersion {
                version,
                latest_version,
            } => Err(ConfigReadError::UpdaterUnknownVersion {
                file_path: config_path,
                version,
                latest_version,
            }),
        };
    }

    populate_shared_scripts(&mut config, shared_config);

    Ok(config)
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
    get_exe_folder_path().join("scripter_logs")
}

fn get_default_work_path() -> PathBuf {
    std::env::current_dir().unwrap_or_default()
}

fn populate_shared_scripts(local_config: &mut LocalConfig, shared_config: &mut AppConfig) {
    // find all the shared scripts that are missing from the local config, and populate them
    let mut previous_script_idx = None;
    let mut has_configs_to_remove = false;
    for script in &shared_config.script_definitions {
        let (original_script_uid, is_hidden) = match script {
            ScriptDefinition::ReferenceToShared(_) => {
                continue;
            }
            ScriptDefinition::Original(script) => (script.uid.clone(), script.is_hidden),
            ScriptDefinition::Preset(preset) => (preset.uid.clone(), false),
        };

        // find position of the script in the local config
        let script_idx =
            local_config
                .script_definitions
                .iter()
                .position(|local_script: &ScriptDefinition| match local_script {
                    ScriptDefinition::ReferenceToShared(reference) => {
                        reference.uid == original_script_uid
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
                            ScriptDefinition::ReferenceToShared(ReferenceToSharedScript {
                                uid: original_script_uid.clone(),
                                is_hidden,
                            }),
                        );
                        *previous_script_idx = *previous_script_idx + 1;
                    }
                    None => {
                        // insert the script at the beginning
                        local_config.script_definitions.insert(
                            0,
                            ScriptDefinition::ReferenceToShared(ReferenceToSharedScript {
                                uid: original_script_uid.clone(),
                                is_hidden,
                            }),
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
                ScriptDefinition::ReferenceToShared(reference) => shared_config
                    .script_definitions
                    .iter()
                    .any(|script| match script {
                        ScriptDefinition::ReferenceToShared(_) => false,
                        ScriptDefinition::Original(script) => reference.uid == script.uid,
                        ScriptDefinition::Preset(preset) => reference.uid == preset.uid,
                    }),
                _ => true,
            },
        );
    }
}

fn has_write_permission(path: &Path) -> bool {
    // check if the file exists, and if it doesn't check for the parent directory
    let path = if path.exists() {
        path
    } else {
        match path.parent() {
            Some(parent) => parent,
            None => return false,
        }
    };

    // check if able to write to the file/directory
    let md = std::fs::metadata(path);
    if let Err(err) = md {
        eprintln!(
            "Can't get metadata for the file/directory '{}': {}",
            path.to_str().unwrap_or_default(),
            err
        );
        return false;
    }
    let md = md.unwrap();

    let permissions = md.permissions();
    !permissions.readonly()
}

fn get_default_app_action_keybinds() -> Vec<AppActionKeybind> {
    let mut keybinds = Vec::new();
    keybinds.push(AppActionKeybind {
        action: AppAction::RequestCloseApp,
        keybind: CustomKeybind {
            key: CustomKeyCode::W,
            modifiers: CustomModifiers::COMMAND,
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::FocusFilter,
        keybind: CustomKeybind {
            key: CustomKeyCode::F,
            modifiers: CustomModifiers::COMMAND,
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::TrySwitchWindowEditMode,
        keybind: CustomKeybind {
            key: CustomKeyCode::E,
            modifiers: CustomModifiers::COMMAND,
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::RescheduleScripts,
        keybind: CustomKeybind {
            key: CustomKeyCode::R,
            modifiers: CustomModifiers::COMMAND | CustomModifiers::SHIFT,
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::RunScriptsAfterExecution,
        keybind: CustomKeybind {
            key: CustomKeyCode::R,
            modifiers: CustomModifiers::COMMAND,
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::RunScriptsInParallel,
        keybind: CustomKeybind {
            key: CustomKeyCode::R,
            modifiers: CustomModifiers::COMMAND | CustomModifiers::ALT,
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::StopScripts,
        keybind: CustomKeybind {
            key: CustomKeyCode::C,
            modifiers: CustomModifiers::COMMAND | CustomModifiers::SHIFT,
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::ClearExecutionScripts,
        keybind: CustomKeybind {
            key: CustomKeyCode::C,
            modifiers: CustomModifiers::COMMAND,
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::MaximizeOrRestoreExecutionPane,
        keybind: CustomKeybind {
            key: CustomKeyCode::Q,
            modifiers: CustomModifiers::COMMAND,
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::CursorConfirm,
        keybind: CustomKeybind {
            key: CustomKeyCode::Enter,
            modifiers: CustomModifiers::empty(),
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::CursorConfirm,
        keybind: CustomKeybind {
            key: CustomKeyCode::Enter,
            modifiers: CustomModifiers::COMMAND,
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::CursorConfirm,
        keybind: CustomKeybind {
            key: CustomKeyCode::Enter,
            modifiers: CustomModifiers::COMMAND | CustomModifiers::ALT,
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::MoveScriptDown,
        keybind: CustomKeybind {
            key: CustomKeyCode::Down,
            modifiers: CustomModifiers::SHIFT,
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::MoveScriptUp,
        keybind: CustomKeybind {
            key: CustomKeyCode::Up,
            modifiers: CustomModifiers::SHIFT,
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::SwitchPaneFocusBackwards,
        keybind: CustomKeybind {
            key: CustomKeyCode::Tab,
            modifiers: CustomModifiers::SHIFT,
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::MoveCursorDown,
        keybind: CustomKeybind {
            key: CustomKeyCode::Down,
            modifiers: CustomModifiers::empty(),
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::MoveCursorUp,
        keybind: CustomKeybind {
            key: CustomKeyCode::Up,
            modifiers: CustomModifiers::empty(),
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::SwitchPaneFocusForward,
        keybind: CustomKeybind {
            key: CustomKeyCode::Tab,
            modifiers: CustomModifiers::empty(),
        },
    });
    keybinds.push(AppActionKeybind {
        action: AppAction::RemoveCursorScript,
        keybind: CustomKeybind {
            key: CustomKeyCode::Delete,
            modifiers: CustomModifiers::empty(),
        },
    });

    keybinds
}

pub fn get_default_executor() -> Vec<String> {
    let mut executor = Vec::with_capacity(2);
    #[cfg(target_os = "windows")]
    {
        executor.push("cmd".to_string());
        executor.push("/C".to_string());
    }

    #[cfg(not(target_os = "windows"))]
    {
        executor.push("sh".to_string());
        executor.push("-c".to_string());
    }

    executor
}
