// Copyright (C) Pavel Grebnev 2023-2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use iced::advanced::image::Handle;
use iced::widget::{pane_grid, text_input};
use iced::window::resize;
use iced::{keyboard, window, Command, Size, Theme};

use crate::color_utils;
use crate::config;
use crate::execution_lists;
use crate::git_support;
use crate::keybind_editing;
use crate::main_window::*;
use crate::style;

const ONE_EXECUTION_LIST_ELEMENT_HEIGHT: f32 = 30.0;
const ONE_TITLE_LINE_HEIGHT: f32 = 20.0;
const ONE_EXECUTION_NAME_HEIGHT: f32 = 32.0;
const EMPTY_EXECUTION_LIST_HEIGHT: f32 = 100.0;
const EDIT_BUTTONS_HEIGHT: f32 = 50.0;

pub fn is_local_edited_script(
    script_idx: usize,
    app_config: &config::AppConfig,
    window_edit_data: &Option<WindowEditData>,
) -> bool {
    if let Some(window_edit_data) = &window_edit_data {
        if window_edit_data.edit_type == ConfigEditType::Local {
            if let Some(scripts) = &app_config.local_config_body {
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
    false
}

pub fn is_script_missing_arguments(script: &config::ScriptDefinition) -> bool {
    match script {
        config::ScriptDefinition::Original(script) => is_original_script_missing_arguments(script),
        _ => false,
    }
}

pub fn is_original_script_missing_arguments(script: &config::OriginalScriptDefinition) -> bool {
    script.requires_arguments && script.arguments.is_empty()
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

pub fn get_theme(config: &config::AppConfig, window_edit_data: &Option<WindowEditData>) -> Theme {
    if let Some(theme) = get_rewritable_config_opt(&config, window_edit_data)
        .custom_theme
        .clone()
    {
        style::get_custom_theme(theme)
    } else {
        Theme::default()
    }
}

pub fn get_rewritable_config<'a>(
    config: &'a config::AppConfig,
    edit_type: &ConfigEditType,
) -> &'a config::RewritableConfig {
    match edit_type {
        ConfigEditType::Shared => &config.rewritable,
        ConfigEditType::Local => {
            if let Some(local_config) = &config.local_config_body {
                &local_config.rewritable
            } else {
                &config.rewritable
            }
        }
    }
}

pub fn get_rewritable_config_opt<'a>(
    config: &'a config::AppConfig,
    edit_data: &Option<WindowEditData>,
) -> &'a config::RewritableConfig {
    match &edit_data {
        Some(edit_data) => get_rewritable_config(config, &edit_data.edit_type),
        None => {
            if let Some(local_config) = &config.local_config_body {
                &local_config.rewritable
            } else {
                &config.rewritable
            }
        }
    }
}

pub fn get_rewritable_config_mut<'a>(
    config: &'a mut config::AppConfig,
    window_edit: &Option<WindowEditData>,
) -> &'a mut config::RewritableConfig {
    match window_edit {
        Some(window_edit) => get_rewritable_config_mut_non_opt(config, window_edit),
        None => &mut config.rewritable,
    }
}

fn get_rewritable_config_mut_non_opt<'a>(
    config: &'a mut config::AppConfig,
    window_edit: &WindowEditData,
) -> &'a mut config::RewritableConfig {
    match window_edit.edit_type {
        ConfigEditType::Shared => &mut config.rewritable,
        ConfigEditType::Local => {
            if let Some(local_config) = &mut config.local_config_body {
                &mut local_config.rewritable
            } else {
                &mut config.rewritable
            }
        }
    }
}

fn get_script_definition_list_opt<'a>(
    config: &'a config::AppConfig,
    window_edit_data: &Option<WindowEditData>,
) -> &'a Vec<config::ScriptDefinition> {
    match window_edit_data {
        Some(window_edit_data) => get_script_definition_list(config, &window_edit_data.edit_type),
        None => {
            if let Some(local_config) = &config.local_config_body {
                &local_config.script_definitions
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
        ConfigEditType::Shared => &config.script_definitions,
        ConfigEditType::Local => {
            if let Some(local_config) = &config.local_config_body {
                &local_config.script_definitions
            } else {
                &config.script_definitions
            }
        }
    }
}

pub fn get_script_definition<'a>(
    app_config: &'a config::AppConfig,
    edit_data: &EditData,
    script_idx: usize,
) -> &'a config::ScriptDefinition {
    let is_looking_at_local_config = if let Some(window_edit_data) = &edit_data.window_edit_data {
        window_edit_data.edit_type == ConfigEditType::Local
    } else {
        app_config.local_config_body.is_some()
    };

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

