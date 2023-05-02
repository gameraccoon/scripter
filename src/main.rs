#![windows_subsystem = "windows"]

use iced::alignment::{self, Alignment};
use iced::executor;
use std::io::{BufRead, Write};
// use iced::keyboard;
use iced::theme::{self, Theme};
use iced::time;
use iced::widget::pane_grid::{self, Configuration, PaneGrid};
use iced::widget::{button, column, container, row, scrollable, text, text_input, Column};
use iced::window::icon;
use iced::{Application, Command, Element, Length, Settings, Subscription};
use iced_lazy::responsive;
use rev_buf_reader::RevBufReader;
use serde::Deserialize;
use std::path::Path;
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

pub fn main() -> iced::Result {
    let app_config = read_config();

    let mut settings = Settings::default();
    settings.window.icon = Option::from(
        icon::from_rgba(include_bytes!("../res/icon.rgba").to_vec(), 128, 128).unwrap(),
    );
    settings.window.position = iced::window::Position::Centered;
    settings.window.always_on_top = app_config.always_on_top;
    MainWindow::run(settings)
}

#[derive(Clone)]
struct ScheduledScript {
    path: Box<Path>,
    arguments_line: String,
}

struct ScriptExecutionData {
    scripts_to_run: Vec<ScheduledScript>,
    start_times: Vec<Instant>,
    running_progress: isize,
    last_execution_status_success: bool,
    progress_receiver: Option<mpsc::Receiver<(isize, Instant, bool)>>,
    termination_condvar: Arc<(Mutex<bool>, Condvar)>,
    currently_edited_script: isize,
}

fn new_execution_data() -> ScriptExecutionData {
    ScriptExecutionData {
        scripts_to_run: Vec::new(),
        start_times: Vec::new(),
        running_progress: -1,
        last_execution_status_success: true,
        progress_receiver: None,
        termination_condvar: Arc::new((Mutex::new(false), Condvar::new())),
        currently_edited_script: -1,
    }
}

struct PathCaches {
    scripts_path: String,
    logs_path: String,
    work_path: String,
}

struct MainWindow {
    panes: pane_grid::State<AppPane>,
    focus: Option<pane_grid::Pane>,
    execution_data: ScriptExecutionData,
    path_caches: PathCaches,
}

#[derive(Debug, Clone)]
enum Message {
    //FocusAdjacent(pane_grid::Direction),
    Clicked(pane_grid::Pane),
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
    Maximize(pane_grid::Pane),
    Restore,
    AddScriptToRun(Box<Path>),
    RunScripts(),
    StopScripts(),
    ClearScripts(),
    Tick(Instant),
    OpenScriptEditing(isize),
    RemoveScript(isize),
    EditArguments(String, isize),
    OpenFile(String),
}

#[derive(Default, Debug, Clone, PartialEq, Deserialize)]
struct AppConfig {
    always_on_top: bool,
}

fn get_default_config() -> AppConfig {
    AppConfig {
        always_on_top: true,
    }
}

fn read_config() -> AppConfig {
    let config_path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("scripter_config.json");

    if !config_path.exists() {
        return get_default_config();
    }
    let data = std::fs::read_to_string(config_path);
    if data.is_err() {
        return get_default_config();
    }
    let data = data.unwrap();
    let config = serde_json::from_str(&data);
    if config.is_err() {
        return get_default_config();
    }
    return config.unwrap();
}

fn get_script_with_arguments(script: &ScheduledScript) -> String {
    if script.arguments_line.is_empty() {
        script.path.to_str().unwrap_or_default().to_string()
    } else {
        format!(
            "{} {}",
            script.path.to_str().unwrap_or_default().to_string(),
            script.arguments_line
        )
    }
}

fn get_scripts_path() -> String {
    return std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join("scripts")
        .to_str()
        .unwrap()
        .to_string();
}

fn get_logs_path() -> String {
    let pid = std::process::id();
    let folder_name = format!("exec_logs_{}", pid);
    return std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join(folder_name)
        .to_str()
        .unwrap()
        .to_string();
}

fn get_work_path() -> String {
    return std::env::current_dir()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
}

