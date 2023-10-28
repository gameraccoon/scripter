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
use iced::{event, executor, keyboard, window, ContentFit, Event};
use iced::{time, Size};
use iced::{Application, Command, Element, Length, Subscription};
use iced_lazy::responsive;
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::mem::swap;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, Instant};

use crate::color_utils;
use crate::config;
use crate::execution;
use crate::file_utils;
use crate::string_constants;
use crate::style;
use crate::ui_icons;

const ONE_EXECUTION_LIST_ELEMENT_HEIGHT: u32 = 30;
const ONE_TITLE_LINE_HEIGHT: u32 = 16;
const EMPTY_EXECUTION_LIST_HEIGHT: u32 = 150;

// these should be static not just const
static FILTER_INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);
static ARGUMENTS_INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);

// caches for visual elements content
pub struct VisualCaches {
    autorerun_count: String,
    recent_logs: Vec<String>,
    icons: ui_icons::IconCaches,
}

pub struct MainWindow {
    panes: pane_grid::State<AppPane>,
    pane_by_pane_type: HashMap<PaneVariant, pane_grid::Pane>,
    execution_data: execution::ScriptExecutionData,
    app_config: config::AppConfig,
    theme: Theme,
    visual_caches: VisualCaches,
    edit_data: EditData,
    window_state: WindowState,
}

#[derive(Debug, Clone)]
pub struct EditData {
    // a string that is used to filter the list of scripts
    script_filter: String,
    // state of the global to the window editing mode
    window_edit_data: Option<WindowEditData>,
    // do we have unsaved changes
    is_dirty: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
    theme_color_caption_text: String,
    theme_color_error_text: String,
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
            theme_color_background: color_utils::rgb_to_hex(&theme.background),
            theme_color_text: color_utils::rgb_to_hex(&theme.text),
            theme_color_primary: color_utils::rgb_to_hex(&theme.primary),
            theme_color_success: color_utils::rgb_to_hex(&theme.success),
            theme_color_danger: color_utils::rgb_to_hex(&theme.danger),
            theme_color_caption_text: color_utils::rgb_to_hex(&theme.caption_text),
            theme_color_error_text: color_utils::rgb_to_hex(&theme.error_text),
        }
    }
}

struct WindowState {
    pane_focus: Option<pane_grid::Pane>,
    cursor_script: Option<EditScriptId>,
    full_window_size: Size,
    is_command_key_down: bool,
    has_maximized_pane: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    WindowResized(Size),
    Clicked(pane_grid::Pane),
    Dragged(pane_grid::DragEvent),
    Resized(pane_grid::ResizeEvent),
    Maximize(pane_grid::Pane, Size),
    Restore,
    MaximizeOrRestoreExecutionPane,
    AddScriptToExecution(config::Guid),
    RunScripts,
    RunOrRescheduleScripts,
    StopScripts,
    ClearScripts,
    StopOrClearScripts,
    RescheduleScripts,
    Tick(Instant),
    OpenScriptEditing(usize),
    CloseScriptEditing,
    DuplicateConfigScript(EditScriptId),
    RemoveScript(EditScriptId),
    AddScriptToConfig,
    MoveExecutionScriptUp(usize),
    MoveExecutionScriptDown(usize),
    EditScriptName(String),
    EditScriptCommand(String),
    ToggleScriptCommandRelativeToScripter(bool),
    EditScriptIconPath(String),
    ToggleScriptIconPathRelativeToScripter(bool),
    EditArguments(String),
    ToggleRequiresArguments(bool),
    EditArgumentsHint(String),
    EditAutorerunCount(String),
    OpenFile(PathBuf),
    ToggleIgnoreFailures(bool),
    EnterWindowEditMode,
    ExitWindowEditMode,
    TrySwitchWindowEditMode,
    SaveConfig,
    RevertConfig,
    OpenScriptConfigEditing(usize),
    MoveConfigScriptUp(usize),
    MoveConfigScriptDown(usize),
    ToggleConfigEditing,
    ConfigToggleAlwaysOnTop(bool),
    ConfigToggleWindowStatusReactions(bool),
    ConfigToggleKeepWindowSize(bool),
    ConfigToggleScriptFiltering(bool),
    ConfigToggleUseCustomTheme(bool),
    ConfigEditThemeBackground(String),
    ConfigEditThemeText(String),
    ConfigEditThemePrimary(String),
    ConfigEditThemeSuccess(String),
    ConfigEditThemeDanger(String),
    ConfigEditThemeCaptionText(String),
    ConfigEditThemeErrorText(String),
    ConfigEditChildConfigPath(String),
    ConfigToggleChildConfigPathRelativeToScripter(bool),
    SwitchToParentConfig,
    SwitchToChildConfig,
    ToggleScriptHidden(bool),
    CreateCopyOfParentScript(EditScriptId),
    MoveToParent(EditScriptId),
    SaveAsPreset,
    ScriptFilterChanged(String),
    RequestCloseApp,
    FocusFilter,
    OnCommandKeyStateChanged(bool),
    MoveCursorUp,
    MoveCursorDown,
    MoveScriptDown,
    MoveScriptUp,
    CursorConfirm,
    RemoveCursorScript,
    SwitchPaneFocus(bool),
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

        let mut pane_by_pane_type = HashMap::new();
        for pane in panes.panes.iter() {
            pane_by_pane_type.insert(pane.1.variant.clone(), *pane.0);
        }

        let app_config = config::get_app_config_copy();

        let mut main_window = MainWindow {
            panes,
            pane_by_pane_type,
            execution_data: execution::new_execution_data(),
            theme: get_theme(&app_config, &None),
            app_config,
            visual_caches: VisualCaches {
                autorerun_count: String::new(),
                recent_logs: Vec::new(),
                icons: ui_icons::IconCaches::new(),
            },
            edit_data: EditData {
                script_filter: String::new(),
                window_edit_data: None,
                is_dirty: false,
            },
            window_state: WindowState {
                pane_focus: None,
                cursor_script: None,
                full_window_size: Size::new(1024.0, 768.0),
                is_command_key_down: false,
                has_maximized_pane: false,
            },
        };

        update_theme_icons(&mut main_window);
        update_config_cache(&mut main_window.app_config, &main_window.edit_data);

