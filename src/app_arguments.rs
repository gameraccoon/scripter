// Copyright (C) Pavel Grebnev 2023-2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use std::ffi::OsString;
use std::str::FromStr;

#[derive(Default, Clone)]
pub struct AppArguments {
    pub custom_config_path: Option<String>,
    pub custom_logs_path: Option<String>,
    pub custom_work_path: Option<String>,
    pub env_vars: Vec<(OsString, OsString)>,
    pub custom_title: Option<String>,
    pub read_error: Option<String>,
}

struct ArgumentDefinition {
    name: &'static str,
    syntax: &'static str,
    description: &'static str,
    number_of_args: usize,
}

pub fn get_app_arguments() -> AppArguments {
    const SUPPORTED_ARGS: &[ArgumentDefinition] = &[
        ArgumentDefinition {
            name: "--help",
            syntax: "--help",
            description: "Show this help",
            number_of_args: 0,
        },
        ArgumentDefinition {
            name: "--version",
            syntax: "--version",
            description: "Show the application version",
            number_of_args: 0,
        },
        ArgumentDefinition {
            name: "--config-path",
            syntax: "--config-path <path>",
            description: "Set custom path to the config file",
            number_of_args: 1,
        },
        ArgumentDefinition {
            name: "--logs-path",
            syntax: "--logs-path <path>",
            description: "Set path to the logs folder",
            number_of_args: 1,
        },
        ArgumentDefinition {
            name: "--work-path",
            syntax: "--work-path <path>",
            description: "Set default working directory for scripts",
            number_of_args: 1,
        },
        ArgumentDefinition {
            name: "--env",
            syntax: "--env <name> <value>",
            description: "Add an enviroment variable that will be set for all the scripts",
            number_of_args: 2,
        },
        ArgumentDefinition {
            name: "--title",
            syntax: "--title <title>",
            description: "Set custom window title",
            number_of_args: 1,
        },
    ];

    let mut custom_config_path = None;
    let mut custom_logs_path = None;
    let mut custom_work_path = None;
    let mut env_vars = Vec::new();
    let mut custom_title = None;

    let args: Vec<String> = std::env::args().collect();

    let mut i: usize = 1;
    while i < args.len() {
        let arg = &args[i];

        let found_arg = if arg.starts_with("--") {
            SUPPORTED_ARGS
                .iter()
                .find(|supported_arg| supported_arg.name == arg)
        } else {
            None
        };

        let Some(found_arg) = found_arg else {
            return AppArguments {
                custom_config_path: None,
                custom_logs_path: None,
                custom_work_path: None,
                env_vars: Vec::new(),
                custom_title: None,
                read_error: Some(format!(
                    "Unknown argument: {}\nUse --help to see the list of supported arguments",
                    arg
                )),
            };
        };

        if found_arg.number_of_args > 0 && i + found_arg.number_of_args >= args.len() {
            return AppArguments {
                custom_config_path: None,
                custom_logs_path: None,
                custom_work_path: None,
                env_vars: Vec::new(),
                custom_title: None,
                read_error: Some(format!(
                    "Not enough arguments for {}\nUse --help to see the list of supported arguments",
                    arg
                )),
            };
        }

        if arg == "--help" {
            let mut help_text = "Supported arguments:\n".to_string();
            let mut max_syntax_len = 0;
            for arg in SUPPORTED_ARGS {
                max_syntax_len = max_syntax_len.max(arg.syntax.len());
            }
            for arg in SUPPORTED_ARGS {
                help_text.push_str(&arg.syntax);
                for _ in 0..max_syntax_len - arg.syntax.len() + 1 {
                    help_text.push(' ');
                }
                help_text.push_str(arg.description);
                help_text.push_str("\n");
            }
            help_text.push_str("\n");
            help_text.push_str("Example: scripter --config-path C:\\config.json --logs-path C:\\logs --work-path C:\\work --env VAR1 value1 --env VAR2 value2");
            return AppArguments {
                custom_config_path: None,
                custom_logs_path: None,
                custom_work_path: None,
                env_vars: Vec::new(),
                custom_title: None,
                read_error: Some(help_text),
            };
        }
        if arg == "--version" {
            return AppArguments {
                custom_config_path: None,
                custom_logs_path: None,
                custom_work_path: None,
                env_vars: Vec::new(),
                custom_title: None,
                read_error: Some(env!("CARGO_PKG_VERSION").to_string()),
            };
        }
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

        i += 1 + found_arg.number_of_args;
    }

    AppArguments {
        custom_config_path,
        custom_logs_path,
        custom_work_path,
        env_vars,
        custom_title,
        read_error: None,
    }
}
