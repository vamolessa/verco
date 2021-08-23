use crate::{
    action::{parallel, serial, task_vec, ActionTask},
    select::{Entry, State},
    version_control_actions::{handle_command, task, VersionControlActions},
};

fn str_to_state(s: &str) -> State {
    match s {
        "?" => State::Untracked,
        "M" => State::Modified,
        "A" => State::Added,
        "R" => State::Deleted,
        "!" => State::Missing,
        "I" => State::Ignored,
        "C" => State::Clean,
        _ => State::Copied,
    }
}

pub struct HgActions {
    pub current_dir: String,
}

impl<'a> VersionControlActions for HgActions {
    fn executable_name(&self) -> &'static str {
        "hg"
    }

    fn current_dir(&self) -> &str {
        &self.current_dir[..]
    }

    fn set_root(&mut self) -> Result<(), String> {
        let mut command = self.command();
        let dir = handle_command(command.arg("root"))?;

        let dir = dir
            .lines()
            .next()
            .expect("root directory is an empty string");
        self.current_dir = dir.to_owned();

        Ok(())
    }

    fn get_root(&self) -> &str {
        &self.current_dir[..]
    }

    fn get_current_changed_files(&self) -> Result<Vec<Entry>, String> {
        let output = handle_command(self.command().arg("status"))?;

        let files = output
            .trim()
            .split('\n')
            .map(|e| e.trim())
            .filter(|e| e.len() > 1)
            .map(|e| {
                let (state, filename) = e.split_at(1);
                Entry {
                    filename: String::from(filename.trim()),
                    selected: false,
                    state: str_to_state(state),
                }
            })
            .collect();
        Ok(files)
    }

    fn get_revision_changed_files(
        &self,
        target: &str,
    ) -> Result<Vec<Entry>, String> {
        let output = handle_command(
            self.command().arg("status").arg("--change").arg(target),
        )?;

        let files = output
            .trim()
            .split('\n')
            .map(|e| e.trim())
            .filter(|e| e.len() > 1)
            .map(|e| {
                let (state, filename) = e.split_at(1);
                Entry {
                    filename: String::from(filename.trim()),
                    selected: false,
                    state: str_to_state(state),
                }
            })
            .collect();
        Ok(files)
    }

    fn version(&self) -> Result<String, String> {
        handle_command(self.command().arg("--version"))
    }

    fn status(&self) -> Box<dyn ActionTask> {
        let mut tasks = task_vec();
        tasks.push(task(self, |command| {
            command.args(&["summary", "--color", "always"]);
        }));
        tasks.push(task(self, |command| {
            command.args(&["status", "--color", "always"]);
        }));
        parallel(tasks)
    }

    fn current_export(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.args(&["export", "--color", "always"]);
        })
    }

    fn log(&self, count: usize) -> Box<dyn ActionTask> {
        task(self, |command| {
            let count_str = format!("{}", count);
            let template = "\x1e{node|short}\x1e{date|shortdate}\x1e{author|person}\x1e{ifeq(phase,'secret','(secret) ','')}{ifeq(phase,'draft','(draft) ','')}{if(topics,'[{topics}] ')}{tags % '{tag} '}{branch}\x1e{desc|firstline|strip}";
            command
                .arg("log")
                .arg("--config")
                .arg("experimental.graphshorten=True")
                .arg("--graph")
                .arg("--template")
                .arg(template)
                .arg("-l")
                .arg(&count_str);
        })
    }

    fn current_diff_all(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.arg("diff").arg("--color").arg("always");
        })
    }

    fn current_diff_selected(
        &self,
        entries: &Vec<Entry>,
    ) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.arg("diff").arg("--color").arg("always").arg("--");
            for e in entries.iter().filter(|e| e.selected) {
                command.arg(&e.filename);
            }
        })
    }

    fn revision_changes(&self, target: &str) -> Box<dyn ActionTask> {
        task(self, |command| {
            command
                .arg("status")
                .arg("--change")
                .arg(target)
                .arg("--color")
                .arg("always");
        })
    }

    fn revision_diff_all(&self, target: &str) -> Box<dyn ActionTask> {
        task(self, |command| {
            command
                .arg("diff")
                .arg("--change")
                .arg(target)
                .arg("--color")
                .arg("always");
        })
    }

    fn revision_diff_selected(
        &self,
        target: &str,
        entries: &Vec<Entry>,
    ) -> Box<dyn ActionTask> {
        task(self, |command| {
            command
                .arg("diff")
                .arg("--change")
                .arg(target)
                .arg("--color")
                .arg("always")
                .arg("--");

            for e in entries.iter().filter(|e| e.selected) {
                command.arg(&e.filename);
            }
        })
    }

    fn commit_all(&self, message: &str) -> Box<dyn ActionTask> {
        task(self, |command| {
            command
                .arg("commit")
                .arg("--addremove")
                .arg("-m")
                .arg(message)
                .arg("--color")
                .arg("always");
        })
    }

    fn commit_selected(
        &self,
        message: &str,
        entries: &Vec<Entry>,
    ) -> Box<dyn ActionTask> {
        let mut tasks = task_vec();
        let mut files_to_commit = Vec::new();
        for e in entries.iter().filter(|e| e.selected) {
            match e.state {
                State::Missing | State::Deleted => {
                    tasks.push(task(self, |command| {
                        command.arg("remove").arg(&e.filename);
                    }))
                }
                State::Untracked => tasks.push(task(self, |command| {
                    command.arg("add").arg(&e.filename);
                })),
                _ => (),
            }
            files_to_commit.push(&e.filename);
        }
        tasks.push(task(self, |command| {
            command
                .arg("commit")
                .arg("-m")
                .arg(message)
                .arg("--color")
                .arg("always");
            for file in files_to_commit {
                command.arg(file);
            }
        }));
        serial(tasks)
    }

    fn revert_all(&self) -> Box<dyn ActionTask> {
        let mut tasks = task_vec();
        tasks.push(task(self, |command| {
            command.args(&["revert", "-C", "--all"]);
        }));
        tasks.push(task(self, |command| {
            command.args(&["purge"]);
        }));
        serial(tasks)
    }

    fn revert_selected(&self, entries: &Vec<Entry>) -> Box<dyn ActionTask> {
        let mut tasks = task_vec();
        let mut files_to_revert = Vec::new();
        for e in entries.iter().filter(|e| e.selected) {
            match e.state {
                State::Untracked => tasks.push(task(self, |command| {
                    command.arg("purge").arg(&e.filename);
                })),
                _ => files_to_revert.push(&e.filename),
            }
        }
        if files_to_revert.len() > 0 {
            tasks.push(task(self, |command| {
                command.arg("revert").arg("-C").arg("--color").arg("always");
                for file in files_to_revert {
                    command.arg(file);
                }
            }));
        }
        parallel(tasks)
    }

    fn update(&self, target: &str) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.arg("update").arg(target);
        })
    }

    fn merge(&self, target: &str) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.arg("merge").arg(target);
        })
    }

    fn conflicts(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.args(&["resolve", "-l", "--color", "always"]);
        })
    }

    fn take_other(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.args(&["resolve", "-a", "-t", "internal:other"]);
        })
    }

    fn take_local(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.args(&["resolve", "-a", "-t", "internal:local"]);
        })
    }

    fn fetch(&self) -> Box<dyn ActionTask> {
        self.pull()
    }

    fn pull(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.arg("pull");
        })
    }

    fn push(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.args(&["push", "--new-branch"]);
        })
    }

    fn create_tag(&self, name: &str) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.arg("tag").arg(name).arg("-f");
        })
    }

    fn list_branches(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.args(&["branches", "--template", "{branch}\n"]);
        })
    }

    fn create_branch(&self, name: &str) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.arg("branch").arg(name);
        })
    }

    fn close_branch(&self, name: &str) -> Box<dyn ActionTask> {
        let changeset =
            handle_command(self.command().args(&["identify", "--num"])).ok();

        let mut tasks = task_vec();
        tasks.push(self.update(name));
        tasks.push(task(self, |command| {
            command.args(&[
                "commit",
                "-m",
                "\"close branch\"",
                "--close-branch",
            ]);
        }));
        if let Some(changeset) = changeset {
            tasks.push(self.update(changeset.trim()));
        }
        serial(tasks)
    }
}