use iced::alignment::{self, Alignment};
use iced::executor;
use iced::theme::{self, Theme};
use iced::time;
use iced::widget::pane_grid::{self, Configuration, PaneGrid};
use iced::widget::{button, column, container, row, scrollable, text, text_input, Column};
use iced::{Application, Command, Element, Length, Subscription};
use iced_lazy::responsive;
use iced_native::widget::checkbox;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, Instant};

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use crate::config;
use crate::execution;
use crate::style;

// caches for visual elements content
pub struct VisualCaches {
    autorerun_count: String,
    recent_logs: Vec<String>,
}

pub struct MainWindow {
    panes: pane_grid::State<AppPane>,
    focus: Option<pane_grid::Pane>,
    execution_data: execution::ScriptExecutionData,
    scripts: Vec<config::ScriptDefinition>,
    app_config: config::AppConfig,
    theme: Theme,
    visual_caches: VisualCaches,
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
    RescheduleScripts(),
    Tick(Instant),
    OpenScriptEditing(isize),
    RemoveScript(isize),
    MoveScriptUp(usize),
    EditScriptName(String, isize),
    EditArguments(String, isize),
    EditAutorerunCount(String, isize),
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
                theme: if app_config.custom_theme.is_some() {
                    style::get_custom_theme(app_config.custom_theme.clone().unwrap())
                } else {
                    Theme::default()
                },
                app_config,
                visual_caches: VisualCaches {
                    autorerun_count: String::new(),
                    recent_logs: Vec::new(),
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
            Message::Maximize(pane) => {
                self.focus = Some(pane);
                self.panes.maximize(&pane)
            }
            Message::Restore => {
                self.panes.restore();
            }
            Message::AddScriptToRun(script) => {
                if !execution::has_started_execution(&self.execution_data) {
                    execution::add_script_to_execution(&mut self.execution_data, script);
                }
                let script_idx = (self.execution_data.scripts_to_run.len() - 1) as isize;
                set_selected_script(
                    &mut self.execution_data,
                    &mut self.visual_caches,
                    script_idx,
                );
            }
            Message::RunScripts() => {
                if self.execution_data.scripts_to_run.is_empty() {
                    return Command::none();
                }

                if !execution::has_started_execution(&self.execution_data) {
                    self.visual_caches.recent_logs.clear();
                    set_selected_script(&mut self.execution_data, &mut self.visual_caches, -1);
                    execution::run_scripts(&mut self.execution_data, &self.app_config);
                }
            }
            Message::StopScripts() => {
                if execution::has_started_execution(&self.execution_data)
                    && !execution::has_finished_execution(&self.execution_data)
                {
                    if let Ok(mut termination_requested) =
                        self.execution_data.termination_condvar.0.lock()
                    {
                        *termination_requested = true;
                        // We notify the condvar that the value has changed.
                        self.execution_data.termination_condvar.1.notify_one();
                    }
                }
            }
            Message::ClearScripts() => {
                join_execution_thread(&mut self.execution_data);
                self.execution_data = execution::new_execution_data();
                self.execution_data.has_started = false;
            }
            Message::RescheduleScripts() => {
                join_execution_thread(&mut self.execution_data);
                if !execution::has_started_execution(&self.execution_data) {
                    return Command::none();
                }

                execution::reset_execution_progress(&mut self.execution_data);
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
                                set_selected_script(
                                    &mut self.execution_data,
                                    &mut self.visual_caches,
                                    progress.0 as isize,
                                );
                            }
                        }
                    }
                }
            }
            Message::OpenScriptEditing(script_idx) => {
                set_selected_script(
                    &mut self.execution_data,
                    &mut self.visual_caches,
                    script_idx,
                );
            }
            Message::RemoveScript(script_idx) => {
                execution::remove_script_from_execution(&mut self.execution_data, script_idx);
                set_selected_script(&mut self.execution_data, &mut self.visual_caches, -1);
            }
            Message::MoveScriptUp(script_idx) => {
                self.execution_data
                    .scripts_to_run
                    .swap(script_idx, script_idx - 1);
                set_selected_script(
                    &mut self.execution_data,
                    &mut self.visual_caches,
                    script_idx as isize - 1,
                );
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
            Message::EditAutorerunCount(new_autorerun_count_str, script_idx) => {
                let parse_result = usize::from_str(&new_autorerun_count_str);
                let mut new_autorerun_count = None;
                if parse_result.is_ok() {
                    self.visual_caches.autorerun_count = new_autorerun_count_str;
                    new_autorerun_count = Some(parse_result.unwrap());
                } else {
                    // if input is empty, then keep it empty and assume 0, otherwise keep the old value
                    if new_autorerun_count_str.is_empty() {
                        self.visual_caches.autorerun_count = new_autorerun_count_str;
                        new_autorerun_count = Some(0);
                    }
                }

                if self.execution_data.currently_selected_script != -1
                    && new_autorerun_count.is_some()
                {
                    self.execution_data.scripts_to_run[script_idx as usize].autorerun_count =
                        new_autorerun_count.unwrap();
                }
            }
            Message::OpenFile(path) => {
                #[cfg(target_os = "windows")]
                {
                    let result = std::process::Command::new("explorer")
                        .creation_flags(0x08000000) // CREATE_NO_WINDOW
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

            let title = row![get_pane_name_from_variant(variant)].spacing(5);

            let title_bar = pane_grid::TitleBar::new(title)
                .controls(view_controls(id, total_panes, is_maximized, &self.panes))
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
                    &self.visual_caches,
                    &self.app_config.custom_title,
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
        time::every(Duration::from_millis(100)).map(Message::Tick)
    }
}

fn set_selected_script(
    execution_data: &mut execution::ScriptExecutionData,
    visual_caches: &mut VisualCaches,
    script_idx: isize,
) {
    execution_data.currently_selected_script = script_idx;
    if script_idx != -1 {
        visual_caches.autorerun_count = execution_data
            .scripts_to_run
            .get(script_idx as usize)
            .unwrap()
            .autorerun_count
            .to_string();
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
        let config_path = paths.config_path.to_str().unwrap_or_default();

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
    custom_title: &Option<String>,
) -> Column<'a, Message> {
    let main_button = |label, message| {
        button(
            text(label)
                .width(Length::Fill)
                .horizontal_alignment(alignment::Horizontal::Center)
                .size(16),
        )
        .width(Length::Shrink)
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

    let mut title: Element<_> = text(path_caches.work_path.to_str().unwrap_or_default())
        .size(16)
        .horizontal_alignment(alignment::Horizontal::Center)
        .width(Length::Fill)
        .into();

    if let Some(new_title) = custom_title {
        title = column![
            title,
            text(new_title)
                .size(16)
                .horizontal_alignment(alignment::Horizontal::Center)
                .width(Length::Fill)
        ]
        .into();
    }

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
                        .unwrap_or(Instant::now())
                        .duration_since(script_status.start_time.unwrap_or(Instant::now()))
                        .as_secs();
                    text(format!(
                        "  {} {} ({:02}:{:02}){}",
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
                        .duration_since(script_status.start_time.unwrap_or(Instant::now()))
                        .as_secs();
                    text(format!(
                        "  [...] {} ({:02}:{:02}){}",
                        script_name,
                        time_taken_sec / 60,
                        time_taken_sec % 60,
                        repeat_text,
                    ))
                    .into()
                } else {
                    text(format!("  {}", script_name)).into()
                };

                let mut row_data: Vec<Element<'_, Message, iced::Renderer>> = Vec::new();
                row_data.push(name);

                let is_enabled = !execution_data.has_started;
                let is_selected = execution_data.currently_selected_script == i as isize;

                if is_enabled && is_selected {
                    if i > 0 {
                        row_data.push(
                            small_button("^", Message::MoveScriptUp(i))
                                .style(theme::Button::Primary)
                                .into(),
                        );
                    }
                    row_data.push(text(" ").width(Length::Fill).into());
                    row_data.push(
                        small_button("del", Message::RemoveScript(i as isize))
                            .style(theme::Button::Destructive)
                            .into(),
                    );
                } else if execution::has_script_started(&script_status) {
                    row_data.push(text(" ").into());
                    if script_status.retry_count > 0 {
                        let log_dir_path =
                            config::get_script_log_directory(&path_caches.logs_path, i as isize);
                        row_data.push(small_button("logs", Message::OpenFile(log_dir_path)).into());
                    } else if !execution::has_script_been_skipped(&script_status) {
                        let output_path = config::get_script_output_path(
                            &path_caches.logs_path,
                            i as isize,
                            script_status.retry_count,
                        );
                        row_data.push(small_button("log", Message::OpenFile(output_path)).into());
                    }
                }

                if is_enabled {
                    let mut list_item = button(row(row_data)).width(Length::Fill).padding(4);
                    if is_selected {
                        list_item = list_item.on_press(Message::OpenScriptEditing(-1));
                    } else {
                        list_item = list_item.on_press(Message::OpenScriptEditing(i as isize));
                    }

                    list_item = list_item.style(if is_selected {
                        theme::Button::Primary
                    } else {
                        theme::Button::Secondary
                    });

                    list_item.height(Length::Fixed(30.0)).into()
                } else {
                    row(row_data).height(Length::Fixed(30.0)).into()
                }
            })
            .collect(),
    )
    .width(Length::Fill)
    .align_items(Alignment::Start)
    .into();

    let controls = column![if execution::has_finished_execution(&execution_data) {
        if execution::has_finished_execution(&execution_data) {
            if !execution::is_waiting_execution_thread_to_finish(&execution_data) {
                row![
                    main_button("Reschedule", Message::RescheduleScripts()),
                    main_button("Clear", Message::ClearScripts()),
                ]
                .align_items(Alignment::Center)
                .spacing(5)
            } else {
                row![text("Waiting for the execution to stop")].align_items(Alignment::Center)
            }
        } else {
            row![main_button("Clear", Message::ClearScripts())].align_items(Alignment::Center)
        }
    } else if execution::has_started_execution(&execution_data) {
        row![main_button("Stop", Message::StopScripts())].align_items(Alignment::Center)
    } else if !execution_data.scripts_to_run.is_empty() {
        row![
            main_button("Run", Message::RunScripts()),
            main_button("Clear", Message::ClearScripts()),
        ]
        .align_items(Alignment::Center)
        .spacing(5)
    } else {
        row![].into()
    }]
    .spacing(5)
    .width(Length::Fill)
    .align_items(Alignment::Center);

    return column![title, scrollable(column![data, text(" "), controls])]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .align_items(Alignment::Center);
}

