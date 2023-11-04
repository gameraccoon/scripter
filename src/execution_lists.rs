use chrono;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::sync::atomic::AtomicBool;

use crate::config;
use crate::execution;
use crate::file_utils;
use crate::ring_buffer;

pub struct ExecutionList {
    execution_data: execution::ScriptExecutionData,
    first_cache_index: usize,
}

pub struct ExecutionLists {
    execution_lists: Vec<ExecutionList>,
    scheduled_scripts_cache: Vec<config::ScriptDefinition>,
    scheduled_scripts_statuses: Vec<execution::ScriptExecutionStatus>,
    current_execution_list: usize,
    has_started_execution: bool,
    has_failed_scripts: bool,
    log_directory: PathBuf,
    recent_logs: Arc<Mutex<execution::LogBuffer>>,
    currently_outputting_script: isize,
}

impl ExecutionLists {
    pub fn new() -> Self {
        Self {
            execution_lists: vec![ExecutionList {
                execution_data: execution::new_execution_data(),
                first_cache_index: 0,
            }],
            scheduled_scripts_cache: Vec::new(),
            scheduled_scripts_statuses: Vec::new(),
            current_execution_list: 0,
            has_started_execution: false,
            has_failed_scripts: false,
            log_directory: PathBuf::new(),
            recent_logs: Arc::new(Mutex::new(ring_buffer::RingBuffer::new(Default::default()))),
            currently_outputting_script: -1,
        }
    }

    pub fn add_script_to_execution(&mut self, script: config::ScriptDefinition) {
        execution::add_script_to_execution(&mut self.get_edited_execution_list_mut(), script);
    }

    pub fn remove_script(&mut self, idx: usize) {
        execution::remove_script_from_execution(&mut self.get_edited_execution_list_mut(), idx);
    }

    pub fn start_execution(&mut self, app_config: &config::AppConfig) {
        if self.get_edited_execution_list().len() == 0 {
            return;
        }

        self.scheduled_scripts_cache.append(
            &mut self
                .execution_lists
                .last()
                .unwrap()
                .execution_data
                .scripts_to_run
                .clone(),
        );
        // append as many empty statuses as we added scripts
        self.scheduled_scripts_statuses.append(&mut vec![
            execution::ScriptExecutionStatus {
                start_time: None,
                finish_time: None,
                result: execution::ScriptResultStatus::Success,
                retry_count: 0,
            };
            self.get_edited_execution_list().len()
        ]);
        // add a new execution list to be the "edited" one
        self.execution_lists.push(ExecutionList {
            execution_data: execution::new_execution_data(),
            first_cache_index: self.scheduled_scripts_cache.len(),
        });

        if !self.has_started_execution() {
            self.log_directory = file_utils::get_script_log_directory(
                &app_config.paths.logs_path,
                &chrono::Local::now(),
            );

            self.run_execution_list(app_config);
        }
    }

    pub fn get_edited_execution_list(&self) -> &Vec<config::ScriptDefinition> {
        &self
            .execution_lists
            .last()
            .unwrap()
            .execution_data
            .scripts_to_run
    }

    pub fn get_edited_execution_list_mut(&mut self) -> &mut Vec<config::ScriptDefinition> {
        &mut self
            .execution_lists
            .last_mut()
            .unwrap()
            .execution_data
            .scripts_to_run
    }

    pub fn get_scheduled_execution_list(&self) -> &Vec<config::ScriptDefinition> {
        &self.scheduled_scripts_cache
    }

    pub fn get_scheduled_execution_statuses(&self) -> &Vec<execution::ScriptExecutionStatus> {
        &self.scheduled_scripts_statuses
    }

    pub fn has_started_execution(&self) -> bool {
        return self.has_started_execution;
    }

    pub fn has_finished_execution(&self) -> bool {
        if let Some(scheduled_script) = self.scheduled_scripts_statuses.last() {
            return execution::has_script_finished(&scheduled_script);
        }
        return false;
    }

    pub fn has_failed_scripts(&self) -> bool {
        self.has_failed_scripts
    }

    pub fn request_stop_execution(&mut self) {
        if self.has_started_execution() && self.current_execution_list < self.execution_lists.len()
        {
            execution::request_stop_execution(
                &mut self.execution_lists[self.current_execution_list].execution_data,
            );
        }
    }

    pub fn is_waiting_execution_to_finish(&self) -> bool {
        if let Some(execution_list) = self.execution_lists.last() {
            return execution::is_waiting_execution_thread_to_finish(
                &execution_list.execution_data,
            );
        }
        return false;
    }

