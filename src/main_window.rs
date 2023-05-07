use iced::alignment::{self, Alignment};
use iced::executor;
use iced::theme::{self, Theme};
use iced::time;
use iced::widget::pane_grid::{self, Configuration, PaneGrid};
use iced::widget::{button, column, container, row, scrollable, text, text_input, Column};
use iced::{Application, Command, Element, Length, Subscription};
use iced_lazy::responsive;
use iced_native::widget::checkbox;
use rev_buf_reader::RevBufReader;
use std::io::BufRead;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::config;
use crate::execution;
use crate::style;

pub struct MainWindow {
    panes: pane_grid::State<AppPane>,
    focus: Option<pane_grid::Pane>,
    execution_data: execution::ScriptExecutionData,
    scripts: Vec<config::ScriptDefinition>,
    app_config: config::AppConfig,
    theme: Theme,
}

#[derive(Debug, Clone)]
pub enum Message {
    Clicked(pane_grid::Pane),
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
    Maximize(pane_grid::Pane),
    Restore,
    AddScriptToRun(config::ScriptDefinition),
    RunScripts(),
    StopScripts(),
    ClearScripts(),
    Tick(Instant),
    OpenScriptEditing(isize),
    RemoveScript(isize),
    EditScriptName(String, isize),
    EditArguments(String, isize),
    EditAutorerunCount(usize, isize),
    OpenFile(PathBuf),
    ToggleIgnoreFailures(isize, bool),
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
        let app_config = config::get_app_config_copy();

        (
            MainWindow {
                panes,
                focus: None,
                scripts: app_config.script_definitions.clone(),
                execution_data: execution::new_execution_data(),
                theme: if app_config.dark_mode {
                    style::get_dark_theme()
                } else {
                    Theme::default()
                },
                app_config,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "scripter".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
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
                if !execution::has_started_execution(&self.execution_data) {
                    execution::add_script_to_execution(&mut self.execution_data, script);
                }
                self.execution_data.currently_selected_script =
                    (self.execution_data.scripts_to_run.len() - 1) as isize;
            }
            Message::RunScripts() => {
                if self.execution_data.scripts_to_run.is_empty() {
                    return Command::none();
                }

                if !execution::has_started_execution(&self.execution_data) {
                    self.execution_data.currently_selected_script = -1;
                    execution::run_scripts(&mut self.execution_data, &self.app_config);
                }
            }
            Message::StopScripts() => {
                if execution::has_started_execution(&self.execution_data) {
                    let mut termination_requested =
                        self.execution_data.termination_condvar.0.lock().unwrap();
                    *termination_requested = true;
                    // We notify the condvar that the value has changed.
                    self.execution_data.termination_condvar.1.notify_one();
                }
            }
            Message::ClearScripts() => {
                self.execution_data = execution::new_execution_data();
                self.execution_data.has_started = false;
            }
            Message::Tick(_now) => {
                if let Some(rx) = &self.execution_data.progress_receiver {
                    if let Ok(progress) = rx.try_recv() {
                        if execution::has_script_failed(&progress.1) {
                            self.execution_data.has_failed_scripts = true;
                        }
                        self.execution_data.scripts_status[progress.0] = progress.1;
                        self.execution_data.currently_outputting_script = progress.0 as isize;
                        let progress_status = &self.execution_data.scripts_status[progress.0];

                        // move selection to the next script if the previous one was selected
                        if self.execution_data.currently_selected_script == -1
                            || (self.execution_data.currently_selected_script
                                == progress.0 as isize - 1)
                        {
                            if !execution::has_script_finished(progress_status)
                                || progress_status.result != execution::ScriptResultStatus::Skipped
                            {
                                self.execution_data.currently_selected_script = progress.0 as isize;
                            }
                        }
                    }
                }
            }
            Message::OpenScriptEditing(script_idx) => {
                self.execution_data.currently_selected_script = script_idx;
            }
            Message::RemoveScript(script_idx) => {
                execution::remove_script_from_execution(&mut self.execution_data, script_idx);
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
                    let result = std::process::Command::new("explorer")
                        .arg(path)
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn();

                    if result.is_err() {
                        return Command::none();
                    }
                }
                #[cfg(target_os = "linux")]
                {
                    let result = std::process::Command::new("xdg-open")
                        .arg(path)
                        .stdin(std::process::Stdio::null())
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::null())
                        .spawn();

                    if result.is_err() {
                        return Command::none();
                    }
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
                    } else if execution::has_finished_execution(&self.execution_data) {
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
                    variant,
                    &self.scripts,
                    &self.theme,
                    &self.app_config.paths,
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

    fn theme(&self) -> Theme {
        self.theme.clone()
    }

    fn subscription(&self) -> Subscription<Message> {
        time::every(Duration::from_millis(10)).map(Message::Tick)
    }
}

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

fn is_file_empty(path: &PathBuf) -> bool {
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
    execution_data: &execution::ScriptExecutionData,
    script_definitions: &Vec<config::ScriptDefinition>,
    paths: &config::PathCaches,
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
        let config_path = paths.config_path.to_str().unwrap();

        return column![text(format!(
            "No scripts found in config file \"{}\", or the config file is invalid.",
            &config_path
        ))];
    }

