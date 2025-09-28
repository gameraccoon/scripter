// Copyright (C) Pavel Grebnev 2023-2025
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use crate::color_utils;
use crate::config;
use crate::custom_keybinds;
use crate::drag_and_drop;
use crate::drag_and_drop::{DragAndDropList, DragResult, DropArea};
use crate::events;
use crate::execution_thread;
use crate::git_support;
use crate::keybind_editing;
use crate::main_window_utils::*;
use crate::main_window_widgets::*;
use crate::parallel_execution_manager;
use crate::style;
use crate::ui_icons;
use drag_and_drop::DropAreaState;

use iced::alignment::{self, Alignment};
use iced::event::listen_with;
use iced::mouse::Event;
use iced::theme::Theme;
use iced::widget::pane_grid::{self, Configuration, PaneGrid};
use iced::widget::scrollable::{AbsoluteOffset, RelativeOffset, Scrollbar};
use iced::widget::text::LineHeight;
use iced::widget::{
    button, checkbox, column, container, horizontal_rule, horizontal_space, image, image::Handle,
    opaque, pick_list, responsive, row, scrollable, stack, text, text_input, tooltip,
    vertical_space, Column, Space,
};
use iced::window::{self, request_user_attention};
use iced::{keyboard, mouse, ContentFit, Task};
use iced::{time, Size};
use iced::{Element, Length, Subscription};
use once_cell::sync::Lazy;
use std::cmp::PartialEq;
use std::collections::HashMap;
use std::mem::swap;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::{Duration, Instant};

static EMPTY_STRING: String = String::new();

const CONFIG_UPDATE_BEHAVIOR_PICK_LIST: &[config::ConfigUpdateBehavior] = &[
    config::ConfigUpdateBehavior::OnStartup,
    config::ConfigUpdateBehavior::OnManualSave,
];
impl std::fmt::Display for config::ConfigUpdateBehavior {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                config::ConfigUpdateBehavior::OnStartup => "On application startup",
                config::ConfigUpdateBehavior::OnManualSave => "Only on manual save",
            }
        )
    }
}

// these should be const not just static
pub(crate) static FILTER_INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);
static ARGUMENTS_INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);
static SCRIPTS_PANE_SCROLL_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);
static EXECUTIONS_PANE_SCROLL_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);
static LOGS_SCROLL_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);

// caches for visual elements content
pub(crate) struct VisualCaches {
    pub(crate) autorerun_count: String,
    pub(crate) is_custom_title_editing: bool,
    pub(crate) icons: ui_icons::IconCaches,
    pub(crate) keybind_hints: HashMap<keybind_editing::KeybindAssociatedData, String>,
    pub(crate) selected_execution_log: Option<parallel_execution_manager::ExecutionId>,
    pub(crate) git_branch_requester: Option<git_support::GitCurrentBranchRequester>,
    pub(crate) button_key_caches: ButtonKeyCaches,
    pub(crate) quick_launch_buttons: Vec<QuickLaunchButton>,
    pub(crate) enable_script_filtering: bool,
    pub(crate) enable_title_editing: bool,
    pub(crate) custom_theme: Option<config::CustomTheme>,
}

#[derive(Default)]
pub(crate) struct ButtonKeyCaches {
    pub(crate) last_stoppable_execution_id: Option<parallel_execution_manager::ExecutionId>,
    pub(crate) last_cleanable_execution_id: Option<parallel_execution_manager::ExecutionId>,
}

pub(crate) struct ScriptListCacheRecord {
    pub(crate) name: String,
    pub(crate) full_icon_path: Option<PathBuf>,
    pub(crate) is_hidden: bool,
    pub(crate) original_script_uid: config::Guid,
}

#[derive(Debug, Clone)]
pub(crate) struct EditData {
    // a string that is used to filter the list of scripts
    pub(crate) script_filter: String,
    // state of the global to the window editing mode
    pub(crate) window_edit_data: Option<WindowEditData>,
    // do we have unsaved changes
    pub(crate) is_dirty: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum EditScriptType {
    ScriptConfig,
    ExecutionList,
}

#[derive(Debug, Clone)]
pub(crate) struct EditScriptId {
    pub(crate) idx: usize,
    pub(crate) script_type: EditScriptType,
}

#[derive(Debug, Clone)]
pub(crate) struct WindowEditData {
    pub(crate) settings_edit_mode: Option<config::ConfigEditMode>,

    pub(crate) keybind_editing: keybind_editing::KeybindEditData,

