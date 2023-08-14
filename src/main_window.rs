#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

use iced::alignment::{self, Alignment};
use iced::theme::{self, Theme};
use iced::widget::pane_grid::{self, Configuration, PaneGrid};
use iced::widget::{
    button, checkbox, column, container, horizontal_space, image, image::Handle, row, scrollable,
    text, text_input, tooltip, vertical_space, Button, Column,
};
use iced::window::{request_user_attention, resize};
use iced::{executor, ContentFit};
use iced::{time, Size};
use iced::{Application, Command, Element, Length, Subscription};
use iced_lazy::responsive;
use std::mem::swap;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, Instant};

use crate::config;
use crate::execution;
use crate::style;

const ONE_EXECUTION_LIST_ELEMENT_HEIGHT: u32 = 30;
const ONE_TITLE_LINE_HEIGHT: u32 = 16;
const EMPTY_EXECUTION_LIST_HEIGHT: u32 = 150;

#[derive(Clone)]
struct ThemedIcons {
    play: Handle,
    stop: Handle,
    retry: Handle,
    remove: Handle,
    plus: Handle,
    settings: Handle,
    up: Handle,
    down: Handle,
    back: Handle,
    log: Handle,
    edit: Handle,
}

struct IconCaches {
    idle: Handle,
    in_progress: Handle,
    succeeded: Handle,
    failed: Handle,
    skipped: Handle,

    bright: ThemedIcons,
    dark: ThemedIcons,

    themed: ThemedIcons,
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
    // do we have unsaved changes
    is_dirty: bool,
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

#[derive(Debug, Clone, PartialEq)]
enum ConfigEditType {
    Child,
    Parent,
}

#[derive(Debug, Clone)]
struct WindowEditData {
    is_editing_config: bool,
    edit_type: ConfigEditType,

    // theme color temp strings
    theme_color_background: String,
    theme_color_text: String,
    theme_color_primary: String,
    theme_color_success: String,
    theme_color_danger: String,
}

impl WindowEditData {
    fn from_config(
        config: &config::AppConfig,
        is_editing_config: bool,
        edit_type: ConfigEditType,
    ) -> Self {
        let theme = if let Some(theme) = &get_rewritable_config(&config, &edit_type).custom_theme {
            theme.clone()
        } else {
            config::CustomTheme::default()
        };

        Self {
            is_editing_config,
            edit_type,
            theme_color_background: rgb_to_hex(&theme.background),
            theme_color_text: rgb_to_hex(&theme.text),
            theme_color_primary: rgb_to_hex(&theme.primary),
            theme_color_success: rgb_to_hex(&theme.success),
            theme_color_danger: rgb_to_hex(&theme.danger),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    Clicked(pane_grid::Pane),
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
    Maximize(pane_grid::Pane, Size),
    Restore,
    AddScriptToRun(config::ScriptDefinition),
    RunScripts,
    StopScripts,
    ClearScripts,
    RescheduleScripts,
    Tick(Instant),
    OpenScriptEditing(usize),
    CloseScriptEditing,
    DuplicateScript(EditScriptId),
    RemoveScript(EditScriptId),
    AddScriptToConfig,
    MoveScriptUp(usize),
    MoveScriptDown(usize),
    EditScriptName(String),
    EditScriptCommand(String),
    ToggleScriptCommandRelativeToScripter(bool),
    EditScriptIconPath(String),
    ToggleScriptIconPathRelativeToScripter(bool),
    EditArguments(String),
    ToggleRequiresArguments(bool),
    EditAutorerunCount(String),
    OpenFile(PathBuf),
    ToggleIgnoreFailures(bool),
    EnterWindowEditMode,
    ExitWindowEditMode,
    SaveConfig,
    RevertConfig,
    OpenScriptConfigEditing(usize),
    MoveConfigScriptUp(usize),
    MoveConfigScriptDown(usize),
    ToggleConfigEditing,
    ConfigToggleAlwaysOnTop(bool),
    ConfigToggleWindowStatusReactions(bool),
    ConfigToggleKeepWindowSize(bool),
    ConfigToggleUseCustomTheme(bool),
    ConfigEditThemeBackground(String),
    ConfigEditThemeText(String),
    ConfigEditThemePrimary(String),
    ConfigEditThemeSuccess(String),
    ConfigEditThemeDanger(String),
    ConfigEditChildConfigPath(String),
    ConfigToggleChildConfigPathRelativeToScripter(bool),
    SwitchToParentConfig,
    SwitchToChildConfig,
    ToggleScriptHidden(bool),
    CreateCopyOfParentScript(EditScriptId),
    MoveToParent(EditScriptId),
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
                b: Box::new(Configuration::Pane(AppPane::new(PaneVariant::Parameters))),
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

        let mut result = (
            MainWindow {
                panes,
                focus: None,
                execution_data: execution::new_execution_data(),
                theme: get_theme(&app_config, &None),
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

                        bright: ThemedIcons {
                            play: Handle::from_memory(include_bytes!("../res/icons/play-w.png")),
                            stop: Handle::from_memory(include_bytes!("../res/icons/stop-w.png")),
                            retry: Handle::from_memory(include_bytes!("../res/icons/retry-w.png")),
                            remove: Handle::from_memory(include_bytes!(
                                "../res/icons/remove-w.png"
                            )),
                            plus: Handle::from_memory(include_bytes!("../res/icons/plus-w.png")),
                            settings: Handle::from_memory(include_bytes!(
                                "../res/icons/settings-w.png"
                            )),
                            up: Handle::from_memory(include_bytes!("../res/icons/up-w.png")),
                            down: Handle::from_memory(include_bytes!("../res/icons/down-w.png")),
                            back: Handle::from_memory(include_bytes!("../res/icons/back-w.png")),
                            log: Handle::from_memory(include_bytes!("../res/icons/log-w.png")),
                            edit: Handle::from_memory(include_bytes!("../res/icons/edit-w.png")),
                        },
                        dark: ThemedIcons {
                            play: Handle::from_memory(include_bytes!("../res/icons/play-b.png")),
                            stop: Handle::from_memory(include_bytes!("../res/icons/stop-b.png")),
                            retry: Handle::from_memory(include_bytes!("../res/icons/retry-b.png")),
                            remove: Handle::from_memory(include_bytes!(
                                "../res/icons/remove-b.png"
                            )),
                            plus: Handle::from_memory(include_bytes!("../res/icons/plus-b.png")),
                            settings: Handle::from_memory(include_bytes!(
                                "../res/icons/settings-b.png"
                            )),
                            up: Handle::from_memory(include_bytes!("../res/icons/up-b.png")),
                            down: Handle::from_memory(include_bytes!("../res/icons/down-b.png")),
                            back: Handle::from_memory(include_bytes!("../res/icons/back-b.png")),
                            log: Handle::from_memory(include_bytes!("../res/icons/log-b.png")),
                            edit: Handle::from_memory(include_bytes!("../res/icons/edit-b.png")),
                        },

                        themed: ThemedIcons {
                            play: Handle::from_memory(include_bytes!("../res/icons/play-b.png")),
                            stop: Handle::from_memory(include_bytes!("../res/icons/stop-b.png")),
                            retry: Handle::from_memory(include_bytes!("../res/icons/retry-b.png")),
                            remove: Handle::from_memory(include_bytes!(
                                "../res/icons/remove-b.png"
                            )),
                            plus: Handle::from_memory(include_bytes!("../res/icons/plus-b.png")),
                            settings: Handle::from_memory(include_bytes!(
                                "../res/icons/settings-b.png"
                            )),
                            up: Handle::from_memory(include_bytes!("../res/icons/up-b.png")),
                            down: Handle::from_memory(include_bytes!("../res/icons/down-b.png")),
                            back: Handle::from_memory(include_bytes!("../res/icons/back-b.png")),
                            log: Handle::from_memory(include_bytes!("../res/icons/log-b.png")),
                            edit: Handle::from_memory(include_bytes!("../res/icons/edit-b.png")),
                        },
                    },
                },
                full_window_size: Size::new(0.0, 0.0),
                edit_data: EditData {
                    window_edit_data: None,
                    currently_edited_script: None,
                    is_dirty: false,
                },
            },
            Command::none(),
        );

