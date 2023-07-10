#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use iced::alignment::{self, Alignment};
use iced::theme::{self, Theme};
use iced::widget::pane_grid::{self, Configuration, PaneGrid};
use iced::widget::{button, column, container, row, scrollable, text, text_input, tooltip, Column};
use iced::{executor, ContentFit};
use iced::{time, Size};
use iced::{Application, Command, Element, Length, Subscription};
use iced_lazy::responsive;
use iced_native::command::Action;
use iced_native::image::Handle;
use iced_native::widget::{checkbox, horizontal_space, image, vertical_space};
use iced_native::window::Action::{RequestUserAttention, Resize};
use iced_native::window::UserAttention;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, Instant};

use crate::config;
use crate::execution;
use crate::style;

struct IconCaches {
    idle: Handle,
    in_progress: Handle,
    succeeded: Handle,
    failed: Handle,
    skipped: Handle,
}

// caches for visual elements content
pub struct VisualCaches {
    autorerun_count: String,
    recent_logs: Vec<String>,
    icons: IconCaches,
}

pub struct MainWindow {
    panes: pane_grid::State<AppPane>,
    focus: Option<pane_grid::Pane>,
    execution_data: execution::ScriptExecutionData,
    scripts: Vec<config::ScriptDefinition>,
    app_config: config::AppConfig,
    theme: Theme,
    visual_caches: VisualCaches,
    full_window_size: Size,
    edit_data: EditData,
}

#[derive(Debug, Clone)]
pub struct EditData {
    // identifies the script being edited, if any
    currently_edited_script: Option<EditScriptId>,
    // state of the global to the window editing mode
    window_edit_data: Option<WindowEditData>,
}

#[derive(Debug, Clone, PartialEq)]
enum EditScriptType {
    ScriptConfig,
    ExecutionList,
}

#[derive(Debug, Clone)]
pub struct EditScriptId {
    idx: usize,
    script_type: EditScriptType,
}

#[derive(Debug, Clone)]
struct WindowEditData {}

#[derive(Debug, Clone)]
pub enum Message {
    Clicked(pane_grid::Pane),
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
    Maximize(pane_grid::Pane, Size),
    SwitchMaximized(PaneVariant),
    Restore,
    AddScriptToRun(config::ScriptDefinition),
    RunScripts,
    StopScripts,
    ClearScripts,
    RescheduleScripts,
    Tick(Instant),
    OpenScriptEditing(usize),
    CloseScriptEditing,
    RemoveScript(EditScriptId),
    MoveScriptUp(usize),
    MoveScriptDown(usize),
    EditScriptName(String),
    EditArguments(String),
    EditAutorerunCount(String),
    OpenFile(PathBuf),
    ToggleIgnoreFailures(bool),
    EnterWindowEditMode,
    ExitWindowEditMode,
    OpenScriptConfigEditing,
    MoveConfigScriptUp(usize),
    MoveConfigScriptDown(usize),
}

impl Application for MainWindow {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let pane_configuration = Configuration::Split {
            axis: pane_grid::Axis::Vertical,
            ratio: 0.25,
            a: Box::new(Configuration::Split {
                axis: pane_grid::Axis::Horizontal,
                ratio: 0.65,
                a: Box::new(Configuration::Pane(AppPane::new(PaneVariant::ScriptList))),
                b: Box::new(Configuration::Pane(AppPane::new(PaneVariant::ScriptEdit))),
            }),
            b: Box::new(Configuration::Split {
                axis: pane_grid::Axis::Vertical,
                ratio: 0.5,
                a: Box::new(Configuration::Pane(AppPane::new(
                    PaneVariant::ExecutionList,
                ))),
                b: Box::new(Configuration::Pane(AppPane::new(PaneVariant::LogOutput))),
            }),
        };
        let panes = pane_grid::State::with_configuration(pane_configuration);
        let app_config = config::get_app_config_copy();