    // theme color temp strings
    theme_color_background: String,
    theme_color_text: String,
    theme_color_primary: String,
    theme_color_success: String,
    theme_color_danger: String,
    theme_color_caption_text: String,
    theme_color_error_text: String,
}

pub(crate) struct QuickLaunchButton {
    pub(crate) icon: Handle,
    pub(crate) label: String,
    pub(crate) script_uid: config::Guid,
}

impl WindowEditData {
    pub(crate) fn from_config(
        config: &config::AppConfig,
        settings_edit_mode: Option<config::ConfigEditMode>,
    ) -> Self {
        let theme = if let Some(settings_edit_mode) = settings_edit_mode {
            if let Some(theme) =
                &config::get_rewritable_config(&config, settings_edit_mode).custom_theme
            {
                theme.clone()
            } else {
                config::CustomTheme::default()
            }
        } else {
            config::CustomTheme::default()
        };

        Self {
            settings_edit_mode,
            keybind_editing: keybind_editing::KeybindEditData::new(),
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

pub(crate) struct DragAndDropLists {
    pub(crate) script_list: DragAndDropList,
    pub(crate) edit_script_list: DragAndDropList,
    pub(crate) execution_edit_list: DragAndDropList,
}

pub(crate) struct DropAreas {
    pub(crate) execution_pane: DropArea,
    // indexed by the execution index
    pub(crate) running_executions: Vec<DropArea>,
}

pub(crate) struct WindowState {
    pub(crate) pane_focus: Option<pane_grid::Pane>,
    pub(crate) cursor_script: Option<EditScriptId>,
    pub(crate) full_window_size: Size,
    pub(crate) is_command_key_down: bool,
    is_alt_key_down: bool,
    mouse_position: iced::Point,
    pub(crate) has_maximized_pane: bool,
    pub(crate) drag_and_drop_lists: DragAndDropLists,
    pub(crate) drop_areas: DropAreas,
    pub(crate) dragged_script: Option<config::Guid>,
    pub(crate) script_list_scroll_offset: f32,
}

#[derive(Debug, Clone)]
pub(crate) enum WindowMessage {
    WindowResized(window::Id, Size),
    WindowOnMouseDown,
    WindowOnMouseUp,
    WindowOnMouseMove(iced::Point),
    WindowOnFileHovered,
    WindowOnFilesHoveredLeft,
    WindowOnFileDropped(PathBuf),
    PaneHeaderClicked(pane_grid::Pane),
    PaneHeaderDragged(pane_grid::DragEvent),
    PaneResized(pane_grid::ResizeEvent),
    OnScriptListScroll(scrollable::Viewport),
    OnExecutionListScroll(scrollable::Viewport),
    EnterFocusMode(pane_grid::Pane, Size),
    ExitFocusMode,
    MaximizeOrRestoreExecutionPane,
    AddScriptToExecutionOrRun(config::Guid),
    AddScriptToExecutionWithoutRunning(config::Guid),
    RunEditedScriptsInParallel,
    RunEditedScriptsAfterExecutionHotkey,
    RunEditedScriptsWithExecution(parallel_execution_manager::ExecutionId),
    StopScripts(parallel_execution_manager::ExecutionId),
    StopScriptsHotkey,
    EditExecutedScripts(parallel_execution_manager::ExecutionId),
    ClearEditedExecutionScripts,
    ClearFinishedExecutionScripts(parallel_execution_manager::ExecutionId),
    ClearExecutionScriptsHotkey,
    RescheduleScripts(parallel_execution_manager::ExecutionId),
    RescheduleScriptsHotkey,
    Tick(Instant),
    OpenScriptEditing(usize),
    CloseScriptEditing,
    DuplicateConfigScript(ConfigScriptId),
    RemoveConfigScript(ConfigScriptId),
    RemoveExecutionListScript(usize),
    AddScriptToConfig,
    EditScriptNameForConfig(ConfigScriptId, String),
    EditScriptNameForExecutionList(String),
    EditScriptCommand(ConfigScriptId, String),
    EditScriptCommandPathType(ConfigScriptId, config::PathType),
    EditScriptWorkingDirectory(ConfigScriptId, String),
    EditScriptWorkingDirectoryPathType(ConfigScriptId, config::PathType),
    EditScriptIconPath(ConfigScriptId, String),
    EditScriptIconPathType(ConfigScriptId, config::PathType),
    EditArgumentsLineForConfig(ConfigScriptId, String),
    EditArgumentsLineForScriptExecution(String),
    EditArgumentsRequirement(ConfigScriptId, config::ArgumentRequirement),
    EditArgumentsHint(ConfigScriptId, String),
    AddArgumentPlaceholder(ConfigScriptId),
    RemoveArgumentPlaceholder(ConfigScriptId, usize),
    EditArgumentPlaceholderName(ConfigScriptId, usize, String),
    EditArgumentPlaceholderPlaceholder(ConfigScriptId, usize, String),
    EditArgumentPlaceholderHint(ConfigScriptId, usize, String),
    ToggleArgumentPlaceholderIsRequired(ConfigScriptId, usize, bool),
    EditArgumentPlaceholderValueForConfig(ConfigScriptId, usize, String),
    EditArgumentPlaceholderValueForScriptExecution(usize, String),
    EditAutorerunCountForConfig(ConfigScriptId, String),
    EditAutorerunCountForExecutionList(String),
    EditReactionToPreviousFailuresForConfig(ConfigScriptId, config::ReactionToPreviousFailures),
    EditReactionToPreviousFailuresForExecutionList(config::ReactionToPreviousFailures),
    ToggleUseCustomExecutor(ConfigScriptId, bool),
    EditCustomExecutor(ConfigScriptId, String, usize),
    EditExecutorArguments(ConfigScriptId, String, usize),
    ToggleAutocleanOnSuccessForConfig(ConfigScriptId, bool),
    ToggleAutocleanOnSuccessForExecutionList(bool),
    ToggleIgnoreOutput(ConfigScriptId, bool),
    ToggleIsHidden(ConfigScriptId, bool),
    EnterWindowEditMode,
    ExitWindowEditMode,
    TrySwitchWindowEditMode,
    SaveConfigAndExitEditing,
    RevertConfigAndExitEditing,
    OpenScriptConfigEditing(usize),
    ToggleConfigEditing,
    SettingsToggleWindowStatusReactions(config::ConfigEditMode, bool),
    SettingsToggleKeepWindowSize(config::ConfigEditMode, bool),
    SettingsToggleScriptFiltering(config::ConfigEditMode, bool),
    SettingsToggleShowWorkingDirectory(config::ConfigEditMode, bool),
    SettingsToggleTitleEditing(config::ConfigEditMode, bool),
    SettingsUpdateBehaviorChanged(config::ConfigEditMode, config::ConfigUpdateBehavior),
    SettingsToggleShowCurrentGitBranch(config::ConfigEditMode, bool),
    SettingsToggleUseCustomTheme(config::ConfigEditMode, bool),
    SettingsEditThemeBackground(config::ConfigEditMode, String),
    SettingsEditThemeText(config::ConfigEditMode, String),
    SettingsEditThemePrimary(config::ConfigEditMode, String),
    SettingsEditThemeSuccess(config::ConfigEditMode, String),
    SettingsEditThemeDanger(config::ConfigEditMode, String),
    SettingsEditThemeCaptionText(config::ConfigEditMode, String),
    SettingsEditThemeErrorText(config::ConfigEditMode, String),
    SettingsSharedEditLocalConfigPath(String),
    SettingsSharedEditLocalConfigPathRelativeToScripter(config::PathType),
    SwitchToSharedSettingsConfig,
    SwitchToLocalSettingsConfig,
    ToggleScriptHidden(bool),
    CreateCopyOfSharedScript(usize),
    MoveToShared(usize),
    SaveAsPreset,
    ScriptFilterChanged(String),
    RequestCloseApp,
    FocusFilter,
    OnCommandKeyStateChanged(bool),
    OnAltKeyStateChanged(bool),
    MoveCursorUp,
    MoveCursorDown,
    MoveScriptDown,
    MoveScriptUp,
    CursorConfirm,
    RemoveCursorScript,
    SwitchPaneFocus(bool),
    SetExecutionListTitleEditing(bool),
    EditExecutionListTitle(String),
    OpenWithDefaultApplication(PathBuf),
    OpenUrl(String),
    OpenLogFolder(parallel_execution_manager::ExecutionId),
    ProcessKeyPress(keyboard::Key, keyboard::Modifiers),
    StartRecordingKeybind(keybind_editing::KeybindAssociatedData),
    StopRecordingKeybind,
    SelectExecutionLog(parallel_execution_manager::ExecutionId),
    OnQuickLaunchButtonPressed(config::Guid),
    AddToQuickLaunchPanel(config::Guid),
    RemoveFromQuickLaunchPanel(config::Guid),
}

pub(crate) struct MainWindow {
    pub(crate) panes: pane_grid::State<AppPane>,
    pub(crate) pane_by_pane_type: HashMap<PaneVariant, pane_grid::Pane>,
    pub(crate) execution_manager: parallel_execution_manager::ParallelExecutionManager,
    pub(crate) app_config: config::AppConfig,
    pub(crate) theme: Theme,
    pub(crate) visual_caches: VisualCaches,
    pub(crate) edit_data: EditData,
    pub(crate) window_state: WindowState,
    pub(crate) keybinds: custom_keybinds::CustomKeybinds<keybind_editing::KeybindAssociatedData>,
    pub(crate) displayed_configs_list_cache: Vec<ScriptListCacheRecord>,
}

impl MainWindow {
    pub(crate) fn new() -> (Self, Task<WindowMessage>) {
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
        let show_current_git_branch =
            config::get_current_rewritable_config(&app_config).show_current_git_branch;

        let mut main_window = MainWindow {
            panes,
            pane_by_pane_type,
            execution_manager: parallel_execution_manager::ParallelExecutionManager::new(),
            theme: get_theme(&app_config),
            app_config,
            visual_caches: VisualCaches {
                autorerun_count: String::new(),
                is_custom_title_editing: false,
                icons: ui_icons::IconCaches::new(),
                keybind_hints: HashMap::new(),
                selected_execution_log: None,
                git_branch_requester: if show_current_git_branch {
                    Some(git_support::GitCurrentBranchRequester::new())
                } else {
                    None
                },
                button_key_caches: ButtonKeyCaches::default(),
                quick_launch_buttons: Vec::new(),
                enable_script_filtering: false,
                enable_title_editing: false,
                custom_theme: None,
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
                is_alt_key_down: false,
                mouse_position: iced::Point::new(0.0, 0.0),
                has_maximized_pane: false,
                drag_and_drop_lists: DragAndDropLists {
                    script_list: DragAndDropList::new(
                        0,
                        drag_and_drop::StaticDragAreaParameters {
                            element_height: ONE_SCRIPT_LIST_ELEMENT_HEIGHT,
                            is_dragging_outside_allowed: true,
                            is_reordering_allowed: false,
                            are_bounds_dynamic: false,
                        },
                    ),
                    edit_script_list: DragAndDropList::new(
                        0,
                        drag_and_drop::StaticDragAreaParameters {
                            element_height: ONE_SCRIPT_LIST_ELEMENT_HEIGHT,
                            is_dragging_outside_allowed: false,
                            is_reordering_allowed: true,
                            are_bounds_dynamic: false,
                        },
                    ),
                    execution_edit_list: DragAndDropList::new(
                        0,
                        drag_and_drop::StaticDragAreaParameters {
                            element_height: ONE_EXECUTION_LIST_ELEMENT_HEIGHT,
                            is_dragging_outside_allowed: true,
                            is_reordering_allowed: true,
                            are_bounds_dynamic: true,
                        },
                    ),
                },
                drop_areas: DropAreas {
                    execution_pane: DropArea::new(),
                    running_executions: Vec::new(),
                },
                dragged_script: None,
                script_list_scroll_offset: 0.0,
            },
            keybinds: custom_keybinds::CustomKeybinds::new(),
            displayed_configs_list_cache: Vec::new(),
        };

        update_theme_icons(&mut main_window);
        update_config_cache(&mut main_window);
        keybind_editing::update_keybinds(&mut main_window);
        let edit_mode = config::get_main_edit_mode(&main_window.app_config);
        keybind_editing::update_keybind_visual_caches(&mut main_window, edit_mode);

        (main_window, Task::none())
    }

    pub(crate) fn title(&self) -> String {
        if self.edit_data.window_edit_data.is_some() {
            "scripter [Editing]".to_string()
        } else if self.execution_manager.has_any_execution_started() {
            if self.execution_manager.has_all_executions_finished() {
                if self.execution_manager.has_any_execution_failed() {
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

    pub(crate) fn update(&mut self, message: WindowMessage) -> Task<WindowMessage> {
        match message {
            WindowMessage::WindowResized(_window_id, size) => events::on_window_resized(self, size),
            WindowMessage::WindowOnMouseDown => {
                let mouse_pos = self.window_state.mouse_position;
                if !self.window_state.has_maximized_pane {
                    for_each_drag_area(self, |area| area.on_mouse_down(mouse_pos));
                }
            }
            WindowMessage::WindowOnMouseUp => {
                let mut dragged_script = self.window_state.dragged_script.take();
                let mouse_pos = self.window_state.mouse_position;

                if !self.edit_data.window_edit_data.is_some() {
                    self.window_state
                        .drag_and_drop_lists
                        .script_list
                        .on_mouse_up(mouse_pos);
                } else {
                    let drop_result = self
                        .window_state
                        .drag_and_drop_lists
                        .edit_script_list
                        .on_mouse_up(mouse_pos);
                    match drop_result {
                        drag_and_drop::DropResult::ItemChangedPosition(index, new_index) => {
                            move_config_script_to_index(self, index, new_index);
                            shift_script_selection(self, index, new_index);
                        }
                        _ => {}
                    }
                }

                let drop_result = self
                    .window_state
                    .drag_and_drop_lists
                    .execution_edit_list
                    .on_mouse_up(mouse_pos);
                match drop_result {
                    drag_and_drop::DropResult::ItemChangedPosition(index, new_index) => {
                        move_vec_element_to_index(
                            self.execution_manager.get_edited_scripts_mut(),
                            index,
                            new_index,
                        );
                        shift_script_selection(self, index, new_index);
                    }
                    _ => {}
                }

                let execution_edit_list_got_item_dropped = self
                    .window_state
                    .drop_areas
                    .execution_pane
                    .on_mouse_up(mouse_pos);

                let mut added_new_edited_scripts = false;
                if execution_edit_list_got_item_dropped {
                    if let Some(dragged_script) = dragged_script.take() {
                        add_script_to_execution(self, dragged_script, true);
                        added_new_edited_scripts = true;
                    }
                }

                let mut has_scheduled_new_script = false;
                for (i, drop_area) in &mut self
                    .window_state
                    .drop_areas
                    .running_executions
                    .iter_mut()
                    .enumerate()
                {
                    let just_dropped = drop_area.on_mouse_up(mouse_pos);
                    if just_dropped {
                        if let Some(dragged_script) = dragged_script.take() {
                            if let Some(script) = take_edited_execution_script(
                                &mut self.execution_manager,
                                dragged_script,
                                |script| !is_original_script_missing_arguments(script),
                            ) {
                                let execution_id =
                                    self.execution_manager.get_started_executions().get_key(i);

                                if let Some(execution_id) = execution_id {
                                    self.execution_manager
                                        .add_execution_scripts_to_running_execution(
                                            &mut self.app_config,
                                            execution_id,
                                            vec![script],
                                        );
                                    has_scheduled_new_script = true;
                                }
                            }
                        }
                    }
                }

                if has_scheduled_new_script {
                    update_edited_execution_list_script_number(self);
                    update_drag_and_drop_area_bounds(self);
                    clear_script_selection(&mut self.window_state.cursor_script);
                }

                if added_new_edited_scripts {
                    return scrollable::snap_to(
                        EXECUTIONS_PANE_SCROLL_ID.clone(),
                        RelativeOffset::END,
                    );
                }
            }
            WindowMessage::WindowOnMouseMove(position) => {
                self.window_state.mouse_position = position;

                let mut script_list_drop_pos = None;

                if !self.edit_data.window_edit_data.is_some() {
                    let move_result = self
                        .window_state
                        .drag_and_drop_lists
                        .script_list
                        .on_mouse_move(position);
                    match move_result {
                        DragResult::JustStartedDragging(script_idx) => {
                            if let Some(script) = self.displayed_configs_list_cache.get(script_idx)
                            {
                                self.window_state
                                    .drop_areas
                                    .execution_pane
                                    .on_started_dragging_compatible_element();
                                self.window_state.dragged_script =
                                    Some(script.original_script_uid.clone());
                            }
                        }
                        _ => {}
                    }
                } else {
                    let move_result = self
                        .window_state
                        .drag_and_drop_lists
                        .edit_script_list
                        .on_mouse_move(position);
                    match move_result {
                        DragResult::JustStartedDragging(script_idx) => {
                            if let Some(script) = self.displayed_configs_list_cache.get(script_idx)
                            {
                                self.window_state.dragged_script =
                                    Some(script.original_script_uid.clone());
                            }
                        }
                        DragResult::DraggingReordering(drop_pos) => {
                            script_list_drop_pos = Some(drop_pos);
                        }
                        _ => {}
                    }
                }

                let move_result = self
                    .window_state
                    .drag_and_drop_lists
                    .execution_edit_list
                    .on_mouse_move(position);
                match move_result {
                    DragResult::JustStartedDragging(script_idx) => {
                        if let Some(script_idx) =
                            self.execution_manager.get_edited_scripts().get(script_idx)
                        {
                            for drop_area in &mut self.window_state.drop_areas.running_executions {
                                drop_area.on_started_dragging_compatible_element();
                            }
                            self.window_state.dragged_script = Some(script_idx.uid.clone());
                        }
                    }
                    _ => {}
                }

                self.window_state
                    .drop_areas
                    .execution_pane
                    .on_mouse_move(position);

                for drop_area in &mut self.window_state.drop_areas.running_executions {
                    drop_area.on_mouse_move(position);
                }

                if let Some(script_list_drop_pos) = script_list_drop_pos {
                    return adjust_script_list_scroll_for_drop_pos(self, script_list_drop_pos);
                }
            }
            WindowMessage::WindowOnFileHovered => {}
            WindowMessage::WindowOnFilesHoveredLeft => {}
            WindowMessage::WindowOnFileDropped(file_path) => {
                if self.execution_manager.has_any_execution_started() {
                    return Task::none();
                }
                enter_window_edit_mode(self);
                create_script_from_file(self, file_path);

                return scrollable::snap_to(SCRIPTS_PANE_SCROLL_ID.clone(), RelativeOffset::END);
            }
            WindowMessage::PaneHeaderClicked(pane) => {
                self.window_state.pane_focus = Some(pane);
            }
            WindowMessage::PaneResized(pane_grid::ResizeEvent { split, ratio }) => {
                self.panes.resize(split, ratio);
                update_drag_and_drop_area_bounds(self);
                cancel_all_drag_and_drop_operations(self);
            }
            WindowMessage::OnScriptListScroll(viewport) => {
                let scroll_offset_y = viewport.absolute_offset().y;
                self.window_state.script_list_scroll_offset = scroll_offset_y;
                get_current_script_list_drag_and_drop(self).set_scroll_offset(scroll_offset_y);
            }
            WindowMessage::OnExecutionListScroll(viewport) => {
                let scroll_offset_y = viewport.absolute_offset().y;
                set_execution_lists_scroll_offset(self, scroll_offset_y);
                update_drag_and_drop_area_bounds(self);
            }
            WindowMessage::PaneHeaderDragged(drag_event) => {
                match drag_event {
                    pane_grid::DragEvent::Picked { pane: _pane } => {}
                    pane_grid::DragEvent::Dropped {
                        pane: _pane,
                        target: _target,
                    } => {
                        self.pane_by_pane_type = HashMap::new();
                        for pane in self.panes.panes.iter() {
                            self.pane_by_pane_type
                                .insert(pane.1.variant.clone(), *pane.0);
                        }
                    }
                    pane_grid::DragEvent::Canceled { pane: _pane } => {}
                };
            }
            WindowMessage::EnterFocusMode(pane, window_size) => {
                return maximize_pane(self, pane, window_size);
            }
            WindowMessage::ExitFocusMode => {
                return restore_window(self);
            }
            WindowMessage::MaximizeOrRestoreExecutionPane => {
                if self.window_state.has_maximized_pane {
                    return restore_window(self);
                } else {
                    if (self.execution_manager.has_any_execution_started()
                        || !self.execution_manager.get_edited_scripts().is_empty())
                        && self.edit_data.window_edit_data.is_none()
                    {
                        return maximize_pane(
                            self,
                            self.pane_by_pane_type[&PaneVariant::ExecutionList],
                            self.window_state.full_window_size,
                        );
                    }
                }
            }
            WindowMessage::AddScriptToExecutionOrRun(script_uid) => {
                // ignore if a script is being dropped on itself
                if self.window_state.dragged_script.is_some() {
                    return Task::none();
                }

                if self.window_state.is_command_key_down {
                    if self.window_state.is_alt_key_down {
                        let scripts = get_resulting_scripts_from_guid(&self.app_config, script_uid);
                        start_new_execution_from_provided_scripts(self, scripts);
                    } else {
                        try_add_script_to_execution_or_start_new(self, script_uid);
                    }
                } else {
                    add_script_to_execution(self, script_uid, true);
                    return scrollable::snap_to(
                        EXECUTIONS_PANE_SCROLL_ID.clone(),
                        RelativeOffset::END,
                    );
                }
            }
            WindowMessage::AddScriptToExecutionWithoutRunning(script_uid) => {
                add_script_to_execution(self, script_uid, true);
            }
            WindowMessage::RunEditedScriptsInParallel => {
                if !self.edit_data.window_edit_data.is_some() {
                    start_new_execution_from_edited_scripts(self);
                }
            }
            WindowMessage::RunEditedScriptsAfterExecutionHotkey => {
                try_add_edited_scripts_to_execution_or_start_new(self);
            }
            WindowMessage::RunEditedScriptsWithExecution(execution_id) => {
                add_edited_scripts_to_started_execution(self, execution_id);
            }
            WindowMessage::StopScripts(execution_id) => {
                self.execution_manager.request_stop_execution(execution_id);
            }
            WindowMessage::StopScriptsHotkey => {
                // we use the same script that we hinted visually
                if let Some(execution_id) = self
                    .visual_caches
                    .button_key_caches
                    .last_stoppable_execution_id
                {
                    self.execution_manager.request_stop_execution(execution_id);
                }
            }
            WindowMessage::EditExecutedScripts(execution_id) => {
                self.execution_manager
                    .request_edit_non_executed_scripts(execution_id);
            }
            WindowMessage::ClearEditedExecutionScripts => clear_edited_scripts(self),
            WindowMessage::ClearFinishedExecutionScripts(execution_id) => {
                remove_execution(self, execution_id);
            }
            WindowMessage::ClearExecutionScriptsHotkey => {
                if !self.execution_manager.get_edited_scripts().is_empty() {
                    clear_edited_scripts(self);
                } else {
                    clear_execution_scripts(self);
                }
            }
            WindowMessage::RescheduleScripts(execution_id) => {
                let mut execution = remove_execution(self, execution_id);
                if let Some(execution) = &mut execution {
                    execution
                        .get_scheduled_scripts_cache_mut()
                        .drain(..)
                        .for_each(|record| {
                            self.execution_manager
                                .add_execution_script_to_edited_list(record.script);
                        });
                }
                update_edited_execution_list_script_number(self);
            }
            WindowMessage::RescheduleScriptsHotkey => {
                // use the same script that we hinted visually
                let execution_to_reschedule = self
                    .visual_caches
                    .button_key_caches
                    .last_cleanable_execution_id
                    .and_then(|execution_id| {
                        self.execution_manager
                            .get_started_executions()
                            .get(execution_id)
                            .filter(|execution| execution.has_finished_execution())
                            .map(|_| execution_id)
                    });

                if let Some(execution_to_reschedule) = execution_to_reschedule {
                    let mut execution = remove_execution(self, execution_to_reschedule);
                    if let Some(execution) = &mut execution {
                        execution
                            .get_scheduled_scripts_cache_mut()
                            .drain(..)
                            .for_each(|record| {
                                self.execution_manager
                                    .add_execution_script_to_edited_list(record.script);
                            });
                    }
                }
            }
            WindowMessage::Tick(_now) => {
                let tick_result = self.execution_manager.tick(&self.app_config);
                if !tick_result.just_finished_executions.is_empty() {
                    for execution_id in tick_result.just_finished_executions {
                        if should_autoclean_on_success(self, execution_id) {
                            remove_execution(self, execution_id);
                        }
                    }

                    update_button_key_hint_caches(self);

                    if config::get_main_rewritable_config(&self.app_config).window_status_reactions
                    {
                        return window::get_oldest().and_then(|window_id| {
                            request_user_attention(
                                window_id,
                                Some(window::UserAttention::Informational),
                            )
                        });
                    }
                }

                if let Some(git_branch_requester) = &mut self.visual_caches.git_branch_requester {
                    git_branch_requester.update();
                }

                if tick_result.has_just_disconnected_executions {
                    events::on_execution_pane_content_height_decreased(self);
                    update_edited_execution_list_script_number(self);
                    update_drag_and_drop_area_bounds(self);
                }

                if !self.window_state.has_maximized_pane {
                    return scrollable::snap_to(LOGS_SCROLL_ID.clone(), RelativeOffset::END);
                }
            }
            WindowMessage::OpenScriptEditing(script_idx) => {
                select_execution_script(self, script_idx);
            }
            WindowMessage::CloseScriptEditing => {
                clear_script_selection(&mut self.window_state.cursor_script);
            }
            WindowMessage::DuplicateConfigScript(config_script_id) => {
                let script_definition_list = config::get_script_definition_list_mut(
                    &mut self.app_config,
                    config_script_id.edit_mode,
                );

                let (new_script, new_script_uid) =
                    make_script_copy(script_definition_list[config_script_id.idx].clone());
                script_definition_list.insert(config_script_id.idx + 1, new_script);

                // if we duplicated a shared script, make a reference to it in the local config
                if config_script_id.edit_mode == config::ConfigEditMode::Shared
                    && self.app_config.local_config_body.is_some()
                {
                    config::populate_shared_scripts_from_config(&mut self.app_config);
                }

                let idx =
                    get_top_level_edited_script_idx_by_uid(&mut self.app_config, &new_script_uid);

                if let Some(idx) = idx {
                    self.window_state.cursor_script = Some(EditScriptId {
                        idx,
                        script_type: EditScriptType::ScriptConfig,
                    });
                }
                update_config_cache(self);
                self.edit_data.is_dirty = true;
                return scroll_cursor_script_into_view(&self);
            }
            WindowMessage::RemoveConfigScript(config_script_id) => {
                remove_config_script(self, config_script_id)
            }
            WindowMessage::RemoveExecutionListScript(script_idx) => {
                remove_execution_list_script(self, script_idx);
                events::on_execution_pane_content_height_decreased(self);
            }
            WindowMessage::AddScriptToConfig => {
                let script = config::OriginalScriptDefinition::default();
                add_script_to_config(
                    self,
                    config::get_main_edit_mode(&self.app_config),
                    config::ScriptDefinition::Original(script),
                );

                return scrollable::snap_to(SCRIPTS_PANE_SCROLL_ID.clone(), RelativeOffset::END);
            }
            WindowMessage::EditScriptNameForConfig(config_script_id, new_name) => {
                if let Some(preset) = get_editing_preset(&mut self.app_config, config_script_id) {
                    preset.name = new_name;
                    self.edit_data.is_dirty = true;
                    update_config_cache(self);
                } else {
                    apply_config_script_edit(self, config_script_id, move |script| {
                        script.name = new_name
                    });
                }
            }
            WindowMessage::EditScriptNameForExecutionList(new_name) => {
                if let Some(script) = &self.window_state.cursor_script {
                    apply_execution_script_edit(self, script.idx, move |script| {
                        script.name = new_name
                    });
                }
            }
            WindowMessage::EditScriptCommand(config_script_id, new_command) => {
                apply_config_script_edit(self, config_script_id, move |script| {
                    script.command.path = new_command
                });
            }
            WindowMessage::EditScriptCommandPathType(config_script_id, value) => {
                apply_config_script_edit(self, config_script_id, |script| {
                    script.command.path_type = value
                });
            }
            WindowMessage::EditScriptWorkingDirectory(config_script_id, new_working_directory) => {
                apply_config_script_edit(self, config_script_id, move |script| {
                    script.working_directory.path = new_working_directory
                });
            }
            WindowMessage::EditScriptWorkingDirectoryPathType(config_script_id, value) => {
                apply_config_script_edit(self, config_script_id, |script| {
                    script.working_directory.path_type = value
                });
            }
            WindowMessage::EditScriptIconPath(config_script_id, new_icon_path) => {
                if let Some(preset) = get_editing_preset(&mut self.app_config, config_script_id) {
                    preset.icon.path = new_icon_path;
                    self.edit_data.is_dirty = true;
                    update_config_cache(self);
                } else {
                    apply_config_script_edit(self, config_script_id, move |script| {
                        script.icon.path = new_icon_path
                    });
                }
            }
            WindowMessage::EditScriptIconPathType(config_script_id, new_path_type) => {
                if let Some(preset) = get_editing_preset(&mut self.app_config, config_script_id) {
                    preset.icon.path_type = new_path_type;
                    self.edit_data.is_dirty = true;
                    update_config_cache(self);
                } else {
                    apply_config_script_edit(self, config_script_id, move |script| {
                        script.icon.path_type = new_path_type;
                    });
                }
            }
            WindowMessage::EditArgumentsLineForConfig(config_script_id, new_arguments) => {
                apply_config_script_edit(self, config_script_id, move |script| {
                    script.arguments_line = new_arguments;
                });
            }
            WindowMessage::EditArgumentsLineForScriptExecution(new_arguments) => {
                if let Some(script) = &self.window_state.cursor_script {
                    apply_execution_script_edit(self, script.idx, move |script| {
                        script.arguments_line = new_arguments;
                    });
                }
            }
            WindowMessage::EditArgumentsRequirement(config_script_id, new_requirement) => {
                apply_config_script_edit(self, config_script_id, move |script| {
                    script.arguments_requirement = new_requirement
                });
            }
            WindowMessage::EditArgumentsHint(config_script_id, new_arguments_hint) => {
                apply_config_script_edit(self, config_script_id, move |script| {
                    script.arguments_hint = new_arguments_hint
                });
            }
            WindowMessage::AddArgumentPlaceholder(config_script_id) => {
                apply_config_script_edit(self, config_script_id, |script| {
                    script
                        .argument_placeholders
                        .push(config::ArgumentPlaceholder {
                            name: String::new(),
                            placeholder: String::new(),
                            value: String::new(),
                            hint: String::new(),
                            is_required: false,
                        });
                });
            }
            WindowMessage::RemoveArgumentPlaceholder(config_script_id, index) => {
                apply_config_script_edit(self, config_script_id, move |script| {
                    if index < script.argument_placeholders.len() {
                        script.argument_placeholders.remove(index);
                    }
                });
            }
            WindowMessage::EditArgumentPlaceholderName(config_script_id, index, new_name) => {
                apply_config_script_edit(self, config_script_id, move |script| {
                    if let Some(placeholder) = script.argument_placeholders.get_mut(index) {
                        placeholder.name = new_name;
                    }
                });
            }
            WindowMessage::EditArgumentPlaceholderPlaceholder(
                config_script_id,
                index,
                new_placeholder,
            ) => {
                apply_config_script_edit(self, config_script_id, move |script| {
                    if let Some(placeholder) = script.argument_placeholders.get_mut(index) {
                        placeholder.placeholder = new_placeholder;
                    }
                });
            }
            WindowMessage::ToggleArgumentPlaceholderIsRequired(
                config_script_id,
                index,
                is_required,
            ) => {
                apply_config_script_edit(self, config_script_id, move |script| {
                    if let Some(placeholder) = script.argument_placeholders.get_mut(index) {
                        placeholder.is_required = is_required;
                    }
                });
            }
            WindowMessage::EditArgumentPlaceholderHint(config_script_id, index, new_hint) => {
                apply_config_script_edit(self, config_script_id, move |script| {
                    if let Some(placeholder) = script.argument_placeholders.get_mut(index) {
                        placeholder.hint = new_hint;
                    }
                });
            }
            WindowMessage::EditArgumentPlaceholderValueForConfig(
                config_script_id,
                index,
                new_value,
            ) => {
                apply_config_script_edit(self, config_script_id, move |script| {
                    if let Some(placeholder) = script.argument_placeholders.get_mut(index) {
                        placeholder.value = new_value;
                    }
                });
            }
            WindowMessage::EditArgumentPlaceholderValueForScriptExecution(index, new_value) => {
                if let Some(script) = &self.window_state.cursor_script {
                    apply_execution_script_edit(self, script.idx, move |script| {
                        if let Some(placeholder) = script.argument_placeholders.get_mut(index) {
                            placeholder.value = new_value;
                        }
                    });
                }
            }
            WindowMessage::EditAutorerunCountForConfig(
                config_script_id,
                new_autorerun_count_str,
            ) => {
                let new_autorerun_count =
                    update_autorerun_count_text(self, new_autorerun_count_str);

                if let Some(new_autorerun_count) = new_autorerun_count {
                    apply_config_script_edit(self, config_script_id, |script| {
                        script.autorerun_count = new_autorerun_count
                    });
                }
            }
            WindowMessage::EditAutorerunCountForExecutionList(new_autorerun_count_str) => {
                let new_autorerun_count =
                    update_autorerun_count_text(self, new_autorerun_count_str);

                if let Some(new_autorerun_count) = new_autorerun_count {
                    if let Some(script) = &self.window_state.cursor_script {
                        apply_execution_script_edit(self, script.idx, |script| {
                            script.autorerun_count = new_autorerun_count;
                        });
                    }
                }
            }
            WindowMessage::EditReactionToPreviousFailuresForConfig(config_script_id, value) => {
                apply_config_script_edit(self, config_script_id, |script| {
                    script.reaction_to_previous_failures = value
                });
            }
            WindowMessage::EditReactionToPreviousFailuresForExecutionList(value) => {
                if let Some(script) = &self.window_state.cursor_script {
                    apply_execution_script_edit(self, script.idx, |script| {
                        script.reaction_to_previous_failures = value
                    });
                }
            }
            WindowMessage::ToggleUseCustomExecutor(config_script_id, should_use_custom) => {
                apply_config_script_edit(self, config_script_id, |script| {
                    if script.custom_executor.is_none() && should_use_custom {
                        script.custom_executor = Some(config::get_default_executor())
                    } else if !should_use_custom {
                        script.custom_executor = None;
                    }
                });
            }
            WindowMessage::EditCustomExecutor(config_script_id, value, index) => {
                apply_config_script_edit(self, config_script_id, |script| {
                    if let Some(executor) = &mut script.custom_executor {
                        if value.is_empty() && index + 1 == executor.len() {
                            executor.pop();
                        } else if !value.is_empty() && index == executor.len() {
                            executor.push(value);
                        } else if index < executor.len() {
                            executor[index] = value;
                        }
                    }
                });
            }
            WindowMessage::EditExecutorArguments(
                config_script_id,
                new_executor_arguments,
                index,
            ) => {
                apply_config_script_edit(self, config_script_id, |script| {
                    if new_executor_arguments.is_empty()
                        && index + 1 == script.executor_arguments.len()
                    {
                        script.executor_arguments.pop();
                    } else if !new_executor_arguments.is_empty()
                        && index == script.executor_arguments.len()
                    {
                        script.executor_arguments.push(new_executor_arguments);
                    } else if index < script.executor_arguments.len() {
                        script.executor_arguments[index] = new_executor_arguments;
                    }
                });
            }
            WindowMessage::ToggleAutocleanOnSuccessForConfig(config_script_id, value) => {
                apply_config_script_edit(self, config_script_id, |script| {
                    script.autoclean_on_success = value
                });
            }
            WindowMessage::ToggleAutocleanOnSuccessForExecutionList(value) => {
                if let Some(script) = &self.window_state.cursor_script {
                    apply_execution_script_edit(self, script.idx, |script| {
                        script.autoclean_on_success = value
                    });
                }
            }
            WindowMessage::ToggleIgnoreOutput(config_script_id, value) => {
                apply_config_script_edit(self, config_script_id, |script| {
                    script.ignore_output = value
                });
            }
            WindowMessage::ToggleIsHidden(config_script_id, value) => {
                apply_config_script_edit(self, config_script_id, |script| script.is_hidden = value);
            }
            WindowMessage::EnterWindowEditMode => enter_window_edit_mode(self),
            WindowMessage::ExitWindowEditMode => exit_window_edit_mode(self),
            WindowMessage::TrySwitchWindowEditMode => {
                if !self.execution_manager.has_any_execution_started() {
                    if !self.edit_data.window_edit_data.is_some() {
                        enter_window_edit_mode(self);
                    } else {
                        exit_window_edit_mode(self);
                    }
                }
            }
            WindowMessage::SaveConfigAndExitEditing => {
                let has_saved = config::save_config_to_file(&self.app_config);
                if has_saved {
                    self.app_config = config::read_config();
                    self.edit_data.is_dirty = false;
                    update_config_cache(self);
                    keybind_editing::update_keybinds(self);
                    let edit_mode = config::get_main_edit_mode(&self.app_config);
                    keybind_editing::update_keybind_visual_caches(self, edit_mode);
                    exit_window_edit_mode(self);
                }
            }
            WindowMessage::RevertConfigAndExitEditing => {
                self.app_config = config::read_config();
                self.edit_data.window_edit_data =
                    Some(WindowEditData::from_config(&self.app_config, None));
                config::populate_shared_scripts_from_config(&mut self.app_config);
                apply_theme(self);
                self.edit_data.is_dirty = false;
                clear_script_selection(&mut self.window_state.cursor_script);
                update_config_cache(self);
                keybind_editing::update_keybinds(self);
                let edit_mode = config::get_main_edit_mode(&self.app_config);
                keybind_editing::update_keybind_visual_caches(self, edit_mode);
                exit_window_edit_mode(self);
            }
            WindowMessage::OpenScriptConfigEditing(script_idx) => {
                select_edited_script(
                    self,
                    ConfigScriptId {
                        idx: script_idx,
                        edit_mode: config::get_main_edit_mode(&self.app_config),
                    },
                );
            }
            WindowMessage::ToggleConfigEditing => {
                switch_to_editing_settings_config(
                    self,
                    config::get_main_edit_mode(&self.app_config),
                );
            }
            WindowMessage::SettingsToggleWindowStatusReactions(edit_mode, is_checked) => {
                config::get_rewritable_config_mut(&mut self.app_config, edit_mode)
                    .window_status_reactions = is_checked;
                self.edit_data.is_dirty = true;
            }
            WindowMessage::SettingsToggleKeepWindowSize(edit_mode, is_checked) => {
                config::get_rewritable_config_mut(&mut self.app_config, edit_mode)
                    .keep_window_size = is_checked;
                self.edit_data.is_dirty = true;
            }
            WindowMessage::SettingsToggleScriptFiltering(edit_mode, is_checked) => {
                config::get_rewritable_config_mut(&mut self.app_config, edit_mode)
                    .enable_script_filtering = is_checked;
                self.edit_data.is_dirty = true;
            }
            WindowMessage::SettingsToggleShowWorkingDirectory(edit_mode, is_checked) => {
                config::get_rewritable_config_mut(&mut self.app_config, edit_mode)
                    .show_working_directory = is_checked;
                self.edit_data.is_dirty = true;
            }
            WindowMessage::SettingsToggleTitleEditing(edit_mode, is_checked) => {
                config::get_rewritable_config_mut(&mut self.app_config, edit_mode)
                    .enable_title_editing = is_checked;
                self.visual_caches.enable_title_editing = is_checked;
                self.edit_data.is_dirty = true;
            }
            WindowMessage::SettingsUpdateBehaviorChanged(edit_mode, value) => {
                config::get_rewritable_config_mut(&mut self.app_config, edit_mode)
                    .config_version_update_behavior = value;
                self.edit_data.is_dirty = true;
            }
            WindowMessage::SettingsToggleShowCurrentGitBranch(edit_mode, is_checked) => {
                config::get_rewritable_config_mut(&mut self.app_config, edit_mode)
                    .show_current_git_branch = is_checked;
                self.edit_data.is_dirty = true;
                update_git_branch_visibility(self);
            }
            WindowMessage::SettingsToggleUseCustomTheme(edit_mode, is_checked) => {
                config::get_rewritable_config_mut(&mut self.app_config, edit_mode).custom_theme =
                    if is_checked {
                        Some(
                            if let Some(window_edit_data) = &self.edit_data.window_edit_data {
                                config::CustomTheme {
                                    background: color_utils::hex_to_rgb(
                                        &window_edit_data.theme_color_background,
                                    )
                                    .unwrap_or_default(),
                                    text: color_utils::hex_to_rgb(
                                        &window_edit_data.theme_color_text,
                                    )
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
            WindowMessage::SettingsEditThemeBackground(edit_mode, new_value) => {
                apply_theme_color_from_string(
                    self,
                    edit_mode,
                    new_value,
                    |theme, value| theme.background = value,
                    |edit_data, value| {
                        edit_data.theme_color_background = value;
                        edit_data.theme_color_background.clone()
                    },
                );
            }
            WindowMessage::SettingsEditThemeText(edit_mode, new_value) => {
                apply_theme_color_from_string(
                    self,
                    edit_mode,
                    new_value,
                    |theme, value| theme.text = value,
                    |edit_data, value| {
                        edit_data.theme_color_text = value;
                        edit_data.theme_color_text.clone()
                    },
                );
            }
            WindowMessage::SettingsEditThemePrimary(edit_mode, new_value) => {
                apply_theme_color_from_string(
                    self,
                    edit_mode,
                    new_value,
                    |theme, value| theme.primary = value,
                    |edit_data, value| {
                        edit_data.theme_color_primary = value;
                        edit_data.theme_color_primary.clone()
                    },
                );
            }
            WindowMessage::SettingsEditThemeSuccess(edit_mode, new_value) => {
                apply_theme_color_from_string(
                    self,
                    edit_mode,
                    new_value,
                    |theme, value| theme.success = value,
                    |edit_data, value| {
                        edit_data.theme_color_success = value;
                        edit_data.theme_color_success.clone()
                    },
                );
            }
            WindowMessage::SettingsEditThemeDanger(edit_mode, new_value) => {
                apply_theme_color_from_string(
                    self,
                    edit_mode,
                    new_value,
                    |theme, value| theme.danger = value,
                    |edit_data, value| {
                        edit_data.theme_color_danger = value;
                        edit_data.theme_color_danger.clone()
                    },
                );
                self.edit_data.is_dirty = true;
            }
            WindowMessage::SettingsEditThemeCaptionText(edit_mode, new_value) => {
                apply_theme_color_from_string(
                    self,
                    edit_mode,
                    new_value,
                    |theme, value| theme.caption_text = value,
                    |edit_data, value| {
                        edit_data.theme_color_caption_text = value;
                        edit_data.theme_color_caption_text.clone()
                    },
                );
                self.edit_data.is_dirty = true;
            }
            WindowMessage::SettingsEditThemeErrorText(edit_mode, new_value) => {
                apply_theme_color_from_string(
                    self,
                    edit_mode,
                    new_value,
                    |theme, value| theme.error_text = value,
                    |edit_data, value| {
                        edit_data.theme_color_error_text = value;
                        edit_data.theme_color_error_text.clone()
                    },
                );
                self.edit_data.is_dirty = true;
            }
            WindowMessage::SettingsSharedEditLocalConfigPath(new_value) => {
                self.app_config.local_config_path.path = new_value;
                self.edit_data.is_dirty = true;
            }
            WindowMessage::SettingsSharedEditLocalConfigPathRelativeToScripter(new_path_type) => {
                self.app_config.local_config_path.path_type = new_path_type;
                self.edit_data.is_dirty = true;
            }
            WindowMessage::SwitchToSharedSettingsConfig => {
                switch_to_editing_settings_config(self, config::ConfigEditMode::Shared);
            }
            WindowMessage::SwitchToLocalSettingsConfig => {
                switch_to_editing_settings_config(self, config::ConfigEditMode::Local);
            }
            WindowMessage::ToggleScriptHidden(is_hidden) => {
                let Some(script_id) = &mut self.window_state.cursor_script else {
                    return Task::none();
                };

                if let Some(config) = &mut self.app_config.local_config_body {
                    let Some(script) = config.script_definitions.get_mut(script_id.idx) else {
                        return Task::none();
                    };

                    match script {
                        config::ScriptDefinition::ReferenceToShared(reference) => {
                            reference.is_hidden = is_hidden;
                            self.edit_data.is_dirty = true;
                        }
                        _ => {}
                    }
                }
                update_config_cache(self);
            }
            WindowMessage::CreateCopyOfSharedScript(script_idx) => {
                let script = if let Some(config) = &self.app_config.local_config_body {
                    if let Some(script) = config.script_definitions.get(script_idx) {
                        script
                    } else {
                        return Task::none();
                    }
                } else {
                    return Task::none();
                };

                let new_script = match script {
                    config::ScriptDefinition::ReferenceToShared(reference) => {
                        if let Some((script, _idx)) = config::get_original_script_definition_by_uid(
                            &self.app_config,
                            &reference.uid,
                        ) {
                            let mut script = script.clone();
                            match &mut script {
                                config::ScriptDefinition::Original(original_script) => {
                                    original_script.uid = config::Guid::new();
                                    original_script.name =
                                        format!("{} (copy)", original_script.name);
                                    script
                                }
                                config::ScriptDefinition::Preset(preset) => {
                                    preset.uid = config::Guid::new();
                                    preset.name = format!("{} (copy)", preset.name);
                                    script
                                }
                                config::ScriptDefinition::ReferenceToShared(_) => {
                                    return Task::none();
                                }
                            }
                        } else {
                            return Task::none();
                        }
                    }
                    _ => {
                        return Task::none();
                    }
                };

                if let Some(config) = &mut self.app_config.local_config_body {
                    config.script_definitions.insert(script_idx + 1, new_script);
                    select_edited_script(
                        self,
                        ConfigScriptId {
                            idx: script_idx + 1,
                            edit_mode: config::ConfigEditMode::Local,
                        },
                    );
                    self.edit_data.is_dirty = true;
                }
                update_config_cache(self);
                return scroll_cursor_script_into_view(&self);
            }
            WindowMessage::MoveToShared(script_idx) => {
                if let Some(config) = &mut self.app_config.local_config_body {
                    if config.script_definitions.len() <= script_idx {
                        return Task::none();
                    }

                    let insert_position = find_best_shared_script_insert_position(
                        &config.script_definitions,
                        &self.app_config.script_definitions,
                        script_idx,
                    );

                    if let Some(script) = config.script_definitions.get_mut(script_idx) {
                        let mut replacement_script = match script {
                            config::ScriptDefinition::Original(definition) => {
                                config::ScriptDefinition::ReferenceToShared(
                                    config::ReferenceToSharedScript {
                                        uid: definition.uid.clone(),
                                        is_hidden: false,
                                    },
                                )
                            }
                            config::ScriptDefinition::Preset(preset) => {
                                config::ScriptDefinition::ReferenceToShared(
                                    config::ReferenceToSharedScript {
                                        uid: preset.uid.clone(),
                                        is_hidden: false,
                                    },
                                )
                            }
                            _ => {
                                return Task::none();
                            }
                        };

                        swap(script, &mut replacement_script);
                        self.app_config
                            .script_definitions
                            .insert(insert_position, replacement_script);
                        self.edit_data.is_dirty = true;
                    }
                }
            }
            WindowMessage::SaveAsPreset => {
                let mut preset = config::ScriptPreset {
                    uid: config::Guid::new(),
                    name: "new preset".to_string(),
                    icon: Default::default(),
                    items: vec![],
                };

                for execution_script in self.execution_manager.get_edited_scripts() {
                    let original_script = config::get_original_script_definition_by_uid(
                        &self.app_config,
                        &execution_script.original.uid,
                    );

                    let script = &execution_script.original;

                    let original_script = if let Some(original_script_tuple) = original_script {
                        match original_script_tuple.0 {
                            config::ScriptDefinition::ReferenceToShared(reference) => {
                                config::get_original_script_definition_by_uid(
                                    &self.app_config,
                                    &reference.uid,
                                )
                            }
                            _ => Some(original_script_tuple),
                        }
                    } else {
                        None
                    };

                    let original_script = if let Some((original_script, _idx)) = original_script {
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

                    let arguments_line = if let Some(original_script) = &original_script {
                        if original_script.arguments_line == script.arguments_line {
                            None
                        } else {
                            Some(script.arguments_line.clone())
                        }
                    } else {
                        Some(script.arguments_line.clone())
                    };

                    let executor_arguments = if let Some(original_script) = &original_script {
                        if original_script.executor_arguments == script.executor_arguments {
                            None
                        } else {
                            Some(script.executor_arguments.clone())
                        }
                    } else {
                        Some(script.executor_arguments.clone())
                    };

                    let mut overridden_placeholder_values = HashMap::new();
                    for placeholder in &script.argument_placeholders {
                        let original_placeholder_value =
                            original_script.as_ref().and_then(|original_script| {
                                original_script
                                    .argument_placeholders
                                    .iter()
                                    .find(|placeholder| {
                                        placeholder.placeholder == placeholder.placeholder
                                    })
                                    .map(|placeholder| placeholder.value.clone())
                            });

                        if let Some(original_placeholder_value) = original_placeholder_value {
                            if original_placeholder_value != placeholder.value {
                                overridden_placeholder_values.insert(
                                    placeholder.placeholder.clone(),
                                    placeholder.value.clone(),
                                );
                            }
                        }
                    }

                    let autorerun_count = if let Some(original_script) = &original_script {
                        if original_script.autorerun_count == script.autorerun_count {
                            None
                        } else {
                            Some(script.autorerun_count)
                        }
                    } else {
                        Some(script.autorerun_count)
                    };

                    let reaction_to_previous_failures =
                        if let Some(original_script) = original_script {
                            if original_script.reaction_to_previous_failures
                                == script.reaction_to_previous_failures
                            {
                                None
                            } else {
                                Some(script.reaction_to_previous_failures)
                            }
                        } else {
                            Some(script.reaction_to_previous_failures)
                        };

                    let autoclean_on_success = if let Some(original_script) = original_script {
                        if original_script.autoclean_on_success == script.autoclean_on_success {
                            None
                        } else {
                            Some(script.autoclean_on_success)
                        }
                    } else {
                        Some(script.autoclean_on_success)
                    };

                    preset.items.push(config::PresetItem {
                        uid: script.uid.clone(),
                        name,
                        arguments_line,
                        executor_arguments,
                        overridden_placeholder_values,
                        autorerun_count,
                        reaction_to_previous_failures,
                        autoclean_on_success,
                    });
                }

                add_script_to_config(
                    self,
                    config::get_main_edit_mode(&self.app_config),
                    config::ScriptDefinition::Preset(preset),
                );

                return scrollable::snap_to(SCRIPTS_PANE_SCROLL_ID.clone(), RelativeOffset::END);
            }
            WindowMessage::ScriptFilterChanged(new_filter_value) => {
                self.edit_data.script_filter = new_filter_value;
                update_config_cache(self);
                clear_script_selection(&mut self.window_state.cursor_script);
                events::on_script_list_pane_content_height_decreased(self);
            }
            WindowMessage::RequestCloseApp => {
                let exit_thread_command = || {
                    Task::perform(async {}, |()| {
                        std::process::exit(0);
                    })
                };

                if self.execution_manager.has_any_execution_started() {
                    if self.execution_manager.has_all_executions_finished() {
                        if !self
                            .execution_manager
                            .is_waiting_on_any_execution_to_finish()
                        {
                            return exit_thread_command();
                        }
                    }
                } else {
                    return exit_thread_command();
                }
            }
            WindowMessage::FocusFilter => {
                if !self.window_state.has_maximized_pane {
                    self.window_state.is_command_key_down = false;
                    self.window_state.is_alt_key_down = false;
                    return focus_filter(self);
                }
            }
            WindowMessage::OnCommandKeyStateChanged(is_command_key_down) => {
                self.window_state.is_command_key_down = is_command_key_down;
            }
            WindowMessage::OnAltKeyStateChanged(is_alt_key_down) => {
                self.window_state.is_alt_key_down = is_alt_key_down;
            }
            WindowMessage::MoveCursorUp => {
                move_cursor(self, true);
                return scroll_cursor_script_into_view(&self);
            }
            WindowMessage::MoveCursorDown => {
                move_cursor(self, false);
                return scroll_cursor_script_into_view(&self);
            }
            WindowMessage::MoveScriptDown => {
                let focused_pane = if let Some(focus) = self.window_state.pane_focus {
                    self.panes.panes[&focus].variant
                } else {
                    return Task::none();
                };

                let mut config_script_scroll_offset = None;

                if focused_pane == PaneVariant::ScriptList {
                    if self.edit_data.window_edit_data.is_some() {
                        if let Some(edited_script) = &self.window_state.cursor_script {
                            let new_index = move_config_script_down(self, edited_script.idx);
                            config_script_scroll_offset =
                                get_script_config_bring_into_view_scroll_offset(
                                    self, new_index, 10.0,
                                );
                        }
                    }
                } else if focused_pane == PaneVariant::ExecutionList {
                    if let Some(cursor_script) = &self.window_state.cursor_script {
                        if cursor_script.script_type == EditScriptType::ExecutionList {
                            if cursor_script.idx + 1
                                >= self.execution_manager.get_edited_scripts().len()
                            {
                                return Task::none();
                            }
                            self.execution_manager
                                .get_edited_scripts_mut()
                                .swap(cursor_script.idx, cursor_script.idx + 1);
                            select_execution_script(self, cursor_script.idx + 1);
                        }
                    }
                }

                if let Some(scroll_offset) = config_script_scroll_offset {
                    return scrollable::scroll_to(
                        SCRIPTS_PANE_SCROLL_ID.clone(),
                        AbsoluteOffset {
                            x: 0.0,
                            y: scroll_offset,
                        },
                    );
                }
            }
            WindowMessage::MoveScriptUp => {
                let focused_pane = if let Some(focus) = self.window_state.pane_focus {
                    self.panes.panes[&focus].variant
                } else {
                    return Task::none();
                };

                let mut config_script_scroll_offset = None;

                if focused_pane == PaneVariant::ScriptList {
                    if self.edit_data.window_edit_data.is_some() {
                        if let Some(edited_script) = &self.window_state.cursor_script {
                            let new_index = move_config_script_up(self, edited_script.idx);
                            config_script_scroll_offset =
                                get_script_config_bring_into_view_scroll_offset(
                                    self, new_index, 10.0,
                                );
                        }
                    }
                } else if focused_pane == PaneVariant::ExecutionList {
                    if let Some(cursor_script) = &self.window_state.cursor_script {
                        if cursor_script.script_type == EditScriptType::ExecutionList {
                            if cursor_script.idx == 0 {
                                return Task::none();
                            }
                            self.execution_manager
                                .get_edited_scripts_mut()
                                .swap(cursor_script.idx, cursor_script.idx - 1);
                            select_execution_script(self, cursor_script.idx - 1);
                        }
                    }
                }

                if let Some(scroll_offset) = config_script_scroll_offset {
                    return scrollable::scroll_to(
                        SCRIPTS_PANE_SCROLL_ID.clone(),
                        AbsoluteOffset {
                            x: 0.0,
                            y: scroll_offset,
                        },
                    );
                }
            }
            WindowMessage::CursorConfirm => {
                if self.edit_data.window_edit_data.is_some() {
                    return Task::none();
                }

                let Some(cursor_script) = &self.window_state.cursor_script else {
                    return Task::none();
                };

                let cursor_script_id = cursor_script.idx;

                if let Some(focus) = self.window_state.pane_focus {
                    if &self.panes.panes[&focus].variant == &PaneVariant::ScriptList {
                        let scripts = &self.displayed_configs_list_cache;

                        if let Some(script) = scripts.get(cursor_script_id) {
                            if self.window_state.is_command_key_down {
                                if self.window_state.is_alt_key_down {
                                    let scripts = get_resulting_scripts_from_guid(
                                        &self.app_config,
                                        script.original_script_uid.clone(),
                                    );
                                    start_new_execution_from_provided_scripts(self, scripts);
                                } else {
                                    try_add_script_to_execution_or_start_new(
                                        self,
                                        script.original_script_uid.clone(),
                                    );
                                }
                            } else {
                                add_script_to_execution(
                                    self,
                                    script.original_script_uid.clone(),
                                    false,
                                );
                            }
                        }
                    }
                }
            }
            WindowMessage::RemoveCursorScript => {
                if let Some(focus) = self.window_state.pane_focus {
                    if &self.panes.panes[&focus].variant != &PaneVariant::ExecutionList {
                        return Task::none();
                    }
                }

                if let Some(cursor_script) = self.window_state.cursor_script.clone() {
                    if cursor_script.script_type == EditScriptType::ExecutionList {
                        remove_execution_list_script(self, cursor_script.idx);
                    }
                }
            }
            WindowMessage::SwitchPaneFocus(is_forward) => {
                let new_selection = get_next_pane_selection(self, is_forward);

                let mut should_select_arguments = false;
                let has_pane_changed = Some(new_selection)
                    != if let Some(focus) = self.window_state.pane_focus {
                        Some(self.panes.panes[&focus].variant)
                    } else {
                        None
                    };

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
                } else if has_pane_changed {
                    return text_input::focus(text_input::Id::new("dummy"));
                }
            }
            WindowMessage::SetExecutionListTitleEditing(is_editing) => {
                self.visual_caches.is_custom_title_editing = is_editing;
            }
            WindowMessage::EditExecutionListTitle(new_title) => {
                self.app_config.custom_title = Some(new_title);
            }
            WindowMessage::OpenWithDefaultApplication(file_path) => {
                if let Err(e) = open::that(file_path) {
                    eprintln!("Failed to open file with default application: {}", e);
                }
            }
            WindowMessage::OpenUrl(url) => {
                if let Err(e) = open::that(url) {
                    eprintln!("Failed to open URL: {}", e);
                }
            }
            WindowMessage::OpenLogFolder(execution_id) => {
                if let Some(execution) = self
                    .execution_manager
                    .get_started_executions()
                    .get(execution_id)
                {
                    open::that_in_background(execution.get_log_folder_path());
                }
            }
            WindowMessage::ProcessKeyPress(iced_key, iced_modifiers) => {
                self.window_state.is_command_key_down = iced_modifiers.command();
                self.window_state.is_alt_key_down = iced_modifiers.alt();

                if keybind_editing::process_key_press(self, iced_key.clone(), iced_modifiers) {
                    return Task::none();
                }

                // if we're not in keybind editing, then try to process keybinds
                let keybind_associated_data =
                    self.keybinds.get_keybind_copy(iced_key, iced_modifiers);

                let Some(keybind_associated_data) = keybind_associated_data else {
                    return Task::none();
                };

                let message = match keybind_associated_data {
                    keybind_editing::KeybindAssociatedData::AppAction(action) => {
                        Some(get_window_message_from_app_action(action))
                    }
                    keybind_editing::KeybindAssociatedData::Script(guid) => {
                        if self.edit_data.window_edit_data.is_none() {
                            get_run_script_window_message_from_guid(&self.app_config, &guid)
                        } else {
                            None
                        }
                    }
                };

                let Some(message) = message else {
                    return Task::none();
                };

                // avoid infinite recursion
                match message {
                    WindowMessage::ProcessKeyPress(_, _) => return Task::none(),
                    _ => {}
                };

                let command = self.update(message);

                return Task::batch([text_input::focus(text_input::Id::new("dummy")), command]);
            }
            WindowMessage::StartRecordingKeybind(data) => {
                if let Some(window_edit_data) = &mut self.edit_data.window_edit_data {
                    window_edit_data.keybind_editing.edited_keybind = Some(data);
                    window_edit_data.keybind_editing.edited_keybind_error = None;
                }
            }
            WindowMessage::StopRecordingKeybind => {
                if let Some(window_edit_data) = &mut self.edit_data.window_edit_data {
                    window_edit_data.keybind_editing.edited_keybind = None;
                    window_edit_data.keybind_editing.edited_keybind_error = None;
                }
            }
            WindowMessage::SelectExecutionLog(execution_id) => {
                self.visual_caches.selected_execution_log = Some(execution_id);
            }
            WindowMessage::OnQuickLaunchButtonPressed(script_uid) => {
                if !self.edit_data.window_edit_data.is_some() {
                    let scripts_to_execute =
                        get_resulting_scripts_from_guid(&self.app_config, script_uid);
                    start_new_execution_from_provided_scripts(self, scripts_to_execute);
                }
            }
            WindowMessage::AddToQuickLaunchPanel(script_uid) => {
                let original_script =
                    config::get_original_script_definition_by_uid(&self.app_config, &script_uid);
                if let Some((original_script, _idx)) = original_script {
                    let original_script_id = match original_script {
                        config::ScriptDefinition::Original(script) => script.uid.clone(),
                        config::ScriptDefinition::Preset(preset) => preset.uid.clone(),
                        _ => return Task::none(),
                    };
                    config::get_main_rewritable_config_mut(&mut self.app_config)
                        .quick_launch_scripts
                        .push(original_script_id);
                    self.edit_data.is_dirty = true;
                    update_config_cache(self);
                }
            }
            WindowMessage::RemoveFromQuickLaunchPanel(script_uid) => {
                let config = config::get_main_rewritable_config_mut(&mut self.app_config);
                let index = config
                    .quick_launch_scripts
                    .iter()
                    .position(|v| *v == script_uid);
                if let Some(index) = index {
                    config.quick_launch_scripts.remove(index);
                    self.edit_data.is_dirty = true;
                    update_config_cache(self);
                }
            }
        }

        Task::none()
    }

    pub(crate) fn view(&self) -> Element<WindowMessage> {
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
                        &self.visual_caches,
                        &self.edit_data,
                        &self.app_config,
                        &self.execution_manager,
                        is_maximized,
                        size,
                        &self.window_state,
                        &self.theme,
                    ))
                    .padding(10)
                    .style(if is_focused {
                        if self.execution_manager.has_any_execution_failed() {
                            style::title_bar_focused_failed
                        } else if self.execution_manager.has_all_executions_finished() {
                            style::title_bar_focused_completed
                        } else {
                            style::title_bar_focused
                        }
                    } else {
                        style::title_bar_active
                    });

                pane_grid::Content::new(responsive(move |_size| {
                    view_content(
                        &self.execution_manager,
                        variant,
                        &self.theme,
                        &self.displayed_configs_list_cache,
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
            .spacing(PANE_SPACING)
            .on_click(WindowMessage::PaneHeaderClicked)
            .on_drag(WindowMessage::PaneHeaderDragged)
            .on_resize(10, WindowMessage::PaneResized)
            .into()
        });

        let outer_container = container(pane_grid)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(1);

        stack![
            outer_container,
            if self.window_state.dragged_script.is_some() {
                row![opaque(
                    iced::widget::mouse_area(
                        vertical_space().width(Length::Fill).height(Length::Fill)
                    )
                    .interaction(mouse::Interaction::Grabbing)
                )]
            } else {
                row![]
            }
        ]
        .into()
    }

    pub(crate) fn theme(&self) -> Theme {
        self.theme.clone()
    }

    pub(crate) fn subscription(&self) -> Subscription<WindowMessage> {
        Subscription::batch([
            listen_with(move |event, _status, id| match event {
                iced::event::Event::Window(window::Event::Resized(size)) => {
                    Some(WindowMessage::WindowResized(id, size))
                }
                iced::event::Event::Window(window::Event::FileHovered(_)) => {
                    Some(WindowMessage::WindowOnFileHovered)
                }
                iced::event::Event::Window(window::Event::FilesHoveredLeft) => {
                    Some(WindowMessage::WindowOnFilesHoveredLeft)
                }
                iced::event::Event::Window(window::Event::FileDropped(file_path)) => {
                    Some(WindowMessage::WindowOnFileDropped(file_path))
                }
                iced::event::Event::Mouse(event) => match event {
                    Event::ButtonPressed(button) => {
                        if button == mouse::Button::Left {
                            Some(WindowMessage::WindowOnMouseDown)
                        } else {
                            None
                        }
                    }
                    Event::ButtonReleased(button) => {
                        if button == mouse::Button::Left {
                            Some(WindowMessage::WindowOnMouseUp)
                        } else {
                            None
                        }
                    }
                    Event::CursorMoved { position } => {
                        Some(WindowMessage::WindowOnMouseMove(position))
                    }
                    _ => None,
                },
                _ => None,
            }),
            keyboard::on_key_press(|key, modifiers| {
                if is_command_key(&key) {
                    return Some(WindowMessage::OnCommandKeyStateChanged(true));
                }
                if key == keyboard::Key::Named(keyboard::key::Named::Alt) {
                    return Some(WindowMessage::OnAltKeyStateChanged(true));
                }

                if key == keyboard::Key::Named(keyboard::key::Named::Control)
                    || key == keyboard::Key::Named(keyboard::key::Named::Shift)
                    || key == keyboard::Key::Named(keyboard::key::Named::Alt)
                    || key == keyboard::Key::Named(keyboard::key::Named::Super)
                    || key == keyboard::Key::Named(keyboard::key::Named::Fn)
                    || key == keyboard::Key::Unidentified
                {
                    return None;
                }

                Some(WindowMessage::ProcessKeyPress(key, modifiers))
            }),
            keyboard::on_key_release(|key, _modifiers| {
                if is_command_key(&key) {
                    Some(WindowMessage::OnCommandKeyStateChanged(false))
                } else if key == keyboard::Key::Named(keyboard::key::Named::Alt) {
                    Some(WindowMessage::OnAltKeyStateChanged(false))
                } else {
                    None
                }
            }),
            time::every(Duration::from_millis(100)).map(WindowMessage::Tick),
        ])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PaneVariant {
    ScriptList,
    ExecutionList,
    LogOutput,
    Parameters,
}

pub(crate) struct AppPane {
    pub(crate) variant: PaneVariant,
}

impl AppPane {
    fn new(variant: PaneVariant) -> Self {
        Self { variant }
    }
}

fn produce_script_list_content<'a>(
    execution_lists: &parallel_execution_manager::ParallelExecutionManager,
    config: &config::AppConfig,
    displayed_configs_list_cache: &Vec<ScriptListCacheRecord>,
    edit_data: &EditData,
    visual_caches: &'a VisualCaches,
    window_state: &WindowState,
    theme: &Theme,
) -> Column<'a, WindowMessage> {
    if let Some(error) = &config.config_read_error {
        return get_config_error_content(error, theme);
    }

    let is_editing = edit_data.window_edit_data.is_some();

    let drag_and_drop_area = if !is_editing {
        &window_state.drag_and_drop_lists.script_list
    } else {
        &window_state.drag_and_drop_lists.edit_script_list
    };

    let dragged_element_index = drag_and_drop_area.get_dragged_element_index();
    let insert_position_index = drag_and_drop_area.get_reordering_target_index();

    let drop_marker = drop_marker(insert_position_index, ONE_SCRIPT_LIST_ELEMENT_HEIGHT);

    let data: Element<_> = column(
        displayed_configs_list_cache
            .iter()
            .enumerate()
            .map(|(i, script)| {
                let mut name_text = script.name.clone();

                if is_editing && is_local_config_script(i, &config) {
                    name_text += " [local]";
                }
                if is_editing && script.is_hidden {
                    name_text += " [hidden]";
                }

                let will_run_on_click =
                    edit_data.window_edit_data.is_none() && window_state.is_command_key_down;

                let icon = if will_run_on_click {
                    row![
                        Space::with_width(6),
                        image(
                            visual_caches
                                .icons
                                .get_theme_for_color(theme.extended_palette().secondary.base.text)
                                .quick_launch
                                .clone()
                        )
                        .width(22)
                        .height(22),
                    ]
                } else if let Some(icon_path) = &script.full_icon_path {
                    row![Space::with_width(6), image(icon_path).width(22).height(22),]
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

                let is_dragged = dragged_element_index == Some(i);

                let item_button = button(
                    row![
                        icon,
                        Space::with_width(6),
                        text(name_text).height(22),
                        horizontal_space()
                    ]
                    .height(22),
                )
                .padding(4)
                .style(if is_dragged {
                    button::success
                } else if is_selected {
                    button::primary
                } else {
                    button::secondary
                })
                .on_press(if edit_data.window_edit_data.is_none() {
                    WindowMessage::AddScriptToExecutionOrRun(script.original_script_uid.clone())
                } else {
                    WindowMessage::OpenScriptConfigEditing(i)
                });

                row![item_button].into()
            })
            .collect::<Vec<_>>(),
    )
    .width(Length::Fill)
    .into();

    let edit_controls = if edit_data.window_edit_data.is_some() {
        column![
            text("Editing config")
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill)
                .size(16),
            Space::with_height(4.0),
            if edit_data.is_dirty {
                column![row![
                    button(text("Save").size(16))
                        .style(button::success)
                        .on_press(WindowMessage::SaveConfigAndExitEditing),
                    Space::with_width(4.0),
                    button(text("Revert").size(16))
                        .style(button::danger)
                        .on_press(WindowMessage::RevertConfigAndExitEditing),
                    Space::with_width(4.0),
                    button(text("Preview").size(16)).on_press(WindowMessage::ExitWindowEditMode),
                ]]
            } else {
                column![button(text("Back").size(16)).on_press(WindowMessage::ExitWindowEditMode)]
            },
            Space::with_height(4.0),
            row![
                main_icon_button(
                    visual_caches.icons.themed.plus.clone(),
                    "Add script",
                    Some(WindowMessage::AddScriptToConfig)
                ),
                Space::with_width(4.0),
                main_icon_button(
                    visual_caches.icons.themed.settings.clone(),
                    "Settings",
                    Some(WindowMessage::ToggleConfigEditing)
                ),
            ],
            Space::with_height(4.0),
        ]
    } else if edit_data.is_dirty {
        column![
            {
                let mut buttons = row![
                    button(text("Save").size(16))
                        .style(button::success)
                        .on_press(WindowMessage::SaveConfigAndExitEditing),
                    Space::with_width(4.0),
                    button(text("Revert").size(16))
                        .style(button::danger)
                        .on_press(WindowMessage::RevertConfigAndExitEditing),
                ];
                if !execution_lists.has_any_execution_started() {
                    buttons = buttons.push(Space::with_width(4.0));
                    buttons = buttons.push(
                        button(text("Back to edit").size(16))
                            .on_press(WindowMessage::EnterWindowEditMode),
                    );
                }
                buttons
            },
            Space::with_height(4.0),
        ]
    } else {
        column![]
    };

    let filter_field =
        if visual_caches.enable_script_filtering && edit_data.window_edit_data.is_none() {
            row![
                Space::with_width(5),
                if window_state.is_command_key_down {
                    text_input(
                        &format_keybind_hint(
                            visual_caches,
                            "Focus filter",
                            config::AppAction::FocusFilter,
                        ),
                        &edit_data.script_filter,
                    )
                } else {
                    text_input("filter", &edit_data.script_filter)
                }
                .id(FILTER_INPUT_ID.clone())
                .on_input(WindowMessage::ScriptFilterChanged)
                .width(Length::Fill),
                Space::with_width(4),
                if !edit_data.script_filter.is_empty() {
                    column![
                        Space::with_height(4.0),
                        button(image(
                            visual_caches
                                .icons
                                .get_theme_for_color(theme.extended_palette().danger.base.color)
                                .remove
                                .clone()
                        ))
                        .padding(4)
                        .style(button::danger)
                        .width(Length::Shrink)
                        .height(Length::Shrink)
                        .on_press(WindowMessage::ScriptFilterChanged("".to_string())),
                    ]
                } else {
                    column![]
                },
                Space::with_width(1),
            ]
        } else {
            row![]
        };

    let quick_launch_buttons = if !visual_caches.quick_launch_buttons.is_empty() {
        column![
            horizontal_rule(1),
            scrollable(column![
                Space::with_height(2.0),
                row(visual_caches
                    .quick_launch_buttons
                    .iter()
                    .map(|button| { quick_launch_button(&button).into() })
                    .collect::<Vec<_>>())
                .spacing(4),
                Space::with_height(4.0),
            ])
            .direction(scrollable::Direction::Horizontal(Scrollbar::default()))
        ]
    } else {
        column![]
    };

    column![
        edit_controls,
        filter_field,
        scrollable(stack![data, drop_marker])
            .height(Length::Fill)
            .on_scroll(move |viewport| WindowMessage::OnScriptListScroll(viewport))
            .id(SCRIPTS_PANE_SCROLL_ID.clone()),
        quick_launch_buttons,
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Alignment::Start)
}

fn produce_execution_list_content<'a>(
    execution_lists: &parallel_execution_manager::ParallelExecutionManager,
    path_caches: &config::PathCaches,
    theme: &Theme,
    config: &config::AppConfig,
    visual_caches: &VisualCaches,
    edit_data: &EditData,
    window_state: &WindowState,
) -> Column<'a, WindowMessage> {
    let icons = &visual_caches.icons;

    let rewritable_config = config::get_main_rewritable_config(&config);

    let drop_area_stata = window_state.drop_areas.execution_pane.get_drop_area_state();

    let main_drop_area = match drop_area_stata {
        DropAreaState::Inactive => column![],
        DropAreaState::VisibleIdle => column![button(
            text("Drop here to schedule")
                .color(theme.extended_palette().secondary.strong.text)
                .center()
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(style::drop_area_button_style),]
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center),
        DropAreaState::HoveredByItem => column![button(
            text("Release to schedule")
                .color(theme.extended_palette().primary.strong.text)
                .center()
                .width(Length::Fill),
        )
        .width(Length::Fill)
        .height(Length::Fill),]
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(Alignment::Center),
    };

    let mut last_offset_y = PANE_HEADER_HEIGHT;
    let running_execution_drop_areas = window_state
        .drop_areas
        .running_executions
        .iter()
        .filter_map(|drop_area| {
            let state = drop_area.get_drop_area_state();
            if state == DropAreaState::Inactive {
                return None;
            }
            let mut bounds = drop_area.get_bounds_scrolled();
            let is_hovered = state == DropAreaState::HoveredByItem;
            let mut spacing_y = bounds.y - last_offset_y;
            let original_bounds_height = bounds.height;
            if spacing_y < 0.0 {
                bounds.height += spacing_y;
                spacing_y = 0.0;
            }
            if bounds.height < 0.0 {
                return None;
            }
            last_offset_y = bounds.y + original_bounds_height;
            Some(
                column![
                    vertical_space().height(spacing_y),
                    button(
                        text("Drop here to run after")
                            .color(theme.extended_palette().secondary.strong.text)
                            .center()
                            .width(bounds.width),
                    )
                    .width(bounds.width)
                    .height(bounds.height)
                    .style(if is_hovered {
                        button::primary
                    } else {
                        style::drop_area_button_style
                    })
                ]
                .into(),
            )
        })
        .collect::<Vec<_>>();

    let title_widget = if visual_caches.is_custom_title_editing {
        row![text_input(
            "Write a note for this execution here",
            &config.custom_title.as_ref().unwrap_or(&EMPTY_STRING)
        )
        .on_input(WindowMessage::EditExecutionListTitle)
        .on_submit(WindowMessage::SetExecutionListTitleEditing(false))
        .size(16)
        .width(Length::Fill),]
    } else if visual_caches.enable_title_editing && edit_data.window_edit_data.is_none() {
        row![
            horizontal_space(),
            text(
                config
                    .custom_title
                    .as_ref()
                    .map(|x| x.to_string())
                    .unwrap_or_else(|| EMPTY_STRING.clone())
            )
            .size(16)
            .align_x(alignment::Horizontal::Center)
            .width(Length::Shrink),
            tooltip(
                button(
                    image(
                        icons
                            .get_theme_for_color(theme.extended_palette().secondary.base.text)
                            .edit
                            .clone()
                    )
                    .width(Length::Fixed(8.0))
                    .height(Length::Fixed(8.0))
                )
                .style(button::secondary)
                .on_press(WindowMessage::SetExecutionListTitleEditing(true)),
                "Edit title",
                tooltip::Position::Right
            ),
            horizontal_space(),
        ]
        .align_y(Alignment::Center)
    } else if let Some(custom_title) = &config.custom_title {
        if !custom_title.is_empty() {
            row![text(custom_title.to_string())
                .size(16)
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),]
            .align_y(Alignment::Center)
        } else {
            row![]
        }
    } else {
        row![]
    };

    let mut title = Column::new();

    if rewritable_config.show_working_directory {
        title = title.push(
            text(
                path_caches
                    .work_path
                    .to_str()
                    .unwrap_or_default()
                    .to_string(),
            )
            .size(16)
            .align_x(alignment::Horizontal::Center)
            .width(Length::Fill),
        );
    }

    if let Some(git_branch_requester) = &visual_caches.git_branch_requester {
        title = title.push(
            text(git_branch_requester.get_current_branch_ref().clone())
                .size(16)
                .align_x(alignment::Horizontal::Center)
                .width(Length::Fill),
        )
    }

    title = title.push(title_widget);

    let started_execution_count = execution_lists.get_started_executions().len();
    let should_show_execution_names = started_execution_count > 1;

    let mut data_lines: Vec<Element<'_, WindowMessage, Theme, iced::Renderer>> =
        Vec::with_capacity(started_execution_count * 10);
    for execution in execution_lists.get_started_executions().values() {
        if should_show_execution_names {
            data_lines.push(
                row![text(execution.get_name().clone())
                    .size(16)
                    .align_x(alignment::Horizontal::Left)
                    .width(Length::Fill),]
                .height(30)
                .into(),
            );
        }

        let execution_id = execution.get_id();
        let scripts = execution.get_scheduled_scripts_cache();
        for i in 0..scripts.len() {
            let record = &scripts[i];
            let script_status = &record.status;

            let repeat_text = if script_status.retry_count > 0 {
                format!(
                    " [{}/{}]",
                    script_status.retry_count, record.script.original.autorerun_count
                )
            } else {
                String::new()
            };

            let status;
            let status_tooltip: &'static str;
            let progress;
            let color = if script_status.has_script_failed() {
                if let Some(custom_theme) = &visual_caches.custom_theme {
                    iced::Color::from_rgb(
                        custom_theme.error_text[0],
                        custom_theme.error_text[1],
                        custom_theme.error_text[2],
                    )
                } else {
                    theme.extended_palette().danger.weak.color
                }
            } else {
                theme.extended_palette().background.strong.text
            };

            if script_status.has_script_finished() {
                status = match script_status.result {
                    execution_thread::ScriptResultStatus::Failed => image(icons.failed.clone()),
                    execution_thread::ScriptResultStatus::Success => image(icons.succeeded.clone()),
                    execution_thread::ScriptResultStatus::Skipped => image(icons.skipped.clone()),
                    execution_thread::ScriptResultStatus::Disconnected => {
                        image(icons.skipped.clone())
                    }
                };
                status_tooltip = match script_status.result {
                    execution_thread::ScriptResultStatus::Failed => "Failed",
                    execution_thread::ScriptResultStatus::Success => "Success",
                    execution_thread::ScriptResultStatus::Skipped => "Skipped",
                    execution_thread::ScriptResultStatus::Disconnected => "",
                };
                if script_status.result != execution_thread::ScriptResultStatus::Skipped {
                    let time_taken_sec = script_status
                        .finish_time
                        .unwrap_or(Instant::now())
                        .duration_since(script_status.start_time.unwrap_or(Instant::now()))
                        .as_secs();
                    progress = format!(
                        "({:02}:{:02}){}",
                        time_taken_sec / 60,
                        time_taken_sec % 60,
                        repeat_text,
                    );
                } else {
                    progress = String::new();
                }
            } else if script_status.has_script_started() {
                let time_taken_sec = Instant::now()
                    .duration_since(script_status.start_time.unwrap_or(Instant::now()))
                    .as_secs();
                status = image(icons.in_progress.clone());
                status_tooltip = "In progress";

                progress = format!(
                    "({:02}:{:02}){}",
                    time_taken_sec / 60,
                    time_taken_sec % 60,
                    repeat_text,
                );
            } else {
                status = image(icons.idle.clone());
                status_tooltip = "Idle";
                progress = String::new();
            };

            let mut row_data: Vec<Element<'_, WindowMessage, Theme, iced::Renderer>> =
                Vec::with_capacity(10);
            row_data.push(
                tooltip(
                    status.width(22).height(22).content_fit(ContentFit::None),
                    status_tooltip,
                    tooltip::Position::Right,
                )
                .style(container::bordered_box)
                .into(),
            );
            row_data.push(Space::with_width(4).into());
            if !record.script.original.icon.path.is_empty() {
                row_data.push(
                    image(config::get_full_path(
                        path_caches,
                        &record.script.original.icon,
                    ))
                    .width(22)
                    .height(22)
                    .into(),
                );
                row_data.push(Space::with_width(4).into());
            }
            row_data.push(
                tooltip(
                    text(format!("{} {}", record.script.original.name, progress)).color(color),
                    text(record.tooltip.clone()),
                    tooltip::Position::Bottom,
                )
                .style(container::bordered_box)
                .into(),
            );

            data_lines.push(row(row_data).height(30).into());
        }

        data_lines.push(Space::with_height(8).into());

        data_lines.push(
            column![if execution.has_finished_execution() {
                if !execution.is_waiting_execution_to_finish() {
                    row![
                        if window_state.is_command_key_down
                            && visual_caches.button_key_caches.last_cleanable_execution_id
                                == Some(execution_id)
                        {
                            main_icon_button_string(
                                icons.themed.retry.clone(),
                                format_keybind_hint(
                                    visual_caches,
                                    "Reschedule",
                                    config::AppAction::RescheduleScripts,
                                ),
                                Some(WindowMessage::RescheduleScripts(execution_id)),
                            )
                        } else {
                            main_icon_button(
                                icons.themed.retry.clone(),
                                "Reschedule",
                                Some(WindowMessage::RescheduleScripts(execution_id)),
                            )
                        },
                        main_icon_button_string(
                            icons.themed.remove.clone(),
                            if window_state.is_command_key_down
                                && execution_lists.get_edited_scripts().is_empty()
                                && visual_caches.button_key_caches.last_cleanable_execution_id
                                    == Some(execution_id)
                            {
                                format_keybind_hint(
                                    visual_caches,
                                    "Clear",
                                    config::AppAction::ClearExecutionScripts,
                                )
                            } else {
                                "Clear".to_string()
                            },
                            Some(WindowMessage::ClearFinishedExecutionScripts(execution_id)),
                        ),
                        if execution.has_non_skipped_scripts() {
                            row![main_icon_button_string(
                                icons.themed.log.clone(),
                                "Logs",
                                Some(WindowMessage::OpenLogFolder(execution_id)),
                            )]
                        } else {
                            row![]
                        }
                    ]
                } else {
                    row![text("Waiting for the execution to stop")]
                }
            } else if execution_lists.has_any_execution_started() {
                let current_script = execution.get_currently_outputting_script();
                if current_script != -1
                    && execution.get_scheduled_scripts_cache()[current_script as usize]
                        .status
                        .has_script_failed()
                {
                    row![text("Waiting for the execution to stop")]
                } else {
                    row![
                        if window_state.is_command_key_down
                            && visual_caches.button_key_caches.last_stoppable_execution_id
                                == Some(execution_id)
                        {
                            row![main_icon_button_string(
                                icons.themed.stop.clone(),
                                format_keybind_hint(
                                    visual_caches,
                                    "Stop",
                                    config::AppAction::StopScripts
                                ),
                                Some(WindowMessage::StopScripts(execution_id))
                            )]
                        } else {
                            row![main_icon_button(
                                icons.themed.stop.clone(),
                                "Stop",
                                Some(WindowMessage::StopScripts(execution_id))
                            )]
                        },
                        if !window_state.has_maximized_pane
                            && execution.has_potentially_editable_scripts()
                        {
                            row![main_icon_button(
                                icons.themed.edit.clone(),
                                "Edit",
                                Some(WindowMessage::EditExecutedScripts(execution_id))
                            )]
                        } else {
                            row![]
                        },
                        if execution.has_non_skipped_scripts() {
                            row![main_icon_button_string(
                                icons.themed.log.clone(),
                                "Logs",
                                Some(WindowMessage::OpenLogFolder(execution_id)),
                            )]
                        } else {
                            row![]
                        }
                    ]
                }
            } else {
                row![]
            }
            .spacing(5)]
            .width(Length::Fill)
            .align_x(Alignment::Center)
            .into(),
        );

        data_lines.push(Space::with_height(8).into());
    }
    let scheduled_block = column(data_lines)
        .width(Length::Fill)
        .align_x(Alignment::Start);

    let dragged_script_idx = window_state
        .drag_and_drop_lists
        .execution_edit_list
        .get_dragged_element_index();
    let insert_position_index = window_state
        .drag_and_drop_lists
        .execution_edit_list
        .get_reordering_target_index();

    let drop_marker = drop_marker(insert_position_index, ONE_EXECUTION_LIST_ELEMENT_HEIGHT);

    let edited_data: Element<_> = column(
        execution_lists
            .get_edited_scripts()
            .iter()
            .enumerate()
            .map(|(i, script)| {
                let is_selected = match &window_state.cursor_script {
                    Some(selected_script) => {
                        selected_script.idx == i
                            && selected_script.script_type == EditScriptType::ExecutionList
                    }
                    None => false,
                };

                let color = if is_selected {
                    theme.extended_palette().primary.strong.text
                } else {
                    theme.extended_palette().background.strong.text
                };

                let mut row_data: Vec<Element<'_, WindowMessage, Theme, iced::Renderer>> =
                    Vec::new();

                row_data.push(Space::with_width(4).into());
                if !script.original.icon.path.is_empty() {
                    row_data.push(
                        image(config::get_full_path(path_caches, &script.original.icon))
                            .width(22)
                            .height(22)
                            .into(),
                    );
                    row_data.push(Space::with_width(4).into());
                }
                row_data.push(text(script.original.name.clone()).color(color).into());

                if is_selected {
                    row_data.push(horizontal_space().into());
                    row_data.push(Space::with_width(30).into());
                    row_data.push(
                        tooltip(
                            inline_icon_button(
                                icons
                                    .get_theme_for_color(theme.extended_palette().danger.base.color)
                                    .remove
                                    .clone(),
                                WindowMessage::RemoveExecutionListScript(i),
                            )
                            .style(button::danger),
                            "Remove script from execution list",
                            tooltip::Position::Left,
                        )
                        .style(container::bordered_box)
                        .into(),
                    );
                }

                let mut list_item = button(row(row_data)).width(Length::Fill).padding(4);
                if dragged_script_idx.is_none() {
                    if is_selected {
                        list_item = list_item.on_press(WindowMessage::CloseScriptEditing);
                    } else {
                        list_item = list_item.on_press(WindowMessage::OpenScriptEditing(i));
                    }
                }

                let is_dragged = dragged_script_idx == Some(i);

                list_item = list_item.style(if is_dragged {
                    button::success
                } else if is_selected {
                    button::primary
                } else {
                    if is_original_script_missing_arguments(&script.original) {
                        button::danger
                    } else {
                        button::secondary
                    }
                });

                list_item.height(30).into()
            })
            .collect::<Vec<_>>(),
    )
    .width(Length::Fill)
    .align_x(Alignment::Start)
    .into();

    let edit_controls = column![if edit_data.window_edit_data.is_some() {
        row![main_button(
            "Save as preset",
            if !execution_lists.get_edited_scripts().is_empty() {
                Some(WindowMessage::SaveAsPreset)
            } else {
                None
            }
        )]
    } else if !execution_lists.get_edited_scripts().is_empty() {
        let have_scripts_missing_arguments = execution_lists
            .get_edited_scripts()
            .iter()
            .any(|script| is_original_script_missing_arguments(&script.original));

        let mut execution_buttons: Vec<Element<'_, WindowMessage, Theme, iced::Renderer>> =
            Vec::with_capacity(6);

        if !have_scripts_missing_arguments {
            if should_show_execution_names {
                for execution in execution_lists.get_started_executions().values() {
                    execution_buttons.push(
                        main_icon_button_string(
                            icons.themed.play.clone(),
                            format!("Run after {}", execution.get_name()),
                            Some(WindowMessage::RunEditedScriptsWithExecution(
                                execution.get_id(),
                            )),
                        )
                        .into(),
                    );
                }
            } else if started_execution_count == 1 {
                execution_buttons.push(
                    main_icon_button_string(
                        icons.themed.play.clone(),
                        if window_state.is_command_key_down {
                            format_keybind_hint(
                                visual_caches,
                                "Run after",
                                config::AppAction::RunScriptsAfterExecution,
                            )
                        } else {
                            "Run after".to_string()
                        },
                        Some(WindowMessage::RunEditedScriptsWithExecution(
                            execution_lists
                                .get_started_executions()
                                .values()
                                .next()
                                .unwrap()
                                .get_id(),
                        )),
                    )
                    .into(),
                );
            }

            if started_execution_count == 0 {
                execution_buttons.push(
                    main_icon_button_string(
                        icons.themed.play.clone(),
                        if window_state.is_command_key_down {
                            format_keybind_hint(
                                visual_caches,
                                "Run",
                                config::AppAction::RunScriptsAfterExecution,
                            )
                        } else {
                            "Run".to_string()
                        },
                        Some(WindowMessage::RunEditedScriptsAfterExecutionHotkey),
                    )
                    .into(),
                );
            } else {
                execution_buttons.push(
                    main_icon_button_string(
                        icons.themed.play.clone(),
                        if window_state.is_command_key_down {
                            format_keybind_hint(
                                visual_caches,
                                "Run in parallel",
                                config::AppAction::RunScriptsInParallel,
                            )
                        } else {
                            "Run in parallel".to_string()
                        },
                        Some(WindowMessage::RunEditedScriptsInParallel),
                    )
                    .into(),
                );
            }

            execution_buttons.push(
                main_icon_button_string(
                    icons.themed.remove.clone(),
                    if window_state.is_command_key_down {
                        format_keybind_hint(
                            visual_caches,
                            "Clear",
                            config::AppAction::ClearExecutionScripts,
                        )
                    } else {
                        "Clear".to_string()
                    },
                    Some(WindowMessage::ClearEditedExecutionScripts),
                )
                .into(),
            );
        } else {
            execution_buttons.push(text("Some scripts are missing arguments").into());
        }

        row![scrollable(column![
            row(execution_buttons).spacing(5),
            Space::with_height(8),
        ])
        .direction(scrollable::Direction::Horizontal(Scrollbar::default()))]
    } else {
        row![]
    }
    .align_y(Alignment::Center)
    .spacing(3)]
    .align_x(Alignment::Center)
    .spacing(5)
    .width(Length::Fill);

    let edited_block = stack![
        column![
            edited_data,
            Space::with_height(8),
            edit_controls,
            Space::with_height(8),
        ],
        drop_marker
    ];

    column![stack![
        column![
            title,
            scrollable(column![
                if !execution_lists.get_started_executions().is_empty() {
                    scheduled_block
                } else {
                    column![]
                },
                if !execution_lists.get_edited_scripts().is_empty()
                    || edit_data.window_edit_data.is_some()
                {
                    edited_block
                } else {
                    stack![]
                },
            ])
            .on_scroll(move |viewport| WindowMessage::OnExecutionListScroll(viewport))
            .id(EXECUTIONS_PANE_SCROLL_ID.clone()),
        ]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .align_x(Alignment::Center),
        if running_execution_drop_areas.is_empty() {
            main_drop_area
        } else {
            column(running_execution_drop_areas)
        }
    ]]
}

fn produce_log_output_content<'a>(
    execution_lists: &parallel_execution_manager::ParallelExecutionManager,
    theme: &Theme,
    main_config: &config::RewritableConfig,
    visual_caches: &VisualCaches,
) -> Column<'a, WindowMessage> {
    if !execution_lists.has_any_execution_started() {
        return Column::new();
    }

    let tabs = if execution_lists.get_started_executions().len() > 1 {
        let tabs = row(execution_lists
            .get_started_executions()
            .values()
            .map(|execution| {
                let is_selected_execution =
                    Some(execution.get_id()) == visual_caches.selected_execution_log;
                let tab_button = button(text(execution.get_name().clone()));
                if is_selected_execution {
                    tab_button
                } else {
                    tab_button.on_press(WindowMessage::SelectExecutionLog(execution.get_id()))
                }
                .into()
            })
            .collect::<Vec<_>>())
        .spacing(5);

        let tabs = row![scrollable(column![tabs, Space::with_height(12),])
            .direction(scrollable::Direction::Horizontal(Scrollbar::default()))];
        tabs
    } else {
        row![]
    };

    let selected_execution = if let Some(execution_id) = visual_caches.selected_execution_log {
        execution_lists.get_started_executions().get(execution_id)
    } else {
        None
    };

    if let Some(selected_execution) = selected_execution {
        let mut data_lines: Vec<Element<'_, WindowMessage, Theme, iced::Renderer>> = Vec::new();
        if let Ok(logs) = selected_execution.get_recent_logs().try_lock() {
            if !logs.is_empty() {
                let (caption_color, error_color) =
                    if let Some(custom_theme) = &main_config.custom_theme {
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
                    .color(match element.output_type {
                        execution_thread::OutputType::StdOut => {
                            theme.extended_palette().secondary.base.text
                        }
                        execution_thread::OutputType::StdErr => error_color,
                        execution_thread::OutputType::Error => error_color,
                        execution_thread::OutputType::Event => caption_color,
                    })
                    .into()
                }));
            }
        }

        let logs = if selected_execution.has_non_skipped_scripts() {
            column![main_button(
                "Open log directory",
                Some(WindowMessage::OpenLogFolder(selected_execution.get_id())),
            )]
        } else {
            column![]
        };

        let data: Element<_> = column(data_lines).spacing(10).width(Length::Fill).into();

        column![
            tabs,
            logs,
            stack![
                scrollable(data)
                    .style(style::log_scrollable_style)
                    .id(LOGS_SCROLL_ID.clone()),
                opaque(row![].width(Length::Fill).height(Length::Fill))
            ]
        ]
    } else {
        column![tabs]
    }
    .width(Length::Fill)
    .height(Length::Fill)
    .spacing(10)
    .align_x(Alignment::Start)
}

fn produce_script_edit_content<'a>(
    execution_lists: &parallel_execution_manager::ParallelExecutionManager,
    visual_caches: &VisualCaches,
    edit_data: &EditData,
    app_config: &config::AppConfig,
    window_state: &WindowState,
    theme: &Theme,
) -> Column<'a, WindowMessage> {
    let Some(edited_script) = &window_state.cursor_script else {
        return Column::new();
    };

    if edited_script.script_type == EditScriptType::ScriptConfig {
        if edit_data.window_edit_data.is_none() {
            return Column::new();
        }

        produce_script_config_edit_content(
            visual_caches,
            edit_data,
            app_config,
            edited_script.idx,
            get_script_definition(
                &app_config,
                config::get_main_edit_mode(app_config),
                edited_script.idx,
            ),
            theme,
        )
    } else {
        match execution_lists.get_edited_scripts().get(edited_script.idx) {
            Some(script) => produce_script_to_execute_edit_content(
                visual_caches,
                edited_script.idx,
                &script.original,
            ),
            _ => {
                eprintln!("Could not find script with index {}", edited_script.idx);
                Column::new()
            }
        }
    }
}

fn produce_script_config_edit_content<'a>(
    visual_caches: &VisualCaches,
    edit_data: &EditData,
    app_config: &config::AppConfig,
    edited_script_idx: usize,
    script: &config::ScriptDefinition,
    theme: &Theme,
) -> Column<'a, WindowMessage> {
    let mut parameters: Vec<Element<'_, WindowMessage, Theme, iced::Renderer>> =
        Vec::with_capacity(40);

    let Some(window_edit_data) = &edit_data.window_edit_data else {
        return Column::new();
    };

    let config_script_id = ConfigScriptId {
        idx: edited_script_idx,
        edit_mode: config::get_main_edit_mode(&app_config),
    };

    match script {
        config::ScriptDefinition::Original(script) => {
            populate_original_script_config_edit_content(
                &mut parameters,
                config_script_id,
                script,
                visual_caches,
                theme,
            );

            parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
            keybind_editing::populate_keybind_editing_content(
                &mut parameters,
                &window_edit_data,
                visual_caches,
                "Keybind to schedule:",
                keybind_editing::KeybindAssociatedData::Script(script.uid.clone()),
            );
            populate_quick_launch_edit_button(&mut parameters, &visual_caches, &script.uid);

            parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
            parameters.push(
                edit_button(
                    "Duplicate script",
                    WindowMessage::DuplicateConfigScript(config_script_id),
                )
                .into(),
            );

            if config_script_id.edit_mode == config::ConfigEditMode::Local
                && is_local_config_script(edited_script_idx, &app_config)
            {
                parameters.push(
                    edit_button(
                        "Make shared",
                        WindowMessage::MoveToShared(edited_script_idx),
                    )
                    .into(),
                );
            }

            parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
            parameters.push(
                edit_button(
                    "Remove script",
                    WindowMessage::RemoveConfigScript(config_script_id),
                )
                .style(button::danger)
                .into(),
            );
        }
        config::ScriptDefinition::ReferenceToShared(reference) => {
            let Some(window_edit) = &edit_data.window_edit_data else {
                return Column::new();
            };

            let Some((original_script, original_idx)) =
                config::get_original_script_definition_by_uid(&app_config, &reference.uid)
            else {
                return Column::new();
            };

            let original_script_id = ConfigScriptId {
                idx: original_idx,
                edit_mode: config::ConfigEditMode::Shared,
            };

            match original_script {
                config::ScriptDefinition::Original(original_script) => {
                    populate_original_script_config_edit_content(
                        &mut parameters,
                        original_script_id,
                        original_script,
                        visual_caches,
                        theme,
                    );
                }
                config::ScriptDefinition::Preset(preset) => {
                    populate_original_preset_edit_content(
                        &mut parameters,
                        original_script_id,
                        preset,
                    );
                }
                _ => {
                    eprintln!(
                        "Reference to shared {:X} was pointing to a reference to shared",
                        reference.uid.data
                    );
                }
            }

            parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
            parameters.push(
                checkbox("Is hidden locally", reference.is_hidden)
                    .on_toggle(move |val| WindowMessage::ToggleScriptHidden(val))
                    .into(),
            );

            parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
            keybind_editing::populate_keybind_editing_content(
                &mut parameters,
                &window_edit,
                visual_caches,
                "Keybind to schedule:",
                keybind_editing::KeybindAssociatedData::Script(reference.uid.clone()),
            );

            populate_quick_launch_edit_button(&mut parameters, &visual_caches, &reference.uid);

            parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
            parameters.push(
                edit_button(
                    "Make local copy",
                    WindowMessage::CreateCopyOfSharedScript(edited_script_idx),
                )
                .into(),
            );
            parameters.push(
                edit_button(
                    "Duplicate shared",
                    WindowMessage::DuplicateConfigScript(original_script_id),
                )
                .into(),
            );

            parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
            parameters.push(
                edit_button(
                    "Remove shared",
                    WindowMessage::RemoveConfigScript(original_script_id),
                )
                .style(button::danger)
                .into(),
            );
        }
        config::ScriptDefinition::Preset(preset) => {
            populate_original_preset_edit_content(&mut parameters, config_script_id, preset);

            if let Some(window_edit_data) = &edit_data.window_edit_data {
                parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
                keybind_editing::populate_keybind_editing_content(
                    &mut parameters,
                    &window_edit_data,
                    visual_caches,
                    "Keybind to schedule:",
                    keybind_editing::KeybindAssociatedData::Script(preset.uid.clone()),
                );

                populate_quick_launch_edit_button(&mut parameters, &visual_caches, &preset.uid);
            }

            parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
            parameters.push(
                edit_button(
                    "Duplicate preset",
                    WindowMessage::DuplicateConfigScript(config_script_id),
                )
                .into(),
            );

            if config_script_id.edit_mode == config::ConfigEditMode::Local
                && is_local_config_script(edited_script_idx, &app_config)
            {
                parameters.push(
                    edit_button(
                        "Make shared",
                        WindowMessage::MoveToShared(edited_script_idx),
                    )
                    .into(),
                );
            }

            parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
            parameters.push(
                edit_button(
                    "Remove preset",
                    WindowMessage::RemoveConfigScript(config_script_id),
                )
                .style(button::danger)
                .into(),
            );
        }
    }

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());