impl Application for MainWindow {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let pane_configuration = Configuration::Split {
            axis: pane_grid::Axis::Vertical,
            ratio: 0.6,
            a: Box::new(Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: 0.4,
                a: Box::new(Configuration::Split {
                    axis: pane_grid::Axis::Horizontal,
                    ratio: 0.7,
                    a: Box::new(Configuration::Pane(AppPane::new(PaneVariant::ScriptList))),
                    b: Box::new(Configuration::Pane(AppPane::new(PaneVariant::ScriptEdit))),
                }),
                b: Box::new(Configuration::Pane(AppPane::new(
                    PaneVariant::ExecutionList,
                ))),
            }),
            b: Box::new(Configuration::Pane(AppPane::new(PaneVariant::LogOutput))),
        };
        let panes = pane_grid::State::with_configuration(pane_configuration);

        (
            MainWindow {
                panes,
                focus: None,
                execution_data: new_execution_data(),
                path_caches: PathCaches {
                    scripts_path: get_scripts_path(),
                    logs_path: get_logs_path(),
                    work_path: get_work_path(),
                },
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        format!("scripter ({})", self.path_caches.work_path)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            // Message::FocusAdjacent(direction) => {
            //     if let Some(pane) = self.focus {
            //         if let Some(adjacent) = self.panes.adjacent(&pane, direction) {
            //             self.focus = Some(adjacent);
            //         }
            //     }
            // }
            Message::Clicked(pane) => {
                self.focus = Some(pane);
            }
            Message::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(&split, ratio);
            }
            Message::Dragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                self.panes.swap(&pane, &target);
            }
            Message::Dragged(_) => {}
            Message::Maximize(pane) => self.panes.maximize(&pane),
            Message::Restore => {
                self.panes.restore();
            }
            Message::AddScriptToRun(script) => {
                if self.execution_data.running_progress == -1 {
                    self.execution_data.scripts_to_run.push(ScheduledScript {
                        path: script,
                        arguments_line: String::new(),
                    });
                }
                self.execution_data.currently_edited_script =
                    (self.execution_data.scripts_to_run.len() - 1) as isize;
            }
            Message::RunScripts() => {
                if self.execution_data.scripts_to_run.is_empty() {
                    return Command::none();
                }

                if self.execution_data.running_progress == -1 {
                    let logs_path = self.path_caches.logs_path.clone();
                    std::fs::remove_dir_all(&logs_path).ok();
                    self.execution_data.currently_edited_script = -1;
                    self.execution_data.running_progress = 0;
                    let (tx, rx) = mpsc::channel();
                    let scripts_to_run = self.execution_data.scripts_to_run.clone();
                    let termination_condvar = self.execution_data.termination_condvar.clone();
                    std::thread::spawn(move || {
                        let mut processed_count = 0;
                        let mut termination_requested = termination_condvar.0.lock().unwrap();
                        for script in scripts_to_run {
                            tx.send((processed_count, Instant::now(), true)).unwrap();

                            std::fs::create_dir_all(&logs_path)
                                .expect(&format!("failed to create \"{}\" directory", &logs_path));

                            let stdout_file =
                                std::fs::File::create(get_stdout_path(&logs_path, processed_count))
                                    .expect("failed to create stdout file");
                            let stderr_file =
                                std::fs::File::create(get_stderr_path(&logs_path, processed_count))
                                    .expect("failed to create stderr file");
                            let stdout = std::process::Stdio::from(stdout_file);
                            let stderr = std::process::Stdio::from(stderr_file);

                            #[cfg(target_os = "windows")]
                            let child = std::process::Command::new("cmd")
                                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                                .arg("/C")
                                .arg(get_script_with_arguments(&script))
                                .stdout(stdout)
                                .stderr(stderr)
                                .spawn();
                            #[cfg(not(target_os = "windows"))]
                            let child = std::process::Command::new("sh")
                                .arg("-c")
                                .arg(get_script_with_arguments(&script))
                                .stdout(stdout)
                                .stderr(stderr)
                                .spawn();

                            if child.is_err() {
                                let err = child.err().unwrap();
                                // write error to a file
                                let error_file = std::fs::File::create(format!(
                                    "{}/{}_error.log",
                                    &logs_path, processed_count
                                ))
                                .expect("failed to create error file");
                                let mut error_writer = std::io::BufWriter::new(error_file);
                                write!(error_writer, "{}", err).expect("failed to write error");
                                tx.send((processed_count + 1, Instant::now(), false))
                                    .unwrap();
                                return;
                            }

                            let mut child = child.unwrap();

                            loop {
                                let result = termination_condvar
                                    .1
                                    .wait_timeout(termination_requested, Duration::from_millis(10))
                                    .unwrap();
                                // 10 milliseconds have passed, or maybe the value changed!
                                termination_requested = result.0;
                                if *termination_requested == true {
                                    // We received the notification and the value has been updated, we can leave.
                                    let kill_result = child.kill();
                                    if kill_result.is_err() {
                                        println!(
                                            "failed to kill child process: {}",
                                            kill_result.err().unwrap()
                                        );
                                        return;
                                    }
                                    tx.send((processed_count + 1, Instant::now(), false))
                                        .unwrap();
                                }

                                if let Ok(Some(status)) = child.try_wait() {
                                    if !status.success() {
                                        tx.send((processed_count + 1, Instant::now(), false))
                                            .unwrap();
                                        return;
                                    }
                                    break;
                                }
                            }

                            processed_count += 1;
                        }
                        tx.send((processed_count, Instant::now(), true)).unwrap();
                    });
                    self.execution_data.progress_receiver = Some(rx);
                }
            }
            Message::StopScripts() => {
                if self.execution_data.running_progress != -1 {
                    let mut termination_requested =
                        self.execution_data.termination_condvar.0.lock().unwrap();
                    *termination_requested = true;
                    // We notify the condvar that the value has changed.
                    self.execution_data.termination_condvar.1.notify_one();
                }
            }
            Message::ClearScripts() => {
                self.execution_data = new_execution_data();
            }
            Message::Tick(_now) => {
                let mut exec_data = &mut self.execution_data;
                if let Some(rx) = &exec_data.progress_receiver {
                    if let Ok(progress) = rx.try_recv() {
                        exec_data.running_progress = progress.0;
                        if progress.0 == exec_data.start_times.len() as isize {
                            exec_data.start_times.push(progress.1);
                        }
                        exec_data.last_execution_status_success = progress.2;
                    }
                }
            }
            Message::OpenScriptEditing(script_idx) => {
                self.execution_data.currently_edited_script = script_idx;
            }
            Message::RemoveScript(script_idx) => {
                self.execution_data
                    .scripts_to_run
                    .remove(script_idx as usize);
                self.execution_data.currently_edited_script = -1;
            }
            Message::EditArguments(new_arguments, script_idx) => {
                if self.execution_data.currently_edited_script != -1 {
                    self.execution_data.scripts_to_run[script_idx as usize].arguments_line =
                        new_arguments;
                }
            }
            Message::OpenFile(path) => {
                #[cfg(target_os = "windows")]
                {
                    std::process::Command::new("explorer")
                        .arg(path)
                        .spawn()
                        .expect("failed to open file");
                }
                #[cfg(target_os = "linux")]
                {
                    std::process::Command::new("xdg-open")
                        .arg(path)
                        .spawn()
                        .expect("failed to open file");
                }
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let focus = self.focus;
        let total_panes = self.panes.len();

        let pane_grid = PaneGrid::new(&self.panes, |id, _pane, is_maximized| {
            let is_focused = focus == Some(id);

            let variant = &self.panes.panes[&id].variant;

            let title = row![match variant {
                PaneVariant::ScriptList => "Scripts",
                PaneVariant::ExecutionList => "Executions",
                PaneVariant::LogOutput => "Log",
                PaneVariant::ScriptEdit => "Script Properties",
            }]
            .spacing(5);

            let title_bar = pane_grid::TitleBar::new(title)
                .controls(view_controls(id, total_panes, is_maximized))
                .padding(10)
                .style(if is_focused {
                    style::title_bar_focused
                } else {
                    style::title_bar_active
                });

            pane_grid::Content::new(responsive(move |_size| {
                view_content(&self.execution_data, &self.path_caches, variant)
            }))
            .title_bar(title_bar)
            .style(if is_focused {
                style::pane_focused
            } else {
                style::pane_active
            })
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(1)
        .on_click(Message::Clicked)
        .on_drag(Message::Dragged)
        .on_resize(10, Message::Resized);

        container(pane_grid)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(1)
            .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        // can't find how to process keyboard events and other events at the same time
        // so for now we just process other events
        /*subscription::events_with(|event, status| {
            if let event::Status::Captured = status {
                return None;
            }

            match event {
                Event::Keyboard(keyboard::Event::KeyPressed {
                    modifiers,
                    key_code,
                }) if modifiers.command() => handle_hotkey(key_code),
                _ => None,
            }
        })*/
        time::every(Duration::from_millis(10)).map(Message::Tick)
    }
}

// fn handle_hotkey(key_code: keyboard::KeyCode) -> Option<Message> {
//     use keyboard::KeyCode;
//     use pane_grid::Direction;
//
//     let direction = match key_code {
//         KeyCode::Up => Some(Direction::Up),
//         KeyCode::Down => Some(Direction::Down),
//         KeyCode::Left => Some(Direction::Left),
//         KeyCode::Right => Some(Direction::Right),
//         _ => None,
//     };
//
//     match key_code {
//         KeyCode::V => Some(Message::SplitFocused(Axis::Vertical)),
//         KeyCode::H => Some(Message::SplitFocused(Axis::Horizontal)),
//         KeyCode::W => Some(Message::CloseFocused),
//         _ => direction.map(Message::FocusAdjacent),
//     }
// }

#[derive(PartialEq)]
enum PaneVariant {
    ScriptList,
    ExecutionList,
    LogOutput,
    ScriptEdit,
}

struct AppPane {
    variant: PaneVariant,
}

impl AppPane {
    fn new(variant: PaneVariant) -> Self {
        Self { variant }
    }
}

fn get_stdout_path(logs_path: &str, script_idx: isize) -> String {
    Path::new(logs_path)
        .join(format!("{}_stdout.log", script_idx))
        .to_str()
        .unwrap()
        .to_string()
}

fn get_stderr_path(logs_path: &str, script_idx: isize) -> String {
    Path::new(logs_path)
        .join(format!("{}_stderr.log", script_idx))
        .to_str()
        .unwrap()
        .to_string()
}

fn is_file_empty(path: &str) -> bool {
    let file = std::fs::File::open(path);
    if let Ok(file) = file {
        let metadata = file.metadata();
        if let Ok(metadata) = metadata {
            return metadata.len() == 0;
        }
    }
    true
}

fn produce_script_list_content<'a>(
    execution_data: &ScriptExecutionData,
    path_caches: &PathCaches,
) -> Column<'a, Message> {
    let button = |label, message| {
        button(
            text(label)
                .vertical_alignment(alignment::Vertical::Center)
                .size(16),
        )
        .padding(4)
        .on_press(message)
    };

    let scripts_folder_path = &path_caches.scripts_path;

    if !Path::new(&scripts_folder_path).exists() {
        return column![text(format!(
            "No scripts found in \"{}\"",
            &scripts_folder_path
        ))];
    }

    let mut files = vec![];
    let dir = std::fs::read_dir(&scripts_folder_path).expect("Failed to read scripts folder");
    for entry in dir {
        let entry = entry.expect("Failed to read script entry");
        let path = entry.path();
        if path.is_file() {
            files.push(path.clone());
        }
    }

    if files.is_empty() {
        return column![text(format!(
            "No scripts found in \"{}\"",
            &scripts_folder_path
        ))];
    }

    let data: Element<_> = column(
        files
            .iter()
            .enumerate()
            .map(|(_i, file)| {
                let file_name_str = file
                    .file_name()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or("[error]")
                    .to_string();

                if execution_data.running_progress == -1 {
                    row![
                        button("Add", Message::AddScriptToRun(Box::from(file.clone()))),
                        text(" "),
                        text(file_name_str),
                    ]
                } else {
                    row![text(file_name_str)]
                }
                .into()
            })
            .collect(),
    )
    .spacing(10)
    .into();

    return column![scrollable(data),]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .align_items(Alignment::Start);
}

