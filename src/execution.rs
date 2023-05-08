#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use std::io::Write;
use std::path::Path;
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::config;

#[derive(Clone)]
pub struct ScheduledScript {
    pub name: String,
    pub path: Box<Path>,
    pub arguments_line: String,
    path_relative_to_scripter: bool,
    pub autorerun_count: usize,
    pub ignore_previous_failures: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ScriptResultStatus {
    Success,
    Failed,
    Skipped,
}

#[derive(Clone)]
pub struct ScriptExecutionStatus {
    pub start_time: Option<Instant>,
    pub finish_time: Option<Instant>,
    pub result: ScriptResultStatus,
    pub retry_count: usize,
}

pub struct ScriptExecutionData {
    pub scripts_to_run: Vec<ScheduledScript>,
    pub scripts_status: Vec<ScriptExecutionStatus>,
    pub has_started: bool,
    pub progress_receiver: Option<mpsc::Receiver<(usize, ScriptExecutionStatus)>>,
    pub termination_condvar: Arc<(Mutex<bool>, Condvar)>,
    pub currently_selected_script: isize,
    pub currently_outputting_script: isize,
    pub has_failed_scripts: bool,
}

pub fn new_execution_data() -> ScriptExecutionData {
    ScriptExecutionData {
        scripts_to_run: Vec::new(),
        scripts_status: Vec::new(),
        has_started: false,
        progress_receiver: None,
        termination_condvar: Arc::new((Mutex::new(false), Condvar::new())),
        currently_selected_script: -1,
        currently_outputting_script: -1,
        has_failed_scripts: false,
    }
}

pub fn has_script_started(status: &ScriptExecutionStatus) -> bool {
    return status.start_time.is_some();
}

pub fn has_script_finished(status: &ScriptExecutionStatus) -> bool {
    if !has_script_started(status) {
        return false;
    }
    return status.finish_time.is_some();
}

pub fn has_script_failed(status: &ScriptExecutionStatus) -> bool {
    return has_script_finished(status) && status.result == ScriptResultStatus::Failed;
}

pub fn has_started_execution(execution_data: &ScriptExecutionData) -> bool {
    return execution_data.has_started;
}

pub fn has_finished_execution(execution_data: &ScriptExecutionData) -> bool {
    if !has_started_execution(&execution_data) {
        return false;
    }
    if let Some(last) = execution_data.scripts_status.last() {
        return has_script_finished(&last);
    }
    return false;
}

pub fn add_script_to_execution(
    execution_data: &mut ScriptExecutionData,
    script: config::ScriptDefinition,
) {
    execution_data.scripts_to_run.push(ScheduledScript {
        name: script.name,
        path: script.command,
        arguments_line: script.arguments,
        path_relative_to_scripter: script.path_relative_to_scripter,
        autorerun_count: script.autorerun_count,
        ignore_previous_failures: script.ignore_previous_failures,
    });
    execution_data
        .scripts_status
        .push(get_default_script_execution_status());
}

pub fn remove_script_from_execution(execution_data: &mut ScriptExecutionData, index: isize) {
    execution_data.scripts_to_run.remove(index as usize);
    execution_data.scripts_status.remove(index as usize);
}

pub fn run_scripts(execution_data: &mut ScriptExecutionData, app_config: &config::AppConfig) {
    let (tx, rx) = mpsc::channel();
    execution_data.progress_receiver = Some(rx);
    execution_data.has_started = true;

    let scripts_to_run = execution_data.scripts_to_run.clone();
    let termination_condvar = execution_data.termination_condvar.clone();
    let logs_path = app_config.paths.logs_path.clone();
    let exe_folder_path = app_config.paths.exe_folder_path.clone();

    std::thread::spawn(move || {
        std::fs::remove_dir_all(&logs_path).ok();

        let termination_requested = termination_condvar.0.lock();
        if termination_requested.is_err() {
            println!("Failed to lock termination mutex");
            return;
        }
        let mut termination_requested = termination_requested.unwrap();

        let mut has_previous_script_failed = false;
        let mut kill_requested = false;
        for script_idx in 0..scripts_to_run.len() {
            let script = &scripts_to_run[script_idx];
            let mut script_state = get_default_script_execution_status();
            script_state.start_time = Some(Instant::now());

            if kill_requested || (has_previous_script_failed && !script.ignore_previous_failures) {
                script_state.result = ScriptResultStatus::Skipped;
                script_state.finish_time = Some(Instant::now());
                send_script_execution_status(&tx, script_idx, script_state.clone());
                continue;
            }
            send_script_execution_status(&tx, script_idx, script_state.clone());

            'retry_loop: loop {
                if kill_requested {
                    break;
                }

                let _ = std::fs::create_dir_all(&logs_path);

                let stdout_file = std::fs::File::create(config::get_stdout_path(
                    logs_path.clone(),
                    script_idx as isize,
                    script_state.retry_count,
                ));
                let stderr_file = std::fs::File::create(config::get_stderr_path(
                    logs_path.clone(),
                    script_idx as isize,
                    script_state.retry_count,
                ));
                let stdout = if let Ok(stdout_file) = stdout_file {
                    std::process::Stdio::from(stdout_file)
                } else {
                    std::process::Stdio::null()
                };
                let stderr = if let Ok(stderr_file) = stderr_file {
                    std::process::Stdio::from(stderr_file)
                } else {
                    std::process::Stdio::null()
                };

                #[cfg(target_os = "windows")]
                let child = std::process::Command::new("cmd")
                    .creation_flags(0x08000000) // CREATE_NO_WINDOW
                    .arg("/C")
                    .arg(get_script_with_arguments(&script, &exe_folder_path))
                    .stdout(stdout)
                    .stderr(stderr)
                    .spawn();
                #[cfg(not(target_os = "windows"))]
                let child = std::process::Command::new("sh")
                    .arg("-c")
                    .arg(get_script_with_arguments(&script, &exe_folder_path))
                    .stdout(stdout)
                    .stderr(stderr)
                    .spawn();

                if child.is_err() {
                    let err = child.err().unwrap();
                    let error_file = std::fs::File::create(
                        logs_path.join(format!("{}_error.log", script_idx as isize)),
                    );
                    if let Ok(error_file) = error_file {
                        let mut error_writer = std::io::BufWriter::new(error_file);
                        let _ = write!(error_writer, "{}", err);
                    }
                    // it doesn't make sense to retry if something is broken on this level
                    script_state.result = ScriptResultStatus::Failed;
                    script_state.finish_time = Some(Instant::now());
                    send_script_execution_status(&tx, script_idx, script_state.clone());
                    has_previous_script_failed = true;
                    break 'retry_loop;
                }

                let mut child = child.unwrap();

                loop {
                    let result = termination_condvar
                        .1
                        .wait_timeout(termination_requested, Duration::from_millis(100))
                        .unwrap();
                    // 100 milliseconds have passed, or maybe the value changed
                    termination_requested = result.0;
                    if *termination_requested == true {
                        kill_process(&mut child);
                        kill_requested = true;
                    }

                    if let Ok(Some(status)) = child.try_wait() {
                        if status.success() {
                            // successfully finished the script, jump to the next script
                            script_state.finish_time = Some(Instant::now());
                            script_state.result = ScriptResultStatus::Success;
                            send_script_execution_status(&tx, script_idx, script_state.clone());
                            has_previous_script_failed = false;
                            break 'retry_loop;
                        } else {
                            if script_state.retry_count < script.autorerun_count && !kill_requested
                            {
                                // script failed, but we can retry
                                script_state.retry_count += 1;
                                send_script_execution_status(&tx, script_idx, script_state.clone());
                                break;
                            } else {
                                // script failed and we can't retry
                                script_state.finish_time = Some(Instant::now());
                                script_state.result = ScriptResultStatus::Failed;
                                send_script_execution_status(&tx, script_idx, script_state.clone());
                                has_previous_script_failed = true;
                                break 'retry_loop;
                            }
                        }
                    }
                }
            }
        }
    });
}

