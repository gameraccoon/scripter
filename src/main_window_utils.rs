// Copyright (C) Pavel Grebnev 2023-2025
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use crate::config;
use crate::drag_and_drop::{DragAndDropList, DropArea};
use crate::events;
use crate::git_support;
use crate::keybind_editing;
use crate::main_window::*;
use crate::parallel_execution_manager;
use crate::style;
use crate::{color_utils, execution_thread};

use crate::config::get_current_rewritable_config;
use crate::sorted_vec::SortedVec;
use iced::advanced::image::Handle;
use iced::widget::{pane_grid, text_input};
use iced::window::resize;
use iced::{keyboard, window, Size, Task, Theme};
use std::path::PathBuf;

pub(crate) const ONE_EXECUTION_LIST_ELEMENT_HEIGHT: f32 = 30.0;
pub(crate) const ONE_SCRIPT_LIST_ELEMENT_HEIGHT: f32 = 30.0;
const ONE_TITLE_LINE_HEIGHT: f32 = 20.0;
const ONE_EXECUTION_NAME_HEIGHT: f32 = 32.0;
const EMPTY_EXECUTION_LIST_HEIGHT: f32 = 70.0;
const EXECUTION_EDIT_BUTTONS_HEIGHT: f32 = 50.0;
const DIRTY_CONFIG_BUTTONS_HEIGHT: f32 = 34.0;
pub(crate) const PANE_SPACING: f32 = 1.0;
pub(crate) const SEPARATOR_HEIGHT: u16 = 8;
pub(crate) const PANE_HEADER_HEIGHT: f32 = 47.0;
const SCRIPT_FILTER_HEIGHT: f32 = 30.0;
const CONFIG_EDIT_HEADER_HEIGHT: f32 = 100.0;
const FIRST_EXECUTION_ELEMENT_OFFSET_Y: f32 = 10.0;
const SCROLL_BAR_WIDTH: f32 = 15.0;

#[derive(Clone, Debug, Copy)]
pub(crate) struct ConfigScriptId {
    pub idx: usize,
    pub edit_mode: config::ConfigEditMode,
}

pub fn is_local_config_script(script_idx: usize, app_config: &config::AppConfig) -> bool {
    if let Some(scripts) = &app_config.local_config_body {
        match scripts.script_definitions.get(script_idx) {
            Some(config::ScriptDefinition::Original(_)) => true,
            Some(config::ScriptDefinition::Preset(_)) => true,
            _ => false,
        }
    } else {
        false
    }
}

pub fn is_original_script_missing_arguments(script: &config::OriginalScriptDefinition) -> bool {
    if script.arguments_requirement == config::ArgumentRequirement::Required
        && script.arguments_line.is_empty()
    {
        return true;
    }

    for argument_placeholder in &script.argument_placeholders {
        if argument_placeholder.is_required && argument_placeholder.value.is_empty() {
            return true;
        }
    }

    false
}

pub fn is_script_in_quick_launch_buttons(
    visual_caches: &VisualCaches,
    script_uid: &config::Guid,
) -> bool {
    // ToDo: this is not scalable, need to make a hash set to search
    visual_caches
        .quick_launch_buttons
        .iter()
        .find(|button| *script_uid == button.script_uid)
        .is_some()
}

pub fn is_command_key(key: &keyboard::Key) -> bool {
    #[cfg(target_os = "macos")]
    {
        key.eq(&keyboard::Key::Named(keyboard::key::Named::Super))
    }
    #[cfg(not(target_os = "macos"))]
    {
        key.eq(&keyboard::Key::Named(keyboard::key::Named::Control))
    }
}

pub fn get_theme(config: &config::AppConfig) -> Theme {
    if let Some(theme) = config::get_rewritable_config(&config, config::get_main_edit_mode(&config))
        .custom_theme
        .clone()
    {
        style::get_custom_theme(theme)
    } else {
        Theme::default()
    }
}

pub fn get_script_definition(
    app_config: &config::AppConfig,
    edit_mode: config::ConfigEditMode,
    script_idx: usize,
) -> &config::ScriptDefinition {
    let is_looking_at_local_config = edit_mode == config::ConfigEditMode::Local;

    if is_looking_at_local_config {
        &app_config
            .local_config_body
            .as_ref()
            .unwrap()
            .script_definitions[script_idx]
    } else {
        &app_config.script_definitions[script_idx]
    }
}

fn get_script_definition_mut(
    app_config: &mut config::AppConfig,
    config_script_id: ConfigScriptId,
) -> &mut config::ScriptDefinition {
    let script_definitions =
        config::get_script_definition_list_mut(app_config, config_script_id.edit_mode);
    &mut script_definitions[config_script_id.idx]
}

pub fn get_resulting_scripts_from_guid(
    app_config: &config::AppConfig,
    script_uid: config::Guid,
) -> Vec<config::OriginalScriptDefinition> {
    let original_script = config::get_original_script_definition_by_uid(&app_config, &script_uid);

    let (original_script, _idx) = if let Some(original_script) = original_script {
        original_script
    } else {
        return Vec::new();
    };

    match original_script {
        config::ScriptDefinition::ReferenceToShared(_) => Vec::new(),
        config::ScriptDefinition::Original(script) => {
            vec![script.clone()]
        }
        config::ScriptDefinition::Preset(preset) => {
            let resulting_scripts = preset
                .items
                .iter()
                .map(|preset_item| {
                    (
                        config::get_original_script_definition_by_uid(
                            &app_config,
                            &preset_item.uid,
                        ),
                        preset_item,
                    )
                })
                .filter(|(optional_definition, _preset_item)| optional_definition.is_some())
                .map(|(optional_definition, preset_item)| {
                    let (new_script, _idx) = optional_definition.unwrap();
                    let script = match new_script.clone() {
                        config::ScriptDefinition::Original(mut script) => {
                            if let Some(name) = &preset_item.name {
                                script.name = name.clone();
                            }

                            if let Some(arguments) = &preset_item.arguments_line {
                                script.arguments_line = arguments.clone();
                            }

                            for (placeholder, value) in &preset_item.overridden_placeholder_values {
                                for script in &mut script.argument_placeholders {
                                    if script.placeholder == *placeholder {
                                        script.value = value.clone();
                                    }
                                }
                            }

                            if let Some(autorerun_count) = preset_item.autorerun_count {
                                script.autorerun_count = autorerun_count;
                            }

                            if let Some(reaction_to_previous_failures) =
                                preset_item.reaction_to_previous_failures
                            {
                                script.reaction_to_previous_failures =
                                    reaction_to_previous_failures;
                            }

                            if let Some(autoclean_on_success) = preset_item.autoclean_on_success {
                                script.autoclean_on_success = autoclean_on_success;
                            }

                            script
                        }
                        _ => {
                            panic!("Preset shouldn't contain presets or references");
                        }
                    };

                    script
                })
                .collect();

            resulting_scripts
        }
    }
}

fn find_script_idx_by_id(
    script_definitions: &Vec<config::ScriptDefinition>,
    script_id: &config::Guid,
) -> Option<usize> {
    for i in 0..script_definitions.len() {
        match &script_definitions[i] {
            config::ScriptDefinition::Original(script) => {
                if script.uid == *script_id {
                    return Some(i);
                }
            }
            config::ScriptDefinition::Preset(preset) => {
                if preset.uid == *script_id {
                    return Some(i);
                }
            }
            config::ScriptDefinition::ReferenceToShared(reference) => {
                if reference.uid == *script_id {
                    return Some(i);
                }
            }
        }
    }
    None
}

pub fn get_editing_preset(
    app_config: &mut config::AppConfig,
    config_script_id: ConfigScriptId,
) -> Option<&mut config::ScriptPreset> {
    let script_definition = get_script_definition_mut(app_config, config_script_id);
    if let config::ScriptDefinition::Preset(preset) = script_definition {
        return Some(preset);
    }
    None
}