    let content = column(parameters).spacing(10);

    column![scrollable(content)]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .align_x(Alignment::Start)
}

fn produce_script_to_execute_edit_content<'a>(
    visual_caches: &VisualCaches,
    edited_script_idx: usize,
    script: &config::OriginalScriptDefinition,
) -> Column<'a, WindowMessage> {
    let mut parameters: Vec<Element<'_, WindowMessage, Theme, iced::Renderer>> =
        Vec::with_capacity(20);

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(text("Name:").into());
    parameters.push(
        text_input("name", &script.name)
            .on_input(move |new_arg| WindowMessage::EditScriptNameForExecutionList(new_arg))
            .padding(5)
            .into(),
    );

    if script.arguments_requirement != config::ArgumentRequirement::Hidden {
        parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
        parameters.push(text("Arguments line:").into());
        parameters.push(
            text_input(&script.arguments_hint, &script.arguments_line)
                .on_input(move |new_value| {
                    WindowMessage::EditArgumentsLineForScriptExecution(new_value)
                })
                .style(
                    if script.arguments_requirement == config::ArgumentRequirement::Required
                        && script.arguments_line.is_empty()
                    {
                        style::invalid_text_input_style
                    } else {
                        text_input::default
                    },
                )
                .padding(5)
                .id(ARGUMENTS_INPUT_ID.clone())
                .into(),
        );
    }

    populate_argument_placeholders_content(&mut parameters, &script.argument_placeholders);

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(text("Retry count:").into());
    parameters.push(
        text_input("0", &visual_caches.autorerun_count)
            .on_input(move |new_value| WindowMessage::EditAutorerunCountForExecutionList(new_value))
            .padding(5)
            .into(),
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(text("Reaction to previous failures:").into());
    parameters.push(
        pick_list(
            CONFIG_REACTION_TO_PREVIOUS_FAILURES_PICK_LIST,
            Some(script.reaction_to_previous_failures.clone()),
            move |val| WindowMessage::EditReactionToPreviousFailuresForExecutionList(val),
        )
        .into(),
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(
        checkbox("Autoclean on success", script.autoclean_on_success)
            .on_toggle(move |val| WindowMessage::ToggleAutocleanOnSuccessForExecutionList(val))
            .into(),
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(
        edit_button(
            "Remove script",
            WindowMessage::RemoveExecutionListScript(edited_script_idx),
        )
        .style(button::danger)
        .into(),
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());

    let content = column(parameters).spacing(10);

    column![scrollable(content)]
        .width(Length::Fill)
        .height(Length::Fill)
        .spacing(10)
        .align_x(Alignment::Start)
}

fn populate_original_script_config_edit_content<'a>(
    parameters: &mut Vec<Element<'_, WindowMessage, Theme, iced::Renderer>>,
    config_script_id: ConfigScriptId,
    script: &config::OriginalScriptDefinition,
    visual_caches: &VisualCaches,
    theme: &Theme,
) {
    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(text("Name:").into());
    parameters.push(
        text_input("name", &script.name)
            .on_input(move |new_arg| {
                WindowMessage::EditScriptNameForConfig(config_script_id, new_arg)
            })
            .padding(5)
            .into(),
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(
        row![
            text("Command:"),
            Space::with_width(4),
            help_icon(COMMAND_HELP_TEXT, visual_caches, theme),
        ]
        .into(),
    );
    populate_path_editing_content(
        "command",
        &script.command,
        parameters,
        move |path| WindowMessage::EditScriptCommand(config_script_id, path),
        move |val| WindowMessage::EditScriptCommandPathType(config_script_id, val),
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(text("Working directory override:").into());
    populate_path_editing_content(
        "path/to/directory",
        &script.working_directory,
        parameters,
        move |path| WindowMessage::EditScriptWorkingDirectory(config_script_id, path),
        move |val| WindowMessage::EditScriptWorkingDirectoryPathType(config_script_id, val),
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(text("Path to the icon:").into());
    populate_path_editing_content(
        "path/to/icon.png",
        &script.icon,
        parameters,
        move |path| WindowMessage::EditScriptIconPath(config_script_id, path),
        move |val| WindowMessage::EditScriptIconPathType(config_script_id, val),
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(text("Default arguments:").into());
    parameters.push(
        text_input(&script.arguments_hint, &script.arguments_line)
            .on_input(move |new_value| {
                WindowMessage::EditArgumentsLineForConfig(config_script_id, new_value)
            })
            .style(text_input::default)
            .padding(5)
            .id(ARGUMENTS_INPUT_ID.clone())
            .into(),
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(text("Argument hint:").into());
    parameters.push(
        text_input("", &script.arguments_hint)
            .on_input(move |new_value| {
                WindowMessage::EditArgumentsHint(config_script_id, new_value)
            })
            .padding(5)
            .into(),
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(
        row![
            text("Argument placeholders:"),
            Space::with_width(4),
            help_icon(ARGUMENT_PLACEHOLDERS_HELP_TEXT, visual_caches, theme),
        ]
        .into(),
    );

    populate_argument_placeholders_config_content(
        parameters,
        &script.argument_placeholders,
        config_script_id,
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(text("Are arguments required:").into());
    parameters.push(
        pick_list(
            ARGUMENT_REQUIREMENT_PICK_LIST,
            Some(script.arguments_requirement.clone()),
            move |val| WindowMessage::EditArgumentsRequirement(config_script_id, val),
        )
        .into(),
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(
        row![
            text("Retry count:"),
            Space::with_width(4),
            help_icon(RETRY_COUNT_HELP_TEXT, visual_caches, theme),
        ]
        .into(),
    );
    parameters.push(
        text_input("0", &visual_caches.autorerun_count)
            .on_input(move |new_value| {
                WindowMessage::EditAutorerunCountForConfig(config_script_id, new_value)
            })
            .padding(5)
            .into(),
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(
        row![
            checkbox("Set custom executor", script.custom_executor.is_some()).on_toggle(
                move |val| WindowMessage::ToggleUseCustomExecutor(config_script_id, val)
            ),
            Space::with_width(4),
            help_icon(CUSTOM_EXECUTOR_HELP_TEXT, visual_caches, theme),
        ]
        .into(),
    );

    if let Some(mut custom_executor) = script.custom_executor.clone() {
        populate_string_vec_edit_content(
            parameters,
            &mut custom_executor,
            move |idx, new_value| {
                WindowMessage::EditCustomExecutor(config_script_id, new_value, idx)
            },
        );

        parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
        parameters.push(
            row![
                text("Executor arguments:"),
                Space::with_width(4),
                help_icon(EXECUTOR_ARGUMENTS_HELP_TEXT, visual_caches, theme),
            ]
            .into(),
        );
        populate_string_vec_edit_content(
            parameters,
            &script.executor_arguments,
            move |idx, new_value| {
                WindowMessage::EditExecutorArguments(config_script_id, new_value, idx)
            },
        );
    }

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(text("Reaction to previous failures:").into());
    parameters.push(
        pick_list(
            CONFIG_REACTION_TO_PREVIOUS_FAILURES_PICK_LIST,
            Some(script.reaction_to_previous_failures.clone()),
            move |val| {
                WindowMessage::EditReactionToPreviousFailuresForConfig(config_script_id, val)
            },
        )
        .into(),
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(
        checkbox("Autoclean on success", script.autoclean_on_success)
            .on_toggle(move |val| {
                WindowMessage::ToggleAutocleanOnSuccessForConfig(config_script_id, val)
            })
            .into(),
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(
        row![
            checkbox("Ignore output", script.ignore_output)
                .on_toggle(move |val| WindowMessage::ToggleIgnoreOutput(config_script_id, val)),
            Space::with_width(4),
            help_icon(IGNORE_OUTPUT_HELP_TEXT, visual_caches, theme),
        ]
        .into(),
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(
        checkbox("Is script hidden", script.is_hidden)
            .on_toggle(move |val| WindowMessage::ToggleIsHidden(config_script_id, val))
            .into(),
    );
}

fn populate_original_preset_edit_content<'a>(
    parameters: &mut Vec<Element<'_, WindowMessage, Theme, iced::Renderer>>,
    config_script_id: ConfigScriptId,
    preset: &config::ScriptPreset,
) {
    parameters.push(text("Preset name:").into());
    parameters.push(
        text_input("name", &preset.name)
            .on_input(move |new_arg| {
                WindowMessage::EditScriptNameForConfig(config_script_id, new_arg)
            })
            .padding(5)
            .into(),
    );

    parameters.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    parameters.push(text("Path to the icon:").into());
    populate_path_editing_content(
        "path/to/icon.png",
        &preset.icon,
        parameters,
        move |path| WindowMessage::EditScriptIconPath(config_script_id, path),
        move |val| WindowMessage::EditScriptIconPathType(config_script_id, val),
    );
}

fn produce_settings_edit_content<'a>(
    config: &config::AppConfig,
    window_edit: &WindowEditData,
    visual_caches: &VisualCaches,
    edit_mode: config::ConfigEditMode,
) -> Column<'a, WindowMessage> {
    let mut header_elements: Vec<Element<'_, WindowMessage, Theme, iced::Renderer>> =
        Vec::with_capacity(4);
    let mut list_elements: Vec<Element<'_, WindowMessage, Theme, iced::Renderer>> =
        Vec::with_capacity(40);

    let rewritable_config = config::get_rewritable_config(&config, edit_mode);

    if config.local_config_body.is_some() {
        match edit_mode {
            config::ConfigEditMode::Local => {
                header_elements.push(
                    button(text("Switch to shared settings").size(16))
                        .on_press(WindowMessage::SwitchToSharedSettingsConfig)
                        .into(),
                );
                header_elements.push(horizontal_rule(SEPARATOR_HEIGHT).into());
                list_elements.push(text("Editing local settings").into());
            }
            config::ConfigEditMode::Shared => {
                header_elements.push(
                    button(text("Switch to local settings").size(16))
                        .on_press(WindowMessage::SwitchToLocalSettingsConfig)
                        .into(),
                );
                header_elements.push(horizontal_rule(SEPARATOR_HEIGHT).into());
                list_elements.push(text("Editing shared settings").into());
            }
        }
    }

    list_elements.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    list_elements.push(
        checkbox(
            "Window status reactions",
            rewritable_config.window_status_reactions,
        )
        .on_toggle(move |val| WindowMessage::SettingsToggleWindowStatusReactions(edit_mode, val))
        .into(),
    );
    list_elements.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    list_elements.push(
        checkbox("Keep window size", rewritable_config.keep_window_size)
            .on_toggle(move |val| WindowMessage::SettingsToggleKeepWindowSize(edit_mode, val))
            .into(),
    );
    list_elements.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    list_elements.push(
        checkbox(
            "Show script filter",
            rewritable_config.enable_script_filtering,
        )
        .on_toggle(move |val| WindowMessage::SettingsToggleScriptFiltering(edit_mode, val))
        .into(),
    );
    list_elements.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    list_elements.push(
        checkbox(
            "Show working directory",
            rewritable_config.show_working_directory,
        )
        .on_toggle(move |val| WindowMessage::SettingsToggleShowWorkingDirectory(edit_mode, val))
        .into(),
    );
    list_elements.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    list_elements.push(
        checkbox(
            "Allow edit custom title",
            rewritable_config.enable_title_editing,
        )
        .on_toggle(move |val| WindowMessage::SettingsToggleTitleEditing(edit_mode, val))
        .into(),
    );
    list_elements.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    list_elements.push(text("Update config version:").into());
    list_elements.push(
        pick_list(
            CONFIG_UPDATE_BEHAVIOR_PICK_LIST,
            Some(rewritable_config.config_version_update_behavior),
            move |selected| WindowMessage::SettingsUpdateBehaviorChanged(edit_mode, selected),
        )
        .into(),
    );
    list_elements.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    list_elements.push(
        checkbox(
            "Show current git branch",
            rewritable_config.show_current_git_branch,
        )
        .on_toggle(move |val| WindowMessage::SettingsToggleShowCurrentGitBranch(edit_mode, val))
        .into(),
    );
    list_elements.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    list_elements.push(
        checkbox("Use custom theme", rewritable_config.custom_theme.is_some())
            .on_toggle(move |val| WindowMessage::SettingsToggleUseCustomTheme(edit_mode, val))
            .into(),
    );

    if let Some(_theme) = &rewritable_config.custom_theme {
        list_elements.push(horizontal_rule(SEPARATOR_HEIGHT).into());
        list_elements.push(text("Background:").into());
        list_elements.push(
            text_input("#000000", &window_edit.theme_color_background)
                .on_input(move |new_value| {
                    WindowMessage::SettingsEditThemeBackground(edit_mode, new_value)
                })
                .padding(5)
                .into(),
        );
        list_elements.push(text("Accent:").into());
        list_elements.push(
            text_input("#000000", &window_edit.theme_color_text)
                .on_input(move |new_value| {
                    WindowMessage::SettingsEditThemeText(edit_mode, new_value)
                })
                .padding(5)
                .into(),
        );
        list_elements.push(text("Primary:").into());
        list_elements.push(
            text_input("#000000", &window_edit.theme_color_primary)
                .on_input(move |new_value| {
                    WindowMessage::SettingsEditThemePrimary(edit_mode, new_value)
                })
                .padding(5)
                .into(),
        );
        list_elements.push(text("Success:").into());
        list_elements.push(
            text_input("#000000", &window_edit.theme_color_success)
                .on_input(move |new_value| {
                    WindowMessage::SettingsEditThemeSuccess(edit_mode, new_value)
                })
                .padding(5)
                .into(),
        );
        list_elements.push(text("Danger:").into());
        list_elements.push(
            text_input("#000000", &window_edit.theme_color_danger)
                .on_input(move |new_value| {
                    WindowMessage::SettingsEditThemeDanger(edit_mode, new_value)
                })
                .padding(5)
                .into(),
        );
        list_elements.push(text("Caption text:").into());
        list_elements.push(
            text_input("#000000", &window_edit.theme_color_caption_text)
                .on_input(move |new_value| {
                    WindowMessage::SettingsEditThemeCaptionText(edit_mode, new_value)
                })
                .padding(5)
                .into(),
        );
        list_elements.push(text("Error text:").into());
        list_elements.push(
            text_input("#000000", &window_edit.theme_color_error_text)
                .on_input(move |new_value| {
                    WindowMessage::SettingsEditThemeErrorText(edit_mode, new_value)
                })
                .padding(5)
                .into(),
        );
    }
    list_elements.push(horizontal_rule(SEPARATOR_HEIGHT).into());
    list_elements.push(text("Keybinds").into());

    keybind_editing::populate_keybind_editing_content(
        &mut list_elements,
        window_edit,
        visual_caches,
        "Enter/exit focus mode:",
        keybind_editing::KeybindAssociatedData::AppAction(
            config::AppAction::MaximizeOrRestoreExecutionPane,
        ),
    );

    keybind_editing::populate_keybind_editing_content(
        &mut list_elements,
        window_edit,
        visual_caches,
        "Enter/exit editing mode:",
        keybind_editing::KeybindAssociatedData::AppAction(
            config::AppAction::TrySwitchWindowEditMode,
        ),
    );

    keybind_editing::populate_keybind_editing_content(
        &mut list_elements,
        window_edit,
        visual_caches,
        "Try safely close app:",
        keybind_editing::KeybindAssociatedData::AppAction(config::AppAction::RequestCloseApp),
    );

    keybind_editing::populate_keybind_editing_content(
        &mut list_elements,
        window_edit,
        visual_caches,
        "Focus filter:",
        keybind_editing::KeybindAssociatedData::AppAction(config::AppAction::FocusFilter),
    );

    keybind_editing::populate_keybind_editing_content(
        &mut list_elements,
        window_edit,
        visual_caches,
        "Run scripts after execution:",
        keybind_editing::KeybindAssociatedData::AppAction(
            config::AppAction::RunScriptsAfterExecution,
        ),
    );

    keybind_editing::populate_keybind_editing_content(
        &mut list_elements,
        window_edit,
        visual_caches,
        "Run scripts in parallel:",
        keybind_editing::KeybindAssociatedData::AppAction(config::AppAction::RunScriptsInParallel),
    );

    keybind_editing::populate_keybind_editing_content(
        &mut list_elements,
        window_edit,
        visual_caches,
        "Stop scripts:",
        keybind_editing::KeybindAssociatedData::AppAction(config::AppAction::StopScripts),
    );

    keybind_editing::populate_keybind_editing_content(
        &mut list_elements,
        window_edit,
        visual_caches,
        "Remove selected script:",
        keybind_editing::KeybindAssociatedData::AppAction(config::AppAction::RemoveCursorScript),
    );

    keybind_editing::populate_keybind_editing_content(
        &mut list_elements,
        window_edit,
        visual_caches,
        "Clear scripts:",
        keybind_editing::KeybindAssociatedData::AppAction(config::AppAction::ClearExecutionScripts),
    );

    keybind_editing::populate_keybind_editing_content(
        &mut list_elements,
        window_edit,
        visual_caches,
        "Reschedule previous execution:",
        keybind_editing::KeybindAssociatedData::AppAction(config::AppAction::RescheduleScripts),
    );

    keybind_editing::populate_keybind_editing_content(
        &mut list_elements,
        window_edit,
        visual_caches,
        "Move selected script down:",
        keybind_editing::KeybindAssociatedData::AppAction(config::AppAction::MoveScriptDown),
    );

    keybind_editing::populate_keybind_editing_content(
        &mut list_elements,
        window_edit,
        visual_caches,
        "Move selected script up:",
        keybind_editing::KeybindAssociatedData::AppAction(config::AppAction::MoveScriptUp),
    );

    if edit_mode == config::ConfigEditMode::Shared {
        list_elements.push(horizontal_rule(SEPARATOR_HEIGHT).into());
        list_elements.push(text("Local config path:").into());
        populate_path_editing_content(
            "path/to/config.json",
            &config.local_config_path,
            &mut list_elements,
            |path| WindowMessage::SettingsSharedEditLocalConfigPath(path),
            |val| WindowMessage::SettingsSharedEditLocalConfigPathRelativeToScripter(val),
        );
    }

    list_elements.push(horizontal_rule(SEPARATOR_HEIGHT).into());

    column![
        column(header_elements),
        scrollable(column(list_elements).spacing(10))
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(Alignment::Start)
}

fn view_content<'a>(
    execution_lists: &parallel_execution_manager::ParallelExecutionManager,
    variant: &PaneVariant,
    theme: &Theme,
    displayed_configs_list_cache: &Vec<ScriptListCacheRecord>,
    paths: &config::PathCaches,
    visual_caches: &'a VisualCaches,
    config: &config::AppConfig,
    edit_data: &EditData,
    window_state: &WindowState,
) -> Element<'a, WindowMessage> {
    let content = match variant {
        PaneVariant::ScriptList => produce_script_list_content(
            execution_lists,
            config,
            displayed_configs_list_cache,
            edit_data,
            &visual_caches,
            window_state,
            theme,
        ),
        PaneVariant::ExecutionList => produce_execution_list_content(
            execution_lists,
            paths,
            theme,
            config,
            &visual_caches,
            edit_data,
            window_state,
        ),
        PaneVariant::LogOutput => produce_log_output_content(
            execution_lists,
            theme,
            config::get_main_rewritable_config(&config),
            &visual_caches,
        ),
        PaneVariant::Parameters => match &edit_data.window_edit_data {
            Some(window_edit_data) if window_edit_data.settings_edit_mode.is_some() => {
                let edit_mode = window_edit_data.settings_edit_mode.unwrap();
                produce_settings_edit_content(config, window_edit_data, visual_caches, edit_mode)
            }
            _ => produce_script_edit_content(
                execution_lists,
                visual_caches,
                edit_data,
                config,
                window_state,
                theme,
            ),
        },
    };

    container(content)
        .width(Length::Fill)
        .padding(5)
        .center_y(Length::Fill)
        .into()
}

fn view_controls<'a>(
    pane: pane_grid::Pane,
    variant: &PaneVariant,
    total_panes: usize,
    visual_caches: &VisualCaches,
    edit_data: &EditData,
    config: &config::AppConfig,
    execution_lists: &parallel_execution_manager::ParallelExecutionManager,
    is_maximized: bool,
    size: Size,
    window_state: &WindowState,
    theme: &Theme,
) -> Element<'a, WindowMessage> {
    let mut row = row![].spacing(5);

    if *variant == PaneVariant::ScriptList
        && !config.is_read_only
        && !edit_data.is_dirty
        && !edit_data.window_edit_data.is_some()
        && !execution_lists.has_any_execution_started()
    {
        row = row.push(
            tooltip(
                edit_mode_button(
                    visual_caches
                        .icons
                        .get_theme_for_color(theme.extended_palette().secondary.base.text)
                        .settings
                        .clone(),
                    WindowMessage::EnterWindowEditMode,
                    window_state,
                    visual_caches,
                ),
                "Edit configuration",
                tooltip::Position::Left,
            )
            .style(container::bordered_box),
        );
    }

    if total_panes > 1
        && (is_maximized
            || (*variant == PaneVariant::ExecutionList
                && (execution_lists.has_any_execution_started()
                    || !execution_lists.get_edited_scripts().is_empty())
                && edit_data.window_edit_data.is_none()))
    {
        let toggle = {
            let (content, message) = if is_maximized {
                (
                    if window_state.is_command_key_down {
                        format_keybind_hint(
                            visual_caches,
                            "Restore full window",
                            config::AppAction::MaximizeOrRestoreExecutionPane,
                        )
                    } else {
                        "Restore full window".to_string()
                    },
                    WindowMessage::ExitFocusMode,
                )
            } else {
                // adjust for window decorations
                let window_size = Size {
                    width: size.width + 3.0,
                    height: size.height + 3.0,
                };

                (
                    if window_state.is_command_key_down {
                        format_keybind_hint(
                            visual_caches,
                            "Focus",
                            config::AppAction::MaximizeOrRestoreExecutionPane,
                        )
                    } else {
                        "Focus".to_string()
                    },
                    WindowMessage::EnterFocusMode(pane, window_size),
                )
            };
            button(
                text(content)
                    .size(14)
                    .line_height(LineHeight::Absolute(iced::Pixels(14.0))),
            )
            .style(button::secondary)
            .padding(3)
            .on_press(message)
        };

        row = row.push(toggle);
    }

    row.into()
}

fn update_autorerun_count_text(
    app: &mut MainWindow,
    new_autorerun_count_str: String,
) -> Option<usize> {
    let parse_result = usize::from_str(&new_autorerun_count_str);
    let mut new_autorerun_count = None;
    if let Ok(parse_result) = parse_result {
        app.visual_caches.autorerun_count = new_autorerun_count_str;
        new_autorerun_count = Some(parse_result);
    } else {
        // if input is empty, then keep it empty and assume 0, otherwise keep the old value
        if new_autorerun_count_str.is_empty() {
            app.visual_caches.autorerun_count = new_autorerun_count_str;
            new_autorerun_count = Some(0);
        }
    }
    new_autorerun_count
}

fn get_script_config_bring_into_view_scroll_offset(
    app: &MainWindow,
    index: usize,
    spacing: f32,
) -> Option<f32> {
    if app.window_state.has_maximized_pane {
        return None;
    }

    let script_offset = get_script_list_script_offset(index);

    let script_scroll_height = {
        let regions = app
            .panes
            .layout()
            .pane_regions(PANE_SPACING, app.window_state.full_window_size);

        let script_list_pane = app.pane_by_pane_type[&PaneVariant::ScriptList];

        let Some(script_list_pane_region) = regions.get(&script_list_pane) else {
            return None;
        };

        script_list_pane_region.height - get_edited_script_list_content_offset_y(&app)
    };

    if script_offset < app.window_state.script_list_scroll_offset + spacing {
        Some(script_offset - spacing)
    } else if script_offset + ONE_SCRIPT_LIST_ELEMENT_HEIGHT
        > script_scroll_height + app.window_state.script_list_scroll_offset - spacing
    {
        Some(script_offset - script_scroll_height + ONE_SCRIPT_LIST_ELEMENT_HEIGHT + spacing)
    } else {
        None
    }
}

fn scroll_cursor_script_into_view(app: &MainWindow) -> Task<WindowMessage> {
    let Some(cursor_script) = &app.window_state.cursor_script else {
        return Task::none();
    };

    match cursor_script.script_type {
        EditScriptType::ScriptConfig => {
            let new_offset =
                get_script_config_bring_into_view_scroll_offset(app, cursor_script.idx, 10.0);
            if let Some(new_offset) = new_offset {
                scrollable::scroll_to(
                    SCRIPTS_PANE_SCROLL_ID.clone(),
                    AbsoluteOffset {
                        x: 0.0,
                        y: new_offset,
                    },
                )
            } else {
                Task::none()
            }
        }
        EditScriptType::ExecutionList => Task::none(),
    }
}

fn adjust_script_list_scroll_for_drop_pos(
    app: &mut MainWindow,
    drop_pos: usize,
) -> Task<WindowMessage> {
    let new_offset = get_script_config_bring_into_view_scroll_offset(
        app,
        drop_pos,
        ONE_SCRIPT_LIST_ELEMENT_HEIGHT + 10.0,
    );
    if let Some(new_offset) = new_offset {
        scrollable::scroll_to(
            SCRIPTS_PANE_SCROLL_ID.clone(),
            AbsoluteOffset {
                x: 0.0,
                y: new_offset,
            },
        )
    } else {
        Task::none()
    }
}