fn produce_execution_list_content<'a>(
    execution_data: &ScriptExecutionData,
    path_caches: &PathCaches,
) -> Column<'a, Message> {
    let main_button = |label, message| {
        button(
            text(label)
                .width(Length::Fill)
                .horizontal_alignment(alignment::Horizontal::Center)
                .size(16),
        )
        .width(Length::Fill)
        .padding(8)
        .on_press(message)
    };

    let small_button = |label, message| {
        button(
            text(label)
                .horizontal_alignment(alignment::Horizontal::Center)
                .size(16),
        )
        .padding(4)
        .on_press(message)
    };

    let has_error = execution_data.last_execution_status_success == false;
    let success_number = if has_error {
        execution_data.running_progress - 1
    } else {
        execution_data.running_progress
    };

    let data: Element<_> = column(
        execution_data
            .scripts_to_run
            .iter()
            .enumerate()
            .map(|(i, element)| {
                let script_name = element
                    .path
                    .file_name()
                    .unwrap_or(&std::ffi::OsStr::new("[error]"))
                    .to_str()
                    .unwrap_or("[error]");
                let name = if (i as isize) == success_number && !has_error {
                    if execution_data.start_times.len() > i {
                        let time_taken_sec = Instant::now()
                            .duration_since(execution_data.start_times[i])
                            .as_secs();
                        format!(
                            "[...] {} ({:02}:{:02})",
                            script_name,
                            time_taken_sec / 60,
                            time_taken_sec % 60
                        )
                    } else {
                        format!("[...] {}", script_name)
                    }
                } else if (i as isize) <= success_number {
                    let status = if (i as isize) == success_number {
                        "[FAILED]"
                    } else {
                        "[DONE]"
                    };
                    if execution_data.start_times.len() > i + 1 {
                        let time_taken_sec = execution_data.start_times[i + 1]
                            .duration_since(execution_data.start_times[i])
                            .as_secs();
                        format!(
                            "{} {} ({:02}:{:02})",
                            status,
                            script_name,
                            time_taken_sec / 60,
                            time_taken_sec % 60
                        )
                    } else {
                        format!("{} {}", status, script_name)
                    }
                } else {
                    if has_error {
                        format!("[SKIPPED] {}", script_name)
                    } else {
                        format!("{}", script_name)
                    }
                };

                let mut row_data: Vec<Element<'_, Message, iced::Renderer>> = Vec::new();
                row_data.push(text(name).into());

                if execution_data.running_progress == -1 {
                    row_data.push(text(" ").into());
                    row_data
                        .push(small_button("Edit", Message::OpenScriptEditing(i as isize)).into());
                } else if execution_data.running_progress >= i as isize {
                    let stdout_path = get_stdout_path(&path_caches.logs_path, i as isize);
                    if !is_file_empty(&stdout_path) {
                        row_data.push(text(" ").into());
                        row_data.push(small_button("log", Message::OpenFile(stdout_path)).into());
                    }
                    let stderr_path = get_stderr_path(&path_caches.logs_path, i as isize);
                    if !is_file_empty(&stderr_path) {
                        row_data.push(text(" ").into());
                        row_data.push(small_button("err", Message::OpenFile(stderr_path)).into());
                    }
                }

                row(row_data).into()
            })
            .collect(),
    )
    .spacing(10)
    .width(Length::Fill)
    .align_items(Alignment::Start)
    .into();

    let controls = column![if has_error
        || success_number >= execution_data.scripts_to_run.len() as isize
    {
        main_button("Clear", Message::ClearScripts())
    } else if success_number >= 0 {
        main_button("Stop", Message::StopScripts())
    } else {
        main_button("Run", Message::RunScripts())
    }]
    .spacing(5)
    .max_width(150)
    .align_items(Alignment::Center);

    return column![scrollable(data), controls]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .align_items(Alignment::Center);
}