pub fn find_best_shared_script_insert_position(
    source_script_definitions: &Vec<config::ScriptDefinition>,
    target_script_definitions: &Vec<config::ScriptDefinition>,
    script_idx: usize,
) -> usize {
    // first search up to find if we have reference to shared scripts
    let mut last_shared_script_idx = script_idx;
    let mut target_shared_script_uid = config::GUID_NULL;
    for i in (0..script_idx).rev() {
        if let config::ScriptDefinition::ReferenceToShared(reference) =
            &source_script_definitions[i]
        {
            last_shared_script_idx = i;
            target_shared_script_uid = reference.uid.clone();
            break;
        }
    }

    if last_shared_script_idx != script_idx {
        return find_script_idx_by_id(target_script_definitions, &target_shared_script_uid)
            .unwrap_or(target_script_definitions.len() - 1)
            + 1;
    }

    // search down
    let mut next_shared_script_idx = script_idx;
    let mut target_shared_script_idx = config::GUID_NULL;
    for i in script_idx..source_script_definitions.len() {
        if let config::ScriptDefinition::ReferenceToShared(reference) =
            &source_script_definitions[i]
        {
            next_shared_script_idx = i;
            target_shared_script_idx = reference.uid.clone();
            break;
        }
    }

    if next_shared_script_idx != script_idx {
        return find_script_idx_by_id(target_script_definitions, &target_shared_script_idx)
            .unwrap_or(target_script_definitions.len());
    }

    // if we didn't find any shared scripts, just insert at the end
    target_script_definitions.len()
}