    let data: Element<_> = column(
        script_definitions
            .iter()
            .map(|script| {
                if !execution::has_started_execution(&execution_data) {
                    row![
                        button("Add", Message::AddScriptToRun(script.clone()),),
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
    execution_data: &execution::ScriptExecutionData,
    path_caches: &config::PathCaches,
    theme: &Theme,
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

    let title: Element<_> = text(path_caches.work_path.to_str().unwrap())
        .size(16)
        .into();

    let data: Element<_> = column(
        execution_data
            .scripts_to_run
            .iter()
            .enumerate()
            .map(|(i, element)| {
                let script_name = &element.name;

                let script_status = &execution_data.scripts_status[i];

                let repeat_text = if script_status.retry_count > 0 {
                    format!(
                        " [{}/{}]",
                        script_status.retry_count, element.autorerun_count
                    )
                } else {
                    String::new()
                };

                let name = if execution::has_script_finished(script_status) {
                    let mut failed = false;
                    let status = match script_status.result {
                        execution::ScriptResultStatus::Failed => {
                            failed = true;
                            "[FAILED]"
                        }
                        execution::ScriptResultStatus::Success => "[DONE]",
                        execution::ScriptResultStatus::Skipped => "[SKIPPED]",
                    };
                    let time_taken_sec = script_status
                        .finish_time
                        .unwrap()
                        .duration_since(script_status.start_time.unwrap())
                        .as_secs();
                    text(format!(
                        "{} {} ({:02}:{:02}){}",
                        status,
                        script_name,
                        time_taken_sec / 60,
                        time_taken_sec % 60,
                        repeat_text,
                    ))
                    .style(if failed {
                        theme.extended_palette().danger.weak.color
                    } else {
                        theme.extended_palette().background.strong.text
                    })
                    .into()
                } else if execution::has_script_started(script_status) {
                    let time_taken_sec = Instant::now()
                        .duration_since(script_status.start_time.unwrap())
                        .as_secs();
                    text(format!(
                        "[...] {} ({:02}:{:02}){}",
                        script_name,
                        time_taken_sec / 60,
                        time_taken_sec % 60,
                        repeat_text,
                    ))
                    .into()
                } else {
                    text(format!("{}", script_name)).into()
                };

                let mut row_data: Vec<Element<'_, Message, iced::Renderer>> = Vec::new();
                row_data.push(name);

                if !execution::has_started_execution(&execution_data) {
                    row_data.push(text(" ").into());
                    row_data
                        .push(small_button("Edit", Message::OpenScriptEditing(i as isize)).into());
                } else if execution::has_script_started(&script_status) {
                    let stdout_path = config::get_stdout_path(
                        path_caches.logs_path.clone(),
                        i as isize,
                        script_status.retry_count,
                    );
                    if !is_file_empty(&stdout_path) {
                        row_data.push(text(" ").into());
                        row_data.push(small_button("log", Message::OpenFile(stdout_path)).into());
                    }
                    let stderr_path = config::get_stderr_path(
                        path_caches.logs_path.clone(),
                        i as isize,
                        script_status.retry_count,
                    );
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

    let controls = column![if execution::has_finished_execution(&execution_data) {
        main_button("Clear", Message::ClearScripts())
    } else if execution::has_started_execution(&execution_data) {
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

fn get_last_n_lines_from_file(file_path: &PathBuf, lines_number: usize) -> Option<Vec<String>> {
    let file = std::fs::File::open(file_path);

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
    execution_data: &execution::ScriptExecutionData,
    path_caches: &config::PathCaches,
) -> Column<'a, Message> {
    if !execution::has_started_execution(&execution_data) {
        return Column::new();
    }

    let current_script_idx = execution_data.currently_selected_script;

    if current_script_idx == -1 {
        return Column::new();
    }

    let current_script = &execution_data.scripts_to_run[current_script_idx as usize];
    let script_status = &execution_data.scripts_status[current_script_idx as usize];

    let stdout_file_path = config::get_stdout_path(
        path_caches.logs_path.clone(),
        current_script_idx,
        script_status.retry_count,
    );
    let stdout_lines = get_last_n_lines_from_file(&stdout_file_path, 30);
    let stderr_file_path = config::get_stderr_path(
        path_caches.logs_path.clone(),
        current_script_idx,
        script_status.retry_count,
    );
    let stderr_lines = get_last_n_lines_from_file(&stderr_file_path, 30);
    let error_file_path = path_caches
        .logs_path
        .join(format!("{}_error.log", current_script_idx));
    let error_lines = get_last_n_lines_from_file(&error_file_path, 10);

    if stdout_lines.is_none() {
        return column![text(
            format!(
                "Can't open script output '{}'",
                stdout_file_path.to_str().unwrap()
            )
            .to_string()
        )];
    }
    if stderr_lines.is_none() {
        return column![text(
            format!(
                "Can't open script output '{}'",
                stderr_file_path.to_str().unwrap()
            )
            .to_string()
        )];
    }

    let stdout_lines = stdout_lines.unwrap();
    let stderr_lines = stderr_lines.unwrap();
    let error_lines = error_lines.unwrap_or(Vec::new());

    let mut data_lines: Vec<Element<'_, Message, iced::Renderer>> = Vec::new();

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

fn produce_script_edit_content<'a>(
    execution_data: &execution::ScriptExecutionData,
) -> Column<'a, Message> {
    if execution::has_started_execution(&execution_data) {
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
        .on_input(move |new_arg| Message::EditScriptName(new_arg, script_idx))
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
    execution_data: &execution::ScriptExecutionData,
    variant: &PaneVariant,
    script_definitions: &Vec<config::ScriptDefinition>,
    theme: &Theme,
    paths: &config::PathCaches,
) -> Element<'a, Message> {
    let content = match variant {
        PaneVariant::ScriptList => {
            produce_script_list_content(execution_data, script_definitions, paths)
        }
        PaneVariant::ExecutionList => produce_execution_list_content(execution_data, paths, theme),
        PaneVariant::LogOutput => produce_log_output_content(execution_data, paths),
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
