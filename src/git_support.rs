// Copyright (C) Pavel Grebnev 2023-2024
// Distributed under the MIT License (license terms are at http://opensource.org/licenses/MIT).

use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::thread;

pub struct GitCurrentBranchFetcher {
    current_branch: String,
    head_watcher: Option<RecommendedWatcher>,
    is_head_changed: Arc<AtomicBool>,
    rx_head_folder: Option<Receiver<String>>,
    rx_branch: Option<Receiver<String>>,
}

impl GitCurrentBranchFetcher {
    pub fn new() -> GitCurrentBranchFetcher {
        let mut new_fetcher = GitCurrentBranchFetcher {
            current_branch: String::new(),
            head_watcher: None,
            is_head_changed: Arc::new(AtomicBool::new(false)),
            rx_head_folder: None,
            rx_branch: None,
        };
        new_fetcher.set_up();
        new_fetcher
    }

    fn set_up(&mut self) {
        // the set up consists of several steps that we perfrom in a chain:
        // 1. fetch the path to HEAD file used for this git repository
        // 2. set up a watcher for the HEAD file
        // 3. request the current branch name
        self.start_setting_up_hooks();
    }

    pub fn get_current_branch_ref(&self) -> &String {
        &self.current_branch
    }

    pub fn fetch_current_branch(&mut self) {
        if self.rx_branch.is_none() {
            let (tx, rx) = channel();
            self.rx_branch = Some(rx);
            thread::spawn(move || {
                let output = Command::new("git")
                    .arg("branch")
                    .arg("--show-current")
                    .output();

                if let Err(_) = output {
                    tx.send(String::new()).unwrap();
                    return;
                }
                let output = output.unwrap();

                let output = String::from_utf8_lossy(&output.stdout);
                let mut current_branch_or_hash = String::new();
                for line in output.lines() {
                    current_branch_or_hash = line.to_string();
                    break;
                }

                if current_branch_or_hash.is_empty() {
                    // git rev-parse --short HEAD will return the short hash of the current commit
                    let output = Command::new("git")
                        .arg("rev-parse")
                        .arg("--short")
                        .arg("HEAD")
                        .output();

                    if let Err(_) = output {
                        tx.send(String::new()).unwrap();
                        return;
                    }

                    let output = output.unwrap();
                    let output = String::from_utf8_lossy(&output.stdout);
                    for line in output.lines() {
                        current_branch_or_hash = line.to_string();
                        break;
                    }
                }

                let _ = tx.send(current_branch_or_hash);
            });
        }
    }

    pub fn update(&mut self) {
        // if we're still setting up file watchers
        if let Some(rx) = &self.rx_head_folder {
            if let Ok(head) = rx.try_recv() {
                if head.is_empty() {
                    return;
                }
                if self.head_watcher.is_none() {
                    let is_head_changed = self.is_head_changed.clone();
                    let watcher =
                        notify::recommended_watcher(move |res: Result<Event, _>| match res {
                            Ok(event) => {
                                if event.paths.iter().any(|path| path.ends_with("HEAD")) {
                                    is_head_changed
                                        .store(true, std::sync::atomic::Ordering::Relaxed);
                                }
                            }
                            Err(_) => {}
                        });
                    let Ok(mut watcher) = watcher else {
                        return;
                    };

                    let result = watcher.watch(Path::new(&head), RecursiveMode::NonRecursive);
                    if let Ok(_) = result {
                        self.head_watcher = Some(watcher);
                    }
                }
                self.rx_head_folder = None;

                // we are now ready to fetch the current branch
                self.fetch_current_branch();
            }
        }

        // if the HEAD file has changed in any way
        if self
            .is_head_changed
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            self.is_head_changed
                .store(false, std::sync::atomic::Ordering::Relaxed);
            self.fetch_current_branch();
        }

        // just received the current branch
        if let Some(rx) = &self.rx_branch {
            if let Ok(branch) = rx.try_recv() {
                self.current_branch = branch;
                self.rx_branch = None;
            }
        }
    }

    fn start_setting_up_hooks(&mut self) {
        let (tx, rx) = channel();
        self.rx_head_folder = Some(rx);

        thread::spawn(move || {
            // git rev-parse --git-dir will return the directory where HEAD is located
            let output = Command::new("git")
                .arg("rev-parse")
                .arg("--git-dir")
                .output();

            if let Err(_) = output {
                tx.send(String::new()).unwrap();
                return;
            }
            let output = output.unwrap();

            let output = String::from_utf8_lossy(&output.stdout);
            let mut head = String::new();
            for line in output.lines() {
                head = line.to_string();
                break;
            }

            let _ = tx.send(head);
        });
    }
}