pub fn get_next_pane_selection(app: &MainWindow, is_forward: bool) -> PaneVariant {
    if let Some(focus) = app.window_state.pane_focus {
        // try to predict what the user wants to do

        let is_editing = app.edit_data.window_edit_data.is_some();
        let selected_script_type = app
            .window_state
            .selected_scripts
            .as_ref()
            .map(|s| &s.script_type);

        let have_scripts_in_execution = !app.execution_manager.get_edited_scripts().is_empty();
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

pub fn get_window_message_from_app_action(app_action: config::AppAction) -> WindowMessage {
    match app_action {
        config::AppAction::RequestCloseApp => WindowMessage::RequestCloseApp,
        config::AppAction::FocusFilter => WindowMessage::FocusFilter,
        config::AppAction::TrySwitchWindowEditMode => WindowMessage::TrySwitchWindowEditMode,
        config::AppAction::RescheduleScripts => WindowMessage::RescheduleScriptsHotkey,
        config::AppAction::RunScriptsInParallel => WindowMessage::RunEditedScriptsInParallel,
        config::AppAction::RunScriptsAfterExecution => {
            WindowMessage::RunEditedScriptsAfterExecutionHotkey
        }
        config::AppAction::StopScripts => WindowMessage::StopScriptsHotkey,
        config::AppAction::ClearExecutionScripts => WindowMessage::ClearExecutionScriptsHotkey,
        config::AppAction::MaximizeOrRestoreExecutionPane => {
            WindowMessage::MaximizeOrRestoreExecutionPane
        }
        config::AppAction::CursorConfirm => WindowMessage::CursorConfirm,
        config::AppAction::MoveScriptDown => WindowMessage::MoveScriptDown,
        config::AppAction::MoveScriptUp => WindowMessage::MoveScriptUp,
        config::AppAction::SwitchPaneFocusForward => WindowMessage::SwitchPaneFocus(true),
        config::AppAction::SwitchPaneFocusBackwards => WindowMessage::SwitchPaneFocus(false),
        config::AppAction::MoveCursorDown => WindowMessage::MoveCursorDown,
        config::AppAction::MoveCursorUp => WindowMessage::MoveCursorUp,
        config::AppAction::RemoveSelectedScripts => WindowMessage::RemoveSelectedScripts,
    }
}

pub fn get_run_script_window_message_from_guid(
    app_config: &config::AppConfig,
    script_uid: &config::Guid,
) -> Option<WindowMessage> {
    let original_script = config::get_original_script_definition_by_uid(app_config, &script_uid);
    if original_script.is_some() {
        return Some(WindowMessage::AddScriptToExecutionWithoutRunning(
            script_uid.clone(),
        ));
    }
    None
}

pub fn try_add_edited_scripts_to_execution_or_start_new(app: &mut MainWindow) {
    // we can accept this hotkey only if we definitely know what execution we
    // supposed to add it to
    let executions_number = app.execution_manager.get_started_executions().len();
    if executions_number == 1 {
        let execution_id = app
            .execution_manager
            .get_started_executions()
            .values()
            .next()
            .unwrap();
        add_edited_scripts_to_started_execution(app, execution_id.get_id());
    } else if executions_number == 0 {
        // if there are no executions, then we can start a new one
        start_new_execution_from_edited_scripts(app);
    }

    events::on_execution_pane_content_height_decreased(app);
}

pub fn try_add_script_to_execution_or_start_new(app: &mut MainWindow, script_uid: config::Guid) {
    // we can accept this hotkey only if we definitely know what execution we
    // supposed to add it to
    let executions_number = app.execution_manager.get_started_executions().len();
    let scripts_to_add = get_resulting_scripts_from_guid(&app.app_config, script_uid);

    if executions_number == 1 {
        let execution_id = app
            .execution_manager
            .get_started_executions()
            .values()
            .next()
            .unwrap()
            .get_id();

        app.execution_manager.add_scripts_to_running_execution(
            &app.app_config,
            execution_id,
            scripts_to_add,
        );
        update_drag_and_drop_area_bounds(app);
    } else if executions_number == 0 {
        // if there are no executions, then we can start a new one
        start_new_execution_from_provided_scripts(app, scripts_to_add);
    }

    events::on_execution_pane_content_height_decreased(app);
}

pub fn update_config_cache(app: &mut MainWindow) {
    let is_looking_at_local_config = app.app_config.local_config_body.is_some();

    let binding = app.edit_data.script_filter.to_lowercase();
    let search_words = binding.split_whitespace().collect::<Vec<&str>>();

    let is_full_list = app.edit_data.window_edit_data.is_some();

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

    let result_list = &mut app.displayed_configs_list_cache;
    let paths = &app.app_config.paths;
    if is_looking_at_local_config {
        let local_config = app.app_config.local_config_body.as_ref().unwrap();
        let shared_script_definitions = &app.app_config.script_definitions;

        result_list.clear();
        for script_definition in &local_config.script_definitions {
            match script_definition {
                config::ScriptDefinition::ReferenceToShared(reference) => {
                    let shared_script =
                        shared_script_definitions
                            .iter()
                            .find(|script| match script {
                                config::ScriptDefinition::Original(script) => {
                                    script.uid == reference.uid
                                }
                                config::ScriptDefinition::Preset(preset) => {
                                    preset.uid == reference.uid
                                }
                                _ => false,
                            });
                    match shared_script {
                        Some(shared_script) => {
                            let name = match &shared_script {
                                config::ScriptDefinition::ReferenceToShared(_) => {
                                    "[Error]".to_string()
                                }
                                config::ScriptDefinition::Original(script) => script.name.clone(),
                                config::ScriptDefinition::Preset(preset) => preset.name.clone(),
                            };
                            let icon = match &shared_script {
                                config::ScriptDefinition::ReferenceToShared(_) => {
                                    config::PathConfig::default()
                                }
                                config::ScriptDefinition::Original(script) => script.icon.clone(),
                                config::ScriptDefinition::Preset(preset) => preset.icon.clone(),
                            };
                            let is_script_hidden =
                                reference.is_hidden || is_script_filtered_out(&name);
                            add_cache_record(
                                result_list,
                                is_full_list,
                                is_script_hidden,
                                name,
                                reference.uid.clone(),
                                config::get_full_optional_path(paths, &icon),
                            );
                        }
                        None => {
                            eprintln!(
                                "Failed to find shared script with uid {}",
                                reference.uid.data
                            )
                        }
                    }
                }
                config::ScriptDefinition::Original(script) => {
                    let is_script_hidden = is_script_filtered_out(&script.name) || script.is_hidden;
                    add_cache_record(
                        result_list,
                        is_full_list,
                        is_script_hidden,
                        script.name.clone(),
                        script.uid.clone(),
                        config::get_full_optional_path(paths, &script.icon),
                    );
                }
                config::ScriptDefinition::Preset(preset) => {
                    let is_script_hidden = is_script_filtered_out(&preset.name);

                    add_cache_record(
                        result_list,
                        is_full_list,
                        is_script_hidden,
                        preset.name.clone(),
                        preset.uid.clone(),
                        config::get_full_optional_path(paths, &preset.icon),
                    );
                }
            }
        }
    } else {
        let script_definitions = &app.app_config.script_definitions;

        result_list.clear();
        for script_definition in script_definitions {
            match script_definition {
                config::ScriptDefinition::ReferenceToShared(_) => {}
                config::ScriptDefinition::Original(script) => {
                    let is_script_hidden = is_script_filtered_out(&script.name) || script.is_hidden;
                    add_cache_record(
                        result_list,
                        is_full_list,
                        is_script_hidden,
                        script.name.clone(),
                        script.uid.clone(),
                        config::get_full_optional_path(paths, &script.icon),
                    );
                }
                config::ScriptDefinition::Preset(preset) => {
                    let is_script_hidden = is_script_filtered_out(&preset.name);
                    add_cache_record(
                        result_list,
                        is_full_list,
                        is_script_hidden,
                        preset.name.clone(),
                        preset.uid.clone(),
                        config::get_full_optional_path(paths, &preset.icon),
                    );
                }
            }
        }
    }

    app.visual_caches.quick_launch_buttons.clear();
    let rewritable_config = config::get_main_rewritable_config(&app.app_config);
    for script_uid in &rewritable_config.quick_launch_scripts {
        let original_script =
            config::get_original_script_definition_by_uid(&app.app_config, &script_uid);
        let Some((script, _idx)) = original_script else {
            continue;
        };

        let (name, icon) = match script {
            config::ScriptDefinition::Original(script) => {
                (script.name.clone(), script.icon.clone())
            }
            config::ScriptDefinition::Preset(preset) => (preset.name.clone(), preset.icon.clone()),
            _ => continue,
        };

        let icon_path = config::get_full_optional_path(&app.app_config.paths, &icon);

        app.visual_caches
            .quick_launch_buttons
            .push(QuickLaunchButton {
                label: name,
                icon: Handle::from_path(icon_path.unwrap_or_default().as_path()),
                script_uid: script_uid.clone(),
            });
    }

    let rewritable_config = config::get_main_rewritable_config(&app.app_config);

    app.visual_caches.enable_script_filtering = rewritable_config.enable_script_filtering;
    app.visual_caches.enable_title_editing = rewritable_config.enable_title_editing;

    if app.edit_data.window_edit_data.is_some() {
        app.window_state
            .drag_and_drop_lists
            .script_list
            .change_number_of_elements(0);
        app.window_state
            .drag_and_drop_lists
            .edit_script_list
            .change_number_of_elements(app.displayed_configs_list_cache.len());
    } else {
        app.window_state
            .drag_and_drop_lists
            .script_list
            .change_number_of_elements(app.displayed_configs_list_cache.len());
        app.window_state
            .drag_and_drop_lists
            .edit_script_list
            .change_number_of_elements(0);
    }

    update_edited_execution_list_script_number(app);
}

pub fn add_cache_record(
    result_list: &mut Vec<ScriptListCacheRecord>,
    is_full_list: bool,
    is_script_hidden: bool,
    script_name: String,
    script_uid: config::Guid,
    script_icon_path: Option<PathBuf>,
) {
    if is_full_list || !is_script_hidden {
        result_list.push(ScriptListCacheRecord {
            name: script_name,
            full_icon_path: script_icon_path,
            is_hidden: is_script_hidden,
            original_script_uid: script_uid,
        });
    }
}

pub fn update_button_key_hint_caches(app: &mut MainWindow) {
    let mut last_stoppable_execution_id = None;
    let mut last_cleanable_execution_id = None;

    for execution in app
        .execution_manager
        .get_started_executions()
        .values()
        .rev()
    {
        if last_stoppable_execution_id.is_none() && !execution.has_finished_execution() {
            last_stoppable_execution_id = Some(execution.get_id());
        }

        if last_cleanable_execution_id.is_none() && execution.has_finished_execution() {
            last_cleanable_execution_id = Some(execution.get_id());
        }

        if last_stoppable_execution_id.is_some() && last_cleanable_execution_id.is_some() {
            break;
        }
    }

    app.visual_caches.button_key_caches = ButtonKeyCaches {
        last_stoppable_execution_id,
        last_cleanable_execution_id,
    }
}

pub fn update_git_branch_visibility(app: &mut MainWindow) {
    if get_current_rewritable_config(&app.app_config).show_current_git_branch {
        if app.visual_caches.git_branch_requester.is_none() {
            app.visual_caches.git_branch_requester =
                Some(git_support::GitCurrentBranchRequester::new());
        }
    } else {
        app.visual_caches.git_branch_requester = None;
    }
}

pub fn update_theme_icons(app: &mut MainWindow) {
    let icons = &mut app.visual_caches.icons;
    icons.themed = icons
        .get_theme_for_color(app.theme.extended_palette().primary.strong.text)
        .clone()
}

pub(crate) fn set_execution_lists_scroll_offset(app: &mut MainWindow, new_offset: f32) {
    app.window_state
        .drag_and_drop_lists
        .execution_edit_list
        .set_scroll_offset(new_offset);

    for drop_area in &mut app.window_state.drop_areas.running_executions {
        drop_area.set_scroll_offset(new_offset);
    }
}

pub fn remove_execution(
    app: &mut MainWindow,
    execution_id: parallel_execution_manager::ExecutionId,
) -> Option<parallel_execution_manager::Execution> {
    let idx = app
        .execution_manager
        .get_started_executions()
        .index(execution_id);

    let removed_execution = app.execution_manager.remove_execution(execution_id);

    // switch current log tab if the removed execution was selected
    if let Some(selected_execution) = app.visual_caches.selected_execution_log {
        if selected_execution == execution_id {
            // this is not actually needed since a wrong index will also not show anything
            // but just for the sake of debugging, let's clean it
            app.visual_caches.selected_execution_log = None;

            let last_execution = app
                .execution_manager
                .get_started_executions()
                .values()
                .last();
            if let Some(first_execution) = last_execution {
                app.visual_caches.selected_execution_log = Some(first_execution.get_id());
            }
        }
    }

    update_button_key_hint_caches(app);

    if let Some(idx) = idx {
        if idx < app.window_state.drop_areas.running_executions.len() {
            app.window_state.drop_areas.running_executions.remove(idx);
        }
    }

    update_drag_and_drop_area_bounds(app);
    events::on_execution_pane_content_height_decreased(app);

    removed_execution
}

pub fn switch_to_editing_settings_config(app: &mut MainWindow, edit_mode: config::ConfigEditMode) {
    clear_script_selection(&mut app.window_state.selected_scripts);
    app.edit_data.window_edit_data = Some(WindowEditData::from_config(
        &app.app_config,
        Some(edit_mode),
    ));
    apply_theme(app);
    keybind_editing::update_keybind_visual_caches(app, edit_mode);
    update_config_cache(app);
}

pub fn maximize_pane(
    app: &mut MainWindow,
    pane: pane_grid::Pane,
    window_size: Size,
) -> Task<WindowMessage> {
    if app.window_state.pane_focus != Some(pane) {
        clear_script_selection(&mut app.window_state.selected_scripts);
    }
    app.window_state.pane_focus = Some(pane);
    app.panes.maximize(pane);
    app.window_state.has_maximized_pane = true;
    if !get_current_rewritable_config(&app.app_config).keep_window_size {
        app.window_state.full_window_size = window_size.clone();
        let regions = app.panes.layout().pane_regions(PANE_SPACING, window_size);
        let size = regions.get(&pane);
        let Some(size) = size else {
            return Task::none();
        };

        let size = size.clone();

        let title_size_y = get_execution_list_title_size_y(app);
        let started_executions_size_y = get_started_execution_list_size_y(app);

        let edited_executions_size_y = {
            let edited_elements_count = app.execution_manager.get_edited_scripts().len() as u32;
            edited_elements_count as f32 * ONE_EXECUTION_LIST_ELEMENT_HEIGHT
                + if edited_elements_count > 0 {
                    EXECUTION_EDIT_BUTTONS_HEIGHT
                } else {
                    0.0
                }
        };

        let new_window_size = Size {
            width: size.width,
            height: f32::min(
                size.height,
                EMPTY_EXECUTION_LIST_HEIGHT
                    + title_size_y
                    + started_executions_size_y
                    + edited_executions_size_y,
            ),
        };
        events::on_window_resized(app, new_window_size);
        return window::get_oldest().and_then(move |window_id| resize(window_id, new_window_size));
    }

    Task::none()
}

pub(crate) fn create_script_from_file(app: &mut MainWindow, file_path: PathBuf) {
    if !file_path.is_file() {
        return;
    }

    let path_config = {
        if let Ok(path) = file_path.strip_prefix(&app.app_config.paths.exe_folder_path) {
            config::PathConfig {
                path: path.to_string_lossy().to_string(),
                path_type: config::PathType::ScripterExecutableRelative,
            }
        } else {
            let path = if let Ok(path) = file_path.strip_prefix(&app.app_config.paths.work_path) {
                path
            } else {
                &file_path
            };

            config::PathConfig {
                path: path.to_string_lossy().to_string(),
                path_type: config::PathType::WorkingDirRelative,
            }
        }
    };

    let name = file_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let (custom_executor, executor_arguments) = {
        if let Some(extension) = name.split(".").last() {
            if extension != name {
                if let Some(association) = get_current_rewritable_config(&app.app_config)
                    .file_associations
                    .iter()
                    .find(|association| association.extension == extension)
                {
                    (
                        Some(association.executor.clone()),
                        association.executor_arguments.clone(),
                    )
                } else {
                    (None, Vec::new())
                }
            } else {
                (None, Vec::new())
            }
        } else {
            (None, Vec::new())
        }
    };

    let script = config::OriginalScriptDefinition {
        name: if name.is_empty() {
            "new script".to_string()
        } else {
            get_pretty_name(name)
        },
        command: path_config,
        custom_executor,
        executor_arguments,
        ..config::OriginalScriptDefinition::default()
    };
    add_script_to_config(
        app,
        config::get_main_edit_mode(&app.app_config),
        config::ScriptDefinition::Original(script),
    );

    update_config_cache(app);
}

pub fn restore_window(app: &mut MainWindow) -> Task<WindowMessage> {
    app.window_state.has_maximized_pane = false;
    app.panes.restore();
    if !config::get_main_rewritable_config(&app.app_config).keep_window_size {
        let window_size = app.window_state.full_window_size.clone();
        events::on_window_resized(app, window_size);
        return window::get_oldest().and_then(move |window_id| resize(window_id, window_size));
    }
    Task::none()
}

pub fn move_cursor(app: &mut MainWindow, is_up: bool) {
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
            PaneVariant::ScriptList => app.displayed_configs_list_cache.len(),
            PaneVariant::ExecutionList => app.execution_manager.get_edited_scripts().len(),
            _ => unreachable!(),
        };

        if scripts_count == 0 {
            return;
        }

        let selected_script_type = app
            .window_state
            .selected_scripts
            .as_ref()
            .map(|x| x.script_type);
        let selected_script_idx =
            get_only_selected_script(&app.window_state.selected_scripts).map(|(idx, _)| idx);

        let next_selection = if selected_script_idx.is_none()
            || (selected_script_idx.is_some() && selected_script_type != Some(pane_script_type))
        {
            if is_up {
                scripts_count - 1
            } else {
                0
            }
        } else {
            let selected_script_idx = selected_script_idx.unwrap_or_default();
            if is_up {
                if selected_script_idx > 0 {
                    selected_script_idx - 1
                } else {
                    scripts_count - 1
                }
            } else {
                if selected_script_idx + 1 < scripts_count {
                    selected_script_idx + 1
                } else {
                    0
                }
            }
        };

        select_script_by_type(
            app,
            ConfigScriptId {
                idx: next_selection,
                edit_mode: config::get_main_edit_mode(&app.app_config),
            },
            pane_script_type,
        );
    }
}

