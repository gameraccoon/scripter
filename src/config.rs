use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const DEFAULT_CONFIG_NAME: &str = "scripter_config.json";
thread_local!(static GLOBAL_CONFIG: AppConfig = read_config());

#[derive(Default, Clone, Deserialize, Serialize)]
pub struct AppConfig {
    pub script_definitions: Vec<ScriptDefinition>,
    pub always_on_top: bool,
    #[serde(skip)]
    pub paths: PathCaches,
    pub custom_theme: Option<CustomTheme>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ScriptDefinition {
    pub name: String,
    pub command: Box<Path>,
    pub arguments: String,
    pub path_relative_to_scripter: bool,
    pub autorerun_count: usize,
    pub ignore_previous_failures: bool,
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
}

#[derive(Default, Clone, Serialize, Deserialize)]
pub struct CustomTheme {
    pub background: [f32; 3],
    pub text: [f32; 3],
    pub primary: [f32; 3],
    pub success: [f32; 3],
    pub danger: [f32; 3],
}

pub fn get_app_config_copy() -> AppConfig {
    GLOBAL_CONFIG.with(|config| config.clone())
}

pub fn is_always_on_top() -> bool {
    GLOBAL_CONFIG.with(|config| config.always_on_top)
}

pub fn get_script_output_path(logs_path: PathBuf, script_idx: isize, retry_count: usize) -> PathBuf {
    if retry_count == 0 {
        logs_path.join(format!("script{}_output.log", script_idx))
    } else {
        logs_path.join(format!("script{}_retry{}_output.log", script_idx, retry_count))
    }
}

fn get_default_config(app_arguments: AppArguments, config_path: PathBuf) -> AppConfig {
    AppConfig {
        script_definitions: Vec::new(),
        always_on_top: true,
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
        custom_theme: None,
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

fn read_config() -> AppConfig {
    let app_arguments = get_app_arguments();

    let config_path = get_config_path(&app_arguments);

    let default_config = get_default_config(app_arguments.clone(), config_path);
    if !default_config.paths.config_path.exists() {
        let data = serde_json::to_string_pretty(&default_config);
        if data.is_err() {
            return default_config;
        }
        let data = data.unwrap();
        let result = std::fs::write(&default_config.paths.config_path, data);
        if result.is_err() {
            return default_config;
        }
    }

    let data = std::fs::read_to_string(&default_config.paths.config_path);
    if data.is_err() {
        return default_config;
    }
    let data = data.unwrap();
    let config = serde_json::from_str(&data);
    if config.is_err() {
        return default_config;
    }

    let mut config: AppConfig = config.unwrap();
    config.paths = default_config.paths;
    return config;
}

fn get_app_arguments() -> AppArguments {
    let mut custom_config_path = None;
    let mut custom_logs_path = None;
    let mut custom_work_path = None;

    let args: Vec<String> = std::env::args().collect();
    for i in 1..args.len() {
        let arg = &args[i];
        println!("'{}'", &arg);
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
        }
    }

    AppArguments {
        custom_config_path,
        custom_logs_path,
        custom_work_path,
    }
}

fn get_exe_folder_path() -> PathBuf {
    return std::env::current_exe()
        .unwrap_or_default()
        .parent()
        .unwrap_or(&PathBuf::from(""))
        .to_path_buf();
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