fn get_script_definition_mut<'a>(
    app_config: &'a mut config::AppConfig,
    edit_data: &EditData,
    script_idx: usize,
) -> &'a mut config::ScriptDefinition {
    let is_looking_at_local_config = if let Some(window_edit_data) = &edit_data.window_edit_data {
        window_edit_data.edit_type == ConfigEditType::Local
    } else {
        app_config.local_config_body.is_some()
    };

    if is_looking_at_local_config {
        &mut app_config
            .local_config_body
            .as_mut()
            .unwrap()
            .script_definitions[script_idx]
    } else {
        &mut app_config.script_definitions[script_idx]
    }
}

pub fn get_resulting_scripts_from_guid(
    app: &mut MainWindow,
    script_uid: config::Guid,
) -> Vec<config::ScriptDefinition> {
    let original_script =
        config::get_original_script_definition_by_uid(&app.app_config, script_uid);

    let original_script = if let Some(original_script) = original_script {
        original_script
    } else {
        return Vec::new();
    };

    match original_script {
        config::ScriptDefinition::ReferenceToShared(_) => Vec::new(),
        config::ScriptDefinition::Original(_) => {
            vec![original_script]
        }
        config::ScriptDefinition::Preset(preset) => {
            let resulting_scripts = preset
                .items
                .iter()
                .map(|preset_item| {
                    (
                        config::get_original_script_definition_by_uid(
                            &app.app_config,
                            preset_item.uid.clone(),
                        ),
                        preset_item,
                    )
                })
                .filter(|(optional_definition, _preset_item)| optional_definition.is_some())
                .map(|(optional_definition, preset_item)| {
                    let mut new_script = optional_definition.unwrap();

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

                    new_script
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

pub fn get_editing_preset<'a>(
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
    None
}

pub fn find_best_shared_script_insert_position(
    source_script_definitions: &Vec<config::ScriptDefinition>,
    target_script_definitions: &Vec<config::ScriptDefinition>,
    script_id: &EditScriptId,
) -> usize {
    let script_idx = script_id.idx;

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
            .cursor_script
            .as_ref()
            .map(|s| &s.script_type);

        let have_scripts_in_execution = !app.execution_data.get_edited_scripts().is_empty();
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
        config::AppAction::RemoveCursorScript => WindowMessage::RemoveCursorScript,
    }
}

pub fn get_run_script_window_message_from_guid(
    app_config: &config::AppConfig,
    script_uid: &config::Guid,
) -> Option<WindowMessage> {
    let original_script =
        config::get_original_script_definition_by_uid(app_config, script_uid.clone());
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
    let executions_number = app.execution_data.get_started_executions().size();
    if executions_number == 1 {
        let execution_id = app
            .execution_data
            .get_started_executions()
            .values()
            .next()
            .unwrap();
        add_edited_scripts_to_started_execution(app, execution_id.get_id());
    } else if executions_number == 0 {
        // if there are no executions, then we can start a new one
        start_new_execution_from_edited_scripts(app);
    }
}

pub fn try_add_script_to_execution_or_start_new(app: &mut MainWindow, script_uid: config::Guid) {
    // we can accept this hotkey only if we definitely know what execution we
    // supposed to add it to
    let executions_number = app.execution_data.get_started_executions().size();
    let scripts_to_add = get_resulting_scripts_from_guid(app, script_uid);

    if executions_number == 1 {
        let execution_id = app
            .execution_data
            .get_started_executions()
            .values()
            .next()
            .unwrap()
            .get_id();

        app.execution_data.add_script_to_running_execution(
            &app.app_config,
            execution_id,
            scripts_to_add,
        );
    } else if executions_number == 0 {
        // if there are no executions, then we can start a new one
        start_new_execution_from_provided_scripts(app, scripts_to_add);
    }
}

pub fn update_config_cache(app: &mut MainWindow) {
    let is_looking_at_local_config = if let Some(window_edit_data) = &app.edit_data.window_edit_data
    {
        window_edit_data.edit_type == ConfigEditType::Local
    } else {
        app.app_config.local_config_body.is_some()
    };

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
                            if is_full_list || !is_script_hidden {
                                result_list.push(ScriptListCacheRecord {
                                    name,
                                    full_icon_path: config::get_full_optional_path(paths, &icon),
                                    is_hidden: is_script_hidden,
                                    original_script_uid: reference.uid.clone(),
                                });
                            }
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
                    if is_full_list || !is_script_hidden {
                        result_list.push(ScriptListCacheRecord {
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
                        result_list.push(ScriptListCacheRecord {
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
        let script_definitions = &app.app_config.script_definitions;

        result_list.clear();
        for script_definition in script_definitions {
            match script_definition {
                config::ScriptDefinition::ReferenceToShared(_) => {}
                config::ScriptDefinition::Original(script) => {
                    let is_script_hidden = is_script_filtered_out(&script.name) || script.is_hidden;
                    if is_full_list || !is_script_hidden {
                        result_list.push(ScriptListCacheRecord {
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
                        result_list.push(ScriptListCacheRecord {
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

    app.visual_caches.quick_launch_buttons.clear();
    let rewritable_config =
        get_rewritable_config_opt(&app.app_config, &app.edit_data.window_edit_data);
    for script_uid in &rewritable_config.quick_launch_scripts {
        let original_script =
            config::get_original_script_definition_by_uid(&app.app_config, script_uid.clone());
        let Some(script) = original_script else {
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
}

pub fn update_button_key_hint_caches(app: &mut MainWindow) {
    let mut last_stoppable_execution_id = None;
    let mut last_cleanable_execution_id = None;

    for execution in app.execution_data.get_started_executions().values().rev() {
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
    if config::get_current_rewritable_config(&app.app_config).show_current_git_branch {
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

pub fn on_execution_removed(app: &mut MainWindow, execution_id: execution_lists::ExecutionId) {
    // switch current log tab if the removed execution was selected
    if let Some(selected_execution) = app.visual_caches.selected_execution_log {
        if selected_execution == execution_id {
            // this is not actually needed since a wrong index will also not show anything
            // but just for the sake of debugging, let's clean it
            app.visual_caches.selected_execution_log = None;

            let last_execution = app.execution_data.get_started_executions().values().last();
            if let Some(first_execution) = last_execution {
                app.visual_caches.selected_execution_log = Some(first_execution.get_id());
            }
        }
    }

    // reset executions count if we removed last execution
    if app.execution_data.get_started_executions().is_empty() {
        app.visual_caches.last_execution_id = 0;
    }

    update_button_key_hint_caches(app);
}

pub fn switch_to_editing_shared_config(app: &mut MainWindow) {
    clean_script_selection(&mut app.window_state.cursor_script);
    switch_config_edit_mode(app, ConfigEditType::Shared);
    apply_theme(app);
    update_config_cache(app);
}

pub fn maximize_pane(
    app: &mut MainWindow,
    pane: pane_grid::Pane,
    window_size: Size,
) -> Command<WindowMessage> {
    if app.window_state.pane_focus != Some(pane) {
        clean_script_selection(&mut app.window_state.cursor_script);
    }
    app.window_state.pane_focus = Some(pane);
    app.panes.maximize(pane);
    app.window_state.has_maximized_pane = true;
    if !config::get_current_rewritable_config(&app.app_config).keep_window_size {
        app.window_state.full_window_size = window_size.clone();
        let regions = app
            .panes
            .layout()
            .pane_regions(1.0, Size::new(window_size.width, window_size.height));
        let size = regions.get(&pane);
        let Some(size) = size else {
            return Command::none();
        };

        let executions_count = app.execution_data.get_started_executions().size() as u32;
        let should_show_execution_names = executions_count > 1;

        let scheduled_elements_count = app
            .execution_data
            .get_started_executions()
            .values()
            .fold(0, |acc, x| {
                acc + x.get_scheduled_scripts_cache().len() as u32
            });
        let edited_elements_count = app.execution_data.get_edited_scripts().len() as u32;
        let mut title_lines = if app.visual_caches.is_custom_title_editing {
            // for now the edit field is only one line high
            1
        } else if let Some(custom_title) = app.app_config.custom_title.as_ref() {
            custom_title.lines().count() as u32
        } else {
            0
        };

        // if title editing enabled, we can't be less than 1 line
        if title_lines == 0
            && config::get_current_rewritable_config(&app.app_config).enable_title_editing
        {
            title_lines = 1;
        }

        if app.visual_caches.git_branch_requester.is_some() {
            title_lines += 1;
        }

        return resize(
            window::Id::MAIN,
            Size {
                width: size.width,
                height: f32::min(
                    size.height,
                    EMPTY_EXECUTION_LIST_HEIGHT
                        + title_lines as f32 * ONE_TITLE_LINE_HEIGHT
                        + if should_show_execution_names {
                            ONE_EXECUTION_NAME_HEIGHT * executions_count as f32
                        } else {
                            0.0
                        }
                        + scheduled_elements_count as f32 * ONE_EXECUTION_LIST_ELEMENT_HEIGHT
                        + EDIT_BUTTONS_HEIGHT * executions_count as f32
                        + edited_elements_count as f32 * ONE_EXECUTION_LIST_ELEMENT_HEIGHT
                        + if edited_elements_count > 0 {
                            EDIT_BUTTONS_HEIGHT
                        } else {
                            0.0
                        },
                ),
            },
        );
    }

    Command::none()
}

pub fn restore_window(app: &mut MainWindow) -> Command<WindowMessage> {
    app.window_state.has_maximized_pane = false;
    app.panes.restore();
    if !get_rewritable_config_opt(&app.app_config, &app.edit_data.window_edit_data).keep_window_size
    {
        return resize(
            window::Id::MAIN,
            Size {
                width: app.window_state.full_window_size.width,
                height: app.window_state.full_window_size.height,
            },
        );
    }
    Command::none()
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
            PaneVariant::ExecutionList => app.execution_data.get_edited_scripts().len(),
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

pub fn start_new_execution_from_edited_scripts(app: &mut MainWindow) {
    if app.execution_data.get_edited_scripts().is_empty() {
        return;
    }

    if app
        .execution_data
        .get_edited_scripts()
        .iter()
        .any(|script| is_script_missing_arguments(script))
    {
        return;
    }

    let scripts_to_execute = app.execution_data.consume_edited_scripts();

    start_new_execution_from_provided_scripts(app, scripts_to_execute);
}

pub fn start_new_execution_from_provided_scripts(
    app: &mut MainWindow,
    scripts: Vec<config::ScriptDefinition>,
) {
    if scripts
        .iter()
        .any(|script| is_script_missing_arguments(script))
    {
        eprintln!("Some scripts are missing arguments");
        return;
    }

    app.visual_caches.last_execution_id += 1;
    let name = format!("Execution #{}", app.visual_caches.last_execution_id);

    clean_script_selection(&mut app.window_state.cursor_script);
    let new_execution_id = app
        .execution_data
        .start_new_execution(&app.app_config, name, scripts);

    app.visual_caches.selected_execution_log = Some(new_execution_id);
    update_button_key_hint_caches(app);
}

pub fn add_edited_scripts_to_started_execution(
    app: &mut MainWindow,
    execution_id: execution_lists::ExecutionId,
) {
    if app.execution_data.get_edited_scripts().is_empty() {
        return;
    }

    if app
        .execution_data
        .get_edited_scripts()
        .iter()
        .any(|script| is_script_missing_arguments(script))
    {
        return;
    }

    clean_script_selection(&mut app.window_state.cursor_script);

    let scripts_to_execute = app.execution_data.consume_edited_scripts();
    app.execution_data.add_script_to_running_execution(
        &app.app_config,
        execution_id,
        scripts_to_execute,
    );
}

pub fn add_script_to_execution(
    app: &mut MainWindow,
    script_uid: config::Guid,
    should_focus: bool,
) -> bool {
    let scripts = get_resulting_scripts_from_guid(app, script_uid);

    if scripts.is_empty() {
        return false;
    }

    for script in scripts {
        app.execution_data.add_script_to_edited_list(script);
    }

    if should_focus {
        let script_idx = app.execution_data.get_edited_scripts().len() - 1;
        select_execution_script(app, script_idx);
        app.window_state.pane_focus = Some(app.pane_by_pane_type[&PaneVariant::ExecutionList]);
    }

    true
}

pub fn add_script_to_config(app: &mut MainWindow, script: config::ScriptDefinition) {
    if let Some(window_edit_data) = &app.edit_data.window_edit_data {
        let script_idx = match window_edit_data.edit_type {
            ConfigEditType::Shared => {
                Some(add_script_to_shared_config(&mut app.app_config, script))
            }
            ConfigEditType::Local => add_script_to_local_config(app, script),
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

pub fn make_script_copy(script: config::ScriptDefinition) -> config::ScriptDefinition {
    match script {
        config::ScriptDefinition::ReferenceToShared(_) => script,
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

pub fn remove_script(app: &mut MainWindow, script_id: &EditScriptId) {
    match script_id.script_type {
        EditScriptType::ScriptConfig => {
            if let Some(window_edit_data) = &mut app.edit_data.window_edit_data {
                match window_edit_data.edit_type {
                    ConfigEditType::Shared => {
                        app.app_config.script_definitions.remove(script_id.idx);
                        app.edit_data.is_dirty = true;
                    }
                    ConfigEditType::Local => {
                        if let Some(config) = &mut app.app_config.local_config_body {
                            config.script_definitions.remove(script_id.idx);
                            app.edit_data.is_dirty = true;
                        }
                    }
                }
            }

            config::populate_shared_scripts_from_config(&mut app.app_config);
            update_config_cache(app);
        }
        EditScriptType::ExecutionList => {
            app.execution_data
                .remove_script_from_edited_list(script_id.idx);
        }
    }
    clean_script_selection(&mut app.window_state.cursor_script);
    keybind_editing::prune_unused_keybinds(app);
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
    app: &mut MainWindow,
    script: config::ScriptDefinition,
) -> Option<usize> {
    if let Some(config) = &mut app.app_config.local_config_body {
        config.script_definitions.push(script);
    } else {
        return None;
    }

    update_config_cache(app);

    if let Some(config) = &mut app.app_config.local_config_body {
        Some(config.script_definitions.len() - 1)
    } else {
        None
    }
}

pub fn select_edited_script(app: &mut MainWindow, script_idx: usize) {
    set_selected_script(
        &mut app.window_state.cursor_script,
        &app.execution_data.get_edited_scripts(),
        &get_script_definition_list_opt(&app.app_config, &app.edit_data.window_edit_data),
        &mut app.visual_caches,
        script_idx,
        EditScriptType::ScriptConfig,
    );
    if let Some(window_edit_data) = &mut app.edit_data.window_edit_data {
        window_edit_data.is_editing_config = false;
    }
}

pub fn select_execution_script(app: &mut MainWindow, script_idx: usize) {
    set_selected_script(
        &mut app.window_state.cursor_script,
        &app.execution_data.get_edited_scripts(),
        &app.execution_data.get_edited_scripts(),
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

fn set_selected_script(
    currently_edited_script: &mut Option<EditScriptId>,
    scripts_to_run: &Vec<config::ScriptDefinition>,
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
                &scripts_to_run.get(script_idx)
            {
                visual_caches.autorerun_count = script.autorerun_count.to_string();
            }
        }
    };
}

pub fn clean_script_selection(currently_edited_script: &mut Option<EditScriptId>) {
    *currently_edited_script = None;
}

pub fn move_config_script_up(app: &mut MainWindow, index: usize) {
    if let Some(window_edit_data) = &mut app.edit_data.window_edit_data {
        match window_edit_data.edit_type {
            ConfigEditType::Shared => {
                if index >= 1 && index < app.app_config.script_definitions.len() {
                    app.app_config.script_definitions.swap(index, index - 1);
                    app.edit_data.is_dirty = true;
                }
            }
            ConfigEditType::Local => {
                if let Some(local_config_body) = &mut app.app_config.local_config_body {
                    if index >= 1 && index < local_config_body.script_definitions.len() {
                        local_config_body.script_definitions.swap(index, index - 1);
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

    update_config_cache(app);
}

pub fn move_config_script_down(app: &mut MainWindow, index: usize) {
    if let Some(window_edit_data) = &mut app.edit_data.window_edit_data {
        match window_edit_data.edit_type {
            ConfigEditType::Shared => {
                if index + 1 < app.app_config.script_definitions.len() {
                    app.app_config.script_definitions.swap(index, index + 1);
                    app.edit_data.is_dirty = true;
                }
            }
            ConfigEditType::Local => {
                if let Some(local_config_body) = &mut app.app_config.local_config_body {
                    if index + 1 < local_config_body.script_definitions.len() {
                        local_config_body.script_definitions.swap(index, index + 1);
                        app.edit_data.is_dirty = true;
                    }
                }
            }
        }
    }

    if let Some(edited_script) = &app.window_state.cursor_script {
        if edited_script.idx == index && index + 1 < app.displayed_configs_list_cache.len() {
            select_edited_script(app, index + 1);
        }
    }

    update_config_cache(app);
}

pub fn apply_script_edit(
    app: &mut MainWindow,
    edit_fn: impl FnOnce(&mut config::OriginalScriptDefinition),
) {
    if let Some(script_id) = &app.window_state.cursor_script {
        match script_id.script_type {
            EditScriptType::ScriptConfig => match &app.edit_data.window_edit_data {
                Some(window_edit_data) if window_edit_data.edit_type == ConfigEditType::Local => {
                    if let Some(config) = &mut app.app_config.local_config_body {
                        match &mut config.script_definitions[script_id.idx] {
                            config::ScriptDefinition::Original(script) => {
                                edit_fn(script);
                                app.edit_data.is_dirty = true;
                                update_config_cache(app);
                            }
                            _ => {}
                        }
                    }
                }
                _ => match &mut app.app_config.script_definitions[script_id.idx] {
                    config::ScriptDefinition::Original(script) => {
                        edit_fn(script);
                        app.edit_data.is_dirty = true;
                        update_config_cache(app);
                    }
                    _ => {}
                },
            },
            EditScriptType::ExecutionList => {
                match &mut app.execution_data.get_edited_scripts_mut()[script_id.idx] {
                    config::ScriptDefinition::Original(script) => {
                        edit_fn(script);
                    }
                    _ => {}
                }
            }
        }
    }
}

pub fn clear_edited_scripts(app: &mut MainWindow) {
    app.execution_data.clear_edited_scripts();
    clean_script_selection(&mut app.window_state.cursor_script);
}

pub fn clear_execution_scripts(app: &mut MainWindow) {
    // use the same script that we hinted visually
    let execution_id = app
        .visual_caches
        .button_key_caches
        .last_cleanable_execution_id
        .and_then(|execution_id| {
            app.execution_data
                .get_started_executions()
                .get(execution_id)
                .filter(|execution| execution.has_finished_execution())
                .map(|_| execution_id)
        });

    let Some(execution_id) = execution_id else {
        return;
    };

    app.execution_data.remove_execution(execution_id);
    clean_script_selection(&mut app.window_state.cursor_script);
    on_execution_removed(app, execution_id);
}

pub fn enter_window_edit_mode(app: &mut MainWindow) {
    if app.app_config.is_read_only {
        return;
    }

    app.edit_data.window_edit_data = Some(WindowEditData::from_config(
        &app.app_config,
        false,
        if app.app_config.local_config_body.is_some() {
            ConfigEditType::Local
        } else {
            ConfigEditType::Shared
        },
    ));
    app.edit_data.script_filter = String::new();
    clean_script_selection(&mut app.window_state.cursor_script);
    update_config_cache(app);
    app.visual_caches.is_custom_title_editing = false;
}

pub fn exit_window_edit_mode(app: &mut MainWindow) {
    app.edit_data.window_edit_data = None;
    clean_script_selection(&mut app.window_state.cursor_script);
    apply_theme(app);
    update_config_cache(app);
    update_git_branch_visibility(app);
}

pub fn switch_config_edit_mode(app: &mut MainWindow, edit_type: ConfigEditType) {
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

pub fn apply_theme_color_from_string(
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

pub fn apply_theme(app: &mut MainWindow) {
    app.theme = get_theme(&app.app_config, &app.edit_data.window_edit_data);
    update_theme_icons(app);
}

pub fn focus_filter(app: &mut MainWindow) -> Command<WindowMessage> {
    if app.panes.maximized().is_none() {
        if let Some(focus) = app.window_state.pane_focus {
            if &app.panes.panes[&focus].variant != &PaneVariant::ScriptList {
                app.window_state.pane_focus = Some(app.pane_by_pane_type[&PaneVariant::ScriptList]);
            }
        } else {
            app.window_state.pane_focus = Some(app.pane_by_pane_type[&PaneVariant::ScriptList]);
        }
    }
    Command::batch([
        text_input::focus(FILTER_INPUT_ID.clone()),
        text_input::select_all(FILTER_INPUT_ID.clone()),
    ])
}

pub fn should_autoclean_on_success(
    app: &mut MainWindow,
    execution_id: execution_lists::ExecutionId,
) -> bool {
    if let Some(execution) = app
        .execution_data
        .get_started_executions()
        .get(execution_id)
    {
        if !execution.has_finished_execution() || execution.has_failed_scripts() {
            return false;
        }

        let execution = app
            .execution_data
            .get_started_executions()
            .get(execution_id)
            .unwrap();
        return execution
            .get_scheduled_scripts_cache()
            .iter()
            .all(|record| match &record.script {
                config::ScriptDefinition::Original(script) => script.autoclean_on_success,
                _ => false,
            });
    }

    false
}