pub fn start_new_execution_from_edited_scripts(app: &mut MainWindow) {
    if app.execution_manager.get_edited_scripts().is_empty() {
        return;
    }

    if app
        .execution_manager
        .get_edited_scripts()
        .iter()
        .any(|script| is_original_script_missing_arguments(&script.original))
    {
        return;
    }

    let scripts_to_execute = app.execution_manager.consume_edited_scripts();

    start_new_execution_from_provided_execution_scripts(app, scripts_to_execute);
}

pub fn start_new_execution_from_provided_scripts(
    app: &mut MainWindow,
    scripts: Vec<config::OriginalScriptDefinition>,
) {
    start_new_execution_from_provided_execution_scripts(
        app,
        scripts
            .into_iter()
            .map(|script| execution_thread::ExecutionScript::from_original(script))
            .collect(),
    );
}

pub fn start_new_execution_from_provided_execution_scripts(
    app: &mut MainWindow,
    scripts: Vec<execution_thread::ExecutionScript>,
) {
    if scripts
        .iter()
        .any(|script| is_original_script_missing_arguments(&script.original))
    {
        eprintln!("Some scripts are missing arguments");
        return;
    }

    clear_script_selection(&mut app.window_state.selected_scripts);
    let new_execution_id = app
        .execution_manager
        .start_new_execution(&app.app_config, scripts);

    app.window_state
        .drop_areas
        .running_executions
        .push(DropArea::new());

    app.visual_caches.selected_execution_log = Some(new_execution_id);
    update_button_key_hint_caches(app);
    update_drag_and_drop_area_bounds(app);
    cancel_all_drag_and_drop_operations(app);
}

pub fn add_edited_scripts_to_started_execution(
    app: &mut MainWindow,
    execution_id: parallel_execution_manager::ExecutionId,
) {
    if app.execution_manager.get_edited_scripts().is_empty() {
        return;
    }

    if app
        .execution_manager
        .get_edited_scripts()
        .iter()
        .any(|script| is_original_script_missing_arguments(&script.original))
    {
        return;
    }

    clear_script_selection(&mut app.window_state.selected_scripts);

    let scripts_to_execute = app.execution_manager.consume_edited_scripts();
    app.execution_manager
        .add_execution_scripts_to_running_execution(
            &app.app_config,
            execution_id,
            scripts_to_execute,
        );

    cancel_all_drag_and_drop_operations(app);
    update_drag_and_drop_area_bounds(app);
}

pub fn add_script_to_execution(
    app: &mut MainWindow,
    script_uid: config::Guid,
    should_focus: bool,
) -> bool {
    let scripts = get_resulting_scripts_from_guid(&app.app_config, script_uid);

    if scripts.is_empty() {
        return false;
    }

    for script in scripts {
        app.execution_manager.add_script_to_edited_list(script);
    }

    if should_focus {
        let script_idx = app.execution_manager.get_edited_scripts().len() - 1;
        select_execution_script(app, script_idx);
        app.window_state.pane_focus = Some(app.pane_by_pane_type[&PaneVariant::ExecutionList]);
    }

    update_edited_execution_list_script_number(app);

    true
}

pub fn take_edited_execution_script(
    execution_manager: &mut parallel_execution_manager::ParallelExecutionManager,
    uid: config::Guid,
    predicate: impl Fn(&config::OriginalScriptDefinition) -> bool,
) -> Option<execution_thread::ExecutionScript> {
    execution_manager
        .get_edited_scripts()
        .iter()
        .position(|script| script.uid == uid && predicate(&script.original))
        .and_then(|idx| Some(execution_manager.get_edited_scripts_mut().remove(idx)))
}

pub fn add_script_to_config(
    app: &mut MainWindow,
    edit_mode: config::ConfigEditMode,
    script: config::ScriptDefinition,
) {
    if app.edit_data.window_edit_data.is_none() {
        return;
    }

    let script_idx = match edit_mode {
        config::ConfigEditMode::Shared => {
            Some(add_script_to_shared_config(&mut app.app_config, script))
        }
        config::ConfigEditMode::Local => add_script_to_local_config(&mut app.app_config, script),
    };

    update_config_cache(app);

    app.edit_data
        .window_edit_data
        .as_mut()
        .unwrap()
        .settings_edit_mode = None;

    if let Some(script_idx) = script_idx {
        select_edited_script(
            app,
            ConfigScriptId {
                idx: script_idx,
                edit_mode,
            },
        );
        app.edit_data.is_dirty = true;
    }
}

pub fn make_script_copy(
    script: config::ScriptDefinition,
) -> (config::ScriptDefinition, config::Guid) {
    match script {
        config::ScriptDefinition::ReferenceToShared(_) => (script, config::GUID_NULL),
        config::ScriptDefinition::Preset(preset) => {
            let new_uid = config::Guid::new();
            (
                config::ScriptDefinition::Preset(config::ScriptPreset {
                    uid: new_uid.clone(),
                    name: format!("{} (copy)", preset.name),
                    ..preset
                }),
                new_uid,
            )
        }
        config::ScriptDefinition::Original(script) => {
            let new_uid = config::Guid::new();
            (
                config::ScriptDefinition::Original(config::OriginalScriptDefinition {
                    uid: new_uid.clone(),
                    name: format!("{} (copy)", script.name),
                    ..script
                }),
                new_uid,
            )
        }
    }
}