        update_theme_icons(&mut result.0);

        return result;
    }

    fn title(&self) -> String {
        if let Some(window_edit_data) = &self.edit_data.window_edit_data {
            match window_edit_data.edit_type {
                ConfigEditType::Parent if self.app_config.child_config_body.is_some() => {
                    "scripter [Editing shared config]".to_string()
                }
                _ => "scripter [Editing]".to_string(),
            }
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
            if self.edit_data.is_dirty {
                "scripter [Unsaved changes]".to_string()
            } else {
                "scripter".to_string()
            }
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
                if !get_rewritable_config_opt(&self.app_config, &self.edit_data.window_edit_data)
                    .keep_window_size
                {
                    self.full_window_size = window_size.clone();
                    let size = self
                        .panes
                        .layout()
                        .pane_regions(1.0, Size::new(window_size.width, window_size.height))
                        .get(&pane)
                        .unwrap() // tried to get an non-existing pane, this should never happen, so panic
                        .clone();

                    let elements_count = self.execution_data.scripts_to_run.len() as u32;
                    let title_lines =
                        if let Some(custom_title) = self.app_config.custom_title.as_ref() {
                            custom_title.lines().count() as u32
                        } else {
                            0
                        };

                    return resize(
                        size.width as u32,
                        std::cmp::min(
                            size.height as u32,
                            EMPTY_EXECUTION_LIST_HEIGHT
                                + elements_count * ONE_EXECUTION_LIST_ELEMENT_HEIGHT
                                + title_lines * ONE_TITLE_LINE_HEIGHT,
                        ),
                    );
                }
            }
            Message::Restore => {
                self.panes.restore();
                if !get_rewritable_config_opt(&self.app_config, &self.edit_data.window_edit_data)
                    .keep_window_size
                {
                    return resize(
                        self.full_window_size.width as u32,
                        self.full_window_size.height as u32,
                    );
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
                    &get_script_definition_list_opt(
                        &self.app_config,
                        &self.edit_data.window_edit_data,
                    ),
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
                            if get_rewritable_config_opt(
                                &self.app_config,
                                &self.edit_data.window_edit_data,
                            )
                            .window_status_reactions
                            {
                                return request_user_attention(Some(
                                    iced::window::UserAttention::Informational,
                                ));
                            }
                        }
                    }
                }
            }
            Message::OpenScriptEditing(script_idx) => {
                set_selected_script(
                    &mut self.edit_data.currently_edited_script,
                    &self.execution_data,
                    &get_script_definition_list_opt(
                        &self.app_config,
                        &self.edit_data.window_edit_data,
                    ),
                    &mut self.visual_caches,
                    script_idx,
                    EditScriptType::ExecutionList,
                );
            }
            Message::CloseScriptEditing => {
                reset_selected_script(&mut self.edit_data.currently_edited_script);
            }
            Message::DuplicateScript(script_id) => {
                let init_duplicated_script =
                    |script: config::ScriptDefinition| config::ScriptDefinition {
                        uid: config::Guid::new(),
                        name: format!("{} (copy)", script.name),
                        ..script
                    };

                match script_id.script_type {
                    EditScriptType::ScriptConfig => match &self.edit_data.window_edit_data {
                        Some(WindowEditData {
                            edit_type: ConfigEditType::Child,
                            ..
                        }) => {
                            if let Some(config) = self.app_config.child_config_body.as_mut() {
                                match &config.script_definitions[script_id.idx] {
                                    config::ChildScriptDefinition::Parent(_, _) => {}
                                    config::ChildScriptDefinition::Added(script) => {
                                        config.script_definitions.insert(
                                            script_id.idx + 1,
                                            config::ChildScriptDefinition::Added(
                                                init_duplicated_script(script.clone()),
                                            ),
                                        );
                                    }
                                }
                            }
                            config::update_child_config_script_cache_from_config(
                                &mut self.app_config,
                            );
                        }
                        _ => {
                            self.app_config.script_definitions.insert(
                                script_id.idx + 1,
                                init_duplicated_script(
                                    self.app_config.script_definitions[script_id.idx].clone(),
                                ),
                            );
                        }
                    },
                    EditScriptType::ExecutionList => self.execution_data.scripts_to_run.insert(
                        script_id.idx + 1,
                        init_duplicated_script(
                            self.execution_data.scripts_to_run[script_id.idx].clone(),
                        ),
                    ),
                };
                if let Some(script) = &mut self.edit_data.currently_edited_script {
                    script.idx = script_id.idx + 1;
                    script.script_type = script_id.script_type;
                }
            }
            Message::RemoveScript(script_id) => {
                match script_id.script_type {
                    EditScriptType::ScriptConfig => {
                        if let Some(window_edit_data) = &mut self.edit_data.window_edit_data {
                            match window_edit_data.edit_type {
                                ConfigEditType::Parent => {
                                    self.app_config.script_definitions.remove(script_id.idx);
                                    self.edit_data.is_dirty = true;
                                }
                                ConfigEditType::Child => {
                                    if let Some(config) = &mut self.app_config.child_config_body {
                                        config.script_definitions.remove(script_id.idx);
                                        self.edit_data.is_dirty = true;
                                    }
                                }
                            }
                        }

                        config::populate_parent_scripts_from_config(&mut self.app_config)
                    }
                    EditScriptType::ExecutionList => {
                        execution::remove_script_from_execution(
                            &mut self.execution_data,
                            script_id.idx,
                        );
                    }
                }
                reset_selected_script(&mut self.edit_data.currently_edited_script);
            }
            Message::AddScriptToConfig => {
                let script = config::ScriptDefinition {
                    uid: config::Guid::new(),
                    name: "new script".to_string(),
                    icon: config::PathConfig::default(),
                    command: config::PathConfig::default(),
                    arguments: "".to_string(),
                    autorerun_count: 0,
                    ignore_previous_failures: false,
                    requires_arguments: false,
                    is_read_only: false,
                    is_hidden: false,
                };
                if let Some(window_edit_data) = &mut self.edit_data.window_edit_data {
                    let script_idx = match window_edit_data.edit_type {
                        ConfigEditType::Parent => {
                            Some(add_script_to_parent_config(&mut self.app_config, script))
                        }
                        ConfigEditType::Child => {
                            add_script_to_child_config(&mut self.app_config, script)
                        }
                    };

                    window_edit_data.is_editing_config = false;

                    if let Some(script_idx) = script_idx {
                        set_selected_script(
                            &mut self.edit_data.currently_edited_script,
                            &self.execution_data,
                            &get_script_definition_list_opt(
                                &self.app_config,
                                &self.edit_data.window_edit_data,
                            ),
                            &mut self.visual_caches,
                            script_idx,
                            EditScriptType::ScriptConfig,
                        );
                        self.edit_data.is_dirty = true;
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
                    &get_script_definition_list_opt(
                        &self.app_config,
                        &self.edit_data.window_edit_data,
                    ),
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
                    &get_script_definition_list_opt(
                        &self.app_config,
                        &self.edit_data.window_edit_data,
                    ),
                    &mut self.visual_caches,
                    script_idx + 1,
                    EditScriptType::ExecutionList,
                );
            }
            Message::EditScriptName(new_name) => {
                apply_script_edit(self, move |script| script.name = new_name)
            }
            Message::EditScriptCommand(new_command) => {
                apply_script_edit(self, move |script| script.command.path = new_command)
            }
            Message::ToggleScriptCommandRelativeToScripter(value) => {
                apply_script_edit(self, |script| {
                    script.command.path_type = if value {
                        config::PathType::ScripterExecutableRelative
                    } else {
                        config::PathType::WorkingDirRelative
                    }
                })
            }
            Message::EditScriptIconPath(new_icon_path) => {
                apply_script_edit(self, move |script| script.icon.path = new_icon_path)
            }
            Message::ToggleScriptIconPathRelativeToScripter(new_relative) => {
                apply_script_edit(self, move |script| {
                    script.icon.path_type = if new_relative {
                        config::PathType::ScripterExecutableRelative
                    } else {
                        config::PathType::WorkingDirRelative
                    }
                })
            }
            Message::EditArguments(new_arguments) => {
                apply_script_edit(self, move |script| script.arguments = new_arguments)
            }
            Message::ToggleRequiresArguments(new_requires_arguments) => {
                apply_script_edit(self, move |script| {
                    script.requires_arguments = new_requires_arguments
                })
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
                    apply_script_edit(self, |script| script.autorerun_count = new_autorerun_count)
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
                #[cfg(target_os = "macos")]
                {
                    let result = std::process::Command::new("open")
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
                apply_script_edit(self, |script| script.ignore_previous_failures = value)
            }
            Message::EnterWindowEditMode => {
                self.edit_data.window_edit_data = Some(WindowEditData::from_config(
                    &self.app_config,
                    false,
                    if self.app_config.child_config_body.is_some() {
                        ConfigEditType::Child
                    } else {
                        ConfigEditType::Parent
                    },
                ));
                reset_selected_script(&mut self.edit_data.currently_edited_script);
            }
            Message::ExitWindowEditMode => {
                self.edit_data.window_edit_data = None;
                reset_selected_script(&mut self.edit_data.currently_edited_script);
                apply_theme(self);
            }
            Message::SaveConfig => {
                config::save_config_to_file(&self.app_config);
                self.app_config = config::read_config();
                self.edit_data.is_dirty = false;
            }
            Message::RevertConfig => {
                self.app_config = config::read_config();
                self.edit_data.window_edit_data = Some(WindowEditData::from_config(
                    &self.app_config,
                    false,
                    match self.edit_data.window_edit_data {
                        Some(WindowEditData {
                            edit_type: ConfigEditType::Child,
                            ..
                        }) => ConfigEditType::Child,
                        _ => ConfigEditType::Parent,
                    },
                ));
                config::populate_parent_scripts_from_config(&mut self.app_config);
                apply_theme(self);
                self.edit_data.is_dirty = false;
                reset_selected_script(&mut self.edit_data.currently_edited_script);
            }
            Message::OpenScriptConfigEditing(script_idx) => {
                set_selected_script(
                    &mut self.edit_data.currently_edited_script,
                    &self.execution_data,
                    &get_script_definition_list_opt(
                        &self.app_config,
                        &self.edit_data.window_edit_data,
                    ),
                    &mut self.visual_caches,
                    script_idx,
                    EditScriptType::ScriptConfig,
                );
                if let Some(window_edit_data) = &mut self.edit_data.window_edit_data {
                    window_edit_data.is_editing_config = false;
                }
            }
            Message::MoveConfigScriptUp(index) => {
                if let Some(window_edit_data) = &mut self.edit_data.window_edit_data {
                    match window_edit_data.edit_type {
                        ConfigEditType::Parent => {
                            if index >= 1 && index < self.app_config.script_definitions.len() {
                                self.app_config.script_definitions.swap(index, index - 1);
                                self.edit_data.is_dirty = true;
                            }
                        }
                        ConfigEditType::Child => {
                            if let Some(child_config_body) = &mut self.app_config.child_config_body
                            {
                                if index >= 1 && index < child_config_body.script_definitions.len()
                                {
                                    child_config_body.script_definitions.swap(index, index - 1);
                                    config::update_child_config_script_cache_from_config(
                                        &mut self.app_config,
                                    );
                                    self.edit_data.is_dirty = true;
                                }
                            }
                        }
                    }
                }
            }
            Message::MoveConfigScriptDown(index) => {
                if let Some(window_edit_data) = &mut self.edit_data.window_edit_data {
                    match window_edit_data.edit_type {
                        ConfigEditType::Parent => {
                            if index < self.app_config.script_definitions.len() - 1 {
                                self.app_config.script_definitions.swap(index, index + 1);
                                self.edit_data.is_dirty = true;
                            }
                        }
                        ConfigEditType::Child => {
                            if let Some(child_config_body) = &mut self.app_config.child_config_body
                            {
                                if index < child_config_body.script_definitions.len() - 1 {
                                    child_config_body.script_definitions.swap(index, index + 1);
                                    config::update_child_config_script_cache_from_config(
                                        &mut self.app_config,
                                    );
                                    self.edit_data.is_dirty = true;
                                }
                            }
                        }
                    }
                }
            }
            Message::ToggleConfigEditing => {
                match &mut self.edit_data.window_edit_data {
                    Some(window_edit_data) => {
                        window_edit_data.is_editing_config = !window_edit_data.is_editing_config;
                    }
                    None => {
                        self.edit_data.window_edit_data = Some(WindowEditData::from_config(
                            &self.app_config,
                            true,
                            if self.app_config.child_config_body.is_some() {
                                ConfigEditType::Child
                            } else {
                                ConfigEditType::Parent
                            },
                        ));
                    }
                };
                reset_selected_script(&mut self.edit_data.currently_edited_script);
            }
            Message::ConfigToggleAlwaysOnTop(is_checked) => {
                get_rewritable_config_mut(&mut self.app_config, &self.edit_data.window_edit_data)
                    .always_on_top = is_checked;
                self.edit_data.is_dirty = true;
            }
            Message::ConfigToggleWindowStatusReactions(is_checked) => {
                get_rewritable_config_mut(&mut self.app_config, &self.edit_data.window_edit_data)
                    .window_status_reactions = is_checked;
                self.edit_data.is_dirty = true;
            }
            Message::ConfigToggleKeepWindowSize(is_checked) => {
                get_rewritable_config_mut(&mut self.app_config, &self.edit_data.window_edit_data)
                    .keep_window_size = is_checked;
                self.edit_data.is_dirty = true;
            }
            Message::ConfigToggleUseCustomTheme(is_checked) => {
                get_rewritable_config_mut(&mut self.app_config, &self.edit_data.window_edit_data)
                    .custom_theme = if is_checked {
                    Some(
                        if let Some(window_edit_data) = &self.edit_data.window_edit_data {
                            config::CustomTheme {
                                background: hex_to_rgb(&window_edit_data.theme_color_background)
                                    .unwrap_or_default(),
                                text: hex_to_rgb(&window_edit_data.theme_color_text)
                                    .unwrap_or_default(),
                                primary: hex_to_rgb(&window_edit_data.theme_color_primary)
                                    .unwrap_or_default(),
                                success: hex_to_rgb(&window_edit_data.theme_color_success)
                                    .unwrap_or_default(),
                                danger: hex_to_rgb(&window_edit_data.theme_color_danger)
                                    .unwrap_or_default(),
                            }
                        } else {
                            config::CustomTheme::default()
                        },
                    )
                } else {
                    None
                };
                apply_theme(self);
                self.edit_data.is_dirty = true;
            }
            Message::ConfigEditThemeBackground(new_value) => {
                apply_theme_color_from_string(
                    self,
                    new_value,
                    |theme, value| theme.background = value,
                    |edit_data, value| {
                        edit_data.theme_color_background = value;
                        edit_data.theme_color_background.clone()
                    },
                );
            }
            Message::ConfigEditThemeText(new_value) => {
                apply_theme_color_from_string(
                    self,
                    new_value,
                    |theme, value| theme.text = value,
                    |edit_data, value| {
                        edit_data.theme_color_text = value;
                        edit_data.theme_color_text.clone()
                    },
                );
            }
            Message::ConfigEditThemePrimary(new_value) => {
                apply_theme_color_from_string(
                    self,
                    new_value,
                    |theme, value| theme.primary = value,
                    |edit_data, value| {
                        edit_data.theme_color_primary = value;
                        edit_data.theme_color_primary.clone()
                    },
                );
            }
            Message::ConfigEditThemeSuccess(new_value) => {
                apply_theme_color_from_string(
                    self,
                    new_value,
                    |theme, value| theme.success = value,
                    |edit_data, value| {
                        edit_data.theme_color_success = value;
                        edit_data.theme_color_success.clone()
                    },
                );
            }
            Message::ConfigEditThemeDanger(new_value) => {
                apply_theme_color_from_string(
                    self,
                    new_value,
                    |theme, value| theme.danger = value,
                    |edit_data, value| {
                        edit_data.theme_color_danger = value;
                        edit_data.theme_color_danger.clone()
                    },
                );
                self.edit_data.is_dirty = true;
            }
            Message::ConfigEditChildConfigPath(new_value) => {
                self.app_config.child_config_path.path = new_value;
                self.edit_data.is_dirty = true;
            }
            Message::ConfigToggleChildConfigPathRelativeToScripter(is_checked) => {
                self.app_config.child_config_path.path_type = if is_checked {
                    config::PathType::ScripterExecutableRelative
                } else {
                    config::PathType::WorkingDirRelative
                };
                self.edit_data.is_dirty = true;
            }
            Message::SwitchToParentConfig => {
                reset_selected_script(&mut self.edit_data.currently_edited_script);
                switch_config_edit_mode(self, ConfigEditType::Parent);
                apply_theme(self);
            }
            Message::SwitchToChildConfig => {
                reset_selected_script(&mut self.edit_data.currently_edited_script);
                switch_config_edit_mode(self, ConfigEditType::Child);
                apply_theme(self);
            }
            Message::ToggleScriptHidden(is_hidden) => {
                let Some(script_id) = &mut self.edit_data.currently_edited_script else {
                    return Command::none();
                };

                if let Some(config) = &mut self.app_config.child_config_body {
                    let Some(script) = config.script_definitions.get_mut(script_id.idx) else {
                        return Command::none();
                    };

                    match script {
                        config::ChildScriptDefinition::Parent(_, is_hidden_value) => {
                            *is_hidden_value = is_hidden;
                            self.edit_data.is_dirty = true;
                        }
                        config::ChildScriptDefinition::Added(_) => {}
                    }
                }
                config::update_child_config_script_cache_from_config(&mut self.app_config);
            }
            Message::CreateCopyOfParentScript(script_id) => {
                let script = if let Some(config) = &self.app_config.child_config_body {
                    if let Some(script) = config.script_definitions.get(script_id.idx) {
                        script
                    } else {
                        return Command::none();
                    }
                } else {
                    return Command::none();
                };

                let new_script = match script {
                    config::ChildScriptDefinition::Parent(parent_script_id, _is_hidden) => {
                        let Some(script) = self.app_config.script_definitions.iter().find_map(|script| {
                            if script.uid == *parent_script_id {
                                Some(script.clone())
                            } else {
                                None
                            }
                        }) else { return Command::none(); };
                        script
                    }
                    config::ChildScriptDefinition::Added(_) => {
                        return Command::none();
                    }
                };

                if let Some(config) = &mut self.app_config.child_config_body {
                    config.script_definitions.insert(
                        script_id.idx + 1,
                        config::ChildScriptDefinition::Added(new_script),
                    );
                    config::update_child_config_script_cache_from_config(&mut self.app_config);
                    set_selected_script(
                        &mut self.edit_data.currently_edited_script,
                        &self.execution_data,
                        &get_script_definition_list_opt(
                            &self.app_config,
                            &self.edit_data.window_edit_data,
                        ),
                        &mut self.visual_caches,
                        script_id.idx + 1,
                        EditScriptType::ScriptConfig,
                    );
                    self.edit_data.is_dirty = true;
                }
            }
            Message::MoveToParent(script_id) => {
                if let Some(config) = &mut self.app_config.child_config_body {
                    if config.script_definitions.len() <= script_id.idx {
                        return Command::none();
                    }

                    if let Some(script) = config.script_definitions.get_mut(script_id.idx) {
                        match script {
                            config::ChildScriptDefinition::Added(definition) => {
                                let mut replacement_script = config::ChildScriptDefinition::Parent(
                                    definition.uid.clone(),
                                    false,
                                );
                                swap(script, &mut replacement_script);
                                match replacement_script {
                                    config::ChildScriptDefinition::Added(original_definition) => {
                                        self.app_config
                                            .script_definitions
                                            .push(original_definition);
                                    }
                                    _ => {}
                                }
                                config::update_child_config_script_cache_from_config(
                                    &mut self.app_config,
                                );
                                if let Some(edit_data) = &mut self.edit_data.window_edit_data {
                                    edit_data.edit_type = ConfigEditType::Parent;
                                }
                                set_selected_script(
                                    &mut self.edit_data.currently_edited_script,
                                    &self.execution_data,
                                    &get_script_definition_list_opt(
                                        &self.app_config,
                                        &self.edit_data.window_edit_data,
                                    ),
                                    &mut self.visual_caches,
                                    self.app_config.script_definitions.len() - 1,
                                    EditScriptType::ScriptConfig,
                                );
                                self.edit_data.is_dirty = true;
                            }
                            _ => {}
                        }
                    }
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
                    .controls(view_controls(
                        id,
                        variant,
                        total_panes,
                        &self.visual_caches.icons,
                        &self.edit_data,
                        &self.execution_data,
                        is_maximized,
                        size,
                    ))
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
                        &self.theme,
                        &self.app_config.paths,
                        &self.visual_caches,
                        &self.app_config,
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
    script_definitions: &Vec<config::ScriptDefinition>,
    visual_caches: &mut VisualCaches,
    script_idx: usize,
    script_type: EditScriptType,
) {
    *currently_edited_script = Some(EditScriptId {
        idx: script_idx,
        script_type: script_type.clone(),
    });

    let scripts_list = match &script_type {
        EditScriptType::ScriptConfig => script_definitions,
        EditScriptType::ExecutionList => &execution_data.scripts_to_run,
    };

    // get autorerun count text from value
    if let Some(script) = &scripts_list.get(script_idx) {
        visual_caches.autorerun_count = script.autorerun_count.to_string();
    }
}

fn reset_selected_script(currently_edited_script: &mut Option<EditScriptId>) {
    *currently_edited_script = None;
}

#[derive(Debug, Clone, PartialEq)]
pub enum PaneVariant {
    ScriptList,
    ExecutionList,
    LogOutput,
    Parameters,
}

struct AppPane {
    variant: PaneVariant,
}

impl AppPane {
    fn new(variant: PaneVariant) -> Self {
        Self { variant }
    }
}

fn inline_icon_button<'a, Message>(icon_handle: Handle, message: Message) -> Button<'a, Message> {
    button(
        image(icon_handle)
            .width(Length::Fixed(14.0))
            .height(Length::Fixed(14.0)),
    )
    .padding(4)
    .on_press(message)
}

fn main_icon_button(icon_handle: Handle, label: &str, message: Option<Message>) -> Button<Message> {
    let new_button = button(row![
        image(icon_handle)
            .width(Length::Fixed(16.0))
            .height(Length::Fixed(16.0)),
        horizontal_space(4),
        text(label).width(Length::Shrink).size(16),
    ])
    .width(Length::Shrink)
    .padding(8);

    if let Some(message) = message {
        new_button.on_press(message)
    } else {
        new_button
    }
}

fn edit_mode_button<'a>(
    icon_handle: Handle,
    message: Message,
    is_dirty: bool,
) -> Button<'a, Message> {
    button(row![image(icon_handle)
        .width(Length::Fixed(12.0))
        .height(Length::Fixed(12.0))])
    .style(if is_dirty {
        theme::Button::Positive
    } else {
        theme::Button::Secondary
    })
    .width(Length::Shrink)
    .padding(4)
    .on_press(message)
}

fn produce_script_list_content<'a>(
    execution_data: &execution::ScriptExecutionData,
    config: &config::AppConfig,
    paths: &config::PathCaches,
    edit_data: &EditData,
    icons: &IconCaches,
) -> Column<'a, Message> {
    if let Some(error) = &config.config_read_error {
        return column![text(format!("Error: {}", error))];
    }

    let has_started_execution = execution::has_started_execution(&execution_data);

    let data: Element<_> = column(
        get_script_definition_list_opt(&config, &edit_data.window_edit_data)
            .iter()
            .filter(|script| !script.is_hidden || edit_data.window_edit_data.is_some())
            .enumerate()
            .map(|(i, script)| {
                if !has_started_execution {
                    let mut name_text = script.name.clone();

                    if is_local_edited_script(i, &config, &edit_data.window_edit_data) {
                        name_text += " [local]";
                    }
                    if script.is_hidden {
                        name_text += " [hidden]";
                    }

                    let edit_buttons = if edit_data.window_edit_data.is_some() {
                        row![
                            inline_icon_button(
                                icons.themed.up.clone(),
                                Message::MoveConfigScriptUp(i)
                            ),
                            horizontal_space(5),
                            inline_icon_button(
                                icons.themed.down.clone(),
                                Message::MoveConfigScriptDown(i)
                            ),
                            horizontal_space(5),
                        ]
                    } else {
                        row![]
                    };

                    let is_selected = match &edit_data.currently_edited_script {
                        Some(EditScriptId { idx, script_type })
                            if *idx == i && *script_type == EditScriptType::ScriptConfig =>
                        {
                            true
                        }
                        _ => false,
                    };

                    let item_button = button(if !script.icon.path.is_empty() {
                        row![
                            horizontal_space(6),
                            image(config::get_full_path(paths, &script.icon))
                                .width(22)
                                .height(22),
                            horizontal_space(6),
                            text(&name_text),
                            horizontal_space(Length::Fill),
                            edit_buttons,
                        ]
                    } else {
                        row![
                            horizontal_space(6),
                            text(&name_text).height(22),
                            horizontal_space(Length::Fill),
                            edit_buttons,
                        ]
                    })
                    .padding(4)
                    .style(if is_selected {
                        theme::Button::Primary
                    } else {
                        theme::Button::Secondary
                    })
                    .on_press(if edit_data.window_edit_data.is_none() {
                        Message::AddScriptToRun(script.clone())
                    } else {
                        Message::OpenScriptConfigEditing(i)
                    });

                    row![item_button]
                } else {
                    if !script.icon.path.is_empty() {
                        row![
                            horizontal_space(10),
                            image(config::get_full_path(paths, &script.icon))
                                .width(22)
                                .height(22),
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
        let data_column = if let Some(window_edit_data) = &edit_data.window_edit_data {
            column![
                data,
                vertical_space(Length::Fixed(4.0)),
                row![
                    main_icon_button(
                        icons.themed.plus.clone(),
                        "Add script",
                        Some(Message::AddScriptToConfig)
                    ),
                    horizontal_space(Length::Fixed(4.0)),
                    main_icon_button(
                        icons.themed.settings.clone(),
                        "Settings",
                        Some(Message::ToggleConfigEditing)
                    ),
                ],
                if config.child_config_body.is_some() {
                    match window_edit_data.edit_type {
                        ConfigEditType::Child => {
                            column![
                                vertical_space(Length::Fixed(4.0)),
                                button(text("Edit shared config").size(16))
                                    .on_press(Message::SwitchToParentConfig)
                            ]
                        }
                        ConfigEditType::Parent => {
                            column![
                                vertical_space(Length::Fixed(4.0)),
                                button(text("Edit local config").size(16))
                                    .on_press(Message::SwitchToChildConfig)
                            ]
                        }
                    }
                } else {
                    column![]
                },
                if edit_data.is_dirty {
                    column![
                        vertical_space(Length::Fixed(4.0)),
                        row![
                            main_icon_button(
                                icons.themed.back.clone(),
                                "Exit editing mode",
                                Some(Message::ExitWindowEditMode)
                            ),
                            horizontal_space(Length::Fixed(4.0)),
                            button(text("Save").size(16))
                                .style(theme::Button::Positive)
                                .on_press(Message::SaveConfig),
                            horizontal_space(Length::Fixed(4.0)),
                            button(text("Revert").size(16))
                                .style(theme::Button::Destructive)
                                .on_press(Message::RevertConfig),
                        ]
                    ]
                } else {
                    column![
                        vertical_space(Length::Fixed(4.0)),
                        main_icon_button(
                            icons.themed.back.clone(),
                            "Exit editing mode",
                            Some(Message::ExitWindowEditMode)
                        ),
                    ]
                }
            ]
        } else {
            column![data]
        };

        column![
            vertical_space(if has_started_execution { 4 } else { 0 }),
            scrollable(data_column),
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

                let is_selected = match &edit_data.currently_edited_script {
                    Some(selected_script) => {
                        selected_script.idx == i
                            && selected_script.script_type == EditScriptType::ExecutionList
                    }
                    None => false,
                };

                let status;
                let status_tooltip;
                let progress;
                let style = if execution::has_script_failed(script_status) {
                    theme.extended_palette().danger.weak.color
                } else {
                    if is_selected {
                        theme.extended_palette().primary.strong.text
                    } else {
                        theme.extended_palette().background.strong.text
                    }
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
                if !script.icon.path.is_empty() {
                    row_data.push(
                        image(config::get_full_path(path_caches, &script.icon))
                            .width(22)
                            .height(22)
                            .into(),
                    );
                    row_data.push(horizontal_space(4).into());
                }
                row_data.push(text(script_name).style(style).into());
                row_data.push(progress.into());

                let is_enabled = !execution_data.has_started;

                if is_enabled && is_selected {
                    row_data.push(horizontal_space(Length::Fill).into());
                    if i > 0 {
                        row_data.push(
                            inline_icon_button(icons.themed.up.clone(), Message::MoveScriptUp(i))
                                .style(theme::Button::Primary)
                                .into(),
                        );
                    }
                    if i < execution_data.scripts_to_run.len() - 1 {
                        row_data.push(
                            inline_icon_button(
                                icons.themed.down.clone(),
                                Message::MoveScriptDown(i),
                            )
                            .style(theme::Button::Primary)
                            .into(),
                        );
                    } else {
                        row_data.push(horizontal_space(22).into());
                    }
                    row_data.push(horizontal_space(8).into());
                    row_data.push(
                        tooltip(
                            inline_icon_button(
                                (if theme.extended_palette().danger.strong.text.r > 0.5 {
                                    &icons.bright
                                } else {
                                    &icons.dark
                                })
                                .remove
                                .clone(),
                                Message::RemoveScript(EditScriptId {
                                    idx: i,
                                    script_type: EditScriptType::ExecutionList,
                                }),
                            )
                            .style(theme::Button::Destructive),
                            "Remove script from execution list",
                            tooltip::Position::Left,
                        )
                        .style(theme::Container::Box)
                        .into(),
                    );
                } else if execution::has_script_started(&script_status) {
                    row_data.push(horizontal_space(8).into());
                    if script_status.retry_count > 0 {
                        let log_dir_path =
                            config::get_script_log_directory(&path_caches.logs_path, i as isize);
                        row_data.push(
                            tooltip(
                                inline_icon_button(
                                    icons.themed.log.clone(),
                                    Message::OpenFile(log_dir_path),
                                ),
                                "Open log directory",
                                tooltip::Position::Right,
                            )
                            .style(theme::Container::Box)
                            .into(),
                        );
                    } else if !execution::has_script_been_skipped(&script_status) {
                        let output_path = config::get_script_output_path(
                            &path_caches.logs_path,
                            i as isize,
                            script_status.retry_count,
                        );
                        row_data.push(
                            tooltip(
                                inline_icon_button(
                                    icons.themed.log.clone(),
                                    Message::OpenFile(output_path),
                                ),
                                "Open log file",
                                tooltip::Position::Right,
                            )
                            .style(theme::Container::Box)
                            .into(),
                        );
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
                        if is_script_missing_arguments(&script) {
                            theme::Button::Destructive
                        } else {
                            theme::Button::Secondary
                        }
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
                main_icon_button(
                    icons.themed.retry.clone(),
                    "Reschedule",
                    Some(Message::RescheduleScripts)
                ),
                main_icon_button(
                    icons.themed.remove.clone(),
                    "Clear",
                    Some(Message::ClearScripts)
                ),
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
            row![main_icon_button(
                icons.themed.stop.clone(),
                "Stop",
                Some(Message::StopScripts)
            )]
            .align_items(Alignment::Center)
        }
    } else if !execution_data.scripts_to_run.is_empty() {
        let has_scripts_missing_arguments = execution_data
            .scripts_to_run
            .iter()
            .any(|script| is_script_missing_arguments(script));

        let run_button = if has_scripts_missing_arguments {
            column![tooltip(
                main_icon_button(icons.themed.play.clone(), "Run", None,),
                "Some scripts are missing arguments",
                tooltip::Position::Top
            )
            .style(theme::Container::Box)]
        } else {
            column![main_icon_button(
                icons.themed.play.clone(),
                "Run",
                Some(Message::RunScripts)
            ),]
        };
        row![
            run_button,
            main_icon_button(
                icons.themed.remove.clone(),
                "Clear",
                Some(Message::ClearScripts)
            ),
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
    if let Ok(logs) = execution_data.recent_logs.try_lock() {
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
    script_definitions: &Vec<config::ScriptDefinition>,
    visual_caches: &VisualCaches,
    edit_data: &EditData,
    app_config: &config::AppConfig,
) -> Column<'a, Message> {
    if execution::has_started_execution(&execution_data) {
        return Column::new();
    }

    let Some(currently_edited_script) = &edit_data.currently_edited_script else {
        return Column::new();
    };

    let edit_button = |label, message| {
        button(
            text(label)
                .vertical_alignment(alignment::Vertical::Center)
                .size(16),
        )
        .padding(4)
        .on_press(message)
    };

    let script = match currently_edited_script.script_type {
        EditScriptType::ScriptConfig => &script_definitions[currently_edited_script.idx],
        EditScriptType::ExecutionList => {
            &execution_data.scripts_to_run[currently_edited_script.idx]
        }
    };

    let mut parameters: Vec<Element<'_, Message, iced::Renderer>> = Vec::new();
    if !script.is_read_only {
        parameters.push(
            text_input("name", &script.name)
                .on_input(move |new_arg| Message::EditScriptName(new_arg))
                .padding(5)
                .into(),
        );

        if currently_edited_script.script_type == EditScriptType::ScriptConfig {
            populate_path_editing_content(
                "Command:",
                "command",
                &script.icon,
                &mut parameters,
                |path| Message::EditScriptCommand(path),
                |val| Message::ToggleScriptCommandRelativeToScripter(val),
            );

            populate_path_editing_content(
                "Path to the icon:",
                "path/to/icon.png",
                &script.icon,
                &mut parameters,
                |path| Message::EditScriptIconPath(path),
                |val| Message::ToggleScriptIconPathRelativeToScripter(val),
            );
        }

        parameters.push(
            text(
                if currently_edited_script.script_type == EditScriptType::ExecutionList {
                    "Arguments line:"
                } else {
                    "Default arguments:"
                },
            )
            .into(),
        );
        parameters.push(
            text_input("\"arg1\" \"arg2\"", &script.arguments)
                .on_input(move |new_value| Message::EditArguments(new_value))
                .style(
                    if currently_edited_script.script_type == EditScriptType::ExecutionList
                        && is_script_missing_arguments(&script)
                    {
                        theme::TextInput::Custom(Box::new(style::InvalidInputStyleSheet))
                    } else {
                        theme::TextInput::Default
                    },
                )
                .padding(5)
                .into(),
        );
        if currently_edited_script.script_type == EditScriptType::ScriptConfig {
            parameters.push(
                checkbox(
                    "Arguments are required",
                    script.requires_arguments,
                    move |val| Message::ToggleRequiresArguments(val),
                )
                .into(),
            );
        }

        parameters.push(text("Retry count:").into());
        parameters.push(
            text_input("0", &visual_caches.autorerun_count)
                .on_input(move |new_value| Message::EditAutorerunCount(new_value))
                .padding(5)
                .into(),
        );

        parameters.push(
            checkbox(
                "Ignore previous failures",
                script.ignore_previous_failures,
                move |val| Message::ToggleIgnoreFailures(val),
            )
            .into(),
        );

        if currently_edited_script.script_type == EditScriptType::ScriptConfig {
            parameters.push(
                edit_button(
                    "Duplicate script",
                    Message::DuplicateScript(currently_edited_script.clone()),
                )
                .into(),
            );
        }

        if is_local_edited_script(
            currently_edited_script.idx,
            &app_config,
            &edit_data.window_edit_data,
        ) {
            parameters.push(
                edit_button(
                    "Make shared",
                    Message::MoveToParent(currently_edited_script.clone()),
                )
                .into(),
            );
        }

        parameters.push(
            edit_button(
                "Remove script",
                Message::RemoveScript(currently_edited_script.clone()),
            )
            .style(theme::Button::Destructive)
            .into(),
        );
    } else {
        parameters.push(
            checkbox("Is script hidden", script.is_hidden, move |val| {
                Message::ToggleScriptHidden(val)
            })
            .into(),
        );

        if currently_edited_script.script_type == EditScriptType::ScriptConfig {
            parameters.push(
                edit_button(
                    "Edit as a copy",
                    Message::CreateCopyOfParentScript(currently_edited_script.clone()),
                )
                .into(),
            );
        }
    }

    let content = column(parameters).spacing(10);

    return column![scrollable(content)]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .align_items(Alignment::Start);
}

fn produce_config_edit_content<'a>(
    config: &config::AppConfig,
    window_edit: &WindowEditData,
) -> Column<'a, Message> {
    let rewritable_config = get_rewritable_config(&config, &window_edit.edit_type);

    let mut list_elements: Vec<Element<'_, Message, iced::Renderer>> = Vec::new();

    list_elements.push(checkbox(
        "Always on top (requires restart)",
        rewritable_config.always_on_top,
        move |val| Message::ConfigToggleAlwaysOnTop(val),
    ).into());
    list_elements.push(checkbox(
        "Window status reactions",
        rewritable_config.window_status_reactions,
        move |val| Message::ConfigToggleWindowStatusReactions(val),
    ).into());
    list_elements.push(checkbox(
        "Keep window size",
        rewritable_config.keep_window_size,
        move |val| Message::ConfigToggleKeepWindowSize(val),
    ).into());
    list_elements.push(checkbox(
        "Use custom theme",
        rewritable_config.custom_theme.is_some(),
        move |val| Message::ConfigToggleUseCustomTheme(val),
    ).into());

    if let Some(_theme) = &rewritable_config.custom_theme {
        list_elements.push(text("Background:").into());
        list_elements.push(text_input("#000000", &window_edit.theme_color_background)
                .on_input(move |new_value| Message::ConfigEditThemeBackground(new_value))
                .padding(5).into());
        list_elements.push(text("Accent:").into());
        list_elements.push(text_input("#000000", &window_edit.theme_color_text)
                .on_input(move |new_value| Message::ConfigEditThemeText(new_value))
                .padding(5).into());
        list_elements.push(text("Primary:").into());
        list_elements.push(text_input("#000000", &window_edit.theme_color_primary)
                .on_input(move |new_value| Message::ConfigEditThemePrimary(new_value))
                .padding(5).into());
        list_elements.push(text("Success:").into());
        list_elements.push(text_input("#000000", &window_edit.theme_color_success)
                .on_input(move |new_value| Message::ConfigEditThemeSuccess(new_value))
                .padding(5).into());
        list_elements.push(text("Danger:").into());
        list_elements.push(text_input("#000000", &window_edit.theme_color_danger)
                .on_input(move |new_value| Message::ConfigEditThemeDanger(new_value))
                .padding(5).into());
    }

    if window_edit.edit_type == ConfigEditType::Parent {
        populate_path_editing_content(
            "Local config path:",
            "path/to/config.json",
            &config.child_config_path,
            &mut list_elements,
            |path| Message::ConfigEditChildConfigPath(path),
            |val| Message::ConfigToggleChildConfigPathRelativeToScripter(val),
        );
    }

    return column![scrollable(column(list_elements))]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .align_items(Alignment::Start);
}

fn view_content<'a>(
    execution_data: &execution::ScriptExecutionData,
    variant: &PaneVariant,
    theme: &Theme,
    paths: &config::PathCaches,
    visual_caches: &VisualCaches,
    config: &config::AppConfig,
    edit_data: &EditData,
) -> Element<'a, Message> {
    let content = match variant {
        PaneVariant::ScriptList => produce_script_list_content(
            execution_data,
            config,
            paths,
            edit_data,
            &visual_caches.icons,
        ),
        PaneVariant::ExecutionList => produce_execution_list_content(
            execution_data,
            paths,
            theme,
            &config.custom_title,
            &visual_caches.icons,
            edit_data,
        ),
        PaneVariant::LogOutput => produce_log_output_content(execution_data, theme),
        PaneVariant::Parameters => match &edit_data.window_edit_data {
            Some(window_edit_data) if window_edit_data.is_editing_config => {
                produce_config_edit_content(config, window_edit_data)
            }
            _ => produce_script_edit_content(
                execution_data,
                &get_script_definition_list_opt(&config, &edit_data.window_edit_data),
                visual_caches,
                edit_data,
                config,
            ),
        },
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
    icons: &IconCaches,
    edit_data: &EditData,
    execution_data: &execution::ScriptExecutionData,
    is_maximized: bool,
    size: Size,
) -> Element<'a, Message> {
    let mut row = row![].spacing(5);

    if *variant == PaneVariant::ScriptList
        && !edit_data.window_edit_data.is_some()
        && !execution_data.has_started
    {
        row = row.push(
            tooltip(
                edit_mode_button(
                    icons.themed.edit.clone(),
                    Message::EnterWindowEditMode,
                    edit_data.is_dirty,
                ),
                "Enter editing mode",
                tooltip::Position::Left,
            )
            .style(theme::Container::Box),
        );
    }

    if total_panes > 1
        && (is_maximized || (*variant == PaneVariant::ExecutionList && execution_data.has_started))
    {
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
        PaneVariant::Parameters => "Parameters",
    }
}

fn apply_script_edit(app: &mut MainWindow, edit_fn: impl FnOnce(&mut config::ScriptDefinition)) {
    if let Some(script_id) = &app.edit_data.currently_edited_script {
        match script_id.script_type {
            EditScriptType::ScriptConfig => match &app.edit_data.window_edit_data {
                Some(window_edit_data) if window_edit_data.edit_type == ConfigEditType::Child => {
                    if let Some(config) = &mut app.app_config.child_config_body {
                        match &mut config.script_definitions[script_id.idx] {
                            config::ChildScriptDefinition::Added(script) => {
                                edit_fn(script);
                                config::update_child_config_script_cache_from_config(
                                    &mut app.app_config,
                                );
                                app.edit_data.is_dirty = true;
                            }
                            _ => {}
                        }
                    }
                }
                _ => {
                    edit_fn(&mut app.app_config.script_definitions[script_id.idx]);
                    app.edit_data.is_dirty = true;
                }
            },
            EditScriptType::ExecutionList => {
                edit_fn(&mut app.execution_data.scripts_to_run[script_id.idx]);
            }
        }
    }
}

fn get_theme(config: &config::AppConfig, window_edit_data: &Option<WindowEditData>) -> Theme {
    if let Some(theme) = get_rewritable_config_opt(&config, window_edit_data)
        .custom_theme
        .clone()
    {
        style::get_custom_theme(theme)
    } else {
        Theme::default()
    }
}

fn hex_to_rgb(hex: &str) -> Option<[f32; 3]> {
    if hex.len() != 7 {
        return None;
    }

    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    return Some([r as f32 / 256.0, g as f32 / 256.0, b as f32 / 256.0]);
}

fn rgb_to_hex(rgb: &[f32; 3]) -> String {
    let rgb = rgb.map(|x| x.max(0.0).min(1.0));
    let r = (256.0 * rgb[0]) as u8;
    let g = (256.0 * rgb[1]) as u8;
    let b = (256.0 * rgb[2]) as u8;
    let hex = format!("{:02x}{:02x}{:02x}", r, g, b);
    return format!("#{}", hex);
}

fn apply_theme_color_from_string(
    app: &mut MainWindow,
    color: String,
    set_theme_fn: impl FnOnce(&mut config::CustomTheme, [f32; 3]),
    set_text_fn: impl FnOnce(&mut WindowEditData, String) -> String,
) {
    if let Some(edit_data) = &mut app.edit_data.window_edit_data {
        let color_string = set_text_fn(edit_data, color);
        if let Some(custom_theme) =
            &mut get_rewritable_config_mut_non_opt(&mut app.app_config, edit_data).custom_theme
        {
            if let Some(new_color) = hex_to_rgb(&color_string) {
                set_theme_fn(custom_theme, new_color);
                apply_theme(app);
                app.edit_data.is_dirty = true;
            }
        }
    }
}

fn get_rewritable_config<'a>(
    config: &'a config::AppConfig,
    edit_type: &ConfigEditType,
) -> &'a config::RewritableConfig {
    match edit_type {
        ConfigEditType::Parent => &config.rewritable,
        ConfigEditType::Child => {
            if let Some(child_config) = &config.child_config_body {
                &child_config.rewritable
            } else {
                &config.rewritable
            }
        }
    }
}

fn get_rewritable_config_opt<'a>(
    config: &'a config::AppConfig,
    edit_data: &Option<WindowEditData>,
) -> &'a config::RewritableConfig {
    match &edit_data {
        Some(edit_data) => get_rewritable_config(config, &edit_data.edit_type),
        None => {
            if let Some(child_config) = &config.child_config_body {
                &child_config.rewritable
            } else {
                &config.rewritable
            }
        }
    }
}

fn get_rewritable_config_mut<'a>(
    config: &'a mut config::AppConfig,
    window_edit: &Option<WindowEditData>,
) -> &'a mut config::RewritableConfig {
    return match window_edit {
        Some(window_edit) => get_rewritable_config_mut_non_opt(config, window_edit),
        None => &mut config.rewritable,
    };
}

