use chrono;
use std::path::PathBuf;
use std::process;

pub fn get_script_log_directory(
    logs_path: &PathBuf,
    execution_start_time: &chrono::DateTime<chrono::Local>,
    script_name: &str,
    script_idx: isize,
) -> PathBuf {
    let mut log_file_name = String::from(script_name).replace(|c: char| !c.is_alphanumeric(), "-");
    log_file_name.truncate(30);
    let directory_name = execution_start_time.format("%Y%m%d-%H%M%S");
    return logs_path
        .join(format!("{}-{}", directory_name, process::id()))
        .join(format!("{}_{}.log", script_idx, log_file_name));
}

pub fn get_script_output_path(
    logs_path: &PathBuf,
    execution_start_time: &chrono::DateTime<chrono::Local>,
    script_name: &str,
    script_idx: isize,
    retry_count: usize,
) -> PathBuf {
    let path = get_script_log_directory(logs_path, execution_start_time, script_name, script_idx);
    if retry_count == 0 {
        path.join("output.log")
    } else {
        path.join(format!("output_retry{}.log", retry_count))
    }
}