fn produce_log_output_content<'a>(
    execution_data: &execution::ScriptExecutionData,
) -> Column<'a, Message> {
    if !execution::has_started_execution(&execution_data) {
        return Column::new();
    }

    let current_script_idx = execution_data.currently_selected_script;

    if current_script_idx == -1 {
        return Column::new();
    }

    let current_script = &execution_data.scripts_to_run[current_script_idx as usize];

    let header = text(format!(
        "command: {} {}",
        current_script
            .path
            .file_name()
            .unwrap_or_default()
            .to_str()
            .unwrap_or("[error]")
            .to_string(),
        current_script.arguments_line,
    ));

    let mut data_lines: Vec<Element<'_, Message, iced::Renderer>> = Vec::new();
    if let Ok(guard) = execution_data.recent_logs.lock() {
        if !guard.is_empty() {
            data_lines.extend(guard.iter().map(|element| text(element).into()));
        }
    }

    let data: Element<_> = column(data_lines).spacing(10).width(Length::Fill).into();

    return column![header, scrollable(data)]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .align_items(Alignment::Start);
}

fn produce_script_edit_content<'a>(
    execution_data: &execution::ScriptExecutionData,
    visual_caches: &VisualCaches,
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
        .on_input(move |new_value| Message::EditArguments(new_value, script_idx))
        .padding(5);

    let autorerun_count = text_input("0", &visual_caches.autorerun_count)
        .on_input(move |new_value| Message::EditAutorerunCount(new_value, script_idx))
        .padding(5);

    let ignore_failures_checkbox = checkbox(
        "Ignore previous failures",
        script.ignore_previous_failures,
        move |val| Message::ToggleIgnoreFailures(script_idx, val),
    );

    let content = column![
        script_name,
        text("Arguments line:"),
        arguments,
        text("Retry count:"),
        autorerun_count,
        ignore_failures_checkbox,
        button(
            "Remove script",
            Message::RemoveScript(execution_data.currently_selected_script)
        )
        .style(theme::Button::Destructive),
    ]
    .spacing(10);

    return column![scrollable(content)]
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
    visual_caches: &VisualCaches,
    custom_title: &Option<String>,
) -> Element<'a, Message> {
    let content = match variant {
        PaneVariant::ScriptList => {
            produce_script_list_content(execution_data, script_definitions, paths)
        }
        PaneVariant::ExecutionList => {
            produce_execution_list_content(execution_data, paths, theme, custom_title)
        }
        PaneVariant::LogOutput => produce_log_output_content(execution_data),
        PaneVariant::ScriptEdit => produce_script_edit_content(execution_data, visual_caches),
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
    pane_grid: &pane_grid::State<AppPane>,
) -> Element<'a, Message> {
    let mut row = row![].spacing(5);

    if total_panes > 1 {
        if is_maximized {
            row = add_pane_switch_button(pane_grid, &pane, true, row);
            row = add_pane_switch_button(pane_grid, &pane, false, row);
        }

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

fn join_execution_thread(execution_data: &mut execution::ScriptExecutionData) {
    // this should never block, since the thread should be finished by now
    // but we do it anyway to avoid missing bugs that create zombie threads
    if let Some(join_handle) = execution_data.thread_join_handle.take() {
        join_handle.join().unwrap();
    };
}

fn get_pane_name_from_variant(variant: &PaneVariant) -> &str {
    match variant {
        PaneVariant::ScriptList => "Scripts",
        PaneVariant::ExecutionList => "Execution",
        PaneVariant::LogOutput => "Log",
        PaneVariant::ScriptEdit => "Script details",
    }
}

fn add_pane_switch_button<'a>(
    pane_grid: &pane_grid::State<AppPane>,
    pane: &pane_grid::Pane,
    is_left: bool,
    row: iced::widget::Row<'a, Message, iced::Renderer>,
) -> iced::widget::Row<'a, Message, iced::Renderer> {
    if let Some(neighbor) = pane_grid.adjacent(
        &pane,
        if is_left {
            pane_grid::Direction::Left
        } else {
            pane_grid::Direction::Right
        },
    ) {
        let variant = &pane_grid.panes[&neighbor].variant;
        let pane_name = get_pane_name_from_variant(variant);
        row.push(
            button(
                text(format!(
                    "{} {}",
                    if is_left { "<-" } else { "->" },
                    pane_name
                ))
                .size(14),
            )
            .style(theme::Button::Secondary)
            .padding(3)
            .on_press(Message::Maximize(neighbor)),
        )
    } else {
        row
    }
}
