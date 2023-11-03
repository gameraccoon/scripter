use crate::execution;
use crate::config;

pub struct ExecutionList {
    execution_data: execution::ScriptExecutionData,
    // is_running: bool,
    // has_been_scheduled: bool,
}

pub struct ExecutionLists {
    execution_lists: Vec<ExecutionList>,
    // has_errors: bool,
}

impl ExecutionLists {
    pub fn new() -> Self {
        Self {
            execution_lists: vec![ExecutionList {
                execution_data: execution::new_execution_data(),
                // is_running: false,
                // has_been_scheduled: false,
            }],
            // has_errors: false,
        }
    }

    pub fn get_edited_execution_list(&self) -> &execution::ScriptExecutionData {
        &self.execution_lists.last().unwrap().execution_data
    }

    pub fn get_edited_execution_list_mut(&mut self) -> &mut execution::ScriptExecutionData {
        &mut self.execution_lists.last_mut().unwrap().execution_data
    }

    pub fn has_started_execution(&self) -> bool {
        execution::has_started_execution(&self.execution_lists.first().unwrap().execution_data)
    }

    pub fn has_finished_execution(&self) -> bool {
        execution::has_finished_execution(&self.execution_lists.last().unwrap().execution_data)
    }

    pub fn has_failed_scripts(&self) -> bool {
        self.execution_lists.last().unwrap().execution_data.has_failed_scripts
    }

    pub fn request_stop_execution(&mut self) {
        execution::request_stop_execution(&mut self.execution_lists.last_mut().unwrap().execution_data);
    }

    pub fn is_waiting_execution_to_finish(&self) -> bool {
        execution::is_waiting_execution_thread_to_finish(&self.execution_lists.last().unwrap().execution_data)
    }

    pub fn add_script_to_execution(&mut self, script: config::ScriptDefinition) {
        execution::add_script_to_execution(&mut self.execution_lists.last_mut().unwrap().execution_data, script);
    }

    pub fn tick(&mut self) -> bool {
        let current_execution_list = self.execution_lists.last_mut().unwrap();
        let current_execution_data = &mut current_execution_list.execution_data;
        if let Some(rx) = &current_execution_data.progress_receiver {
            if let Ok(progress) = rx.try_recv() {
                if execution::has_script_failed(&progress.1) {
                    current_execution_data.has_failed_scripts = true;
                }
                current_execution_data.scripts_status[progress.0] = progress.1;
                current_execution_data.currently_outputting_script = progress.0 as isize;

                if execution::has_finished_execution(&current_execution_data) {
                    return true;
                }
            }
        }
        return false;
    }

    fn join_execution_thread(&mut self) {
        // this should never block, since the thread should be finished by now
        // but we do it anyway not to miss bugs that create zombie threads
        if let Some(join_handle) = self.get_edited_execution_list_mut().thread_join_handle.take() {
            join_handle.join().unwrap(); // have no idea what to do if this fails, crashing is probably fine
        };
    }

    fn reset_execution_progress(&mut self) {
        self.execution_lists.last_mut().unwrap().execution_data = execution::get_reset_execution_progress(&self.execution_lists.last().unwrap().execution_data);
    }

    pub fn reschedule_scripts(&mut self) {
        if !self.has_started_execution() {
            return;
        }
        self.join_execution_thread();
        self.reset_execution_progress();
    }

    pub fn clear_scripts(&mut self) {
        self.join_execution_thread();
        self.execution_lists.last_mut().unwrap().execution_data = execution::new_execution_data();
    }

    pub fn remove_script(&mut self, idx: usize) {
        execution::remove_script_from_execution(&mut self.get_edited_execution_list_mut(), idx);
    }
}