fn get_last_n_lines_from_file(file_name: &str, lines_number: usize) -> Option<Vec<String>> {
    let file = std::fs::File::open(file_name);

    if file.is_err() {
        return None;
    }

    let file = file.unwrap();
    let text_buffer = RevBufReader::new(file);
    return Some(
        text_buffer
            .lines()
            .take(lines_number)
            .map(|l| l.expect("Could not parse line"))
            .collect(),
    );
}

fn produce_log_output_content<'a>(
    execution_data: &ScriptExecutionData,
    path_caches: &PathCaches,
) -> Column<'a, Message> {
    if execution_data.running_progress == -1 {
        return Column::new();
    }

    let current_script_idx = if execution_data.last_execution_status_success
        && execution_data.running_progress < execution_data.scripts_to_run.len() as isize
    {
        execution_data.running_progress
    } else {
        execution_data.running_progress - 1
    };

    if current_script_idx == -1 {
        return Column::new();
    }

    let stdout_file_name = get_stdout_path(&path_caches.logs_path, current_script_idx);
    let stdout_lines = get_last_n_lines_from_file(&stdout_file_name, 30);
    let stderr_file_name = get_stderr_path(&path_caches.logs_path, current_script_idx);
    let stderr_lines = get_last_n_lines_from_file(&stderr_file_name, 30);
    let error_file_name = format!(
        "{}/{}_error.log",
        &path_caches.logs_path, current_script_idx
    );
    let error_lines = get_last_n_lines_from_file(&error_file_name, 10);

    if stdout_lines.is_none() {
        return column![text(
            format!("Can't open script output '{}'", stdout_file_name).to_string()
        )];
    }
    if stderr_lines.is_none() {
        return column![text(
            format!("Can't open script output '{}'", stderr_file_name).to_string()
        )];
    }

    let stdout_lines = stdout_lines.unwrap();
    let stderr_lines = stderr_lines.unwrap();
    let error_lines = error_lines.unwrap_or(Vec::new());

    let mut data_lines: Vec<Element<'_, Message, iced::Renderer>> = Vec::new();

    data_lines.push(
        text(format!(
            "Script: {}",
            execution_data.scripts_to_run[current_script_idx as usize]
                .path
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("[error]")
                .to_string(),
        ))
        .into(),
    );

    if !stdout_lines.is_empty() {
        data_lines.extend(
            stdout_lines
                .iter()
                .rev()
                .map(|element| text(element).into()),
        );
    }

    if !stderr_lines.is_empty() {
        data_lines.push(text("STDERR:").into());
        data_lines.extend(
            stderr_lines
                .iter()
                .rev()
                .map(|element| text(element).into()),
        );
    }

    if !error_lines.is_empty() {
        data_lines.push(text("RUN ERROR:").into());
        data_lines.extend(error_lines.iter().rev().map(|element| text(element).into()));
    }

    let data: Element<_> = column(data_lines).spacing(10).into();

    return column![scrollable(data)]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .align_items(Alignment::Start);
}

