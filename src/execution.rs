#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crossbeam_channel::{unbounded, Receiver, RecvError, Sender};
use std::io::{BufRead, Write};
use std::path::Path;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use crate::config;
use crate::config::get_script_log_directory;

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
    pub progress_receiver: Option<Receiver<(usize, ScriptExecutionStatus)>>,
    pub log_receiver: Option<Receiver<String>>,
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
        log_receiver: None,
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
    execution_data.has_started = true;
    let (progress_sender, process_receiver) = unbounded();
    execution_data.progress_receiver = Some(process_receiver);
    let (log_sender, log_receiver) = unbounded();
    execution_data.log_receiver = Some(log_receiver);
    let log_sender = Arc::new(log_sender);

    let scripts_to_run = execution_data.scripts_to_run.clone();
    let termination_condvar = execution_data.termination_condvar.clone();
    let logs_path = app_config.paths.logs_path.clone();
    let exe_folder_path = app_config.paths.exe_folder_path.clone();
    let env_vars = app_config.env_vars.clone();

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
                send_script_execution_status(&progress_sender, script_idx, script_state.clone());
                continue;
            }
            send_script_execution_status(&progress_sender, script_idx, script_state.clone());

            'retry_loop: loop {
                if kill_requested {
                    break;
                }

                let _ = std::fs::create_dir_all(get_script_log_directory(
                    &logs_path,
                    script_idx as isize,
                ));

                let output_file = std::fs::File::create(config::get_script_output_path(
                    &logs_path,
                    script_idx as isize,
                    script_state.retry_count,
                ));

                let (stdout_type, stderr_type) = if output_file.is_ok() {
                    (std::process::Stdio::piped(), std::process::Stdio::piped())
                } else {
                    (std::process::Stdio::null(), std::process::Stdio::null())
                };

                #[cfg(target_os = "windows")]
                let mut command = std::process::Command::new("cmd");

                #[cfg(target_os = "windows")]
                {
                    command
                        .creation_flags(0x08000000) // CREATE_NO_WINDOW
                        .arg("/C");
                }
                #[cfg(not(target_os = "windows"))]
                let mut command = std::process::Command::new("sh");

                #[cfg(not(target_os = "windows"))]
                {
                    command.arg("-c");
                }

                command
                    .arg(get_script_with_arguments(&script, &exe_folder_path))
                    .envs(env_vars.clone())
                    .stdin(std::process::Stdio::null())
                    .stdout(stdout_type)
                    .stderr(stderr_type);

                let child = command.spawn();

                // avoid potential deadlocks (cargo culted from os_pipe readme)
                drop(command);

                if child.is_err() {
                    if output_file.is_ok() {
                        let err = child.err().unwrap();
                        let mut output_writer = std::io::BufWriter::new(output_file.unwrap());
                        send_log_line(err.to_string(), &log_sender, &mut output_writer);
                    }
                    // it doesn't make sense to retry if something is broken on this level
                    script_state.result = ScriptResultStatus::Failed;
                    script_state.finish_time = Some(Instant::now());
                    send_script_execution_status(
                        &progress_sender,
                        script_idx,
                        script_state.clone(),
                    );
                    has_previous_script_failed = true;
                    break 'retry_loop;
                }

                let mut child = child.unwrap();

                if child.stdout.is_some() && child.stderr.is_some() && output_file.is_ok() {
                    join_and_split_output(
                        child.stdout.take().unwrap(),
                        child.stderr.take().unwrap(),
                        log_sender.clone(),
                        output_file.unwrap(),
                    );
                }

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
                        *termination_requested = false;
                    }

                    if let Ok(Some(status)) = child.try_wait() {
                        if status.success() {
                            // successfully finished the script, jump to the next script
                            script_state.finish_time = Some(Instant::now());
                            script_state.result = ScriptResultStatus::Success;
                            send_script_execution_status(
                                &progress_sender,
                                script_idx,
                                script_state.clone(),
                            );
                            has_previous_script_failed = false;
                            break 'retry_loop;
                        } else {
                            if script_state.retry_count < script.autorerun_count && !kill_requested
                            {
                                // script failed, but we can retry
                                script_state.retry_count += 1;
                                send_script_execution_status(
                                    &progress_sender,
                                    script_idx,
                                    script_state.clone(),
                                );
                                break;
                            } else {
                                // script failed and we can't retry
                                script_state.finish_time = Some(Instant::now());
                                script_state.result = ScriptResultStatus::Failed;
                                send_script_execution_status(
                                    &progress_sender,
                                    script_idx,
                                    script_state.clone(),
                                );
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
    execution_data
        .scripts_status
        .fill(get_default_script_execution_status());
    execution_data.has_started = false;
    execution_data.has_failed_scripts = false;
    execution_data.currently_outputting_script = -1;
    execution_data.termination_condvar = Arc::new((Mutex::new(false), Condvar::new()));
}

fn send_script_execution_status(
    tx: &Sender<(usize, ScriptExecutionStatus)>,
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
    let kill_result = process.kill();
    if let Err(result) = kill_result {
        println!("failed to kill child process: {}", result);
    }
}

fn join_and_split_output(
    stdout: std::process::ChildStdout,
    stderr: std::process::ChildStderr,
    out_channel: Arc<Sender<String>>,
    output_file: std::fs::File,
) {
    let (sender_out, receiver_out) = unbounded();
    let (sender_err, receiver_err) = unbounded();

    let _ = std::thread::spawn(move || {
        read_one_stdio(stdout, sender_out);
    });

    let _ = std::thread::spawn(move || {
        read_one_stdio(stderr, sender_err);
    });

    let _ = std::thread::spawn(move || {
        let mut output_writer = std::io::BufWriter::new(output_file);
        loop {
            crossbeam_channel::select! {
                recv(receiver_out) -> log => {
                    if try_split_log(log, &out_channel, &mut output_writer).is_err() {
                        break;
                    }
                },
                recv(receiver_err) -> log => {
                    if try_split_log(log, &out_channel, &mut output_writer).is_err() {
                        break;
                    }
                }
            }
        }
    });
}

fn read_one_stdio<R: std::io::Read>(stdio: R, out_channel: Sender<(String, bool)>) {
    let mut stdout_reader = std::io::BufReader::new(stdio);
    loop {
        let mut line = String::new();
        let read_result = stdout_reader.read_line(&mut line);
        if read_result.is_err() {
            let _ = out_channel.try_send((line, true));
            break;
        }
        if read_result.unwrap() == 0 {
            let _ = out_channel.try_send((line, true));
            break;
        }

        let _ = out_channel.try_send((line, false));
    }
}

fn try_split_log(
    log: Result<(String, bool), RecvError>,
    out_channel: &Sender<String>,
    output_writer: &mut std::io::BufWriter<std::fs::File>,
) -> Result<(), ()> {
    if let Ok((line, should_exit)) = log {
        if should_exit {
            return Err(());
        }
        else {
            send_log_line(line, out_channel, output_writer);
        }
    } else {
        return Err(());
    }
    return Ok(());
}

fn send_log_line(line: String, out_channel: &Sender<String>, output_writer: &mut std::io::BufWriter<std::fs::File>) {
    let _ = write!(output_writer, "{}", line);
    let _ = output_writer.flush();
    let _ = out_channel.send(line);
}