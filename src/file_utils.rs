use chrono;
use std::path::PathBuf;
use std::process;

pub fn get_script_log_directory(
    logs_path: &PathBuf,
    execution_start_time: &chrono::DateTime<chrono::Local>,
    script_name: &str,
    script_idx: isize,
) -> PathBuf {
    let run_directory_name = execution_start_time.format("%Y%m%d-%H%M%S");
    let mut script_directory_name =
        String::from(script_name).replace(|c: char| !c.is_alphanumeric(), "-");
    script_directory_name.truncate(30);
    return logs_path
        .join(format!("{}-{}", run_directory_name, process::id()))
        .join(format!("{}_{}", script_idx, script_directory_name));
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
