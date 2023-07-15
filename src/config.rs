use crate::config_updaters::{update_config_to_the_latest_version, LATEST_CONFIG_VERSION};
use crate::json_config_updater::UpdateResult;
use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use rand::RngCore;

const DEFAULT_CONFIG_NAME: &str = "scripter_config.json";
thread_local!(static GLOBAL_CONFIG: AppConfig = read_config());

#[derive(Default, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub script_definitions: Vec<ScriptDefinition>,
    pub version: String,
    pub always_on_top: bool,
    pub window_status_reactions: bool,
    pub icon_path_relative_to_scripter: bool,
    pub keep_window_size: bool,
    pub custom_theme: Option<CustomTheme>,
    #[serde(skip)]
    pub paths: PathCaches,
    #[serde(skip)]
    pub env_vars: Vec<(OsString, OsString)>,
    #[serde(skip)]
    pub custom_title: Option<String>,
    #[serde(skip)]
    pub config_read_error: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScriptDefinition {
    pub uid: Guid,
    pub name: String,
    pub icon: Option<String>,
    pub command: String,
    pub arguments: String,
    pub path_relative_to_scripter: bool,
    pub autorerun_count: usize,
    pub ignore_previous_failures: bool,
}

#[derive(Debug, Clone)]
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
        return Guid{ data: u128::from_be_bytes(bytes) };
    }
}

#[derive(Default, Clone)]
pub struct PathCaches {
    pub logs_path: PathBuf,
    pub work_path: PathBuf,
    pub exe_folder_path: PathBuf,
    pub config_path: PathBuf,
    pub icons_path: PathBuf,
}

#[derive(Default, Clone)]
struct AppArguments {
    custom_config_path: Option<String>,
    custom_logs_path: Option<String>,
    custom_work_path: Option<String>,
    env_vars: Vec<(OsString, OsString)>,
    custom_title: Option<String>,
    icons_path: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CustomTheme {
    pub background: [f32; 3],
    pub text: [f32; 3],
    pub primary: [f32; 3],
    pub success: [f32; 3],
    pub danger: [f32; 3],
}

impl Default for CustomTheme {
    fn default() -> Self {
        CustomTheme {
            background: [0.25, 0.26, 0.29],
            text: [0.0, 0.0, 0.0],
            primary: [0.45, 0.53, 0.855],
            success: [0.31, 0.5, 0.17],
            danger: [1.0, 0.0, 0.0],
        }
    }
}

pub fn get_app_config_copy() -> AppConfig {
    GLOBAL_CONFIG.with(|config| config.clone())
}

pub fn is_always_on_top() -> bool {
    GLOBAL_CONFIG.with(|config| config.always_on_top)
}

pub fn get_script_log_directory(logs_path: &PathBuf, script_idx: isize) -> PathBuf {
    logs_path.join(format!("script_{}", script_idx))
}

pub fn get_script_output_path(
    logs_path: &PathBuf,
    script_idx: isize,
    retry_count: usize,
) -> PathBuf {
    let path = get_script_log_directory(logs_path, script_idx);
    if retry_count == 0 {
        path.join("output.log")
    } else {
        path.join(format!("retry{}_output.log", retry_count))
    }
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
}

fn get_default_config(app_arguments: AppArguments, config_path: PathBuf) -> AppConfig {
    AppConfig {
        script_definitions: Vec::new(),
        version: LATEST_CONFIG_VERSION.to_string(),
        always_on_top: false,
        window_status_reactions: true,
        icon_path_relative_to_scripter: true,
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
            icons_path: if let Some(icons_path) = app_arguments.icons_path.clone() {
                PathBuf::from(icons_path)
            } else {
                get_default_icons_path()
            },
            config_path,
        },
        custom_theme: None,
        keep_window_size: false,
        env_vars: app_arguments.env_vars,
        custom_title: app_arguments.custom_title,
        config_read_error: None,
    }
}

fn get_config_path(app_arguments: &AppArguments) -> PathBuf {
    if let Some(config_path) = app_arguments.custom_config_path.clone() {
        PathBuf::from(config_path.clone())
    } else {
        std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or(Path::new(""))
            .join(DEFAULT_CONFIG_NAME)
    }
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

    config.paths = default_config.paths;
    config.env_vars = app_arguments.env_vars;
    config.custom_title = app_arguments.custom_title;

    if !app_arguments.icons_path.is_some() && !config.icon_path_relative_to_scripter {
        config.paths.icons_path = config.paths.work_path.clone();
    }

    for script_definition in &mut config.script_definitions {
        if let Some(icon) = &script_definition.icon {
            if icon.is_empty() {
                script_definition.icon = None;
            }
        }
    }

    return config;
}

fn get_app_arguments() -> AppArguments {
    let mut custom_config_path = None;
    let mut custom_logs_path = None;
    let mut custom_work_path = None;
    let mut env_vars = Vec::new();
    let mut custom_title = None;
    let mut icons_path = None;

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
        } else if arg == "--icons-path" {
            if i + 1 < args.len() {
                icons_path = Some(args[i + 1].clone());
            }
        }
    }

    AppArguments {
        custom_config_path,
        custom_logs_path,
        custom_work_path,
        env_vars,
        custom_title,
        icons_path,
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
    let pid = std::process::id();
    return get_exe_folder_path()
        .join("scripter_logs")
        .join(format!("exec_logs_{}", pid));
}

fn get_default_work_path() -> PathBuf {
    return std::env::current_dir().unwrap_or_default();
}

fn get_default_icons_path() -> PathBuf {
    return get_exe_folder_path();
}