pub fn get_top_level_edited_script_idx_by_uid(
    app_config: &mut config::AppConfig,
    script_uid: &config::Guid,
) -> Option<usize> {
    let script_definitions = config::get_main_script_definition_list(app_config);

    for (idx, script) in script_definitions.iter().enumerate() {
        match script {
            config::ScriptDefinition::ReferenceToShared(script) => {
                if script.uid == *script_uid {
                    return Some(idx);
                }
            }
            config::ScriptDefinition::Original(script) => {
                if script.uid == *script_uid {
                    return Some(idx);
                }
            }
            config::ScriptDefinition::Preset(preset) => {
                if preset.uid == *script_uid {
                    return Some(idx);
                }
            }
        };
    }

    None
}

pub fn remove_config_script(app: &mut MainWindow, config_script_id: ConfigScriptId) {
    if app.edit_data.window_edit_data.is_some() {
        match config_script_id.edit_mode {
            config::ConfigEditMode::Shared => {
                app.app_config
                    .script_definitions
                    .remove(config_script_id.idx);
                app.edit_data.is_dirty = true;
            }
            config::ConfigEditMode::Local => {
                if let Some(config) = &mut app.app_config.local_config_body {
                    config.script_definitions.remove(config_script_id.idx);
                    app.edit_data.is_dirty = true;
                }
            }
        }
    }

    config::populate_shared_scripts_from_config(&mut app.app_config);
    update_config_cache(app);
    clear_script_selection(&mut app.window_state.selected_scripts);
    keybind_editing::prune_unused_keybinds(app);
}

pub fn remove_execution_list_scripts(app: &mut MainWindow, indexes: SortedVec<usize>) {
    app.execution_manager
        .remove_scripts_from_edited_list(indexes);
    update_edited_execution_list_script_number(app);
    clear_script_selection(&mut app.window_state.selected_scripts);
}

fn add_script_to_shared_config(
    app_config: &mut config::AppConfig,
    script: config::ScriptDefinition,
) -> usize {
    app_config.script_definitions.push(script);
    let script_idx = app_config.script_definitions.len() - 1;
    config::populate_shared_scripts_from_config(app_config);
    script_idx
}

fn add_script_to_local_config(
    app_config: &mut config::AppConfig,
    script: config::ScriptDefinition,
) -> Option<usize> {
    if let Some(config) = &mut app_config.local_config_body {
        config.script_definitions.push(script);
    } else {
        return None;
    }

    if let Some(config) = &mut app_config.local_config_body {
        Some(config.script_definitions.len() - 1)
    } else {
        None
    }
}

pub fn get_only_selected_script(
    selected_scripts: &Option<SelectedScripts>,
) -> Option<(usize, EditScriptType)> {
    match selected_scripts {
        Some(scripts) if scripts.indexes.len() == 1 => {
            Some((scripts.indexes[0], scripts.script_type))
        }
        _ => None,
    }
}

pub fn select_edited_script(app: &mut MainWindow, config_script_id: ConfigScriptId) {
    set_selected_script(
        &mut app.window_state.selected_scripts,
        config_script_id.idx,
        EditScriptType::ScriptConfig,
    );

    if let Some(script) =
        &config::get_script_definition_list(&app.app_config, config_script_id.edit_mode)
            .get(config_script_id.idx)
    {
        match script {
            config::ScriptDefinition::Original(script) => {
                app.visual_caches.autorerun_count = script.autorerun_count.to_string();
            }
            config::ScriptDefinition::ReferenceToShared(reference) => {
                let Some((script, _idx)) =
                    config::get_original_script_definition_by_uid(&app.app_config, &reference.uid)
                else {
                    app.visual_caches.autorerun_count = "Error 1".to_string();
                    return;
                };

                match script {
                    config::ScriptDefinition::Original(script) => {
                        app.visual_caches.autorerun_count = script.autorerun_count.to_string();
                    }
                    config::ScriptDefinition::ReferenceToShared(_) => {
                        app.visual_caches.autorerun_count = "Error 2".to_string();
                    }
                    config::ScriptDefinition::Preset(_) => {
                        app.visual_caches.autorerun_count = "Error 3".to_string();
                    }
                }
            }
            config::ScriptDefinition::Preset(_) => {
                app.visual_caches.autorerun_count = "Error 4".to_string();
            }
        }
    }

    if let Some(window_edit_data) = &mut app.edit_data.window_edit_data {
        window_edit_data.settings_edit_mode = None;
    }
}

pub fn select_execution_script(app: &mut MainWindow, script_idx: usize) {
    set_selected_script(
        &mut app.window_state.selected_scripts,
        script_idx,
        EditScriptType::ExecutionList,
    );

    if let Some(script) = &app.execution_manager.get_edited_scripts().get(script_idx) {
        app.visual_caches.autorerun_count = script.original.autorerun_count.to_string();
    }
}

fn select_script_by_type(
    app: &mut MainWindow,
    config_script_idx: ConfigScriptId,
    script_type: EditScriptType,
) {
    match script_type {
        EditScriptType::ScriptConfig => select_edited_script(app, config_script_idx),
        EditScriptType::ExecutionList => select_execution_script(app, config_script_idx.idx),
    }
}

fn set_selected_script(
    currently_edited_script: &mut Option<SelectedScripts>,
    script_idx: usize,
    script_type: EditScriptType,
) {
    *currently_edited_script = Some(SelectedScripts {
        indexes: SortedVec::from_one_value(script_idx),
        script_type: script_type.clone(),
    });
}

pub fn extend_selection_range(indexes: &mut SortedVec<usize>, new_range_point: usize) {
    if indexes.is_empty() {
        eprintln!("Should not call extend_selection_range with empty indexes");
        return;
    }

    indexes.unsafe_modify(|indexes_unsafe: &mut Vec<usize>| {
        let first_index = indexes_unsafe[0];
        // add all indexes between the first and the new point
        let mut insertion_idx = 1;
        if new_range_point > first_index {
            for idx in first_index + 1..=new_range_point {
                match indexes_unsafe.get(insertion_idx) {
                    None => indexes_unsafe.push(idx),
                    Some(existing_idx) if existing_idx > &idx => {
                        indexes_unsafe.insert(insertion_idx, idx);
                    }
                    _ => {}
                }
                insertion_idx += 1;
            }
        } else {
            indexes_unsafe.reserve(first_index - new_range_point);
            for new_idx in new_range_point..first_index {
                indexes_unsafe.push(new_idx)
            }
            indexes_unsafe.rotate_right(first_index - new_range_point);
        }
    });
}

pub fn clear_script_selection(currently_edited_script: &mut Option<SelectedScripts>) {
    *currently_edited_script = None;
}

pub fn shift_script_selection(app: &mut MainWindow, old_index: usize, new_index: usize) {
    if let Some((mut selected_script_idx, script_type)) =
        get_only_selected_script(&mut app.window_state.selected_scripts)
    {
        if old_index == selected_script_idx {
            if new_index <= old_index {
                selected_script_idx = new_index
            } else {
                selected_script_idx = new_index - 1
            }
        } else if old_index < selected_script_idx && new_index > selected_script_idx {
            selected_script_idx -= 1;
        } else if old_index > selected_script_idx && new_index <= selected_script_idx {
            selected_script_idx += 1;
        }

        app.window_state.selected_scripts = Some(SelectedScripts {
            indexes: SortedVec::from_one_value(selected_script_idx),
            script_type,
        });
    }
}

pub fn move_config_script_up(app: &mut MainWindow, index: usize) -> usize {
    let mut new_position = index;
    if app.edit_data.window_edit_data.is_some() {
        match config::get_main_edit_mode(&app.app_config) {
            config::ConfigEditMode::Shared => {
                if index >= 1 && index < app.app_config.script_definitions.len() {
                    app.app_config.script_definitions.swap(index, index - 1);
                    app.edit_data.is_dirty = true;
                    new_position = index - 1;
                }
            }
            config::ConfigEditMode::Local => {
                if let Some(local_config_body) = &mut app.app_config.local_config_body {
                    if index >= 1 && index < local_config_body.script_definitions.len() {
                        local_config_body.script_definitions.swap(index, index - 1);
                        config::update_shared_config_script_positions_from_local_config(
                            &mut app.app_config,
                        );
                        app.edit_data.is_dirty = true;
                        new_position = index - 1;
                    }
                }
            }
        }
    }

    if let Some((idx, _)) = get_only_selected_script(&app.window_state.selected_scripts) {
        if idx == index && index > 0 {
            select_edited_script(
                app,
                ConfigScriptId {
                    idx: index - 1,
                    edit_mode: config::get_main_edit_mode(&app.app_config),
                },
            );
        }
    }

    update_config_cache(app);

    new_position
}