fn get_rewritable_config_mut_non_opt<'a>(
    config: &'a mut config::AppConfig,
    window_edit: &WindowEditData,
) -> &'a mut config::RewritableConfig {
    match window_edit.edit_type {
        ConfigEditType::Parent => &mut config.rewritable,
        ConfigEditType::Child => {
            if let Some(child_config) = &mut config.child_config_body {
                &mut child_config.rewritable
            } else {
                &mut config.rewritable
            }
        }
    }
}

fn switch_config_edit_mode(app: &mut MainWindow, edit_type: ConfigEditType) {
    let is_config_editing = if let Some(window_edit) = &app.edit_data.window_edit_data {
        window_edit.is_editing_config
    } else {
        false
    };
    app.edit_data.window_edit_data = Some(WindowEditData::from_config(
        &app.app_config,
        is_config_editing,
        edit_type,
    ));
}

fn get_script_definition_list_opt<'a>(
    config: &'a config::AppConfig,
    window_edit_data: &Option<WindowEditData>,
) -> &'a Vec<config::ScriptDefinition> {
    match window_edit_data {
        Some(window_edit_data) => get_script_definition_list(config, &window_edit_data.edit_type),
        None => {
            if let Some(child_config) = &config.child_config_body {
                &child_config.config_definition_cache
            } else {
                &config.script_definitions
            }
        }
    }
}

