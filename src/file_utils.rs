use chrono;
use std::path::PathBuf;
use std::process;

pub fn get_script_log_directory(
    logs_path: &PathBuf,
    execution_start_time: &chrono::DateTime<chrono::Local>,
) -> PathBuf {
    let run_directory_name = execution_start_time.format("%Y%m%d-%H%M%S");
    return logs_path.join(format!("{}-{}", run_directory_name, process::id()));
}

pub fn get_script_output_path(
    logs_path: &PathBuf,
    execution_start_time: &chrono::DateTime<chrono::Local>,
    script_name: &str,
    script_idx: isize,
    retry_count: usize,
) -> PathBuf {
    let mut script_file_name =
        String::from(script_name).replace(|c: char| !c.is_alphanumeric(), "-");
    script_file_name.truncate(30);

    let path = get_script_log_directory(logs_path, execution_start_time);
    if retry_count == 0 {
        path.join(format!("{}_{}_output.log", script_idx, script_file_name))
    } else {
        path.join(format!(
            "{}_{}_output_retry{}.log",
            script_idx, script_file_name, retry_count
        ))
    }
}