fn produce_script_edit_content<'a>(execution_data: &ScriptExecutionData) -> Column<'a, Message> {
    if execution_data.currently_edited_script == -1 {
        return Column::new();
    }

    let button = |label, message| {
        button(
            text(label)
                .vertical_alignment(alignment::Vertical::Center)
                .size(16),
        )
        .padding(4)
        .on_press(message)
    };

    let script = &execution_data.scripts_to_run[execution_data.currently_edited_script as usize];

    let script_idx = execution_data.currently_edited_script as isize;
    let arguments = text_input("\"arg1\" \"arg2\"", &script.arguments_line)
        .on_input(move |new_arg| Message::EditArguments(new_arg, script_idx))
        .padding(5);

    let content = column![
        text(format!(
            "{}",
            script
                .path
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("[error]")
        )),
        button(
            "Remove script",
            Message::RemoveScript(execution_data.currently_edited_script)
        ),
        text("Arguments line:"),
        arguments,
    ]
    .spacing(10);

    return column![scrollable(content),]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .align_items(Alignment::Start);
}

fn view_content<'a>(
    execution_data: &ScriptExecutionData,
    path_caches: &PathCaches,
    variant: &PaneVariant,
) -> Element<'a, Message> {
    let content = match variant {
        PaneVariant::ScriptList => produce_script_list_content(execution_data, path_caches),
        PaneVariant::ExecutionList => produce_execution_list_content(execution_data, path_caches),
        PaneVariant::LogOutput => produce_log_output_content(execution_data, path_caches),
        PaneVariant::ScriptEdit => produce_script_edit_content(execution_data),
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(5)
        .center_y()
        .into()
}