fn get_script_definition_list<'a>(
    config: &'a config::AppConfig,
    edit_type: &ConfigEditType,
) -> &'a Vec<config::ScriptDefinition> {
    match edit_type {
        ConfigEditType::Parent => &config.script_definitions,
        ConfigEditType::Child => {
            if let Some(child_config) = &config.child_config_body {
                &child_config.config_definition_cache
            } else {
                &config.script_definitions
            }
        }
    }
}

fn apply_theme(app: &mut MainWindow) {
    app.theme = get_theme(&app.app_config, &app.edit_data.window_edit_data);
    update_theme_icons(app);
}

fn update_theme_icons(app: &mut MainWindow) {
    let icons = &mut app.visual_caches.icons;
    if app.theme.extended_palette().primary.strong.text.r > 0.5 {
        icons.themed = icons.bright.clone()
    } else {
        icons.themed = icons.dark.clone();
    }
}

fn is_local_edited_script(
    script_idx: usize,
    app_config: &config::AppConfig,
    window_edit_data: &Option<WindowEditData>,
) -> bool {
    if let Some(window_edit_data) = &window_edit_data {
        if window_edit_data.edit_type == ConfigEditType::Child {
            if let Some(scripts) = &app_config.child_config_body {
                match scripts.script_definitions.get(script_idx) {
                    Some(config::ChildScriptDefinition::Added(_)) => {
                        return true;
                    }
                    _ => {}
                }
            }
        }
    }
    return false;
}

