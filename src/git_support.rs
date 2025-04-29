// Copyright (C) Pavel Grebnev 2023-2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use notify::{self, Watcher};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread;

#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

pub struct GitCurrentBranchRequester {
    // the latest known branch
    current_branch: String,

    // the watcher for the HEAD file that stores the information about the current branch
    head_watcher: Option<notify::RecommendedWatcher>,
    is_head_changed: Arc<AtomicBool>,

    // the receiver for the path to the HEAD file
    head_folder_receiver: Option<Receiver<String>>,
    head_folder_request_thread: Option<thread::JoinHandle<()>>,

    // the receiver for the current branch name
    branch_receiver: Option<Receiver<String>>,
    branch_request_thread: Option<thread::JoinHandle<()>>,

    // time of the last branch request
    last_request_time: Option<std::time::Instant>,
    // minimal time between requests to avoid spamming threads
    // e.g. the HEAD file can change a lot during a rebase
    min_request_interval: std::time::Duration,
    // a flag to communicate that we delayed the request
    waiting_to_start_requesting_branch: bool,
}

impl GitCurrentBranchRequester {
    pub fn new() -> GitCurrentBranchRequester {
        let mut new_requester = GitCurrentBranchRequester {
            current_branch: String::new(),
            head_watcher: None,
            is_head_changed: Arc::new(AtomicBool::new(false)),
            head_folder_receiver: None,
            head_folder_request_thread: None,
            branch_receiver: None,
            branch_request_thread: None,
            last_request_time: None,
            min_request_interval: std::time::Duration::from_secs(3),
            waiting_to_start_requesting_branch: false,
        };
        new_requester.set_up();
        new_requester
    }

    pub fn get_current_branch_ref(&self) -> &String {
        &self.current_branch
    }

    pub fn update(&mut self) {
        // got a result for the HEAD file path
        if let Some(rx) = &self.head_folder_receiver {
            if let Ok(head) = rx.try_recv() {
                if head.is_empty() {
                    return;
                }
                self.set_up_head_watcher(&head);
                self.head_folder_receiver = None;

                // we are now ready to request the current branch for the first time
                self.request_current_branch();
            }
        }

        // if the HEAD file has changed in any way
        if self
            .is_head_changed
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            self.is_head_changed
                .store(false, std::sync::atomic::Ordering::Relaxed);
            self.request_current_branch();
        }

        // just received the result of the branch request result
        if let Some(rx) = &self.branch_receiver {
            if let Ok(branch) = rx.try_recv() {
                self.current_branch = branch;
                self.branch_receiver = None;
            }
        }

        // if we are waiting for being able to request a branch, we should try now
        if self.waiting_to_start_requesting_branch {
            self.request_current_branch();
        }
    }

    fn set_up(&mut self) {
        // the set-up consists of several steps that we perform in a chain:
        // 1. request the path to HEAD file used for this git repository
        // 2. set up a watcher for the HEAD file
        // 3. request the current branch name
        self.start_setting_up_file_watcher();
    }

    fn can_request_branch_name(&self) -> bool {
        let now = std::time::Instant::now();
        if let Some(last_request_time) = self.last_request_time {
            if now.duration_since(last_request_time) < self.min_request_interval {
                return false;
            }
        }

        if self.branch_receiver.is_some() {
            return false;
        }

        if let Some(branch_request_thread) = &self.branch_request_thread {
            if !branch_request_thread.is_finished() {
                return false;
            }
        }

        true
    }

    fn request_current_branch(&mut self) {
        if self.can_request_branch_name() {
            self.last_request_time = Some(std::time::Instant::now());
            self.waiting_to_start_requesting_branch = false;

            let (branch_sender, branch_receiver) = channel();
            self.branch_receiver = Some(branch_receiver);
            if let Some(branch_request_thread) = self.branch_request_thread.take() {
                // should never block since can_request_branch_name() checks if the thread is done
                let _ = branch_request_thread.join();
            }
            self.branch_request_thread = Some(thread::spawn(move || {
                let mut current_branch_or_hash =
                    run_command("git", vec!["branch", "--show-current"]);

                if current_branch_or_hash.is_empty() {
                    // git rev-parse --short HEAD will return the short hash of the current commit
                    current_branch_or_hash =
                        run_command("git", vec!["rev-parse", "--short", "HEAD"]);
                }

                let _ = branch_sender.send(current_branch_or_hash);
            }));
        } else {
            self.waiting_to_start_requesting_branch = true;
        }
    }

    fn set_up_head_watcher(&mut self, head: &String) {
        if self.head_watcher.is_none() {
            let is_head_changed = self.is_head_changed.clone();
            let watcher =
                notify::recommended_watcher(move |res: Result<notify::Event, _>| match res {
                    Ok(event) => {
                        if event.paths.iter().any(|path| path.ends_with("HEAD")) {
                            is_head_changed.store(true, std::sync::atomic::Ordering::Relaxed);
                        }
                    }
                    Err(_) => {}
                });
            let Ok(mut watcher) = watcher else {
                return;
            };

            let result = watcher.watch(Path::new(&head), notify::RecursiveMode::NonRecursive);
            if let Ok(_) = result {
                self.head_watcher = Some(watcher);
            }
        }
    }

    fn start_setting_up_file_watcher(&mut self) {
        let (head_folder_sender, head_folder_receiver) = channel();
        self.head_folder_receiver = Some(head_folder_receiver);

        self.head_folder_request_thread = Some(thread::spawn(move || {
            // git rev-parse --git-dir will return the directory where HEAD is located
            let head = run_command("git", vec!["rev-parse", "--git-dir"]);

            let _ = head_folder_sender.send(head);
        }));
    }
}

fn run_command(command: &str, args: Vec<&str>) -> String {
    let mut command = std::process::Command::new(command);

    #[cfg(target_os = "windows")]
    {
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }

    let result = command.args(args).output();

    match result {
        Ok(output) => {
            let output = String::from_utf8_lossy(&output.stdout);
            for line in output.lines() {
                return line.to_string();
            }
        }
        Err(_) => {}
    }

    String::new()
}
