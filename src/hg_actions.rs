use crate::{
    application::{action_aggregator, ActionResult},
    select::{Entry, State},
    version_control_actions::{handle_command, task, VersionControlActions},
    worker::{parallel, serial, task_vec, Task},
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
            .expect("Root directory is an empty string");
        self.current_dir = dir.to_owned();

        Ok(())
    }

    fn get_root(&self) -> &str {
        &self.current_dir[..]
    }

    fn get_current_changed_files(&mut self) -> Result<Vec<Entry>, String> {
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
        &mut self,
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

    fn version(&mut self) -> Result<String, String> {
        handle_command(self.command().arg("--version"))
    }

    fn status(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        let mut tasks = task_vec();
        tasks.push(task(self, |command| {
            command.args(&["summary", "--color", "always"]);
        }));
        tasks.push(task(self, |command| {
            command.args(&["status", "--color", "always"]);
        }));
        parallel(tasks, action_aggregator)
    }

    fn current_export(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.args(&["export", "--color", "always"]);
        })
    }

    fn log(&mut self, count: usize) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            let count_str = format!("{}", count);
            let template = "{label('green', if(topics, '[{topics}]'))} {label(ifeq(phase, 'secret', 'yellow', ifeq(phase, 'draft', 'yellow', 'red')), node|short)}{ifeq(branch, 'default', '', label('green', ' ({branch})'))}{bookmarks % ' {bookmark}{ifeq(bookmark, active, '*')}{bookmark}'}{label('yellow', tags % ' {tag}')} {label('magenta', author|person)} {desc|firstline|strip}";
            command
                .arg("log")
                .arg("--config")
                .arg("experimental.graphshorten=True")
                .arg("--graph")
                .arg("--template")
                .arg(template)
                .arg("-l")
                .arg(&count_str)
                .arg("--color")
                .arg("always");
        })
    }

    fn current_diff_all(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.arg("diff").arg("--color").arg("always");
        })
    }

    fn current_diff_selected(
        &mut self,
        entries: &Vec<Entry>,
    ) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.arg("diff").arg("--color").arg("always").arg("--");
            for e in entries.iter().filter(|e| e.selected) {
                command.arg(&e.filename);
            }
        })
    }

    fn revision_changes(
        &mut self,
        target: &str,
    ) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command
                .arg("status")
                .arg("--change")
                .arg(target)
                .arg("--color")
                .arg("always");
        })
    }

    fn revision_diff_all(
        &mut self,
        target: &str,
    ) -> Box<dyn Task<Output = ActionResult>> {
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
        &mut self,
        target: &str,
        entries: &Vec<Entry>,
    ) -> Box<dyn Task<Output = ActionResult>> {
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

    fn commit_all(
        &mut self,
        message: &str,
    ) -> Box<dyn Task<Output = ActionResult>> {
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
        &mut self,
        message: &str,
        entries: &Vec<Entry>,
    ) -> Box<dyn Task<Output = ActionResult>> {
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
        serial(tasks, action_aggregator)
    }

    fn revert_all(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        let mut tasks = task_vec();
        tasks.push(task(self, |command| {
            command.args(&["revert", "-C", "--all"]);
        }));
        tasks.push(task(self, |command| {
            command.args(&["purge"]);
        }));
        serial(tasks, action_aggregator)
    }

    fn revert_selected(
        &mut self,
        entries: &Vec<Entry>,
    ) -> Box<dyn Task<Output = ActionResult>> {
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
        parallel(tasks, action_aggregator)
    }

    fn update(&mut self, target: &str) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.arg("update").arg(target);
        })
    }

    fn merge(&mut self, target: &str) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.arg("merge").arg(target);
        })
    }

    fn conflicts(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.args(&["resolve", "-l", "--color", "always"]);
        })
    }

    fn take_other(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.args(&["resolve", "-a", "-t", "internal:other"]);
        })
    }

    fn take_local(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.args(&["resolve", "-a", "-t", "internal:local"]);
        })
    }

    fn fetch(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        self.pull()
    }

    fn pull(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.arg("pull");
        })
    }

    fn push(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.args(&["push", "--new-branch"]);
        })
    }

    fn create_tag(
        &mut self,
        name: &str,
    ) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.arg("tag").arg(name).arg("-f");
        })
    }

    fn list_branches(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.args(&["branches", "--color", "always"]);
        })
    }

    fn create_branch(
        &mut self,
        name: &str,
    ) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.arg("branch").arg(name);
        })
    }

    fn close_branch(
        &mut self,
        name: &str,
    ) -> Box<dyn Task<Output = ActionResult>> {
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
        serial(tasks, action_aggregator)
    }
}