pub fn move_config_script_down(app: &mut MainWindow, index: usize) -> usize {
    let mut new_position = index;
    if app.edit_data.window_edit_data.is_some() {
        match config::get_main_edit_mode(&app.app_config) {
            config::ConfigEditMode::Shared => {
                if index + 1 < app.app_config.script_definitions.len() {
                    app.app_config.script_definitions.swap(index, index + 1);
                    app.edit_data.is_dirty = true;
                }
                new_position = index + 1;
            }
            config::ConfigEditMode::Local => {
                if let Some(local_config_body) = &mut app.app_config.local_config_body {
                    if index + 1 < local_config_body.script_definitions.len() {
                        local_config_body.script_definitions.swap(index, index + 1);
                        config::update_shared_config_script_positions_from_local_config(
                            &mut app.app_config,
                        );
                        app.edit_data.is_dirty = true;
                        new_position = index + 1;
                    }
                }
            }
        }
    }

    if let Some((idx, _)) = get_only_selected_script(&app.window_state.selected_scripts) {
        if idx == index && index + 1 < app.displayed_configs_list_cache.len() {
            select_edited_script(
                app,
                ConfigScriptId {
                    idx: index + 1,
                    edit_mode: config::get_main_edit_mode(&app.app_config),
                },
            );
        }
    }

    update_config_cache(app);

    new_position
}

pub fn move_vec_element_to_index<T>(vec: &mut Vec<T>, index: usize, new_index: usize) {
    if index >= vec.len() || new_index > vec.len() {
        return;
    }

    if index < new_index {
        vec[index..new_index].rotate_left(1);
    } else {
        vec[new_index..=index].rotate_right(1);
    }
}

pub fn move_config_script_to_index(app: &mut MainWindow, index: usize, new_index: usize) {
    if index == new_index {
        return;
    }

    if app.edit_data.window_edit_data.is_some() {
        match config::get_main_edit_mode(&app.app_config) {
            config::ConfigEditMode::Shared => {
                move_vec_element_to_index(&mut app.app_config.script_definitions, index, new_index);
                app.edit_data.is_dirty = true;
            }
            config::ConfigEditMode::Local => {
                if let Some(local_config_body) = &mut app.app_config.local_config_body {
                    move_vec_element_to_index(
                        &mut local_config_body.script_definitions,
                        index,
                        new_index,
                    );
                    config::update_shared_config_script_positions_from_local_config(
                        &mut app.app_config,
                    );
                    app.edit_data.is_dirty = true;
                }
            }
        }
    }

    update_config_cache(app);
}

pub fn apply_config_script_edit(
    app: &mut MainWindow,
    config_script_id: ConfigScriptId,
    edit_fn: impl FnOnce(&mut config::OriginalScriptDefinition),
) {
    match config_script_id.edit_mode {
        config::ConfigEditMode::Local => {
            if let Some(config) = &mut app.app_config.local_config_body {
                match &mut config.script_definitions.get_mut(config_script_id.idx) {
                    Some(config::ScriptDefinition::Original(script)) => {
                        edit_fn(script);
                        app.edit_data.is_dirty = true;
                        update_config_cache(app);
                    }
                    _ => {}
                }
            }
        }
        config::ConfigEditMode::Shared => match &mut app
            .app_config
            .script_definitions
            .get_mut(config_script_id.idx)
        {
            Some(config::ScriptDefinition::Original(script)) => {
                edit_fn(script);
                app.edit_data.is_dirty = true;
                update_config_cache(app);
            }
            _ => {}
        },
    }
}

pub fn apply_execution_script_edit(
    app: &mut MainWindow,
    script_idx: usize,
    edit_fn: impl FnOnce(&mut config::OriginalScriptDefinition),
) {
    match &mut app
        .execution_manager
        .get_edited_scripts_mut()
        .get_mut(script_idx)
    {
        Some(script) => {
            edit_fn(&mut script.original);
        }
        _ => {}
    }
}

pub fn clear_edited_scripts(app: &mut MainWindow) {
    app.execution_manager.clear_edited_scripts();
    clear_script_selection(&mut app.window_state.selected_scripts);
    // we could be dragging a script from the list
    cancel_all_drag_and_drop_operations(app);
}

pub fn clear_execution_scripts(app: &mut MainWindow) {
    // use the same script that we hinted visually
    let execution_id = app
        .visual_caches
        .button_key_caches
        .last_cleanable_execution_id
        .and_then(|execution_id| {
            app.execution_manager
                .get_started_executions()
                .get(execution_id)
                .filter(|execution| {
                    execution.has_finished_execution()
                        && !execution.is_waiting_execution_to_finish()
                })
                .map(|_| execution_id)
        });

    let Some(execution_id) = execution_id else {
        return;
    };

    remove_execution(app, execution_id);
    clear_script_selection(&mut app.window_state.selected_scripts);
}

pub fn enter_window_edit_mode(app: &mut MainWindow) {
    if app.app_config.is_read_only {
        return;
    }

    app.edit_data.window_edit_data = Some(WindowEditData::from_config(&app.app_config, None));
    app.edit_data.script_filter = String::new();
    clear_script_selection(&mut app.window_state.selected_scripts);
    update_config_cache(app);
    app.visual_caches.is_custom_title_editing = false;
    update_drag_and_drop_area_bounds(app);
    cancel_all_drag_and_drop_operations(app);
}

pub fn exit_window_edit_mode(app: &mut MainWindow) {
    app.edit_data.window_edit_data = None;
    clear_script_selection(&mut app.window_state.selected_scripts);
    apply_theme(app);
    keybind_editing::update_keybind_visual_caches(app, config::get_main_edit_mode(&app.app_config));
    update_config_cache(app);
    update_git_branch_visibility(app);
    update_drag_and_drop_area_bounds(app);
    cancel_all_drag_and_drop_operations(app);
}

pub fn apply_theme_color_from_string(
    app: &mut MainWindow,
    edit_mode: config::ConfigEditMode,
    color: String,
    set_theme_fn: impl FnOnce(&mut config::CustomTheme, [f32; 3]),
    set_text_fn: impl FnOnce(&mut WindowEditData, String) -> String,
) {
    if let Some(edit_data) = &mut app.edit_data.window_edit_data {
        let color_string = set_text_fn(edit_data, color);
        if let Some(custom_theme) =
            &mut config::get_rewritable_config_mut(&mut app.app_config, edit_mode).custom_theme
        {
            if let Some(new_color) = color_utils::hex_to_rgb(&color_string) {
                set_theme_fn(custom_theme, new_color);
                apply_theme(app);
                app.edit_data.is_dirty = true;
            }
        }
    }
}

pub fn apply_theme(app: &mut MainWindow) {
    app.theme = get_theme(&app.app_config);
    app.visual_caches.custom_theme =
        config::get_rewritable_config(&app.app_config, config::get_main_edit_mode(&app.app_config))
            .custom_theme
            .clone();
    update_theme_icons(app);
}

pub fn focus_filter(app: &mut MainWindow) -> Task<WindowMessage> {
    if app.panes.maximized().is_none() {
        if let Some(focus) = app.window_state.pane_focus {
            if &app.panes.panes[&focus].variant != &PaneVariant::ScriptList {
                app.window_state.pane_focus = Some(app.pane_by_pane_type[&PaneVariant::ScriptList]);
            }
        } else {
            app.window_state.pane_focus = Some(app.pane_by_pane_type[&PaneVariant::ScriptList]);
        }
    }
    Task::batch([
        text_input::focus(FILTER_INPUT_ID.clone()),
        text_input::select_all(FILTER_INPUT_ID.clone()),
    ])
}

pub fn should_autoclean_on_success(
    app: &mut MainWindow,
    execution_id: parallel_execution_manager::ExecutionId,
) -> bool {
    if let Some(execution) = app
        .execution_manager
        .get_started_executions()
        .get(execution_id)
    {
        if !execution.has_finished_execution() || execution.has_failed_scripts() {
            return false;
        }

        let execution = app
            .execution_manager
            .get_started_executions()
            .get(execution_id)
            .unwrap();
        return execution
            .get_scheduled_scripts_cache()
            .iter()
            .all(|record| record.script.original.autoclean_on_success);
    }

    false
}

pub(crate) fn get_script_list_script_offset(index: usize) -> f32 {
    index as f32 * ONE_SCRIPT_LIST_ELEMENT_HEIGHT
}

pub(crate) fn for_each_drag_area(app: &mut MainWindow, mut f: impl FnMut(&mut DragAndDropList)) {
    if !app.edit_data.window_edit_data.is_some() {
        f(&mut app.window_state.drag_and_drop_lists.script_list);
    } else {
        f(&mut app.window_state.drag_and_drop_lists.edit_script_list);
    }

    f(&mut app.window_state.drag_and_drop_lists.execution_edit_list);
}