    pub fn tick(&mut self, app_config: &config::AppConfig) -> bool {
        let current_execution_list = &mut self.execution_lists[self.current_execution_list];
        if let Some(rx) = &current_execution_list.execution_data.progress_receiver {
            if let Ok(progress) = rx.try_recv() {
                if execution::has_script_failed(&progress.1) {
                    self.has_failed_scripts = true;
                }
                let script_local_idx = progress.0;
                let script_status = progress.1;

                let execution_list = &mut self.execution_lists[self.current_execution_list];

                let script_idx = execution_list.first_cache_index + script_local_idx;

                self.scheduled_scripts_statuses[script_idx] = script_status;
                self.currently_outputting_script = progress.0 as isize;

                if execution::has_script_finished(&self.scheduled_scripts_statuses[script_idx])
                    && execution_list.execution_data.scripts_to_run.len() == script_local_idx + 1
                {
                    self.current_execution_list += 1;
                    if self.try_join_execution_thread(self.current_execution_list - 1) {
                        self.run_execution_list(app_config);
                    }
                }

                if self.has_finished_execution() {
                    return true;
                }
            }
        } else {
            if self.current_execution_list > 0 {
                if self.try_join_execution_thread(self.current_execution_list - 1) {
                    self.run_execution_list(app_config);
                }
            }
        }
        return false;
    }

    fn run_execution_list(&mut self, app_config: &config::AppConfig) {
        if self.current_execution_list + 1 >= self.execution_lists.len() {
            return;
        }

        let had_failures_before =
            if let Some(last_script) = self.get_previous_execution_list_status() {
                execution::has_script_failed(last_script) || execution::has_script_been_skipped(last_script)
            } else {
                false
            };

        let execution_list = &mut self.execution_lists[self.current_execution_list];

        if execution_list.execution_data.scripts_to_run.len() == 0 {
            return;
        }

        execution::run_scripts(
            &mut execution_list.execution_data,
            &self.log_directory,
            had_failures_before,
            &app_config,
            self.recent_logs.clone(),
            execution_list.first_cache_index,
        );

        self.has_started_execution = true;
    }

    pub fn get_log_path(&self) -> PathBuf {
        self.log_directory.clone()
    }

    fn join_execution_thread(&mut self, list_idx: usize) {
        // this should never block, since the thread should be finished by now
        // but we do it anyway not to miss bugs that create zombie threads
        if let Some(join_handle) = self.execution_lists[list_idx]
            .execution_data
            .thread_join_handle
            .take()
        {
            join_handle.join().unwrap(); // have no idea what to do if this fails, crashing is probably fine
        };
    }

    fn try_join_execution_thread(&mut self, list_idx: usize) -> bool {
        if let Some(handle) = &self.execution_lists[list_idx]
            .execution_data
            .thread_join_handle
        {
            if handle.is_finished() {
                self.join_execution_thread(list_idx);
                return true;
            }
        } else {
            // the thread has already been joined
            return true;
        }

        return false;
    }

    fn reset_execution_progress(&mut self) {
        let mut joined_execution_list = execution::new_execution_data();
        for execution_list in &mut self.execution_lists {
            joined_execution_list
                .scripts_to_run
                .append(&mut execution_list.execution_data.scripts_to_run);
        }

        self.scheduled_scripts_statuses.clear();
        self.scheduled_scripts_cache.clear();

        self.execution_lists = vec![ExecutionList {
            execution_data: joined_execution_list,
            first_cache_index: 0,
        }];

        self.current_execution_list = 0;
        self.has_started_execution = false;
        self.has_failed_scripts = false;
        self.log_directory = PathBuf::new();
        self.recent_logs = Arc::new(Mutex::new(ring_buffer::RingBuffer::new(Default::default())));
        self.currently_outputting_script = -1;
    }

    pub fn reschedule_scripts(&mut self) {
        if self.current_execution_list > 0 {
            self.try_join_execution_thread(self.current_execution_list - 1);
        }
        self.try_join_execution_thread(self.current_execution_list);

        self.reset_execution_progress();
    }

    pub fn clear_edited_scripts(&mut self) {
        self.get_edited_execution_list_mut().clear();
    }

    pub fn clear_execution_scripts(&mut self) {
        if self.current_execution_list > 0 {
            self.try_join_execution_thread(self.current_execution_list - 1);
        }
        self.try_join_execution_thread(self.current_execution_list);

        self.execution_lists = vec![ExecutionList {
            execution_data: execution::ScriptExecutionData {
                scripts_to_run: self.get_edited_execution_list().clone(),
                progress_receiver: None,
                is_termination_requested: Arc::new(AtomicBool::new(false)),
                thread_join_handle: None,
            },
            first_cache_index: 0,
        }];
        self.scheduled_scripts_cache.clear();
        self.scheduled_scripts_statuses.clear();
        self.current_execution_list = 0;
        self.has_started_execution = false;
        self.has_failed_scripts = false;
        self.log_directory = PathBuf::new();
        self.recent_logs = Arc::new(Mutex::new(ring_buffer::RingBuffer::new(Default::default())));
        self.currently_outputting_script = -1;
    }

    fn get_previous_execution_list_status(&self) -> Option<&execution::ScriptExecutionStatus> {
        if self.current_execution_list > 0 {
            let previous_execution_list = &self.execution_lists[self.current_execution_list - 1];
            Some(
                &self.scheduled_scripts_statuses[previous_execution_list.first_cache_index
                    + previous_execution_list.execution_data.scripts_to_run.len()
                    - 1],
            )
        } else {
            None
        }
    }

    pub fn get_recent_logs(&self) -> &Arc<Mutex<execution::LogBuffer>> {
        &self.recent_logs
    }

    pub fn get_currently_outputting_script(&self) -> isize {
        self.currently_outputting_script
    }
}
