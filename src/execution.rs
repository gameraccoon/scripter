#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use chrono::{DateTime, Utc};
use crossbeam_channel::{unbounded, Receiver, RecvError, Sender};
use std::io::{BufRead, Write};
use std::path::Path;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant};

use crate::config;
use crate::ring_buffer::RingBuffer;

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

#[derive(Default)]
pub enum OutputType {
    #[default]
    StdOut,
    StdErr,
    Error,
    Event,
}

#[derive(Default)]
pub struct OutputLine {
    pub text: String,
    pub output_type: OutputType,
    pub timestamp: DateTime<Utc>,
}

type LogBuffer = RingBuffer<OutputLine, 30>;

pub struct ScriptExecutionData {
    pub scripts_to_run: Vec<config::ScriptDefinition>,
    pub scripts_status: Vec<ScriptExecutionStatus>,
    pub has_started: bool,
    pub recent_logs: Arc<Mutex<LogBuffer>>,
    pub progress_receiver: Option<Receiver<(usize, ScriptExecutionStatus)>>,
    pub is_termination_requested: Arc<AtomicBool>,
    pub currently_outputting_script: isize,
    pub has_failed_scripts: bool,
    pub thread_join_handle: Option<std::thread::JoinHandle<()>>,
}

pub fn new_execution_data() -> ScriptExecutionData {
    ScriptExecutionData {
        scripts_to_run: Vec::new(),
        scripts_status: Vec::new(),
        has_started: false,
        progress_receiver: None,
        recent_logs: Arc::new(Mutex::new(RingBuffer::new(Default::default()))),
        is_termination_requested: Arc::new(AtomicBool::new(false)),
        currently_outputting_script: -1,
        has_failed_scripts: false,
        thread_join_handle: None,
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

pub fn has_script_been_skipped(status: &ScriptExecutionStatus) -> bool {
    return has_script_finished(status) && status.result == ScriptResultStatus::Skipped;
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

pub fn is_waiting_execution_thread_to_finish(execution_data: &ScriptExecutionData) -> bool {
    // wait for the thread to finish, otherwise we can let the user to break their state
    if let Some(join_handle) = &execution_data.thread_join_handle {
        if !join_handle.is_finished() {
            return true;
        }
    }
    return false;
}

pub fn add_script_to_execution(
    execution_data: &mut ScriptExecutionData,
    script: config::ScriptDefinition,
) {
    execution_data
        .scripts_to_run
        .push(config::ScriptDefinition {
            name: script.name,
            icon: script.icon,
            command: script.command,
            arguments: script.arguments,
            path_relative_to_scripter: script.path_relative_to_scripter,
            autorerun_count: script.autorerun_count,
            ignore_previous_failures: script.ignore_previous_failures,
        });
    execution_data
        .scripts_status
        .push(get_default_script_execution_status());
}

pub fn remove_script_from_execution(execution_data: &mut ScriptExecutionData, index: usize) {
    execution_data.scripts_to_run.remove(index);
    execution_data.scripts_status.remove(index);
}

pub fn run_scripts(execution_data: &mut ScriptExecutionData, app_config: &config::AppConfig) {
    execution_data.has_started = true;
    let (progress_sender, process_receiver) = unbounded();
    execution_data.progress_receiver = Some(process_receiver);
    let recent_logs = execution_data.recent_logs.clone();

    let scripts_to_run = execution_data.scripts_to_run.clone();
    let is_termination_requested = execution_data.is_termination_requested.clone();
    let logs_path = app_config.paths.logs_path.clone();
    let exe_folder_path = app_config.paths.exe_folder_path.clone();
    let env_vars = app_config.env_vars.clone();

    execution_data.thread_join_handle = Some(std::thread::spawn(move || {
        std::fs::remove_dir_all(&logs_path).ok();

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

                {
                    let mut recent_logs = recent_logs.lock().unwrap(); // it is fine to panic on a poisoned mutex
                    recent_logs.push(OutputLine {
                        text: format!(
                            "Running \"{}\"{}\n[{} {}]",
                            script.name,
                            if script_state.retry_count > 0 {
                                format!(" retry #{}", script_state.retry_count)
                            } else {
                                "".to_string()
                            },
                            script.command,
                            script.arguments
                        ),
                        output_type: OutputType::Event,
                        timestamp: Utc::now(),
                    });
                }

                let _ = std::fs::create_dir_all(config::get_script_log_directory(
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

                let command_line = get_script_with_arguments(&script, &exe_folder_path);

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
                    .arg(command_line)
                    .envs(env_vars.clone())
                    .stdin(std::process::Stdio::null())
                    .stdout(stdout_type)
                    .stderr(stderr_type);

                let child = command.spawn();

                // avoid potential deadlocks (cargo culted from os_pipe readme)
                drop(command);

                let mut child = match child {
                    Ok(child) => child,
                    Err(err) => {
                        if let Ok(output_file) = output_file {
                            let error_text = format!("Failed to start the process: {}", err);
                            let mut output_writer = std::io::BufWriter::new(output_file);
                            send_log_line(
                                OutputLine {
                                    text: error_text,
                                    output_type: OutputType::Error,
                                    timestamp: Utc::now(),
                                },
                                &recent_logs,
                                &mut output_writer,
                            );
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
                };

                let mut threads_to_join = Vec::new();
                match (child.stdout.take(), child.stderr.take(), output_file) {
                    (Some(stdout), Some(stderr), Ok(output_file)) => {
                        threads_to_join =
                            join_and_split_output(stdout, stderr, recent_logs.clone(), output_file);
                    }
                    _ => {
                        println!(
                            "Failed to redirect stdout/stderr. No diagnostic is provided for now"
                        );
                    }
                }

                loop {
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
                            join_threads(threads_to_join);
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
                                join_threads(threads_to_join);
                                break 'retry_loop;
                            }
                        }
                    }

                    if is_termination_requested.load(Ordering::Acquire) {
                        kill_process(&mut child);
                        kill_requested = true;
                        is_termination_requested.store(false, Ordering::Release);
                    }

                    std::thread::sleep(Duration::from_millis(100));
                }
                join_threads(threads_to_join);
            }
        }
    }));
}

pub fn request_stop_execution(execution_data: &mut ScriptExecutionData) {
    execution_data
        .is_termination_requested
        .store(true, Ordering::Relaxed);
}

pub fn reset_execution_progress(execution_data: &mut ScriptExecutionData) {
    execution_data
        .scripts_status
        .fill(get_default_script_execution_status());
    execution_data.has_started = false;
    execution_data.has_failed_scripts = false;
    execution_data.currently_outputting_script = -1;
    execution_data.is_termination_requested = Arc::new(AtomicBool::new(false));
    execution_data.recent_logs.lock().unwrap().set_empty(); // it is fine to panic on a poisoned mutex
}

fn send_script_execution_status(
    tx: &Sender<(usize, ScriptExecutionStatus)>,
    script_idx: usize,
    script_state: ScriptExecutionStatus,
) {
    let _result = tx.send((script_idx, script_state));
}

fn get_script_with_arguments(script: &config::ScriptDefinition, exe_folder_path: &Path) -> String {
    let path = if script.path_relative_to_scripter {
        exe_folder_path
            .join(&script.command)
            .to_str()
            .unwrap_or_default()
            .to_string()
    } else {
        script.command.clone()
    };

    let escaped_path = escape_path(path);

    if script.arguments.is_empty() {
        escaped_path
    } else {
        format!("{} {}", escaped_path, script.arguments)
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

fn escape_path(path: String) -> String {
    #[cfg(not(target_os = "windows"))]
    return path;

    #[cfg(target_os = "windows")]
    {
        let mut escaped_path = String::with_capacity(path.len() + 9);

        for c in path.chars() {
            if c == ' '
                || c == '&'
                || c == '('
                || c == ')'
                || c == ','
                || c == ';'
                || c == '='
                || c == '^'
                || c == '['
                || c == ']'
            {
                escaped_path.push('^');
            }
            if c == '/' {
                escaped_path.push('\\');
            } else {
                escaped_path.push(c);
            }
        }
        return escaped_path;
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
    recent_logs: Arc<Mutex<LogBuffer>>,
    output_file: std::fs::File,
) -> Vec<std::thread::JoinHandle<()>> {
    let (sender_out, receiver_out) = unbounded();
    let (sender_err, receiver_err) = unbounded();

    let read_stdio_thread = std::thread::spawn(move || {
        read_one_stdio(stdout, sender_out);
    });

    let read_stderr_thread = std::thread::spawn(move || {
        read_one_stdio(stderr, sender_err);
    });

    let join_and_split_thread = std::thread::spawn(move || {
        let mut output_writer = std::io::BufWriter::new(output_file);
        loop {
            crossbeam_channel::select! {
                recv(receiver_out) -> log => {
                    if try_split_log(log, OutputType::StdOut, &recent_logs, &mut output_writer).is_err() {
                        break;
                    }
                },
                recv(receiver_err) -> log => {
                    if try_split_log(log, OutputType::StdErr, &recent_logs, &mut output_writer).is_err() {
                        break;
                    }
                }
            }
        }
    });

    return vec![read_stdio_thread, read_stderr_thread, join_and_split_thread];
}

fn read_one_stdio<R: std::io::Read>(stdio: R, out_channel: Sender<(String, bool)>) {
    let mut stdout_reader = std::io::BufReader::new(stdio);
    loop {
        let mut line = String::new();
        let read_result = stdout_reader.read_line(&mut line);
        let should_stop = read_result.unwrap_or(0) == 0;

        let _ = out_channel.try_send((line, should_stop));
        if should_stop {
            break;
        }
    }
}

fn try_split_log(
    log: Result<(String, bool), RecvError>,
    output_type: OutputType,
    recent_logs: &Arc<Mutex<LogBuffer>>,
    output_writer: &mut std::io::BufWriter<std::fs::File>,
) -> Result<(), ()> {
    if let Ok((text, should_exit)) = log {
        if should_exit {
            return Err(());
        } else {
            send_log_line(
                OutputLine {
                    text,
                    output_type,
                    timestamp: Utc::now(),
                },
                recent_logs,
                output_writer,
            );
        }
    } else {
        return Err(());
    }
    return Ok(());
}

fn send_log_line(
    line: OutputLine,
    recent_logs: &Arc<Mutex<LogBuffer>>,
    output_writer: &mut std::io::BufWriter<std::fs::File>,
) {
    let _ = write!(output_writer, "{}", line.text);
    let _ = output_writer.flush();

    recent_logs.lock().unwrap().push(line); // it is fine to panic on a poisoned mutex
}

fn join_threads(threads: Vec<std::thread::JoinHandle<()>>) {
    for thread in threads {
        let _ = thread.join();
    }
}