fn add_script_to_parent_config(
    app_config: &mut config::AppConfig,
    script: config::ScriptDefinition,
) -> usize {
    app_config.script_definitions.push(script);
    let script_idx = app_config.script_definitions.len() - 1;
    config::populate_parent_scripts_from_config(app_config);
    return script_idx;
}

fn add_script_to_child_config(
    app_config: &mut config::AppConfig,
    script: config::ScriptDefinition,
) -> Option<usize> {
    if let Some(config) = &mut app_config.child_config_body {
        config
            .script_definitions
            .push(config::ChildScriptDefinition::Added(script));
    } else {
        return None;
    }

    config::update_child_config_script_cache_from_config(app_config);

    if let Some(config) = &mut app_config.child_config_body {
        return Some(config.script_definitions.len() - 1);
    } else {
        return None;
    }
}

fn is_script_missing_arguments(script: &config::ScriptDefinition) -> bool {
    return script.requires_arguments && script.arguments.is_empty();
}

fn populate_path_editing_content(
    caption: &str,
    hint: &str,
    path: &config::PathConfig,
    edit_content: &mut Vec<Element<'_, Message, iced::Renderer>>,
    on_path_changed: impl Fn(String) -> Message + 'static,
    on_path_type_changed: impl Fn(bool) -> Message + 'static,
) {
    edit_content.push(text(caption).into());
    edit_content.push(
        text_input(hint, &path.path)
            .on_input(on_path_changed)
            .padding(5)
            .into(),
    );
    edit_content.push(
        checkbox(
            "Path relative to scripter executable",
            path.path_type == config::PathType::ScripterExecutableRelative,
            on_path_type_changed,
        )
        .into(),
    );
}
