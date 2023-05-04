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
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::{mpsc, Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use iced_native::widget::checkbox;
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

const CONFIG_NAME: &str = "scripter_config.json";
thread_local!(static GLOBAL_CONFIG: AppConfig = read_config());

pub fn main() -> iced::Result {
    let mut settings = Settings::default();
    settings.window.icon = Option::from(
        icon::from_rgba(include_bytes!("../res/icon.rgba").to_vec(), 128, 128).unwrap(),
    );
    settings.window.position = iced::window::Position::Centered;
    settings.window.always_on_top = GLOBAL_CONFIG.with(|config| config.always_on_top);
    MainWindow::run(settings)
}

#[derive(Clone)]
struct ScheduledScript {
    name: String,
    path: Box<Path>,
    arguments_line: String,
    autorerun_count: usize,
    ignore_previous_failures: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ScriptResultStatus {
    Success,
    Failed,
    Skipped,
}

#[derive(Clone)]
struct ScriptExecutionStatus {
    start_time: Option<Instant>,
    finish_time: Option<Instant>,
    result: ScriptResultStatus,
    retry_count: usize,
}

fn has_script_started(status: &ScriptExecutionStatus) -> bool {
    return status.start_time.is_some();
}

fn has_script_finished(status: &ScriptExecutionStatus) -> bool {
    if !has_script_started(status) {
        return false;
    }
    return status.finish_time.is_some();
}

fn has_script_failed(status: &ScriptExecutionStatus) -> bool {
    return has_script_finished(status) && status.result == ScriptResultStatus::Failed;
}

fn get_default_script_execution_status() -> ScriptExecutionStatus {
    ScriptExecutionStatus {
        start_time: None,
        finish_time: None,
        result: ScriptResultStatus::Skipped,
        retry_count: 0,
    }
}

struct ScriptExecutionData {
    scripts_to_run: Vec<ScheduledScript>,
    scripts_status: Vec<ScriptExecutionStatus>,
    has_started: bool,
    progress_receiver: Option<mpsc::Receiver<(usize, ScriptExecutionStatus)>>,
    termination_condvar: Arc<(Mutex<bool>, Condvar)>,
    currently_selected_script: isize,
    currently_outputting_script: isize,
    has_failed_scripts: bool,
}

fn new_execution_data() -> ScriptExecutionData {
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

fn has_started_execution(execution_data: &ScriptExecutionData) -> bool {
    return execution_data.has_started;
}

fn has_finished_execution(execution_data: &ScriptExecutionData) -> bool {
    if !has_started_execution(&execution_data) {
        return false;
    }
    return has_script_finished(&execution_data.scripts_status.last().unwrap());
}

fn add_script_to_execution(execution_data: &mut ScriptExecutionData, script: ScriptDefinition) {
    execution_data.scripts_to_run.push(ScheduledScript {
        name: script.name,
        path: script.command,
        arguments_line: script.arguments,
        autorerun_count: script.autorerun_count,
        ignore_previous_failures: script.ignore_previous_failures,
    });
    execution_data
        .scripts_status
        .push(get_default_script_execution_status());
}

fn remove_script_from_execution(execution_data: &mut ScriptExecutionData, index: isize) {
    execution_data.scripts_to_run.remove(index as usize);
    execution_data.scripts_status.remove(index as usize);
}

struct PathCaches {
    logs_path: String,
    work_path: String,
}

struct MainWindow {
    panes: pane_grid::State<AppPane>,
    focus: Option<pane_grid::Pane>,
    execution_data: ScriptExecutionData,
    scripts: Vec<ScriptDefinition>,
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
    AddScriptToRun(ScriptDefinition),
    RunScripts(),
    StopScripts(),
    ClearScripts(),
    Tick(Instant),
    OpenScriptEditing(isize),
    RemoveScript(isize),
    EditScriptName(String, isize),
    EditArguments(String, isize),
    EditAutorerunCount(usize, isize),
    OpenFile(String),
    ToggleIgnoreFailures(isize, bool),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ScriptDefinition {
    name: String,
    command: Box<Path>,
    arguments: String,
    autorerun_count: usize,
    ignore_previous_failures: bool,
}

#[derive(Default, Clone, Deserialize, Serialize)]
struct AppConfig {
    script_definitions: Vec<ScriptDefinition>,
    always_on_top: bool,
}

fn get_default_config() -> AppConfig {
    AppConfig {
        script_definitions: Vec::new(),
        always_on_top: true,
    }
}

fn read_config() -> AppConfig {
    let config_path = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .join(CONFIG_NAME);

    if !config_path.exists() {
        // create the file
        let config = get_default_config();
        let data = serde_json::to_string_pretty(&config);
        if data.is_err() {
            return get_default_config();
        }
        let data = data.unwrap();
        let result = std::fs::write(&config_path, data);
        if result.is_err() {
            return get_default_config();
        }
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

fn get_logs_path() -> String {
    let pid = std::process::id();
    let folder_name = format!("scripter_logs/exec_logs_{}", pid);
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

fn run_scripts(execution_data: &mut ScriptExecutionData, path_caches: &PathCaches) {
    let (tx, rx) = mpsc::channel();
    execution_data.progress_receiver = Some(rx);
    execution_data.has_started = true;

    let scripts_to_run = execution_data.scripts_to_run.clone();
    let termination_condvar = execution_data.termination_condvar.clone();
    let logs_path = path_caches.logs_path.clone();

    std::thread::spawn(move || {
        std::fs::remove_dir_all(&logs_path).ok();

        let mut termination_requested = termination_condvar.0.lock().unwrap();
        let mut has_previous_script_failed = false;
        let mut kill_requested = false;
        for script_idx in 0..scripts_to_run.len() {
            let script = &scripts_to_run[script_idx];
            let mut script_state = get_default_script_execution_status();
            script_state.start_time = Some(Instant::now());

            if kill_requested || (has_previous_script_failed && !script.ignore_previous_failures) {
                script_state.result = ScriptResultStatus::Skipped;
                script_state.finish_time = Some(Instant::now());
                tx.send((script_idx, script_state.clone())).unwrap();
                continue;
            }
            tx.send((script_idx, script_state.clone())).unwrap();

            'retry_loop: loop {
                std::fs::create_dir_all(&logs_path)
                    .expect(&format!("failed to create \"{}\" directory", &logs_path));

                let stdout_file =
                    std::fs::File::create(get_stdout_path(&logs_path, script_idx as isize))
                        .expect("failed to create stdout file");
                let stderr_file =
                    std::fs::File::create(get_stderr_path(&logs_path, script_idx as isize))
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
                        &logs_path, script_idx as isize
                    ))
                    .expect("failed to create error file");
                    let mut error_writer = std::io::BufWriter::new(error_file);
                    write!(error_writer, "{}", err).expect("failed to write error");
                    // it doesn't make sense to retry if something is broken on this level
                    script_state.result = ScriptResultStatus::Failed;
                    script_state.finish_time = Some(Instant::now());
                    tx.send((script_idx, script_state.clone())).unwrap();
                    has_previous_script_failed = true;
                    break 'retry_loop;
                }

                let mut child = child.unwrap();

                loop {
                    let result = termination_condvar
                        .1
                        .wait_timeout(termination_requested, Duration::from_millis(10))
                        .unwrap();
                    // 10 milliseconds have passed, or maybe the value changed
                    termination_requested = result.0;
                    if *termination_requested == true {
                        // we received the notification and the value has been updated, we can leave
                        let kill_result = child.kill();
                        if kill_result.is_err() {
                            println!(
                                "failed to kill child process: {}",
                                kill_result.err().unwrap()
                            );
                        }
                        script_state.finish_time = Some(Instant::now());
                        script_state.result = ScriptResultStatus::Failed;
                        tx.send((script_idx, script_state.clone())).unwrap();
                        kill_requested = true;
                        break 'retry_loop;
                    }

                    if let Ok(Some(status)) = child.try_wait() {
                        if status.success() {
                            // successfully finished the script, jump to the next script
                            script_state.finish_time = Some(Instant::now());
                            script_state.result = ScriptResultStatus::Success;
                            tx.send((script_idx, script_state.clone())).unwrap();
                            has_previous_script_failed = false;
                            break 'retry_loop;
                        } else {
                            if script_state.retry_count < script.autorerun_count {
                                // script failed, but we can retry
                                script_state.retry_count += 1;
                                tx.send((script_idx, script_state.clone())).unwrap();
                            } else {
                                // script failed and we can't retry
                                script_state.finish_time = Some(Instant::now());
                                script_state.result = ScriptResultStatus::Failed;
                                tx.send((script_idx, script_state.clone())).unwrap();
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
                scripts: GLOBAL_CONFIG.with(|config| config.script_definitions.clone()),
                execution_data: new_execution_data(),
                path_caches: PathCaches {
                    logs_path: get_logs_path(),
                    work_path: get_work_path(),
                },
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "scripter".to_string()
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
                if !has_started_execution(&self.execution_data) {
                    add_script_to_execution(&mut self.execution_data, script);
                }
                self.execution_data.currently_selected_script =
                    (self.execution_data.scripts_to_run.len() - 1) as isize;
            }
            Message::RunScripts() => {
                if self.execution_data.scripts_to_run.is_empty() {
                    return Command::none();
                }

                if !has_started_execution(&self.execution_data) {
                    self.execution_data.currently_selected_script = -1;
                    run_scripts(&mut self.execution_data, &self.path_caches);
                }
            }
            Message::StopScripts() => {
                if has_started_execution(&self.execution_data) {
                    let mut termination_requested =
                        self.execution_data.termination_condvar.0.lock().unwrap();
                    *termination_requested = true;
                    // We notify the condvar that the value has changed.
                    self.execution_data.termination_condvar.1.notify_one();
                }
            }
            Message::ClearScripts() => {
                self.execution_data = new_execution_data();
                self.execution_data.has_started = false;
            }
            Message::Tick(_now) => {
                if let Some(rx) = &self.execution_data.progress_receiver {
                    if let Ok(progress) = rx.try_recv() {
                        if has_script_failed(&progress.1) {
                            self.execution_data.has_failed_scripts = true;
                        }
                        self.execution_data.scripts_status[progress.0] = progress.1;
                        self.execution_data.currently_outputting_script = progress.0 as isize;

                        if self.execution_data.currently_selected_script == -1
                            || (self.execution_data.currently_selected_script
                                == progress.0 as isize - 1)
                        {
                            self.execution_data.currently_selected_script = progress.0 as isize;
                        }
                    }
                }
            }
            Message::OpenScriptEditing(script_idx) => {
                self.execution_data.currently_selected_script = script_idx;
            }
            Message::RemoveScript(script_idx) => {
                remove_script_from_execution(&mut self.execution_data, script_idx);
                self.execution_data.currently_selected_script = -1;
            }
            Message::EditScriptName(new_name, script_idx) => {
                if self.execution_data.currently_selected_script != -1 {
                    self.execution_data.scripts_to_run[script_idx as usize].name = new_name;
                }
            }
            Message::EditArguments(new_arguments, script_idx) => {
                if self.execution_data.currently_selected_script != -1 {
                    self.execution_data.scripts_to_run[script_idx as usize].arguments_line =
                        new_arguments;
                }
            }
            Message::EditAutorerunCount(new_autorerun_count, script_idx) => {
                if self.execution_data.currently_selected_script != -1 {
                    self.execution_data.scripts_to_run[script_idx as usize].autorerun_count =
                        new_autorerun_count;
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
            Message::ToggleIgnoreFailures(script_idx, value) => {
                if self.execution_data.currently_selected_script != -1 {
                    self.execution_data.scripts_to_run[script_idx as usize]
                        .ignore_previous_failures = value;
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
                PaneVariant::LogOutput => "Logs",
                PaneVariant::ScriptEdit => "Script Properties",
            }]
            .spacing(5);

            let title_bar = pane_grid::TitleBar::new(title)
                .controls(view_controls(id, total_panes, is_maximized))
                .padding(10)
                .style(if is_focused {
                    if self.execution_data.has_failed_scripts {
                        style::title_bar_focused_failed
                    } else if has_finished_execution(&self.execution_data) {
                        style::title_bar_focused_completed
                    } else {
                        style::title_bar_focused
                    }
                } else {
                    style::title_bar_active
                });

            pane_grid::Content::new(responsive(move |_size| {
                view_content(
                    &self.execution_data,
                    &self.path_caches,
                    variant,
                    &self.scripts,
                )
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
    script_definitions: &Vec<ScriptDefinition>,
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

    if script_definitions.is_empty() {
        return column![text(format!(
            "No scripts found in config file \"{}\", or the config file is invalid.",
            CONFIG_NAME
        ))];
    }

    let data: Element<_> = column(
        script_definitions
            .iter()
            .map(|script| {
                if !has_started_execution(&execution_data) {
                    row![
                        button(
                            "Add",
                            Message::AddScriptToRun(script.clone()),
                        ),
                        text(" "),
                        text(&script.name),
                    ]
                } else {
                    row![text(&script.name)]
                }
                .into()
            })
            .collect(),
    )
    .spacing(10)
    .width(Length::Fill)
    .into();

    return column![scrollable(data)]
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

    let title: Element<_> = text(format!("{}", path_caches.work_path)).size(16).into();

    let data: Element<_> = column(
        execution_data
            .scripts_to_run
            .iter()
            .enumerate()
            .map(|(i, element)| {
                let script_name = &element.name;

                let script_status = &execution_data.scripts_status[i];

                let name = if has_script_finished(script_status) {
                    let mut failed = false;
                    let status = match script_status.result {
                        ScriptResultStatus::Failed => {
                            failed = true;
                            "[FAILED]"
                        }
                        ScriptResultStatus::Success => "[DONE]",
                        ScriptResultStatus::Skipped => "[SKIPPED]",
                    };
                    let time_taken_sec = script_status
                        .finish_time
                        .unwrap()
                        .duration_since(script_status.finish_time.unwrap())
                        .as_secs();
                    text(format!(
                        "{} {} ({:02}:{:02})",
                        status,
                        script_name,
                        time_taken_sec / 60,
                        time_taken_sec % 60
                    ))
                    .style(theme::Text::Color(if failed {
                        iced::Color::from([1.0, 0.0, 0.0])
                    } else {
                        iced::Color::from([0.0, 0.0, 0.0])
                    }))
                    .into()
                } else if has_script_started(script_status) {
                    let time_taken_sec = Instant::now()
                        .duration_since(script_status.start_time.unwrap())
                        .as_secs();
                    text(format!(
                        "[...] {} ({:02}:{:02})",
                        script_name,
                        time_taken_sec / 60,
                        time_taken_sec % 60
                    ))
                    .style(theme::Text::Color(iced::Color::from([0.0, 0.0, 1.0])))
                    .into()
                } else {
                    text(format!("{}", script_name))
                        .style(if execution_data.currently_selected_script == i as isize {
                            theme::Text::Color(iced::Color::from([0.0, 0.0, 0.8]))
                        } else {
                            theme::Text::Default
                        })
                        .into()
                };

                let mut row_data: Vec<Element<'_, Message, iced::Renderer>> = Vec::new();
                row_data.push(name);

                if !has_started_execution(&execution_data) {
                    row_data.push(text(" ").into());
                    row_data
                        .push(small_button("Edit", Message::OpenScriptEditing(i as isize)).into());
                } else if has_script_started(&script_status) {
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

    let controls = column![if has_finished_execution(&execution_data) {
        main_button("Clear", Message::ClearScripts())
    } else if has_started_execution(&execution_data) {
        main_button("Stop", Message::StopScripts())
    } else {
        main_button("Run", Message::RunScripts())
    }]
    .spacing(5)
    .max_width(150)
    .align_items(Alignment::Center);

    return column![title, scrollable(data), controls]
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
    if !has_started_execution(&execution_data) {
        return Column::new();
    }

    let current_script_idx = execution_data.currently_selected_script;

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

    let current_script = &execution_data.scripts_to_run[current_script_idx as usize];
    data_lines.push(
        text(format!(
            "command: {} {}",
            current_script
                .path
                .file_name()
                .unwrap_or_default()
                .to_str()
                .unwrap_or("[error]")
                .to_string(),
            current_script.arguments_line,
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
    if has_started_execution(&execution_data) {
        return Column::new();
    }

    if execution_data.currently_selected_script == -1 {
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

    let script_idx = execution_data.currently_selected_script;
    let script = &execution_data.scripts_to_run[script_idx as usize];

    let script_name = text_input("name", &script.name)
        .on_input(move |new_arg| {
            Message::EditScriptName(new_arg, script_idx)
        })
        .padding(5);

    let arguments = text_input("\"arg1\" \"arg2\"", &script.arguments_line)
        .on_input(move |new_arg| Message::EditArguments(new_arg, script_idx))
        .padding(5);

    let autorerun_count = text_input("0", &script.autorerun_count.to_string())
        .on_input(move |new_arg| {
            Message::EditAutorerunCount(new_arg.parse().unwrap_or_default(), script_idx)
        })
        .padding(5);

    let ignore_failures_checkbox = checkbox(
        "Ignore previous failures",
        script.ignore_previous_failures,
        move |val| Message::ToggleIgnoreFailures(script_idx, val),
    );

    let content = column![
        script_name,
        button(
            "Remove script",
            Message::RemoveScript(execution_data.currently_selected_script)
        ),
        text("Arguments line:"),
        arguments,
        text("Retry count:"),
        autorerun_count,
        ignore_failures_checkbox,
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
    script_definitions: &Vec<ScriptDefinition>,
) -> Element<'a, Message> {
    let content = match variant {
        PaneVariant::ScriptList => produce_script_list_content(execution_data, script_definitions),
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

    pub fn title_bar_focused_completed(theme: &Theme) -> container::Appearance {
        let palette = theme.extended_palette();

        container::Appearance {
            text_color: Some(palette.background.strong.text),
            background: Some(iced::Color::from_rgb(0.3, 0.96, 0.21).into()),
            ..Default::default()
        }
    }

    pub fn title_bar_focused_failed(theme: &Theme) -> container::Appearance {
        let palette = theme.extended_palette();

        container::Appearance {
            text_color: Some(palette.background.strong.text),
            background: Some(iced::Color::from_rgb(0.96, 0.21, 0.13).into()),
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