        return (main_window, Command::none());
    }

    fn title(&self) -> String {
        if let Some(window_edit_data) = &self.edit_data.window_edit_data {
            match window_edit_data.edit_type {
                ConfigEditType::Parent if self.app_config.child_config_body.is_some() => {
                    "scripter [Editing shared config]".to_string()
                }
                _ => "scripter [Editing]".to_string(),
            }
        } else if execution::has_started_execution(&self.execution_data) {
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
            Message::WindowResized(size) => {
                if !self.window_state.has_maximized_pane {
                    self.window_state.full_window_size = size;
                }
            }
            Message::Clicked(pane) => {
                self.window_state.pane_focus = Some(pane);
            }
            Message::Resized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(&split, ratio);
            }
            Message::Dragged(pane_grid::DragEvent::Dropped { pane, target }) => {
                self.panes.swap(&pane, &target);
            }
            Message::Dragged(_) => {}
            Message::Maximize(pane, window_size) => {
                return maximize_pane(self, pane, window_size);
            }
            Message::Restore => {
                return restore_window(self);
            }
            Message::MaximizeOrRestoreExecutionPane => {
                if execution::has_started_execution(&self.execution_data) {
                    if self.window_state.has_maximized_pane {
                        return restore_window(self);
                    } else {
                        return maximize_pane(
                            self,
                            self.pane_by_pane_type[&PaneVariant::ExecutionList],
                            self.window_state.full_window_size,
                        );
                    }
                }
            }
            Message::AddScriptToExecution(script_uid) => {
                let is_added = add_script_to_execution(self, script_uid, true);

                if is_added && self.window_state.is_command_key_down {
                    run_scheduled_scripts(self);
                }
            }
            Message::RunScripts => {
                if !execution::has_started_execution(&self.execution_data)
                    && !self.edit_data.window_edit_data.is_some()
                {
                    run_scheduled_scripts(self);
                }
            }
            Message::RunOrRescheduleScripts => {
                if !execution::has_started_execution(&self.execution_data) {
                    if self.edit_data.window_edit_data.is_none() {
                        run_scheduled_scripts(self);
                    }
                } else {
                    reschedule_scripts(self);
                }
            }
            Message::StopScripts => {
                if execution::has_started_execution(&self.execution_data)
                    && !execution::has_finished_execution(&self.execution_data)
                {
                    execution::request_stop_execution(&mut self.execution_data);
                }
            }
            Message::StopOrClearScripts => {
                if execution::has_started_execution(&self.execution_data)
                    && !execution::has_finished_execution(&self.execution_data)
                {
                    execution::request_stop_execution(&mut self.execution_data);
                } else if !execution::is_waiting_execution_thread_to_finish(&self.execution_data) {
                    clear_scripts(self);
                }
            }
            Message::ClearScripts => clear_scripts(self),
            Message::RescheduleScripts => reschedule_scripts(self),
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
                select_execution_script(self, script_idx);
            }
            Message::CloseScriptEditing => {
                clean_script_selection(&mut self.window_state.cursor_script);
            }
            Message::DuplicateConfigScript(script_id) => {
                match script_id.script_type {
                    EditScriptType::ScriptConfig => match &self.edit_data.window_edit_data {
                        Some(WindowEditData {
                            edit_type: ConfigEditType::Child,
                            ..
                        }) => {
                            if let Some(config) = self.app_config.child_config_body.as_mut() {
                                config.script_definitions.insert(
                                    script_id.idx + 1,
                                    make_script_copy(
                                        config.script_definitions[script_id.idx].clone(),
                                    ),
                                );
                            }
                        }
                        _ => {
                            self.app_config.script_definitions.insert(
                                script_id.idx + 1,
                                make_script_copy(
                                    self.app_config.script_definitions[script_id.idx].clone(),
                                ),
                            );
                        }
                    },
                    EditScriptType::ExecutionList => {}
                };
                if let Some(script) = &mut self.window_state.cursor_script {
                    script.idx = script_id.idx + 1;
                    script.script_type = script_id.script_type;
                }
                update_config_cache(&mut self.app_config, &self.edit_data);
            }
            Message::RemoveScript(script_id) => remove_script(self, &script_id),
            Message::AddScriptToConfig => {
                let script = config::OriginalScriptDefinition {
                    uid: config::Guid::new(),
                    name: "new script".to_string(),
                    icon: config::PathConfig::default(),
                    command: config::PathConfig::default(),
                    arguments: "".to_string(),
                    autorerun_count: 0,
                    ignore_previous_failures: false,
                    requires_arguments: false,
                    arguments_hint: "\"arg1\" \"arg2\"".to_string(),
                };
                add_script_to_config(self, config::ScriptDefinition::Original(script));

                update_config_cache(&mut self.app_config, &self.edit_data);
            }
            Message::MoveExecutionScriptUp(script_idx) => {
                self.execution_data
                    .scripts_to_run
                    .swap(script_idx, script_idx - 1);
                select_execution_script(self, script_idx - 1);
            }
            Message::MoveExecutionScriptDown(script_idx) => {
                self.execution_data
                    .scripts_to_run
                    .swap(script_idx, script_idx + 1);
                select_execution_script(self, script_idx + 1);
            }
            Message::EditScriptName(new_name) => {
                if let Some(preset) =
                    get_editing_preset(&mut self.app_config, &self.edit_data, &self.window_state)
                {
                    preset.name = new_name;
                    self.edit_data.is_dirty = true;
                    update_config_cache(&mut self.app_config, &self.edit_data);
                } else {
                    apply_script_edit(self, move |script| script.name = new_name);
                }
            }
            Message::EditScriptCommand(new_command) => {
                apply_script_edit(self, move |script| script.command.path = new_command);
            }
            Message::ToggleScriptCommandRelativeToScripter(value) => {
                apply_script_edit(self, |script| {
                    script.command.path_type = if value {
                        config::PathType::ScripterExecutableRelative
                    } else {
                        config::PathType::WorkingDirRelative
                    }
                });
            }
            Message::EditScriptIconPath(new_icon_path) => {
                if let Some(preset) =
                    get_editing_preset(&mut self.app_config, &self.edit_data, &self.window_state)
                {
                    preset.icon.path = new_icon_path;
                    self.edit_data.is_dirty = true;
                    update_config_cache(&mut self.app_config, &self.edit_data);
                } else {
                    apply_script_edit(self, move |script| script.icon.path = new_icon_path);
                }
            }
            Message::ToggleScriptIconPathRelativeToScripter(new_relative) => {
                let new_path_type = if new_relative {
                    config::PathType::ScripterExecutableRelative
                } else {
                    config::PathType::WorkingDirRelative
                };

                if let Some(preset) =
                    get_editing_preset(&mut self.app_config, &self.edit_data, &self.window_state)
                {
                    preset.icon.path_type = new_path_type;
                    self.edit_data.is_dirty = true;
                    update_config_cache(&mut self.app_config, &self.edit_data);
                } else {
                    apply_script_edit(self, move |script| {
                        script.icon.path_type = new_path_type;
                    });
                }
            }
            Message::EditArguments(new_arguments) => {
                apply_script_edit(self, move |script| script.arguments = new_arguments)
            }
            Message::ToggleRequiresArguments(new_requires_arguments) => {
                apply_script_edit(self, move |script| {
                    script.requires_arguments = new_requires_arguments
                })
            }
            Message::EditArgumentsHint(new_arguments_hint) => {
                apply_script_edit(self, move |script| {
                    script.arguments_hint = new_arguments_hint
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
            Message::EnterWindowEditMode => enter_window_edit_mode(self),
            Message::ExitWindowEditMode => exit_window_edit_mode(self),
            Message::TrySwitchWindowEditMode => {
                if !execution::has_started_execution(&self.execution_data) {
                    if !self.edit_data.window_edit_data.is_some() {
                        enter_window_edit_mode(self);
                    } else {
                        exit_window_edit_mode(self);
                    }
                }
            }
            Message::SaveConfig => {
                config::save_config_to_file(&self.app_config);
                self.app_config = config::read_config();
                self.edit_data.is_dirty = false;
                update_config_cache(&mut self.app_config, &self.edit_data);
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
                clean_script_selection(&mut self.window_state.cursor_script);
                update_config_cache(&mut self.app_config, &self.edit_data);
            }
            Message::OpenScriptConfigEditing(script_idx) => {
                select_edited_script(self, script_idx);
            }
            Message::MoveConfigScriptUp(index) => {
                move_config_script_up(self, index);
            }
            Message::MoveConfigScriptDown(index) => {
                move_config_script_down(self, index);
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
                clean_script_selection(&mut self.window_state.cursor_script);
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
            Message::ConfigToggleScriptFiltering(is_checked) => {
                get_rewritable_config_mut(&mut self.app_config, &self.edit_data.window_edit_data)
                    .enable_script_filtering = is_checked;
                self.edit_data.is_dirty = true;
            }
            Message::ConfigToggleUseCustomTheme(is_checked) => {
                get_rewritable_config_mut(&mut self.app_config, &self.edit_data.window_edit_data)
                    .custom_theme = if is_checked {
                    Some(
                        if let Some(window_edit_data) = &self.edit_data.window_edit_data {
                            config::CustomTheme {
                                background: color_utils::hex_to_rgb(
                                    &window_edit_data.theme_color_background,
                                )
                                .unwrap_or_default(),
                                text: color_utils::hex_to_rgb(&window_edit_data.theme_color_text)
                                    .unwrap_or_default(),
                                primary: color_utils::hex_to_rgb(
                                    &window_edit_data.theme_color_primary,
                                )
                                .unwrap_or_default(),
                                success: color_utils::hex_to_rgb(
                                    &window_edit_data.theme_color_success,
                                )
                                .unwrap_or_default(),
                                danger: color_utils::hex_to_rgb(
                                    &window_edit_data.theme_color_danger,
                                )
                                .unwrap_or_default(),
                                caption_text: color_utils::hex_to_rgb(
                                    &window_edit_data.theme_color_caption_text,
                                )
                                .unwrap_or_default(),
                                error_text: color_utils::hex_to_rgb(
                                    &window_edit_data.theme_color_error_text,
                                )
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
            Message::ConfigEditThemeCaptionText(new_value) => {
                apply_theme_color_from_string(
                    self,
                    new_value,
                    |theme, value| theme.caption_text = value,
                    |edit_data, value| {
                        edit_data.theme_color_caption_text = value;
                        edit_data.theme_color_caption_text.clone()
                    },
                );
                self.edit_data.is_dirty = true;
            }
            Message::ConfigEditThemeErrorText(new_value) => {
                apply_theme_color_from_string(
                    self,
                    new_value,
                    |theme, value| theme.error_text = value,
                    |edit_data, value| {
                        edit_data.theme_color_error_text = value;
                        edit_data.theme_color_error_text.clone()
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
                clean_script_selection(&mut self.window_state.cursor_script);
                switch_config_edit_mode(self, ConfigEditType::Parent);
                apply_theme(self);
                update_config_cache(&mut self.app_config, &self.edit_data);
            }
            Message::SwitchToChildConfig => {
                clean_script_selection(&mut self.window_state.cursor_script);
                switch_config_edit_mode(self, ConfigEditType::Child);
                apply_theme(self);
                update_config_cache(&mut self.app_config, &self.edit_data);
            }
            Message::ToggleScriptHidden(is_hidden) => {
                let Some(script_id) = &mut self.window_state.cursor_script else {
                    return Command::none();
                };

                if let Some(config) = &mut self.app_config.child_config_body {
                    let Some(script) = config.script_definitions.get_mut(script_id.idx) else {
                        return Command::none();
                    };

                    match script {
                        config::ScriptDefinition::ReferenceToParent(_, is_hidden_value) => {
                            *is_hidden_value = is_hidden;
                            self.edit_data.is_dirty = true;
                        }
                        _ => {}
                    }
                }
                update_config_cache(&mut self.app_config, &self.edit_data);
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
                    config::ScriptDefinition::ReferenceToParent(parent_script_id, _is_hidden) => {
                        if let Some(script) = config::get_original_script_definition_by_uid(
                            &self.app_config,
                            parent_script_id.clone(),
                        ) {
                            script
                        } else {
                            return Command::none();
                        }
                    }
                    _ => {
                        return Command::none();
                    }
                };

                if let Some(config) = &mut self.app_config.child_config_body {
                    config
                        .script_definitions
                        .insert(script_id.idx + 1, new_script);
                    select_edited_script(self, script_id.idx + 1);
                    self.edit_data.is_dirty = true;
                }
                update_config_cache(&mut self.app_config, &self.edit_data);
            }
            Message::MoveToParent(script_id) => {
                if let Some(config) = &mut self.app_config.child_config_body {
                    if config.script_definitions.len() <= script_id.idx {
                        return Command::none();
                    }

                    if let Some(script) = config.script_definitions.get_mut(script_id.idx) {
                        let mut replacement_script = match script {
                            config::ScriptDefinition::Original(definition) => {
                                config::ScriptDefinition::ReferenceToParent(
                                    definition.uid.clone(),
                                    false,
                                )
                            }
                            config::ScriptDefinition::Preset(preset) => {
                                config::ScriptDefinition::ReferenceToParent(
                                    preset.uid.clone(),
                                    false,
                                )
                            }
                            _ => {
                                return Command::none();
                            }
                        };

                        swap(script, &mut replacement_script);
                        self.app_config.script_definitions.push(replacement_script);
                        select_edited_script(self, self.app_config.script_definitions.len() - 1);
                        self.edit_data.is_dirty = true;
                    }
                }
                update_config_cache(&mut self.app_config, &self.edit_data);
            }
            Message::SaveAsPreset => {
                let mut preset = config::ScriptPreset {
                    uid: config::Guid::new(),
                    name: "new preset".to_string(),
                    icon: Default::default(),
                    items: vec![],
                };

                for script in &self.execution_data.scripts_to_run {
                    match script {
                        config::ScriptDefinition::Original(script) => {
                            let original_script = config::get_original_script_definition_by_uid(
                                &self.app_config,
                                script.uid.clone(),
                            );

                            let original_script = if let Some(original_script) = original_script {
                                match original_script {
                                    config::ScriptDefinition::ReferenceToParent(uid, _) => {
                                        config::get_original_script_definition_by_uid(
                                            &self.app_config,
                                            uid,
                                        )
                                    }
                                    _ => Some(original_script),
                                }
                            } else {
                                None
                            };

                            let original_script = if let Some(original_script) = original_script {
                                match original_script {
                                    config::ScriptDefinition::Original(script) => Some(script),
                                    _ => None,
                                }
                            } else {
                                None
                            };

                            let name = if let Some(original_script) = &original_script {
                                if original_script.name == script.name {
                                    None
                                } else {
                                    Some(script.name.clone())
                                }
                            } else {
                                Some(script.name.clone())
                            };

                            let arguments = if let Some(original_script) = &original_script {
                                if original_script.arguments == script.arguments {
                                    None
                                } else {
                                    Some(script.arguments.clone())
                                }
                            } else {
                                Some(script.arguments.clone())
                            };

                            let autorerun_count = if let Some(original_script) = &original_script {
                                if original_script.autorerun_count == script.autorerun_count {
                                    None
                                } else {
                                    Some(script.autorerun_count)
                                }
                            } else {
                                Some(script.autorerun_count)
                            };

                            let ignore_previous_failures =
                                if let Some(original_script) = original_script {
                                    if original_script.ignore_previous_failures
                                        == script.ignore_previous_failures
                                    {
                                        None
                                    } else {
                                        Some(script.ignore_previous_failures)
                                    }
                                } else {
                                    Some(script.ignore_previous_failures)
                                };

                            preset.items.push(config::PresetItem {
                                uid: script.uid.clone(),
                                name,
                                arguments,
                                autorerun_count,
                                ignore_previous_failures,
                            });
                        }
                        _ => {}
                    }
                }

                add_script_to_config(self, config::ScriptDefinition::Preset(preset));
            }
            Message::ScriptFilterChanged(new_filter_value) => {
                self.edit_data.script_filter = new_filter_value;
                update_config_cache(&mut self.app_config, &self.edit_data);
                clean_script_selection(&mut self.window_state.cursor_script);
            }
            Message::RequestCloseApp => {
                let exit_thread_command = || {
                    Command::perform(async {}, |()| {
                        std::process::exit(0);
                    })
                };

                if execution::has_started_execution(&self.execution_data) {
                    if execution::has_finished_execution(&self.execution_data) {
                        if !execution::is_waiting_execution_thread_to_finish(&self.execution_data) {
                            return exit_thread_command();
                        }
                    }
                } else {
                    return exit_thread_command();
                }
            }
            Message::FocusFilter => {
                return focus_filter(self);
            }
            Message::OnCommandKeyStateChanged(is_command_key_down) => {
                self.window_state.is_command_key_down = is_command_key_down;
            }
            Message::MoveCursorUp => {
                move_cursor(self, true);
            }
            Message::MoveCursorDown => {
                move_cursor(self, false);
            }
            Message::MoveScriptDown => {
                if execution::has_started_execution(&self.execution_data) {
                    return Command::none();
                }

                let focused_pane = if let Some(focus) = self.window_state.pane_focus {
                    self.panes.panes[&focus].variant
                } else {
                    return Command::none();
                };

                if focused_pane == PaneVariant::ScriptList {
                    if self.edit_data.window_edit_data.is_some() {
                        if let Some(edited_script) = &self.window_state.cursor_script {
                            move_config_script_down(self, edited_script.idx);
                        }
                    }
                } else if focused_pane == PaneVariant::ExecutionList {
                    if let Some(cursor_script) = &self.window_state.cursor_script {
                        if cursor_script.script_type == EditScriptType::ExecutionList {
                            if cursor_script.idx + 1 >= self.execution_data.scripts_to_run.len() {
                                return Command::none();
                            }
                            self.execution_data
                                .scripts_to_run
                                .swap(cursor_script.idx, cursor_script.idx + 1);
                            select_execution_script(self, cursor_script.idx + 1);
                        }
                    }
                }
            }
            Message::MoveScriptUp => {
                if execution::has_started_execution(&self.execution_data) {
                    return Command::none();
                }

                let focused_pane = if let Some(focus) = self.window_state.pane_focus {
                    self.panes.panes[&focus].variant
                } else {
                    return Command::none();
                };

                if focused_pane == PaneVariant::ScriptList {
                    if self.edit_data.window_edit_data.is_some() {
                        if let Some(edited_script) = &self.window_state.cursor_script {
                            move_config_script_up(self, edited_script.idx);
                        }
                    }
                } else if focused_pane == PaneVariant::ExecutionList {
                    if let Some(cursor_script) = &self.window_state.cursor_script {
                        if cursor_script.script_type == EditScriptType::ExecutionList {
                            if cursor_script.idx == 0 {
                                return Command::none();
                            }
                            self.execution_data
                                .scripts_to_run
                                .swap(cursor_script.idx, cursor_script.idx - 1);
                            select_execution_script(self, cursor_script.idx - 1);
                        }
                    }
                }
            }
            Message::CursorConfirm => {
                if execution::has_started_execution(&self.execution_data) {
                    return Command::none();
                }

                if self.edit_data.window_edit_data.is_some() {
                    return Command::none();
                }

                let Some(cursor_script) = &self.window_state.cursor_script else {
                    return Command::none();
                };

                let cursor_script_id = cursor_script.idx;

                if let Some(focus) = self.window_state.pane_focus {
                    if &self.panes.panes[&focus].variant == &PaneVariant::ScriptList {
                        let scripts = &self.app_config.displayed_configs_list_cache;

                        if let Some(script) = scripts.get(cursor_script_id) {
                            let is_added = add_script_to_execution(
                                self,
                                script.original_script_uid.clone(),
                                false,
                            );

                            if is_added && self.window_state.is_command_key_down {
                                run_scheduled_scripts(self);
                            }
                        }
                    }
                }
            }
            Message::RemoveCursorScript => {
                if execution::has_started_execution(&self.execution_data) {
                    return Command::none();
                }

                if let Some(focus) = self.window_state.pane_focus {
                    if &self.panes.panes[&focus].variant != &PaneVariant::ExecutionList {
                        return Command::none();
                    }
                }

                if let Some(cursor_script) = self.window_state.cursor_script.clone() {
                    if cursor_script.script_type == EditScriptType::ExecutionList {
                        remove_script(self, &cursor_script);
                    }
                }
            }
            Message::SwitchPaneFocus(is_forward) => {
                let new_selection = get_next_pane_selection(self, is_forward);

                let mut should_select_arguments = false;

                if new_selection == PaneVariant::Parameters {
                    if let Some(focus) = self.window_state.pane_focus {
                        if self.panes.panes[&focus].variant != PaneVariant::Parameters {
                            if let Some(cursor_script) = &self.window_state.cursor_script {
                                match cursor_script.script_type {
                                    EditScriptType::ScriptConfig => {
                                        should_select_arguments =
                                            self.edit_data.window_edit_data.is_some();
                                    }
                                    EditScriptType::ExecutionList => {
                                        should_select_arguments = true;
                                    }
                                }
                            }
                        }
                    }
                }

                self.window_state.pane_focus = Some(self.pane_by_pane_type[&new_selection]);

                if should_select_arguments {
                    return text_input::focus(ARGUMENTS_INPUT_ID.clone());
                }
            }
        }

        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let focus = self.window_state.pane_focus;
        let total_panes = self.panes.len();

        let pane_grid = responsive(move |size| {
            PaneGrid::new(&self.panes, |id, pane, is_maximized| {
                let is_focused = focus == Some(id);

                let variant = &pane.variant;

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
                        &self.window_state,
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
                        &self.window_state,
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
        use keyboard::KeyCode;

        Subscription::batch([
            iced::subscription::events_with(|event, status| {
                let is_command_key = |key_code: KeyCode| {
                    #[cfg(target_os = "macos")]
                    {
                        key_code.eq(&KeyCode::LWin) || key_code.eq(&KeyCode::RWin)
                    }
                    #[cfg(not(target_os = "macos"))]
                    {
                        key_code.eq(&KeyCode::LControl) || key_code.eq(&KeyCode::RControl)
                    }
                };

                match event {
                    Event::Window(window::Event::Resized { width, height }) => {
                        Some(Message::WindowResized(Size {
                            width: width as f32,
                            height: height as f32,
                        }))
                    }
                    Event::Keyboard(keyboard::Event::KeyPressed {
                        modifiers,
                        key_code,
                    }) => {
                        if is_command_key(key_code) {
                            return Some(Message::OnCommandKeyStateChanged(true));
                        }

                        let is_input_captured_by_a_widget = if let event::Status::Captured = status
                        {
                            true
                        } else {
                            false
                        };

                        let is_command_key_down = modifiers.command();
                        let is_shift_key_down = modifiers.shift();
                        if is_command_key_down {
                            handle_command_hotkey(key_code, &status, is_input_captured_by_a_widget)
                        } else if is_shift_key_down {
                            handle_shift_hotkey(key_code, &status, is_input_captured_by_a_widget)
                        } else {
                            handle_key_press(key_code, &status, is_input_captured_by_a_widget)
                        }
                    }
                    Event::Keyboard(keyboard::Event::KeyReleased {
                        modifiers: _modifiers,
                        key_code,
                    }) => {
                        if is_command_key(key_code) {
                            Some(Message::OnCommandKeyStateChanged(false))
                        } else {
                            None
                        }
                    }
                    _ => None,
                }
            }),
            time::every(Duration::from_millis(100)).map(Message::Tick),
        ])
    }
}

fn handle_command_hotkey(
    key_code: keyboard::KeyCode,
    _status: &event::Status,
    is_input_captured_by_a_widget: bool,
) -> Option<Message> {
    use keyboard::KeyCode;

    match key_code {
        KeyCode::W => Some(Message::RequestCloseApp),
        KeyCode::F => Some(Message::FocusFilter),
        KeyCode::E => Some(Message::TrySwitchWindowEditMode),
        KeyCode::R => Some(Message::RunOrRescheduleScripts),
        KeyCode::C => {
            if !is_input_captured_by_a_widget {
                Some(Message::StopOrClearScripts)
            } else {
                None
            }
        },
        KeyCode::Q => Some(Message::MaximizeOrRestoreExecutionPane),
        KeyCode::Enter => Some(Message::CursorConfirm),
        _ => None,
    }
}

fn handle_shift_hotkey(
    key_code: keyboard::KeyCode,
    _status: &event::Status,
    _is_input_captured_by_a_widget: bool,
) -> Option<Message> {
    use keyboard::KeyCode;

    match key_code {
        KeyCode::Down => Some(Message::MoveScriptDown),
        KeyCode::Up => Some(Message::MoveScriptUp),
        KeyCode::Tab => Some(Message::SwitchPaneFocus(false)),
        _ => None,
    }
}

fn handle_key_press(
    key_code: keyboard::KeyCode,
    _status: &event::Status,
    _is_input_captured_by_a_widget: bool,
) -> Option<Message> {
    use keyboard::KeyCode;

    match key_code {
        KeyCode::Down => Some(Message::MoveCursorDown),
        KeyCode::Up => Some(Message::MoveCursorUp),
        KeyCode::Enter => Some(Message::CursorConfirm),
        KeyCode::Tab => Some(Message::SwitchPaneFocus(true)),
        KeyCode::Delete => Some(Message::RemoveCursorScript),
        _ => None,
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

    // get autorerun count text from value
    match &script_type {
        EditScriptType::ScriptConfig => {
            if let Some(script) = &script_definitions.get(script_idx) {
                match script {
                    config::ScriptDefinition::Original(script) => {
                        visual_caches.autorerun_count = script.autorerun_count.to_string();
                    }
                    _ => {}
                }
            }
        }
        EditScriptType::ExecutionList => {
            if let Some(config::ScriptDefinition::Original(script)) =
                &execution_data.scripts_to_run.get(script_idx)
            {
                visual_caches.autorerun_count = script.autorerun_count.to_string();
            }
        }
    };
}

fn clean_script_selection(currently_edited_script: &mut Option<EditScriptId>) {
    *currently_edited_script = None;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

fn main_button(label: &str, message: Message) -> Button<Message> {
    button(row![text(label).width(Length::Shrink).size(16),])
        .width(Length::Shrink)
        .padding(8)
        .on_press(message)
}

fn edit_mode_button<'a>(
    icon_handle: Handle,
    message: Message,
    is_dirty: bool,
    window_state: &WindowState,
) -> Button<'a, Message> {
    let icon = image(icon_handle)
        .width(Length::Fixed(12.0))
        .height(Length::Fixed(12.0));

    button(if window_state.is_command_key_down {
        row![
            text(string_constants::EDIT_COMMAND_HINT).size(12),
            horizontal_space(4),
            icon
        ]
    } else {
        row![icon]
    })
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
    rewritable_config: &config::RewritableConfig,
    edit_data: &EditData,
    icons: &ui_icons::IconCaches,
    window_state: &WindowState,
    theme: &Theme,
) -> Column<'a, Message> {
    if let Some(error) = &config.config_read_error {
        return column![text(format!("Error: {}", error))];
    }

    let has_started_execution = execution::has_started_execution(&execution_data);

    let data: Element<_> = column(
        config
            .displayed_configs_list_cache
            .iter()
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

                    let will_run_on_click =
                        edit_data.window_edit_data.is_none() && window_state.is_command_key_down;

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

                    let icon = if will_run_on_click {
                        row![
                            horizontal_space(6),
                            image(icons.themed.quick_launch.clone())
                                .width(22)
                                .height(22),
                        ]
                    } else if let Some(icon_path) = &script.full_icon_path {
                        row![horizontal_space(6), image(icon_path).width(22).height(22),]
                    } else {
                        row![]
                    };

                    let is_selected = match &window_state.cursor_script {
                        Some(EditScriptId { idx, script_type })
                            if *idx == i && *script_type == EditScriptType::ScriptConfig =>
                        {
                            true
                        }
                        _ => false,
                    };

                    let item_button = button(
                        row![
                            icon,
                            horizontal_space(6),
                            text(&name_text).height(22),
                            horizontal_space(Length::Fill),
                            edit_buttons,
                        ]
                        .height(22),
                    )
                    .padding(4)
                    .style(if is_selected {
                        theme::Button::Primary
                    } else {
                        theme::Button::Secondary
                    })
                    .on_press(if edit_data.window_edit_data.is_none() {
                        Message::AddScriptToExecution(script.original_script_uid.clone())
                    } else {
                        Message::OpenScriptConfigEditing(i)
                    });

                    row![item_button]
                } else {
                    if let Some(icon_path) = &script.full_icon_path {
                        row![
                            horizontal_space(10),
                            image(icon_path).width(22).height(22),
                            horizontal_space(6),
                            text(&script.name).height(22)
                        ]
                    } else {
                        row![horizontal_space(10), text(&script.name).height(22)]
                    }
                    .height(22)
                }
                .into()
            })
            .collect(),
    )
    .spacing(if has_started_execution { 8 } else { 0 })
    .width(Length::Fill)
    .into();

    return if has_started_execution {
        column![vertical_space(4), scrollable(data),]
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

        let filter_field = if rewritable_config.enable_script_filtering
            && !has_started_execution
            && edit_data.window_edit_data.is_none()
        {
            row![
                horizontal_space(5),
                text_input(
                    if window_state.is_command_key_down {
                        string_constants::FILTER_COMMAND_HINT
                    } else {
                        "filter"
                    },
                    &edit_data.script_filter
                )
                .id(FILTER_INPUT_ID.clone())
                .on_input(Message::ScriptFilterChanged)
                .width(Length::Fill),
                horizontal_space(4),
                if !edit_data.script_filter.is_empty() {
                    column![
                        vertical_space(Length::Fixed(4.0)),
                        button(image(
                            (if theme.extended_palette().danger.base.text.r > 0.5 {
                                &icons.bright
                            } else {
                                &icons.dark
                            })
                            .remove
                            .clone()
                        ))
                        .style(theme::Button::Destructive)
                        .height(Length::Fixed(22.0))
                        .on_press(Message::ScriptFilterChanged("".to_string())),
                    ]
                } else {
                    column![]
                },
                horizontal_space(1),
            ]
        } else {
            row![]
        };

        column![filter_field, scrollable(data_column),]
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
    icons: &ui_icons::IconCaches,
    edit_data: &EditData,
    rewritable_config: &config::RewritableConfig,
    window_state: &WindowState,
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
                let config::ScriptDefinition::Original(script) = script else {
                    panic!("execution list definition is not Original");
                };
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

                let is_selected = match &window_state.cursor_script {
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
                    if let Some(custom_theme) = &rewritable_config.custom_theme {
                        iced::Color::from_rgb(
                            custom_theme.error_text[0],
                            custom_theme.error_text[1],
                            custom_theme.error_text[2],
                        )
                    } else {
                        theme.extended_palette().danger.weak.color
                    }
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

                if execution::has_started_execution(&execution_data) {
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

                let is_enabled = !execution::has_started_execution(&execution_data);

                if is_enabled && is_selected {
                    row_data.push(horizontal_space(Length::Fill).into());
                    if i > 0 {
                        row_data.push(
                            inline_icon_button(
                                icons.themed.up.clone(),
                                Message::MoveExecutionScriptUp(i),
                            )
                            .style(theme::Button::Primary)
                            .into(),
                        );
                    }
                    if i + 1 < execution_data.scripts_to_run.len() {
                        row_data.push(
                            inline_icon_button(
                                icons.themed.down.clone(),
                                Message::MoveExecutionScriptDown(i),
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
                                (if theme.extended_palette().danger.base.text.r > 0.5 {
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
                        let log_dir_path = file_utils::get_script_log_directory(
                            &path_caches.logs_path,
                            &execution_data.execution_start_time.unwrap_or_default(),
                            script_name,
                            i as isize,
                        );
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
                        let output_path = file_utils::get_script_output_path(
                            &path_caches.logs_path,
                            &execution_data.execution_start_time.unwrap_or_default(),
                            script_name,
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
                        if is_original_script_missing_arguments(&script) {
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

    let clear_name = if window_state.is_command_key_down {
        string_constants::CLEAR_COMMAND_HINT
    } else {
        "Clear"
    };

    let controls = column![if edit_data.window_edit_data.is_some() {
        if !execution_data.scripts_to_run.is_empty() {
            row![main_button("Save as preset", Message::SaveAsPreset)]
                .align_items(Alignment::Center)
                .spacing(5)
        } else {
            row![]
        }
    } else if execution::has_finished_execution(&execution_data) {
        if !execution::is_waiting_execution_thread_to_finish(&execution_data) {
            row![
                main_icon_button(
                    icons.themed.retry.clone(),
                    if window_state.is_command_key_down {
                        string_constants::RESCHEDULE_COMMAND_HINT
                    } else {
                        "Reschedule"
                    },
                    Some(Message::RescheduleScripts)
                ),
                main_icon_button(
                    icons.themed.remove.clone(),
                    clear_name,
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
                if window_state.is_command_key_down {
                    string_constants::STOP_COMMAND_HINT
                } else {
                    "Stop"
                },
                Some(Message::StopScripts)
            )]
            .align_items(Alignment::Center)
        }
    } else if !execution_data.scripts_to_run.is_empty() {
        let has_scripts_missing_arguments = execution_data
            .scripts_to_run
            .iter()
            .any(|script| is_script_missing_arguments(script));

        let run_name = if window_state.is_command_key_down {
            string_constants::RUN_COMMAND_HINT
        } else {
            "Run"
        };

        let run_button = if has_scripts_missing_arguments {
            column![tooltip(
                main_icon_button(icons.themed.play.clone(), run_name, None,),
                "Some scripts are missing arguments",
                tooltip::Position::Top
            )
            .style(theme::Container::Box)]
        } else {
            column![main_icon_button(
                icons.themed.play.clone(),
                run_name,
                Some(Message::RunScripts)
            ),]
        };
        row![
            run_button,
            main_icon_button(
                icons.themed.remove.clone(),
                clear_name,
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
    rewritable_config: &config::RewritableConfig,
) -> Column<'a, Message> {
    if !execution::has_started_execution(&execution_data) {
        return Column::new();
    }

    let mut data_lines: Vec<Element<'_, Message, iced::Renderer>> = Vec::new();
    if let Ok(logs) = execution_data.recent_logs.try_lock() {
        if !logs.is_empty() {
            let (caption_color, error_color) =
                if let Some(custom_theme) = &rewritable_config.custom_theme {
                    (
                        iced::Color::from_rgb(
                            custom_theme.caption_text[0],
                            custom_theme.caption_text[1],
                            custom_theme.caption_text[2],
                        ),
                        iced::Color::from_rgb(
                            custom_theme.error_text[0],
                            custom_theme.error_text[1],
                            custom_theme.error_text[2],
                        ),
                    )
                } else {
                    (
                        theme.extended_palette().primary.strong.color,
                        theme.extended_palette().danger.weak.color,
                    )
                };

            data_lines.extend(logs.iter().map(|element| {
                text(format!(
                    "[{}] {}",
                    element.timestamp.format("%H:%M:%S"),
                    element.text
                ))
                .style(match element.output_type {
                    execution::OutputType::StdOut => theme.extended_palette().primary.weak.text,
                    execution::OutputType::StdErr => error_color,
                    execution::OutputType::Error => error_color,
                    execution::OutputType::Event => caption_color,
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
    app_config: &config::AppConfig,
    window_state: &WindowState,
) -> Column<'a, Message> {
    if execution::has_started_execution(&execution_data) {
        return Column::new();
    }

    let Some(currently_edited_script) = &window_state.cursor_script else {
        return Column::new();
    };

    if currently_edited_script.script_type == EditScriptType::ScriptConfig {
        if edit_data.window_edit_data.is_none() {
            return Column::new();
        }
    }

    let edit_button = |label, message| {
        button(
            text(label)
                .vertical_alignment(alignment::Vertical::Center)
                .size(16),
        )
        .padding(4)
        .on_press(message)
    };

    let script = if currently_edited_script.script_type == EditScriptType::ScriptConfig {
        get_script_definition(&app_config, edit_data, currently_edited_script.idx)
    } else {
        &execution_data.scripts_to_run[currently_edited_script.idx]
    };

    let mut parameters: Vec<Element<'_, Message, iced::Renderer>> = Vec::new();

    match script {
        config::ScriptDefinition::Original(script) => {
            parameters.push(text("Name:").into());
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
                    &script.command,
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
                text_input(&script.arguments_hint, &script.arguments)
                    .on_input(move |new_value| Message::EditArguments(new_value))
                    .style(
                        if currently_edited_script.script_type == EditScriptType::ExecutionList
                            && is_original_script_missing_arguments(&script)
                        {
                            theme::TextInput::Custom(Box::new(style::InvalidInputStyleSheet))
                        } else {
                            theme::TextInput::Default
                        },
                    )
                    .padding(5)
                    .id(ARGUMENTS_INPUT_ID.clone())
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

                parameters.push(text("Argument hint:").into());
                parameters.push(
                    text_input("", &script.arguments_hint)
                        .on_input(move |new_value| Message::EditArgumentsHint(new_value))
                        .padding(5)
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
                        Message::DuplicateConfigScript(currently_edited_script.clone()),
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
        }
        config::ScriptDefinition::ReferenceToParent(_, is_hidden) => {
            parameters.push(
                checkbox("Is script hidden", *is_hidden, move |val| {
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
        config::ScriptDefinition::Preset(preset) => {
            parameters.push(text("Preset name:").into());
            parameters.push(
                text_input("name", &preset.name)
                    .on_input(move |new_arg| Message::EditScriptName(new_arg))
                    .padding(5)
                    .into(),
            );

            populate_path_editing_content(
                "Path to the icon:",
                "path/to/icon.png",
                &preset.icon,
                &mut parameters,
                |path| Message::EditScriptIconPath(path),
                |val| Message::ToggleScriptIconPathRelativeToScripter(val),
            );

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
                    "Remove preset",
                    Message::RemoveScript(currently_edited_script.clone()),
                )
                .style(theme::Button::Destructive)
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

    list_elements.push(
        checkbox(
            "Always on top (requires restart)",
            rewritable_config.always_on_top,
            move |val| Message::ConfigToggleAlwaysOnTop(val),
        )
        .into(),
    );
    list_elements.push(
        checkbox(
            "Window status reactions",
            rewritable_config.window_status_reactions,
            move |val| Message::ConfigToggleWindowStatusReactions(val),
        )
        .into(),
    );
    list_elements.push(
        checkbox(
            "Keep window size",
            rewritable_config.keep_window_size,
            move |val| Message::ConfigToggleKeepWindowSize(val),
        )
        .into(),
    );
    list_elements.push(
        checkbox(
            "Show script filter",
            rewritable_config.enable_script_filtering,
            move |val| Message::ConfigToggleScriptFiltering(val),
        )
        .into(),
    );
    list_elements.push(
        checkbox(
            "Use custom theme",
            rewritable_config.custom_theme.is_some(),
            move |val| Message::ConfigToggleUseCustomTheme(val),
        )
        .into(),
    );

    if let Some(_theme) = &rewritable_config.custom_theme {
        list_elements.push(text("Background:").into());
        list_elements.push(
            text_input("#000000", &window_edit.theme_color_background)
                .on_input(move |new_value| Message::ConfigEditThemeBackground(new_value))
                .padding(5)
                .into(),
        );
        list_elements.push(text("Accent:").into());
        list_elements.push(
            text_input("#000000", &window_edit.theme_color_text)
                .on_input(move |new_value| Message::ConfigEditThemeText(new_value))
                .padding(5)
                .into(),
        );
        list_elements.push(text("Primary:").into());
        list_elements.push(
            text_input("#000000", &window_edit.theme_color_primary)
                .on_input(move |new_value| Message::ConfigEditThemePrimary(new_value))
                .padding(5)
                .into(),
        );
        list_elements.push(text("Success:").into());
        list_elements.push(
            text_input("#000000", &window_edit.theme_color_success)
                .on_input(move |new_value| Message::ConfigEditThemeSuccess(new_value))
                .padding(5)
                .into(),
        );
        list_elements.push(text("Danger:").into());
        list_elements.push(
            text_input("#000000", &window_edit.theme_color_danger)
                .on_input(move |new_value| Message::ConfigEditThemeDanger(new_value))
                .padding(5)
                .into(),
        );
        list_elements.push(text("Caption text:").into());
        list_elements.push(
            text_input("#000000", &window_edit.theme_color_caption_text)
                .on_input(move |new_value| Message::ConfigEditThemeCaptionText(new_value))
                .padding(5)
                .into(),
        );
        list_elements.push(text("Error text:").into());
        list_elements.push(
            text_input("#000000", &window_edit.theme_color_error_text)
                .on_input(move |new_value| Message::ConfigEditThemeErrorText(new_value))
                .padding(5)
                .into(),
        );
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
    window_state: &WindowState,
) -> Element<'a, Message> {
    let rewritable_config = get_rewritable_config_opt(&config, &edit_data.window_edit_data);

    let content = match variant {
        PaneVariant::ScriptList => produce_script_list_content(
            execution_data,
            config,
            rewritable_config,
            edit_data,
            &visual_caches.icons,
            window_state,
            theme,
        ),
        PaneVariant::ExecutionList => produce_execution_list_content(
            execution_data,
            paths,
            theme,
            &config.custom_title,
            &visual_caches.icons,
            edit_data,
            rewritable_config,
            window_state,
        ),
        PaneVariant::LogOutput => {
            produce_log_output_content(execution_data, theme, rewritable_config)
        }
        PaneVariant::Parameters => match &edit_data.window_edit_data {
            Some(window_edit_data) if window_edit_data.is_editing_config => {
                produce_config_edit_content(config, window_edit_data)
            }
            _ => produce_script_edit_content(
                execution_data,
                visual_caches,
                edit_data,
                config,
                window_state,
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
    icons: &ui_icons::IconCaches,
    edit_data: &EditData,
    execution_data: &execution::ScriptExecutionData,
    is_maximized: bool,
    size: Size,
    window_state: &WindowState,
) -> Element<'a, Message> {
    let mut row = row![].spacing(5);

    if *variant == PaneVariant::ScriptList
        && !edit_data.window_edit_data.is_some()
        && !execution::has_started_execution(&execution_data)
    {
        row = row.push(
            tooltip(
                edit_mode_button(
                    icons.themed.edit.clone(),
                    Message::EnterWindowEditMode,
                    edit_data.is_dirty,
                    window_state,
                ),
                "Enter editing mode",
                tooltip::Position::Left,
            )
            .style(theme::Container::Box),
        );
    }

    if total_panes > 1
        && (is_maximized
            || (*variant == PaneVariant::ExecutionList
                && execution::has_started_execution(&execution_data)))
    {
        let toggle = {
            let (content, message) = if is_maximized {
                (
                    if window_state.is_command_key_down {
                        string_constants::UNFOCUS_COMMAND_HINT
                    } else {
                        "Restore full window"
                    },
                    Message::Restore,
                )
            } else {
                // adjust for window decorations
                let window_size = Size {
                    width: size.width + 3.0,
                    height: size.height + 3.0,
                };

                (
                    if window_state.is_command_key_down {
                        string_constants::FOCUS_COMMAND_HINT
                    } else {
                        "Focus"
                    },
                    Message::Maximize(pane, window_size),
                )
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

fn apply_script_edit(
    app: &mut MainWindow,
    edit_fn: impl FnOnce(&mut config::OriginalScriptDefinition),
) {
    if let Some(script_id) = &app.window_state.cursor_script {
        match script_id.script_type {
            EditScriptType::ScriptConfig => match &app.edit_data.window_edit_data {
                Some(window_edit_data) if window_edit_data.edit_type == ConfigEditType::Child => {
                    if let Some(config) = &mut app.app_config.child_config_body {
                        match &mut config.script_definitions[script_id.idx] {
                            config::ScriptDefinition::Original(script) => {
                                edit_fn(script);
                                app.edit_data.is_dirty = true;
                                update_config_cache(&mut app.app_config, &app.edit_data);
                            }
                            _ => {}
                        }
                    }
                }
                _ => match &mut app.app_config.script_definitions[script_id.idx] {
                    config::ScriptDefinition::Original(script) => {
                        edit_fn(script);
                        app.edit_data.is_dirty = true;
                        update_config_cache(&mut app.app_config, &app.edit_data);
                    }
                    _ => {}
                },
            },
            EditScriptType::ExecutionList => {
                match &mut app.execution_data.scripts_to_run[script_id.idx] {
                    config::ScriptDefinition::Original(script) => {
                        edit_fn(script);
                    }
                    _ => {}
                }
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
            if let Some(new_color) = color_utils::hex_to_rgb(&color_string) {
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
                &child_config.script_definitions
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
                &child_config.script_definitions
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
                    Some(config::ScriptDefinition::Original(_)) => {
                        return true;
                    }
                    Some(config::ScriptDefinition::Preset(_)) => {
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
    edit_data: &EditData,
    script: config::ScriptDefinition,
) -> Option<usize> {
    if let Some(config) = &mut app_config.child_config_body {
        config.script_definitions.push(script);
    } else {
        return None;
    }

    update_config_cache(app_config, edit_data);

    return if let Some(config) = &mut app_config.child_config_body {
        Some(config.script_definitions.len() - 1)
    } else {
        None
    };
}

fn is_script_missing_arguments(script: &config::ScriptDefinition) -> bool {
    match script {
        config::ScriptDefinition::Original(script) => is_original_script_missing_arguments(script),
        _ => false,
    }
}

fn is_original_script_missing_arguments(script: &config::OriginalScriptDefinition) -> bool {
    script.requires_arguments && script.arguments.is_empty()
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

fn make_script_copy(script: config::ScriptDefinition) -> config::ScriptDefinition {
    match script {
        config::ScriptDefinition::ReferenceToParent(_, _) => script,
        config::ScriptDefinition::Preset(preset) => {
            config::ScriptDefinition::Preset(config::ScriptPreset {
                uid: config::Guid::new(),
                name: format!("{} (copy)", preset.name),
                ..preset
            })
        }
        config::ScriptDefinition::Original(script) => {
            config::ScriptDefinition::Original(config::OriginalScriptDefinition {
                uid: config::Guid::new(),
                name: format!("{} (copy)", script.name),
                ..script
            })
        }
    }
}

fn update_config_cache(app_config: &mut config::AppConfig, edit_data: &EditData) {
    let is_looking_at_child_config = if let Some(window_edit_data) = &edit_data.window_edit_data {
        window_edit_data.edit_type == ConfigEditType::Child
    } else {
        app_config.child_config_body.is_some()
    };

    let binding = edit_data.script_filter.to_lowercase();
    let search_words = binding.split_whitespace().collect::<Vec<&str>>();

    let is_full_list = edit_data.window_edit_data.is_some();

    let is_script_filtered_out = |name: &str| -> bool {
        !search_words.is_empty() && {
            let mut is_filtered_out = false;
            let lowercase_name = name.to_lowercase();
            for search_word in &search_words {
                if !lowercase_name.contains(search_word) {
                    is_filtered_out = true;
                    break;
                }
            }
            is_filtered_out
        }
    };

    let result_list = &mut app_config.displayed_configs_list_cache;
    let paths = &app_config.paths;
    if is_looking_at_child_config {
        let child_config = app_config.child_config_body.as_ref().unwrap();
        let parent_script_definitions = &app_config.script_definitions;

        result_list.clear();
        for script_definition in &child_config.script_definitions {
            match script_definition {
                config::ScriptDefinition::ReferenceToParent(parent_script_uid, is_hidden) => {
                    let parent_script =
                        parent_script_definitions
                            .iter()
                            .find(|script| match script {
                                config::ScriptDefinition::Original(script) => {
                                    script.uid == *parent_script_uid
                                }
                                config::ScriptDefinition::Preset(preset) => {
                                    preset.uid == *parent_script_uid
                                }
                                _ => false,
                            });
                    match parent_script {
                        Some(parent_script) => {
                            let name = match &parent_script {
                                config::ScriptDefinition::ReferenceToParent(_, _) => {
                                    "[Error]".to_string()
                                }
                                config::ScriptDefinition::Original(script) => script.name.clone(),
                                config::ScriptDefinition::Preset(preset) => preset.name.clone(),
                            };
                            let icon = match &parent_script {
                                config::ScriptDefinition::ReferenceToParent(_, _) => {
                                    config::PathConfig::default()
                                }
                                config::ScriptDefinition::Original(script) => script.icon.clone(),
                                config::ScriptDefinition::Preset(preset) => preset.icon.clone(),
                            };
                            let is_script_hidden = *is_hidden || is_script_filtered_out(&name);
                            if is_full_list || !is_script_hidden {
                                result_list.push(config::ScriptListCacheRecord {
                                    name,
                                    full_icon_path: config::get_full_optional_path(paths, &icon),
                                    is_hidden: is_script_hidden,
                                    original_script_uid: parent_script_uid.clone(),
                                });
                            }
                        }
                        None => {
                            eprintln!(
                                "Failed to find parent script with uid {}",
                                parent_script_uid.data
                            )
                        }
                    }
                }
                config::ScriptDefinition::Original(script) => {
                    let is_script_hidden = is_script_filtered_out(&script.name);
                    if is_full_list || !is_script_hidden {
                        result_list.push(config::ScriptListCacheRecord {
                            name: script.name.clone(),
                            full_icon_path: config::get_full_optional_path(paths, &script.icon),
                            is_hidden: is_script_hidden,
                            original_script_uid: script.uid.clone(),
                        });
                    }
                }
                config::ScriptDefinition::Preset(preset) => {
                    let is_script_hidden = is_script_filtered_out(&preset.name);

                    if is_full_list || !is_script_hidden {
                        result_list.push(config::ScriptListCacheRecord {
                            name: preset.name.clone(),
                            full_icon_path: config::get_full_optional_path(paths, &preset.icon),
                            is_hidden: is_script_hidden,
                            original_script_uid: preset.uid.clone(),
                        });
                    }
                }
            }
        }
    } else {
        let script_definitions = &app_config.script_definitions;

        result_list.clear();
        for script_definition in script_definitions {
            match script_definition {
                config::ScriptDefinition::ReferenceToParent(_, _) => {}
                config::ScriptDefinition::Original(script) => {
                    let is_script_hidden = is_script_filtered_out(&script.name);
                    if is_full_list || !is_script_hidden {
                        result_list.push(config::ScriptListCacheRecord {
                            name: script.name.clone(),
                            full_icon_path: config::get_full_optional_path(paths, &script.icon),
                            is_hidden: is_script_hidden,
                            original_script_uid: script.uid.clone(),
                        });
                    }
                }
                config::ScriptDefinition::Preset(preset) => {
                    let is_script_hidden = is_script_filtered_out(&preset.name);
                    if is_full_list || !is_script_hidden {
                        result_list.push(config::ScriptListCacheRecord {
                            name: preset.name.clone(),
                            full_icon_path: config::get_full_optional_path(paths, &preset.icon),
                            is_hidden: is_script_hidden,
                            original_script_uid: preset.uid.clone(),
                        });
                    }
                }
            }
        }
    }
}

fn get_script_definition<'a>(
    app_config: &'a config::AppConfig,
    edit_data: &EditData,
    script_idx: usize,
) -> &'a config::ScriptDefinition {
    let is_looking_at_child_config = if let Some(window_edit_data) = &edit_data.window_edit_data {
        window_edit_data.edit_type == ConfigEditType::Child
    } else {
        app_config.child_config_body.is_some()
    };

    return if is_looking_at_child_config {
        &app_config
            .child_config_body
            .as_ref()
            .unwrap()
            .script_definitions[script_idx]
    } else {
        &app_config.script_definitions[script_idx]
    };
}

fn get_script_definition_mut<'a>(
    app_config: &'a mut config::AppConfig,
    edit_data: &EditData,
    script_idx: usize,
) -> &'a mut config::ScriptDefinition {
    let is_looking_at_child_config = if let Some(window_edit_data) = &edit_data.window_edit_data {
        window_edit_data.edit_type == ConfigEditType::Child
    } else {
        app_config.child_config_body.is_some()
    };

    return if is_looking_at_child_config {
        &mut app_config
            .child_config_body
            .as_mut()
            .unwrap()
            .script_definitions[script_idx]
    } else {
        &mut app_config.script_definitions[script_idx]
    };
}

fn add_script_to_config(app: &mut MainWindow, script: config::ScriptDefinition) {
    if let Some(window_edit_data) = &app.edit_data.window_edit_data {
        let script_idx = match window_edit_data.edit_type {
            ConfigEditType::Parent => {
                Some(add_script_to_parent_config(&mut app.app_config, script))
            }
            ConfigEditType::Child => {
                add_script_to_child_config(&mut app.app_config, &app.edit_data, script)
            }
        };

        app.edit_data
            .window_edit_data
            .as_mut()
            .unwrap()
            .is_editing_config = false;

        if let Some(script_idx) = script_idx {
            select_edited_script(app, script_idx);
            app.edit_data.is_dirty = true;
        }
    }
}

fn get_editing_preset<'a>(
    app_config: &'a mut config::AppConfig,
    edit_data: &EditData,
    window_state: &WindowState,
) -> Option<&'a mut config::ScriptPreset> {
    if let Some(script_id) = &window_state.cursor_script {
        if script_id.script_type == EditScriptType::ScriptConfig {
            let script_definition = get_script_definition_mut(app_config, edit_data, script_id.idx);
            if let config::ScriptDefinition::Preset(preset) = script_definition {
                return Some(preset);
            }
        }
    }
    return None;
}

fn enter_window_edit_mode(app: &mut MainWindow) {
    app.edit_data.window_edit_data = Some(WindowEditData::from_config(
        &app.app_config,
        false,
        if app.app_config.child_config_body.is_some() {
            ConfigEditType::Child
        } else {
            ConfigEditType::Parent
        },
    ));
    app.edit_data.script_filter = String::new();
    clean_script_selection(&mut app.window_state.cursor_script);
    update_config_cache(&mut app.app_config, &app.edit_data);
}

fn exit_window_edit_mode(app: &mut MainWindow) {
    app.edit_data.window_edit_data = None;
    clean_script_selection(&mut app.window_state.cursor_script);
    apply_theme(app);
    update_config_cache(&mut app.app_config, &app.edit_data);
}

fn run_scheduled_scripts(app: &mut MainWindow) {
    if app.execution_data.scripts_to_run.is_empty() {
        return;
    }

    if app
        .execution_data
        .scripts_to_run
        .iter()
        .any(|script| is_script_missing_arguments(script))
    {
        return;
    }

    if !execution::has_started_execution(&app.execution_data) {
        app.visual_caches.recent_logs.clear();
        clean_script_selection(&mut app.window_state.cursor_script);
        execution::run_scripts(&mut app.execution_data, &app.app_config);
    }

    app.edit_data.script_filter = String::new();
    update_config_cache(&mut app.app_config, &app.edit_data);
}

fn add_script_to_execution(
    app: &mut MainWindow,
    script_uid: config::Guid,
    should_focus: bool,
) -> bool {
    if execution::has_started_execution(&app.execution_data) {
        return false;
    }

    let original_script =
        config::get_original_script_definition_by_uid(&app.app_config, script_uid);

    let original_script = if let Some(original_script) = original_script {
        original_script
    } else {
        return false;
    };

    match original_script {
        config::ScriptDefinition::ReferenceToParent(_, _) => {
            return false;
        }
        config::ScriptDefinition::Original(_) => {
            execution::add_script_to_execution(&mut app.execution_data, original_script.clone());
        }
        config::ScriptDefinition::Preset(preset) => {
            for preset_item in &preset.items {
                if let Some(script) = config::get_original_script_definition_by_uid(
                    &app.app_config,
                    preset_item.uid.clone(),
                ) {
                    let mut new_script = script.clone();

                    match &mut new_script {
                        config::ScriptDefinition::Original(script) => {
                            if let Some(name) = &preset_item.name {
                                script.name = name.clone();
                            }

                            if let Some(arguments) = &preset_item.arguments {
                                script.arguments = arguments.clone();
                            }

                            if let Some(autorerun_count) = preset_item.autorerun_count {
                                script.autorerun_count = autorerun_count;
                            }

                            if let Some(ignore_previous_failures) =
                                preset_item.ignore_previous_failures
                            {
                                script.ignore_previous_failures = ignore_previous_failures;
                            }
                        }
                        _ => {}
                    };

                    execution::add_script_to_execution(&mut app.execution_data, new_script);
                }
            }
        }
    }

    if should_focus {
        let script_idx = app.execution_data.scripts_to_run.len() - 1;
        select_execution_script(app, script_idx);
        app.window_state.pane_focus = Some(app.pane_by_pane_type[&PaneVariant::ExecutionList]);
    }

    return true;
}

fn focus_filter(app: &mut MainWindow) -> Command<Message> {
    if app.panes.maximized().is_none() {
        if let Some(focus) = app.window_state.pane_focus {
            if &app.panes.panes[&focus].variant != &PaneVariant::ScriptList {
                app.window_state.pane_focus = Some(app.pane_by_pane_type[&PaneVariant::ScriptList]);
            }
        } else {
            app.window_state.pane_focus = Some(app.pane_by_pane_type[&PaneVariant::ScriptList]);
        }
    }
    return text_input::focus(FILTER_INPUT_ID.clone());
}

fn reschedule_scripts(app: &mut MainWindow) {
    if !execution::has_started_execution(&app.execution_data) {
        return;
    }
    join_execution_thread(&mut app.execution_data);

    execution::reset_execution_progress(&mut app.execution_data);
}

fn clear_scripts(app: &mut MainWindow) {
    join_execution_thread(&mut app.execution_data);
    execution::reset_execution_progress(&mut app.execution_data);
    app.execution_data.scripts_to_run.clear();
    clean_script_selection(&mut app.window_state.cursor_script);
}

fn select_edited_script(app: &mut MainWindow, script_idx: usize) {
    set_selected_script(
        &mut app.window_state.cursor_script,
        &app.execution_data,
        &get_script_definition_list_opt(&app.app_config, &app.edit_data.window_edit_data),
        &mut app.visual_caches,
        script_idx,
        EditScriptType::ScriptConfig,
    );
    if let Some(window_edit_data) = &mut app.edit_data.window_edit_data {
        window_edit_data.is_editing_config = false;
    }
}

fn select_execution_script(app: &mut MainWindow, script_idx: usize) {
    set_selected_script(
        &mut app.window_state.cursor_script,
        &app.execution_data,
        &app.execution_data.scripts_to_run,
        &mut app.visual_caches,
        script_idx,
        EditScriptType::ExecutionList,
    );
}

fn select_script_by_type(app: &mut MainWindow, script_idx: usize, script_type: EditScriptType) {
    match script_type {
        EditScriptType::ScriptConfig => select_edited_script(app, script_idx),
        EditScriptType::ExecutionList => select_execution_script(app, script_idx),
    }
}

fn move_config_script_up(app: &mut MainWindow, index: usize) {
    if let Some(window_edit_data) = &mut app.edit_data.window_edit_data {
        match window_edit_data.edit_type {
            ConfigEditType::Parent => {
                if index >= 1 && index < app.app_config.script_definitions.len() {
                    app.app_config.script_definitions.swap(index, index - 1);
                    app.edit_data.is_dirty = true;
                }
            }
            ConfigEditType::Child => {
                if let Some(child_config_body) = &mut app.app_config.child_config_body {
                    if index >= 1 && index < child_config_body.script_definitions.len() {
                        child_config_body.script_definitions.swap(index, index - 1);
                        app.edit_data.is_dirty = true;
                    }
                }
            }
        }
    }

    if let Some(edited_script) = &app.window_state.cursor_script {
        if edited_script.idx == index && index > 0 {
            select_edited_script(app, index - 1);
        }
    }

    update_config_cache(&mut app.app_config, &app.edit_data);
}

fn move_config_script_down(app: &mut MainWindow, index: usize) {
    if let Some(window_edit_data) = &mut app.edit_data.window_edit_data {
        match window_edit_data.edit_type {
            ConfigEditType::Parent => {
                if index + 1 < app.app_config.script_definitions.len() {
                    app.app_config.script_definitions.swap(index, index + 1);
                    app.edit_data.is_dirty = true;
                }
            }
            ConfigEditType::Child => {
                if let Some(child_config_body) = &mut app.app_config.child_config_body {
                    if index + 1 < child_config_body.script_definitions.len() {
                        child_config_body.script_definitions.swap(index, index + 1);
                        app.edit_data.is_dirty = true;
                    }
                }
            }
        }
    }

    if let Some(edited_script) = &app.window_state.cursor_script {
        if edited_script.idx == index
            && index + 1 < app.app_config.displayed_configs_list_cache.len()
        {
            select_edited_script(app, index + 1);
        }
    }

    update_config_cache(&mut app.app_config, &app.edit_data);
}

fn move_cursor(app: &mut MainWindow, is_up: bool) {
    if execution::has_started_execution(&app.execution_data) {
        return;
    }

    let focused_pane = if let Some(focus) = app.window_state.pane_focus {
        app.panes.panes[&focus].variant
    } else {
        return;
    };

    if focused_pane == PaneVariant::ScriptList || focused_pane == PaneVariant::ExecutionList {
        let pane_script_type = match focused_pane {
            PaneVariant::ScriptList => EditScriptType::ScriptConfig,
            PaneVariant::ExecutionList => EditScriptType::ExecutionList,
            _ => unreachable!(),
        };

        let scripts_count = match focused_pane {
            PaneVariant::ScriptList => app.app_config.displayed_configs_list_cache.len(),
            PaneVariant::ExecutionList => app.execution_data.scripts_to_run.len(),
            _ => unreachable!(),
        };

        if scripts_count == 0 {
            return;
        }

        let cursor_script_type = app
            .window_state
            .cursor_script
            .as_ref()
            .map(|x| x.script_type);
        let cursor_script_idx = app.window_state.cursor_script.as_ref().map(|x| x.idx);

        let next_selection = if cursor_script_idx.is_none()
            || (cursor_script_idx.is_some() && cursor_script_type != Some(pane_script_type))
        {
            if is_up {
                scripts_count - 1
            } else {
                0
            }
        } else {
            let cursor_script_idx = cursor_script_idx.unwrap_or_default();
            if is_up {
                if cursor_script_idx > 0 {
                    cursor_script_idx - 1
                } else {
                    scripts_count - 1
                }
            } else {
                if cursor_script_idx + 1 < scripts_count {
                    cursor_script_idx + 1
                } else {
                    0
                }
            }
        };

        select_script_by_type(app, next_selection, pane_script_type);
    }
}

fn get_next_pane_selection(app: &MainWindow, is_forward: bool) -> PaneVariant {
    if let Some(focus) = app.window_state.pane_focus {
        // try to predict what the user wants to do

        let is_editing = app.edit_data.window_edit_data.is_some();
        let selected_script_type = app
            .window_state
            .cursor_script
            .as_ref()
            .map(|s| &s.script_type);

        let have_scripts_in_execution = !app.execution_data.scripts_to_run.is_empty();
        let have_parameters_open = if let Some(selected_script_type) = selected_script_type {
            selected_script_type == &EditScriptType::ExecutionList || is_editing
        } else {
            false
        };
        let circle_clockwise = if let Some(selected_script_type) = selected_script_type {
            let editing_script =
                selected_script_type == &EditScriptType::ScriptConfig && is_editing;
            editing_script != is_forward
        } else {
            is_forward
        };

        if &app.panes.panes[&focus].variant == &PaneVariant::ScriptList {
            if !have_scripts_in_execution || !have_parameters_open {
                if !have_scripts_in_execution && !have_parameters_open {
                    PaneVariant::ScriptList
                } else if !have_scripts_in_execution {
                    PaneVariant::Parameters
                } else {
                    PaneVariant::ExecutionList
                }
            } else if circle_clockwise {
                PaneVariant::ExecutionList
            } else {
                PaneVariant::Parameters
            }
        } else if &app.panes.panes[&focus].variant == &PaneVariant::ExecutionList {
            if !have_parameters_open {
                PaneVariant::ScriptList
            } else if circle_clockwise {
                PaneVariant::Parameters
            } else {
                PaneVariant::ScriptList
            }
        } else if &app.panes.panes[&focus].variant == &PaneVariant::Parameters {
            if !have_scripts_in_execution {
                PaneVariant::ScriptList
            } else if circle_clockwise {
                PaneVariant::ScriptList
            } else {
                PaneVariant::ExecutionList
            }
        } else {
            // if we're in the log pane, go to the script list
            PaneVariant::ScriptList
        }
    } else {
        // if no panes selected, select ScriptList
        PaneVariant::ScriptList
    }
}

fn remove_script(app: &mut MainWindow, script_id: &EditScriptId) {
    match script_id.script_type {
        EditScriptType::ScriptConfig => {
            if let Some(window_edit_data) = &mut app.edit_data.window_edit_data {
                match window_edit_data.edit_type {
                    ConfigEditType::Parent => {
                        app.app_config.script_definitions.remove(script_id.idx);
                        app.edit_data.is_dirty = true;
                    }
                    ConfigEditType::Child => {
                        if let Some(config) = &mut app.app_config.child_config_body {
                            config.script_definitions.remove(script_id.idx);
                            app.edit_data.is_dirty = true;
                        }
                    }
                }
            }

            config::populate_parent_scripts_from_config(&mut app.app_config);
            update_config_cache(&mut app.app_config, &app.edit_data);
        }
        EditScriptType::ExecutionList => {
            execution::remove_script_from_execution(&mut app.execution_data, script_id.idx);
        }
    }
    clean_script_selection(&mut app.window_state.cursor_script);
}

fn maximize_pane(
    app: &mut MainWindow,
    pane: pane_grid::Pane,
    window_size: Size,
) -> Command<Message> {
    if app.window_state.pane_focus != Some(pane) {
        clean_script_selection(&mut app.window_state.cursor_script);
    }
    app.window_state.pane_focus = Some(pane);
    app.panes.maximize(&pane);
    app.window_state.has_maximized_pane = true;
    if !get_rewritable_config_opt(&app.app_config, &app.edit_data.window_edit_data).keep_window_size
    {
        app.window_state.full_window_size = window_size.clone();
        let size = app
            .panes
            .layout()
            .pane_regions(1.0, Size::new(window_size.width, window_size.height))
            .get(&pane)
            .unwrap() // tried to get an non-existing pane, this should never happen, so panic
            .clone();

        let elements_count = app.execution_data.scripts_to_run.len() as u32;
        let title_lines = if let Some(custom_title) = app.app_config.custom_title.as_ref() {
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

    return Command::none();
}

fn restore_window(app: &mut MainWindow) -> Command<Message> {
    app.window_state.has_maximized_pane = false;
    app.panes.restore();
    if !get_rewritable_config_opt(&app.app_config, &app.edit_data.window_edit_data).keep_window_size
    {
        return resize(
            app.window_state.full_window_size.width as u32,
            app.window_state.full_window_size.height as u32,
        );
    }
    return Command::none();
}