pub(crate) fn for_each_drop_area(app: &mut MainWindow, mut f: impl FnMut(&mut DropArea)) {
    f(&mut app.window_state.drop_areas.execution_pane);
    for drop_area in &mut app.window_state.drop_areas.running_executions {
        f(drop_area);
    }
}

pub(crate) fn cancel_all_drag_and_drop_operations(app: &mut MainWindow) {
    for_each_drag_area(app, |area| {
        area.cancel_operations();
    });

    for_each_drop_area(app, |area| {
        area.cancel_operations();
    });

    app.window_state.dragged_script = None;
}

pub(crate) fn update_drag_and_drop_area_bounds(app: &mut MainWindow) {
    let regions = app
        .panes
        .layout()
        .pane_regions(PANE_SPACING, app.window_state.full_window_size);

    let script_list_pane = app.pane_by_pane_type[&PaneVariant::ScriptList];

    if let Some(script_list_pane_region) = regions.get(&script_list_pane) {
        if app.edit_data.window_edit_data.is_none() {
            let header_height = get_script_list_content_offset_y(&app);
            let mut content_region = script_list_pane_region.clone();
            content_region.y += header_height;
            content_region.height -= header_height;
            content_region.width -= SCROLL_BAR_WIDTH;
            app.window_state
                .drag_and_drop_lists
                .script_list
                .set_bounds(content_region);
        } else {
            let header_height = get_edited_script_list_content_offset_y(&app);
            let mut content_region = script_list_pane_region.clone();
            content_region.y += header_height;
            content_region.height -= header_height;
            content_region.width -= SCROLL_BAR_WIDTH;
            app.window_state
                .drag_and_drop_lists
                .edit_script_list
                .set_bounds(content_region);
        }
    }

    update_execution_list_drag_and_drop_list_bounds(app);

    update_execution_list_drop_area_bounds(app);
}

fn update_execution_list_drag_and_drop_list_bounds(app: &mut MainWindow) {
    let regions = app
        .panes
        .layout()
        .pane_regions(PANE_SPACING, app.window_state.full_window_size);
    let execution_list_pane = app.pane_by_pane_type[&PaneVariant::ExecutionList];
    if let Some(execution_list_pane_region) = regions.get(&execution_list_pane) {
        {
            let content_offset_y = PANE_HEADER_HEIGHT
                + get_execution_list_title_size_y(&app)
                + get_started_execution_list_size_y(app);
            let mut content_region = execution_list_pane_region.clone();
            content_region.y += content_offset_y;
            content_region.height -= content_offset_y;
            content_region.width -= SCROLL_BAR_WIDTH;
            app.window_state
                .drag_and_drop_lists
                .execution_edit_list
                .set_bounds(content_region);
        }

        app.window_state
            .drop_areas
            .execution_pane
            .set_bounds(execution_list_pane_region.clone());
    }
}

fn update_execution_list_drop_area_bounds(app: &mut MainWindow) {
    if app.execution_manager.get_started_executions().len()
        != app.window_state.drop_areas.running_executions.len()
    {
        eprintln!(
            "The number of running executions got desynchronized with the number of drop areas {}, {}",
            app.execution_manager.get_started_executions().len(),
            app.window_state.drop_areas.running_executions.len()
        );
        return;
    }

    let regions = app
        .panes
        .layout()
        .pane_regions(PANE_SPACING, app.window_state.full_window_size);

    let execution_list_pane = app.pane_by_pane_type[&PaneVariant::ExecutionList];
    let Some(script_list_pane_region) = regions.get(&execution_list_pane) else {
        return;
    };

    let executions_count = app.execution_manager.get_started_executions().len();

    let should_show_execution_names = executions_count > 1;

    let mut accumulated_height = PANE_HEADER_HEIGHT + get_execution_list_title_size_y(app);

    for (idx, execution) in app
        .execution_manager
        .get_started_executions()
        .values()
        .enumerate()
    {
        let drop_area = &mut app.window_state.drop_areas.running_executions[idx];
        let mut content_region = script_list_pane_region.clone();
        content_region.y += accumulated_height;
        content_region.height = ONE_EXECUTION_LIST_ELEMENT_HEIGHT
            * execution.get_scheduled_scripts_cache().len() as f32
            + if should_show_execution_names {
                ONE_EXECUTION_NAME_HEIGHT
            } else {
                0.0
            }
            + EXECUTION_EDIT_BUTTONS_HEIGHT;
        content_region.width -= SCROLL_BAR_WIDTH;
        accumulated_height += content_region.height;
        drop_area.set_bounds(content_region);
    }
}

pub(crate) fn update_edited_execution_list_script_number(app: &mut MainWindow) {
    app.window_state
        .drag_and_drop_lists
        .execution_edit_list
        .change_number_of_elements(app.execution_manager.get_edited_scripts().len());

    update_execution_list_drag_and_drop_list_bounds(app);
}

pub(crate) fn get_current_script_list_drag_and_drop(app: &mut MainWindow) -> &mut DragAndDropList {
    if app.edit_data.window_edit_data.is_some() {
        &mut app.window_state.drag_and_drop_lists.edit_script_list
    } else {
        &mut app.window_state.drag_and_drop_lists.script_list
    }
}

fn get_pretty_name(name: String) -> String {
    let mut prettified_name = String::with_capacity(name.len());
    let mut was_separator = true;
    let mut was_upper_case = false;
    for char in name.chars() {
        if char == '.' {
            break;
        }
        if char == '_' || char == ' ' || char == '-' {
            prettified_name.push(' ');
            was_upper_case = false;
            was_separator = true;
        } else if char.is_uppercase() {
            if was_separator {
                prettified_name.push(char);
            } else if was_upper_case {
                prettified_name.extend(char.to_lowercase());
            } else {
                prettified_name.push(' ');
                prettified_name.push(char);
            }
            was_upper_case = true;
            was_separator = false;
        } else {
            if was_separator {
                prettified_name.extend(char.to_uppercase())
            } else if was_upper_case {
                prettified_name.push(char);
            } else {
                prettified_name.push(char);
            }
            was_upper_case = false;
            was_separator = false;
        }
    }

    prettified_name
}

pub(crate) fn get_script_list_content_offset_y(app: &MainWindow) -> f32 {
    PANE_HEADER_HEIGHT
        + if app.visual_caches.enable_script_filtering {
            SCRIPT_FILTER_HEIGHT
        } else {
            0.0
        }
        + if app.edit_data.is_dirty {
            DIRTY_CONFIG_BUTTONS_HEIGHT
        } else {
            0.0
        }
}

pub(crate) fn get_edited_script_list_content_offset_y(_app: &MainWindow) -> f32 {
    PANE_HEADER_HEIGHT + CONFIG_EDIT_HEADER_HEIGHT
}

pub(crate) fn get_execution_list_title_size_y(app: &MainWindow) -> f32 {
    let mut title_lines = if app.visual_caches.is_custom_title_editing {
        // for now the edit field is only one line high
        1
    } else if let Some(custom_title) = app.app_config.custom_title.as_ref() {
        custom_title.lines().count() as u32
    } else {
        0
    };

    let config = get_current_rewritable_config(&app.app_config);

    // if title editing enabled, we can't have less than 1 line
    if title_lines == 0 && config.enable_title_editing {
        title_lines = 1;
    }

    if app.visual_caches.git_branch_requester.is_some() {
        title_lines += 1;
    }

    if config.show_working_directory {
        title_lines += 1;
    }

    FIRST_EXECUTION_ELEMENT_OFFSET_Y + title_lines as f32 * ONE_TITLE_LINE_HEIGHT
}