pub fn reset_execution_progress(execution_data: &mut ScriptExecutionData) {
    execution_data.scripts_status.fill(get_default_script_execution_status());
    execution_data.has_started = false;
    execution_data.has_failed_scripts = false;
    execution_data.currently_outputting_script = -1;
    execution_data.termination_condvar = Arc::new((Mutex::new(false), Condvar::new()));
}

fn send_script_execution_status(
    tx: &mpsc::Sender<(usize, ScriptExecutionStatus)>,
    script_idx: usize,
    script_state: ScriptExecutionStatus,
) {
    let _result = tx.send((script_idx, script_state));
}

fn get_script_with_arguments(script: &ScheduledScript, exe_folder_path: &Path) -> String {
    let path = if script.path_relative_to_scripter {
        exe_folder_path
            .join(&script.path)
            .to_str()
            .unwrap_or_default()
            .to_string()
    } else {
        script.path.to_str().unwrap_or_default().to_string()
    };

    if script.arguments_line.is_empty() {
        path
    } else {
        format!("{} {}", path, script.arguments_line)
    }
}

fn get_default_script_execution_status() -> ScriptExecutionStatus {
    ScriptExecutionStatus {
        start_time: None,
        finish_time: None,
        result: ScriptResultStatus::Skipped,
        retry_count: 0,
    }
}

fn kill_process(process: &mut std::process::Child) {
    #[cfg(not(target_os = "windows"))]
    {
        let kill_output = std::process::Command::new("kill")
            .args([&process.id().to_string()])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output();

        let kill_result = process.kill();
        if let Err(result) = kill_result {
            println!("failed to kill child process: {}", result);
        }
    }

    let kill_result = process.kill();
    if let Err(result) = kill_result {
        println!("failed to kill child process: {}", result);
    }
}