        (
            MainWindow {
                panes,
                focus: None,
                scripts: app_config.script_definitions.clone(),
                execution_data: execution::new_execution_data(),
                theme: if let Some(theme) = app_config.custom_theme.clone() {
                    style::get_custom_theme(theme)
                } else {
                    Theme::default()
                },
                app_config,
                visual_caches: VisualCaches {
                    autorerun_count: String::new(),
                    recent_logs: Vec::new(),
                    icons: IconCaches {
                        idle: Handle::from_memory(include_bytes!("../res/icons/idle.png")),
                        in_progress: Handle::from_memory(include_bytes!(
                            "../res/icons/in-progress.png"
                        )),
                        succeeded: Handle::from_memory(include_bytes!("../res/icons/positive.png")),
                        failed: Handle::from_memory(include_bytes!("../res/icons/negative.png")),
                        skipped: Handle::from_memory(include_bytes!("../res/icons/skip.png")),
                    },
                },
                full_window_size: Size::new(0.0, 0.0),
                edit_data: EditData {
                    window_edit_data: None,
                    currently_edited_script: None,
                },
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        if self.edit_data.window_edit_data.is_some() {
            "scripter [Editing]".to_string()
        } else if self.execution_data.has_started {
            if execution::has_finished_execution(&self.execution_data) {
                if self.execution_data.has_failed_scripts {
                    "scripter [Finished with errors]".to_string()
                } else {
                    "scripter [Finished]".to_string()
                }
            } else {
                "scripter [Running]".to_string()
            }
        } else {
            "scripter".to_string()
        }
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
            Message::Maximize(pane, window_size) => {
                self.focus = Some(pane);
                self.panes.maximize(&pane);
                if !self.app_config.keep_window_size {
                    self.full_window_size = window_size.clone();
                    let size = self
                        .panes
                        .layout()
                        .pane_regions(1.0, Size::new(window_size.width, window_size.height))
                        .get(&pane)
                        .unwrap() // tried to get an non-existing pane, this should never happen, so panic
                        .clone();
                    return Command::single(Action::Window(Resize {
                        height: size.height as u32,
                        width: size.width as u32,
                    }));
                }
            }
            Message::SwitchMaximized(pane_variant) => {
                let pane = self
                    .panes
                    .iter()
                    .find(|pane| pane.1.variant == pane_variant);
                if let Some((pane, _app_pane)) = pane {
                    let pane = pane.clone();
                    self.panes.maximize(&pane);
                    self.focus = Some(pane);
                }
            }
            Message::Restore => {
                self.panes.restore();
                if !self.app_config.keep_window_size {
                    return Command::single(Action::Window(Resize {
                        height: self.full_window_size.height as u32,
                        width: self.full_window_size.width as u32,
                    }));
                }
            }
            Message::AddScriptToRun(script) => {
                if !execution::has_started_execution(&self.execution_data) {
                    execution::add_script_to_execution(&mut self.execution_data, script);
                }
                let script_idx = self.execution_data.scripts_to_run.len() - 1;
                set_selected_script(
                    &mut self.edit_data.currently_edited_script,
                    &self.execution_data,
                    &mut self.visual_caches,
                    script_idx,
                    EditScriptType::ExecutionList,
                );
            }
            Message::RunScripts => {
                if self.execution_data.scripts_to_run.is_empty() {
                    return Command::none();
                }

                if !execution::has_started_execution(&self.execution_data) {
                    self.visual_caches.recent_logs.clear();
                    reset_selected_script(&mut self.edit_data.currently_edited_script);
                    execution::run_scripts(&mut self.execution_data, &self.app_config);
                }
            }
            Message::StopScripts => {
                if execution::has_started_execution(&self.execution_data)
                    && !execution::has_finished_execution(&self.execution_data)
                {
                    execution::request_stop_execution(&mut self.execution_data);
                }
            }
            Message::ClearScripts => {
                join_execution_thread(&mut self.execution_data);
                self.execution_data = execution::new_execution_data();
                self.execution_data.has_started = false;
                reset_selected_script(&mut self.edit_data.currently_edited_script);
            }
            Message::RescheduleScripts => {
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

                        if execution::has_finished_execution(&self.execution_data) {
                            if self.app_config.window_status_reactions {
                                return Command::single(Action::Window(RequestUserAttention(
                                    Some(UserAttention::Informational),
                                )));
                            }
                        }
                    }
                }
            }
            Message::OpenScriptEditing(script_idx) => {
                set_selected_script(
                    &mut self.edit_data.currently_edited_script,
                    &self.execution_data,
                    &mut self.visual_caches,
                    script_idx,
                    EditScriptType::ExecutionList,
                );
            }
            Message::CloseScriptEditing => {
                reset_selected_script(&mut self.edit_data.currently_edited_script);
            }
            Message::RemoveScript(script_id) => {
                match script_id.script_type {
                    EditScriptType::ScriptConfig => {}
                    EditScriptType::ExecutionList => {
                        execution::remove_script_from_execution(&mut self.execution_data, script_id.idx);
                        reset_selected_script(&mut self.edit_data.currently_edited_script);
                    }
                }
            }
            Message::MoveScriptUp(script_idx) => {
                self.execution_data
                    .scripts_to_run
                    .swap(script_idx, script_idx - 1);
                set_selected_script(
                    &mut self.edit_data.currently_edited_script,
                    &self.execution_data,
                    &mut self.visual_caches,
                    script_idx - 1,
                    EditScriptType::ExecutionList,
                );
            }
            Message::MoveScriptDown(script_idx) => {
                self.execution_data
                    .scripts_to_run
                    .swap(script_idx, script_idx + 1);
                set_selected_script(
                    &mut self.edit_data.currently_edited_script,
                    &self.execution_data,
                    &mut self.visual_caches,
                    script_idx + 1,
                    EditScriptType::ExecutionList,
                );
            }
            Message::EditScriptName(new_name) => {
                apply_script_edit(
                    self,
                    move |script| script.name = new_name.clone(),
                )
            }
            Message::EditArguments(new_arguments) => {
                apply_script_edit(
                    self,
                    move |script| script.arguments = new_arguments.clone(),
                )
            }
            Message::EditAutorerunCount(new_autorerun_count_str) => {
                let parse_result = usize::from_str(&new_autorerun_count_str);
                let mut new_autorerun_count = None;
                if let Ok(parse_result) = parse_result {
                    self.visual_caches.autorerun_count = new_autorerun_count_str;
                    new_autorerun_count = Some(parse_result);
                } else {
                    // if input is empty, then keep it empty and assume 0, otherwise keep the old value
                    if new_autorerun_count_str.is_empty() {
                        self.visual_caches.autorerun_count = new_autorerun_count_str;
                        new_autorerun_count = Some(0);
                    }
                }

                if let Some(new_autorerun_count) = new_autorerun_count {
                    apply_script_edit(
                        self,
                        |script| script.autorerun_count = new_autorerun_count,
                    )
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
            Message::ToggleIgnoreFailures(value) => {
                apply_script_edit(
                    self,
                    |script| script.ignore_previous_failures = value,
                )
            }
            Message::EnterWindowEditMode => {
                self.edit_data.window_edit_data = Some(WindowEditData {});
            }
            Message::ExitWindowEditMode => {
                self.edit_data.window_edit_data = None;
            }
            Message::OpenScriptConfigEditing => {}
            Message::MoveConfigScriptUp(index) => {
                if index >= 1 && index < self.app_config.script_definitions.len() {
                    self.app_config.script_definitions.swap(index, index - 1);
                    self.scripts.swap(index, index - 1);
                }
            }
            Message::MoveConfigScriptDown(index) => {
                if index < self.app_config.script_definitions.len() - 1 {
                    self.app_config.script_definitions.swap(index, index + 1);
                    self.scripts.swap(index, index + 1);
                }
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let focus = self.focus;
        let total_panes = self.panes.len();

        let pane_grid = responsive(move |size| {
            PaneGrid::new(&self.panes, |id, _pane, is_maximized| {
                let is_focused = focus == Some(id);

                let variant = &self.panes.panes[&id].variant;

                let title = row![get_pane_name_from_variant(variant)].spacing(5);

                let title_bar = pane_grid::TitleBar::new(title)
                    .controls(view_controls(id, variant, total_panes, is_maximized, size))
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
                        &self.app_config.config_read_error,
                        &self.edit_data,
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
            .on_resize(10, Message::Resized)
            .into()
        });

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
    currently_edited_script: &mut Option<EditScriptId>,
    execution_data: &execution::ScriptExecutionData,
    visual_caches: &mut VisualCaches,
    script_idx: usize,
    script_type: EditScriptType,
) {
    *currently_edited_script = Some(EditScriptId{idx: script_idx, script_type});

    // get autorerun count text from value
    visual_caches.autorerun_count = execution_data
        .scripts_to_run
        .get(script_idx)
        .unwrap() // access out of bounds, should never happen, it's OK to crash
        .autorerun_count
        .to_string();
}

fn reset_selected_script(currently_edited_script: &mut Option<EditScriptId>) {
    *currently_edited_script = None;
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaneVariant {
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
    config_read_error: &Option<String>,
    edit_data: &EditData,
) -> Column<'a, Message> {
    let small_button = |label, message| {
        button(
            text(label)
                .horizontal_alignment(alignment::Horizontal::Center)
                .size(16),
        )
        .padding(4)
        .on_press(message)
    };

    if let Some(error) = config_read_error {
        return column![text(format!("Error: {}", error))];
    }

    if script_definitions.is_empty() {
        let config_path = paths.config_path.to_str().unwrap_or_default();

        return column![text(format!(
            "No scripts found in config file \"{}\".\nAdd scripts to the config file and restart the application.",
            &config_path
        )), button("Open config file").on_press(Message::OpenFile(paths.config_path.clone()))];
    }

    let has_started_execution = execution::has_started_execution(&execution_data);

    let data: Element<_> = column(
        script_definitions
            .iter()
            .enumerate()
            .map(|(i, script)| {
                if !has_started_execution {
                    let edit_buttons = if edit_data.window_edit_data.is_some() {
                        row![
                            small_button("^", Message::MoveConfigScriptUp(i)),
                            horizontal_space(5),
                            small_button("v", Message::MoveConfigScriptDown(i)),
                        ]
                    } else {
                        row![]
                    };

                    row![button(if let Some(icon) = &script.icon {
                        row![
                            horizontal_space(6),
                            image(paths.icons_path.join(icon)).width(22).height(22),
                            horizontal_space(6),
                            text(&script.name),
                            horizontal_space(Length::Fill),
                            edit_buttons,
                        ]
                    } else {
                        row![
                            horizontal_space(6),
                            text(&script.name).height(22),
                            horizontal_space(Length::Fill),
                            edit_buttons,
                        ]
                    })
                    .padding(4)
                    .style(theme::Button::Secondary)
                    .on_press(if edit_data.window_edit_data.is_none() {
                        Message::AddScriptToRun(script.clone())
                    } else {
                        Message::OpenScriptConfigEditing
                    })]
                } else {
                    if let Some(icon) = &script.icon {
                        row![
                            horizontal_space(10),
                            image(paths.icons_path.join(icon)).width(22).height(22),
                            horizontal_space(6),
                            text(&script.name)
                        ]
                    } else {
                        row![horizontal_space(10), text(&script.name).height(22)]
                    }
                }
                .into()
            })
            .collect(),
    )
    .spacing(if has_started_execution { 8 } else { 0 })
    .width(Length::Fill)
    .into();

    return if has_started_execution {
        column![
            vertical_space(if has_started_execution { 4 } else { 0 }),
            scrollable(data),
        ]
    } else {
        let edit_button = if edit_data.window_edit_data.is_none() {
            button(
                text("Edit")
                    .width(Length::Fill)
                    .horizontal_alignment(alignment::Horizontal::Center)
                    .size(12),
            )
            .on_press(Message::EnterWindowEditMode)
        } else {
            button(
                text("Stop editing")
                    .width(Length::Fill)
                    .horizontal_alignment(alignment::Horizontal::Center)
                    .size(16),
            )
            .on_press(Message::ExitWindowEditMode)
        };

        column![
            vertical_space(if has_started_execution { 4 } else { 0 }),
            scrollable(data),
            vertical_space(Length::Fill),
            edit_button,
        ]
    }
    .width(Length::Fill)
    .height(Length::Fill)
    .align_items(Alignment::Start);
}

fn produce_execution_list_content<'a>(
    execution_data: &execution::ScriptExecutionData,
    path_caches: &config::PathCaches,
    theme: &Theme,
    custom_title: &Option<String>,
    icons: &IconCaches,
    edit_data: &EditData,
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
            .map(|(i, script)| {
                let script_name = &script.name;

                let script_status = &execution_data.scripts_status[i];

                let repeat_text = if script_status.retry_count > 0 {
                    format!(
                        " [{}/{}]",
                        script_status.retry_count, script.autorerun_count
                    )
                } else {
                    String::new()
                };

                let status;
                let status_tooltip;
                let progress;
                let style = if execution::has_script_failed(script_status) {
                    theme.extended_palette().danger.weak.color
                } else {
                    theme.extended_palette().background.strong.text
                };

                if execution::has_script_finished(script_status) {
                    status = match script_status.result {
                        execution::ScriptResultStatus::Failed => image(icons.failed.clone()),
                        execution::ScriptResultStatus::Success => image(icons.succeeded.clone()),
                        execution::ScriptResultStatus::Skipped => image(icons.skipped.clone()),
                    };
                    status_tooltip = match script_status.result {
                        execution::ScriptResultStatus::Failed => "Failed",
                        execution::ScriptResultStatus::Success => "Success",
                        execution::ScriptResultStatus::Skipped => "Skipped",
                    };
                    if script_status.result != execution::ScriptResultStatus::Skipped {
                        let time_taken_sec = script_status
                            .finish_time
                            .unwrap_or(Instant::now())
                            .duration_since(script_status.start_time.unwrap_or(Instant::now()))
                            .as_secs();
                        progress = text(format!(
                            " ({:02}:{:02}){}",
                            time_taken_sec / 60,
                            time_taken_sec % 60,
                            repeat_text,
                        ))
                        .style(style);
                    } else {
                        progress = text("").style(style);
                    }
                } else if execution::has_script_started(script_status) {
                    let time_taken_sec = Instant::now()
                        .duration_since(script_status.start_time.unwrap_or(Instant::now()))
                        .as_secs();
                    status = image(icons.in_progress.clone());
                    status_tooltip = "In progress";

                    progress = text(format!(
                        " ({:02}:{:02}){}",
                        time_taken_sec / 60,
                        time_taken_sec % 60,
                        repeat_text,
                    ))
                    .style(style);
                } else {
                    status = image(icons.idle.clone());
                    status_tooltip = "Idle";
                    progress = text("").style(style);
                };

                let mut row_data: Vec<Element<'_, Message, iced::Renderer>> = Vec::new();

                if execution_data.has_started {
                    row_data.push(
                        tooltip(
                            status.width(22).height(22).content_fit(ContentFit::None),
                            status_tooltip,
                            tooltip::Position::Right,
                        )
                        .style(theme::Container::Box)
                        .into(),
                    );
                }
                row_data.push(horizontal_space(4).into());
                if let Some(icon) = &script.icon {
                    row_data.push(
                        image(path_caches.icons_path.join(&icon))
                            .width(22)
                            .height(22)
                            .into(),
                    );
                    row_data.push(horizontal_space(4).into());
                }
                row_data.push(text(script_name).style(style).into());
                row_data.push(progress.into());

                let is_enabled = !execution_data.has_started;

                let is_selected = match &edit_data.currently_edited_script {
                    Some(selected_script) => selected_script.idx == i
                        && selected_script.script_type == EditScriptType::ExecutionList,
                    None => false,
                };

                if is_enabled && is_selected {
                    row_data.push(horizontal_space(Length::Fill).into());
                    if i > 0 {
                        row_data.push(
                            small_button("^", Message::MoveScriptUp(i))
                                .style(theme::Button::Primary)
                                .into(),
                        );
                    }
                    if i < execution_data.scripts_to_run.len() - 1 {
                        row_data.push(
                            small_button("v", Message::MoveScriptDown(i))
                                .style(theme::Button::Primary)
                                .into(),
                        );
                    } else {
                        row_data.push(horizontal_space(16).into());
                    }
                    row_data.push(horizontal_space(8).into());
                    row_data.push(
                        small_button("del", Message::RemoveScript(EditScriptId{
                            idx: i,
                            script_type: EditScriptType::ExecutionList,
                        }))
                            .style(theme::Button::Destructive)
                            .into(),
                    );
                } else if execution::has_script_started(&script_status) {
                    row_data.push(horizontal_space(8).into());
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
                        list_item = list_item.on_press(Message::CloseScriptEditing);
                    } else {
                        list_item = list_item.on_press(Message::OpenScriptEditing(i));
                    }

                    list_item = list_item.style(if is_selected {
                        theme::Button::Primary
                    } else {
                        theme::Button::Secondary
                    });

                    list_item.height(30).into()
                } else {
                    row(row_data).height(30).into()
                }
            })
            .collect(),
    )
    .width(Length::Fill)
    .align_items(Alignment::Start)
    .into();

    let controls = column![if edit_data.window_edit_data.is_some() {
        row![]
    } else if execution::has_finished_execution(&execution_data) {
        if !execution::is_waiting_execution_thread_to_finish(&execution_data) {
            row![
                main_button("Reschedule", Message::RescheduleScripts),
                main_button("Clear", Message::ClearScripts),
            ]
            .align_items(Alignment::Center)
            .spacing(5)
        } else {
            row![text("Waiting for the execution to stop")].align_items(Alignment::Center)
        }
    } else if execution::has_started_execution(&execution_data) {
        let current_script = execution_data.currently_outputting_script;
        if current_script != -1
            && execution::has_script_failed(&execution_data.scripts_status[current_script as usize])
        {
            row![text("Waiting for the execution to stop")].align_items(Alignment::Center)
        } else {
            row![main_button("Stop", Message::StopScripts)].align_items(Alignment::Center)
        }
    } else if !execution_data.scripts_to_run.is_empty() {
        row![
            main_button("Run", Message::RunScripts),
            main_button("Clear", Message::ClearScripts),
        ]
        .align_items(Alignment::Center)
        .spacing(5)
    } else {
        row![].into()
    }]
    .spacing(5)
    .width(Length::Fill)
    .align_items(Alignment::Center);

    return column![
        title,
        scrollable(column![data, vertical_space(8), controls])
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .spacing(10)
    .align_items(Alignment::Center);
}

fn produce_log_output_content<'a>(
    execution_data: &execution::ScriptExecutionData,
    theme: &Theme,
) -> Column<'a, Message> {
    if !execution::has_started_execution(&execution_data) {
        return Column::new();
    }

    let mut data_lines: Vec<Element<'_, Message, iced::Renderer>> = Vec::new();
    if let Ok(logs) = execution_data.recent_logs.lock() {
        if !logs.is_empty() {
            data_lines.extend(logs.iter().map(|element| {
                text(format!(
                    "[{}] {}",
                    element.timestamp.format("%H:%M:%S"),
                    element.text
                ))
                .style(match element.output_type {
                    execution::OutputType::StdOut => theme.extended_palette().primary.weak.text,
                    execution::OutputType::StdErr => theme.extended_palette().danger.weak.color,
                    execution::OutputType::Error => theme.extended_palette().danger.weak.color,
                    execution::OutputType::Event => theme.extended_palette().primary.strong.color,
                })
                .into()
            }));
        }
    }

    let data: Element<_> = column(data_lines).spacing(10).width(Length::Fill).into();

    return column![scrollable(data)]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .align_items(Alignment::Start);
}

fn produce_script_edit_content<'a>(
    execution_data: &execution::ScriptExecutionData,
    visual_caches: &VisualCaches,
    edit_data: &EditData,
) -> Column<'a, Message> {
    if execution::has_started_execution(&execution_data) {
        return Column::new();
    }

    let Some(currently_edited_script) = &edit_data.currently_edited_script else {
        return Column::new();
    };

    let button = |label, message| {
        button(
            text(label)
                .vertical_alignment(alignment::Vertical::Center)
                .size(16),
        )
        .padding(4)
        .on_press(message)
    };

    let script = &execution_data.scripts_to_run[currently_edited_script.idx];

    let script_name = text_input("name", &script.name)
        .on_input(move |new_arg| Message::EditScriptName(new_arg))
        .padding(5);

    let arguments = text_input("\"arg1\" \"arg2\"", &script.arguments)
        .on_input(move |new_value| Message::EditArguments(new_value))
        .padding(5);

    let autorerun_count = text_input("0", &visual_caches.autorerun_count)
        .on_input(move |new_value| Message::EditAutorerunCount(new_value))
        .padding(5);

    let ignore_failures_checkbox = checkbox(
        "Ignore previous failures",
        script.ignore_previous_failures,
        move |val| Message::ToggleIgnoreFailures(val),
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
            Message::RemoveScript(currently_edited_script.clone())
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
    config_read_error: &Option<String>,
    edit_data: &EditData,
) -> Element<'a, Message> {
    let content = match variant {
        PaneVariant::ScriptList => produce_script_list_content(
            execution_data,
            script_definitions,
            paths,
            config_read_error,
            edit_data,
        ),
        PaneVariant::ExecutionList => produce_execution_list_content(
            execution_data,
            paths,
            theme,
            custom_title,
            &visual_caches.icons,
            edit_data,
        ),
        PaneVariant::LogOutput => produce_log_output_content(execution_data, theme),
        PaneVariant::ScriptEdit => produce_script_edit_content(execution_data, visual_caches, edit_data),
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
    variant: &PaneVariant,
    total_panes: usize,
    is_maximized: bool,
    size: Size,
) -> Element<'a, Message> {
    let mut row = row![].spacing(5);

    if total_panes > 1 {
        if is_maximized {
            row = add_pane_switch_button(variant, PaneVariant::ScriptList, row);
            row = add_pane_switch_button(variant, PaneVariant::ExecutionList, row);
            row = add_pane_switch_button(variant, PaneVariant::LogOutput, row);
        }
        let toggle = {
            let (content, message) = if is_maximized {
                ("Back to full window", Message::Restore)
            } else {
                // adjust for window decorations
                let window_size = Size {
                    width: size.width + 3.0,
                    height: size.height + 3.0,
                };

                ("Focus", Message::Maximize(pane, window_size))
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
    // but we do it anyway not to miss bugs that create zombie threads
    if let Some(join_handle) = execution_data.thread_join_handle.take() {
        join_handle.join().unwrap(); // have no idea what to do if this fails, crashing is probably fine
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
    focused_variant: &PaneVariant,
    variant: PaneVariant,
    row: iced::widget::Row<'a, Message, iced::Renderer>,
) -> iced::widget::Row<'a, Message, iced::Renderer> {
    if *focused_variant == variant {
        return row;
    }

    let pane_name = get_pane_name_from_variant(&variant);
    row.push(
        button(text(format!("{}", pane_name)).size(14))
            .style(theme::Button::Secondary)
            .padding(3)
            .on_press(Message::SwitchMaximized(variant)),
    )
}

fn apply_script_edit(app: &mut MainWindow, edit_fn: impl Fn(&mut config::ScriptDefinition)) {
    if let Some(script) = &app.edit_data.currently_edited_script {
        match script.script_type {
            EditScriptType::ScriptConfig => {
                edit_fn(&mut app.app_config.script_definitions[script.idx]);
                edit_fn(&mut app.scripts[script.idx]);
            }
            EditScriptType::ExecutionList => {
                edit_fn(&mut app.execution_data.scripts_to_run[script.idx]);
            }
        }
    }
}