fn view_controls<'a>(
    pane: pane_grid::Pane,
    total_panes: usize,
    is_maximized: bool,
) -> Element<'a, Message> {
    let mut row = row![].spacing(5);

    if total_panes > 1 {
        let toggle = {
            let (content, message) = if is_maximized {
                ("Restore", Message::Restore)
            } else {
                ("Maximize", Message::Maximize(pane))
            };
            button(text(content).size(14))
                .style(theme::Button::Secondary)
                .padding(3)
                .on_press(message)
        };

        row = row.push(toggle);
    }

    row.into()
}

mod style {
    use iced::widget::container;
    use iced::Theme;

    pub fn title_bar_active(theme: &Theme) -> container::Appearance {
        let palette = theme.extended_palette();

        container::Appearance {
            text_color: Some(palette.background.strong.text),
            background: Some(palette.background.strong.color.into()),
            ..Default::default()
        }
    }

    pub fn title_bar_focused(theme: &Theme) -> container::Appearance {
        let palette = theme.extended_palette();

        container::Appearance {
            text_color: Some(palette.primary.strong.text),
            background: Some(palette.primary.strong.color.into()),
            ..Default::default()
        }
    }

    pub fn pane_active(theme: &Theme) -> container::Appearance {
        let palette = theme.extended_palette();

        container::Appearance {
            background: Some(palette.background.weak.color.into()),
            border_width: 2.0,
            border_color: palette.background.strong.color,
            ..Default::default()
        }
    }

    pub fn pane_focused(theme: &Theme) -> container::Appearance {
        let palette = theme.extended_palette();

        container::Appearance {
            background: Some(palette.background.weak.color.into()),
            border_width: 2.0,
            border_color: palette.primary.strong.color,
            ..Default::default()
        }
    }
}
