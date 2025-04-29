// Copyright (C) Pavel Grebnev 2023
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use chrono;
use std::path::PathBuf;
use std::process;

pub fn get_script_log_directory(
    logs_path: &PathBuf,
    execution_start_time: &chrono::DateTime<chrono::Local>,
) -> PathBuf {
    let run_directory_name = execution_start_time.format("%Y%m%d-%H%M%S");
    logs_path.join(format!("{}-{}", run_directory_name, process::id()))
}

pub fn get_script_output_path(
    script_log_directory: PathBuf,
    script_name: &str,
    script_idx: isize,
    retry_count: usize,
) -> PathBuf {
    let mut script_file_name =
        String::from(script_name).replace(|c: char| !c.is_alphanumeric(), "-");
    script_file_name.truncate(30);

    if retry_count == 0 {
        script_log_directory.join(format!(
            "{}_{}_output.log",
            script_idx + 1,
            script_file_name
        ))
    } else {
        script_log_directory.join(format!(
            "{}_{}_output_retry{}.log",
            script_idx + 1,
            script_file_name,
            retry_count
        ))
    }
}