pub(crate) fn get_started_execution_list_size_y(app: &MainWindow) -> f32 {
    let executions_count = app.execution_manager.get_started_executions().len() as u32;
    let should_show_execution_names = executions_count > 1;

    let scheduled_elements_count = app
        .execution_manager
        .get_started_executions()
        .values()
        .fold(0, |acc, x| {
            acc + x.get_scheduled_scripts_cache().len() as u32
        });

    scheduled_elements_count as f32 * ONE_EXECUTION_LIST_ELEMENT_HEIGHT
        + EXECUTION_EDIT_BUTTONS_HEIGHT * executions_count as f32
        + if should_show_execution_names {
            ONE_EXECUTION_NAME_HEIGHT * executions_count as f32
        } else {
            0.0
        }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FileAssociation;

    const APP_CONFIG_WITH_DIFFERENT_SCRIPTS: fn() -> (config::AppConfig, Vec<config::Guid>) =
        || {
            let test_script_guid_1 = config::Guid::new();
            let test_script_guid_2 = config::Guid::new();
            let test_script_guid_3 = config::Guid::new();
            let test_script_guid_4 = config::Guid::new();

            (
                config::AppConfig {
                    version: "1.0.0".to_string(),
                    rewritable: config::RewritableConfig {
                        window_status_reactions: true,
                        keep_window_size: false,
                        enable_script_filtering: true,
                        show_working_directory: true,
                        enable_title_editing: true,
                        config_version_update_behavior: config::ConfigUpdateBehavior::OnStartup,
                        custom_theme: None,
                        app_actions_keybinds: Vec::new(),
                        script_keybinds: Vec::new(),
                        show_current_git_branch: false,
                        quick_launch_scripts: Vec::new(),
                        file_associations: vec![FileAssociation {
                            extension: "py".to_string(),
                            executor: vec!["python".to_string()],
                            executor_arguments: vec![],
                        }],
                    },
                    script_definitions: vec![
                        config::ScriptDefinition::Original(config::OriginalScriptDefinition {
                            uid: test_script_guid_1.clone(),
                            name: "Shared script 1".to_string(),
                            icon: config::PathConfig::default(),
                            command: config::PathConfig::default(),
                            working_directory: config::PathConfig::default(),
                            arguments_line: "".to_string(),
                            argument_placeholders: Vec::new(),
                            autorerun_count: 0,
                            reaction_to_previous_failures:
                                config::ReactionToPreviousFailures::SkipOnFailure,
                            arguments_requirement: config::ArgumentRequirement::Optional,
                            arguments_hint: String::new(),
                            custom_executor: None,
                            executor_arguments: Vec::new(),
                            is_hidden: false,
                            autoclean_on_success: false,
                            ignore_output: false,
                        }),
                        config::ScriptDefinition::Original(config::OriginalScriptDefinition {
                            uid: test_script_guid_2.clone(),
                            name: "Original script 2".to_string(),
                            icon: config::PathConfig::default(),
                            command: config::PathConfig::default(),
                            working_directory: config::PathConfig::default(),
                            arguments_line: "".to_string(),
                            argument_placeholders: Vec::new(),
                            autorerun_count: 0,
                            reaction_to_previous_failures:
                                config::ReactionToPreviousFailures::SkipOnFailure,
                            arguments_requirement: config::ArgumentRequirement::Optional,
                            arguments_hint: String::new(),
                            custom_executor: None,
                            executor_arguments: Vec::new(),
                            is_hidden: false,
                            autoclean_on_success: false,
                            ignore_output: false,
                        }),
                        config::ScriptDefinition::Preset(config::ScriptPreset {
                            uid: test_script_guid_3.clone(),
                            name: "Original preset".to_string(),
                            icon: config::PathConfig::default(),
                            items: vec![
                                config::PresetItem {
                                    uid: test_script_guid_1.clone(),
                                    name: None,
                                    arguments_line: None,
                                    executor_arguments: None,
                                    overridden_placeholder_values: std::collections::HashMap::new(),
                                    autorerun_count: None,
                                    reaction_to_previous_failures: None,
                                    autoclean_on_success: None,
                                },
                                config::PresetItem {
                                    uid: test_script_guid_2.clone(),
                                    name: None,
                                    arguments_line: None,
                                    executor_arguments: None,
                                    overridden_placeholder_values: std::collections::HashMap::new(),
                                    autorerun_count: None,
                                    reaction_to_previous_failures: None,
                                    autoclean_on_success: None,
                                },
                            ],
                        }),
                    ],
                    is_read_only: false,
                    paths: config::PathCaches {
                        logs_path: PathBuf::new(),
                        work_path: PathBuf::new(),
                        exe_folder_path: PathBuf::new(),
                        config_path: PathBuf::new(),
                    },
                    env_vars: Vec::new(),
                    custom_title: None,
                    config_read_error: None,
                    local_config_path: config::PathConfig::default(),
                    arguments_read_error: None,
                    local_config_body: Some(Box::new(config::LocalConfig {
                        version: "1.0.0".to_string(),
                        rewritable: config::RewritableConfig {
                            window_status_reactions: false,
                            keep_window_size: false,
                            enable_script_filtering: false,
                            show_working_directory: false,
                            enable_title_editing: false,
                            config_version_update_behavior: config::ConfigUpdateBehavior::OnStartup,
                            custom_theme: None,
                            app_actions_keybinds: vec![],
                            script_keybinds: vec![],
                            show_current_git_branch: false,
                            quick_launch_scripts: vec![],
                            file_associations: vec![FileAssociation {
                                extension: "py".to_string(),
                                executor: vec!["python".to_string()],
                                executor_arguments: vec![],
                            }],
                        },
                        script_definitions: vec![
                            config::ScriptDefinition::ReferenceToShared(
                                config::ReferenceToSharedScript {
                                    uid: test_script_guid_1.clone(),
                                    is_hidden: false,
                                },
                            ),
                            config::ScriptDefinition::ReferenceToShared(
                                config::ReferenceToSharedScript {
                                    uid: test_script_guid_2.clone(),
                                    is_hidden: false,
                                },
                            ),
                            config::ScriptDefinition::ReferenceToShared(
                                config::ReferenceToSharedScript {
                                    uid: test_script_guid_3.clone(),
                                    is_hidden: true,
                                },
                            ),
                            config::ScriptDefinition::Original(config::OriginalScriptDefinition {
                                uid: test_script_guid_4.clone(),
                                name: "Original script".to_string(),
                                icon: config::PathConfig::default(),
                                command: config::PathConfig::default(),
                                working_directory: config::PathConfig::default(),
                                arguments_line: "".to_string(),
                                argument_placeholders: Vec::new(),
                                autorerun_count: 0,
                                reaction_to_previous_failures:
                                    config::ReactionToPreviousFailures::SkipOnFailure,
                                arguments_requirement: config::ArgumentRequirement::Optional,
                                arguments_hint: "\"arg1\" \"arg2\"".to_string(),
                                custom_executor: None,
                                executor_arguments: Vec::new(),
                                is_hidden: false,
                                autoclean_on_success: false,
                                ignore_output: false,
                            }),
                        ],
                    })),
                },
                vec![
                    test_script_guid_1,
                    test_script_guid_2,
                    test_script_guid_3,
                    test_script_guid_4,
                ],
            )
        };

    #[test]
    fn test_given_app_config_with_different_scripts_when_check_for_is_local_then_returns_true_only_for_local_configs(
    ) {
        let (app_config, _) = APP_CONFIG_WITH_DIFFERENT_SCRIPTS();

        assert_eq!(is_local_config_script(0, &app_config), false);
        assert_eq!(is_local_config_script(1, &app_config), false);
        assert_eq!(is_local_config_script(2, &app_config), false);
        assert_eq!(is_local_config_script(3, &app_config), true);
        // non-existing script
        assert_eq!(is_local_config_script(4, &app_config), false);
    }

    #[test]
    fn test_given_script_id_when_get_resulting_scripts_from_guid_then_returns_correct_definition() {
        let (mut app_config, all_guids) = APP_CONFIG_WITH_DIFFERENT_SCRIPTS();

        assert_eq!(
            get_resulting_scripts_from_guid(&mut app_config, all_guids[0].clone()).len(),
            1
        );
        assert_eq!(
            get_resulting_scripts_from_guid(&mut app_config, all_guids[1].clone()).len(),
            1
        );
        assert_eq!(
            get_resulting_scripts_from_guid(&mut app_config, all_guids[2].clone()).len(),
            2
        );
        assert_eq!(
            get_resulting_scripts_from_guid(&mut app_config, all_guids[3].clone()).len(),
            1
        );
    }

    #[test]
    fn get_pretty_name_test() {
        assert_eq!(get_pretty_name("test.sh".to_string()), "Test".to_string());
        assert_eq!(
            get_pretty_name("test_script.sh".to_string()),
            "Test Script".to_string()
        );
        assert_eq!(
            get_pretty_name("TestScript.sh".to_string()),
            "Test Script".to_string()
        );
        assert_eq!(
            get_pretty_name("Test-Script".to_string()),
            "Test Script".to_string()
        );
        assert_eq!(
            get_pretty_name("TEST_SCRIPT.txt.sh".to_string()),
            "Test Script".to_string()
        );
    }
}
