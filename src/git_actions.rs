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
        "D" => State::Deleted,
        "R" => State::Renamed,
        "C" => State::Copied,
        "U" => State::Unmerged,
        _ => State::Unmodified,
    }
}

pub struct GitActions {
    pub current_dir: String,
}

impl VersionControlActions for GitActions {
    fn executable_name(&self) -> &'static str {
        "git"
    }

    fn current_dir(&self) -> &str {
        &self.current_dir[..]
    }

    fn set_root(&mut self) -> Result<(), String> {
        let mut command = self.command();
        let dir =
            handle_command(command.args(&["rev-parse", "--show-toplevel"]))?;

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

    fn get_current_changed_files(&mut self) -> Result<Vec<Entry>, String> {
        let output = handle_command(self.command().args(&["status", "-z"]))?;

        let files = output
            .trim()
            .split('\0')
            .map(|e| e.trim())
            .filter(|e| e.len() > 2)
            .map(|e| {
                let (state, filename) = e.split_at(2);
                Entry {
                    filename: String::from(filename.trim()),
                    selected: false,
                    state: str_to_state(&state[..1]),
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
            self.command()
                .arg("diff-tree")
                .arg("--no-commit-id")
                .arg("--name-status")
                .arg("-z")
                .arg("-r")
                .arg(target),
        )?;

        let state_iter = output.split('\0').map(|e| e.trim()).step_by(2);
        let filename_iter =
            output.split('\0').map(|e| e.trim()).skip(1).step_by(2);

        let files = state_iter
            .zip(filename_iter)
            .map(|(s, f)| Entry {
                filename: String::from(f),
                selected: false,
                state: str_to_state(s),
            })
            .collect();
        Ok(files)
    }

    fn version(&mut self) -> Result<String, String> {
        handle_command(self.command().arg("--version"))
    }

    fn status(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.args(&["-c", "color.status=always", "status"]);
        })
    }

    fn current_export(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.args(&["show", "--color"]);
        })
    }

    fn log(&mut self, count: usize) -> Box<dyn Task<Output = ActionResult>> {
        return task(self, |command| {
            let template = "--format=format:%C(auto,yellow)%h %C(auto,blue)%>(10,trunc)%ad %C(auto,green)%<(10,trunc)%aN %C(auto)%d %C(auto,reset)%s";
            command
                .arg("log")
                .arg("--all")
                .arg("--decorate")
                .arg("--oneline")
                .arg("--graph")
                .arg("-54")
                .arg("--color")
                .arg(template)
                .arg("--date=short");
        });

        task(self, |command| {
            let count_str = format!("-{}", count);
            let template = "--format=format:%C(auto,yellow)%h %C(auto,blue)%>(10,trunc)%ad %C(auto,green)%<(10,trunc)%aN %C(auto)%d %C(auto,reset)%s";
            command
                .arg("log")
                .arg("--all")
                //.arg("--decorate")
                .arg("--oneline")
                .arg("--graph")
                .arg(&count_str)
                .arg("--color")
                .arg(template)
                .arg("--date=short");
        })
    }

    fn current_diff_all(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.args(&["diff", "--color"]);
        })
    }

    fn current_diff_selected(
        &mut self,
        entries: &Vec<Entry>,
    ) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.arg("diff").arg("--color").arg("--");
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
                .arg("diff-tree")
                .arg("--no-commit-id")
                .arg("--name-status")
                .arg("-r")
                .arg(target)
                .arg("--color");
        })
    }

    fn revision_diff_all(
        &mut self,
        target: &str,
    ) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            let mut parents = String::from(target);
            parents.push_str("^@");
            command.arg("diff").arg(parents).arg(target).arg("--color");
        })
    }

    fn revision_diff_selected(
        &mut self,
        target: &str,
        entries: &Vec<Entry>,
    ) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            let mut parents = String::from(target);
            parents.push_str("^@");

            command
                .arg("diff")
                .arg("--color")
                .arg(parents)
                .arg(target)
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
        let mut tasks = task_vec();
        tasks.push(task(self, |command| {
            command.args(&["add", "--all"]);
        }));
        tasks.push(task(self, |command| {
            command.arg("commit").arg("-m").arg(message);
        }));
        serial(tasks, action_aggregator)
    }

    fn commit_selected(
        &mut self,
        message: &str,
        entries: &Vec<Entry>,
    ) -> Box<dyn Task<Output = ActionResult>> {
        let mut tasks = task_vec();
        for e in entries.iter().filter(|e| e.selected) {
            tasks.push(task(self, |command| {
                command.arg("add").arg("--").arg(&e.filename);
            }));
        }

        tasks.push(task(self, |command| {
            command.arg("commit").arg("-m").arg(message);
        }));
        serial(tasks, action_aggregator)
    }

    fn revert_all(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        let mut tasks = task_vec();
        tasks.push(task(self, |command| {
            command.args(&["reset", "--hard"]);
        }));
        tasks.push(task(self, |command| {
            command.args(&["clean", "-df"]);
        }));
        serial(tasks, action_aggregator)
    }

    fn revert_selected(
        &mut self,
        entries: &Vec<Entry>,
    ) -> Box<dyn Task<Output = ActionResult>> {
        let mut tasks = task_vec();
        for e in entries.iter().filter(|e| e.selected) {
            match e.state {
                State::Untracked => {
                    tasks.push(task(self, |command| {
                        command
                            .arg("clean")
                            .arg("-f")
                            .arg("--")
                            .arg(&e.filename);
                    }));
                }
                State::Added => {
                    tasks.push(task(self, |command| {
                        command.arg("rm").arg("-f").arg("--").arg(&e.filename);
                    }));
                }
                _ => {
                    tasks.push(task(self, |command| {
                        command.arg("checkout").arg("--").arg(&e.filename);
                    }));
                }
            }
        }
        parallel(tasks, action_aggregator)
    }

    fn update(&mut self, target: &str) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.arg("checkout").arg(target);
        })
    }

    fn merge(&mut self, target: &str) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.arg("merge").arg(target);
        })
    }

    fn conflicts(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.args(&["diff", "--name-only", "--diff-filter=U"]);
        })
    }

    fn take_other(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.args(&["checkout", ".", "--theirs"]);
        })
    }

    fn take_local(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.args(&["checkout", ".", "--ours"]);
        })
    }

    fn fetch(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.args(&["fetch", "--all"]);
        })
    }

    fn pull(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.args(&["pull", "--all"]);
        })
    }

    fn push(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.arg("push");
        })
    }

    fn create_tag(
        &mut self,
        name: &str,
    ) -> Box<dyn Task<Output = ActionResult>> {
        let mut tasks = task_vec();
        tasks.push(task(self, |command| {
            command.arg("tag").arg(name).arg("-f");
        }));
        tasks.push(task(self, |command| {
            command.arg("push").arg("origin").arg(name);
        }));
        serial(tasks, action_aggregator)
    }

    fn list_branches(&mut self) -> Box<dyn Task<Output = ActionResult>> {
        task(self, |command| {
            command.args(&["branch", "--all", "--color"]);
        })
    }

    fn create_branch(
        &mut self,
        name: &str,
    ) -> Box<dyn Task<Output = ActionResult>> {
        let mut tasks = task_vec();
        tasks.push(task(self, |command| {
            command.arg("branch").arg(name);
        }));
        tasks.push(self.update(name));
        tasks.push(task(self, |command| {
            command
                .arg("push")
                .arg("--set-upstream")
                .arg("origin")
                .arg(name);
        }));
        serial(tasks, action_aggregator)
    }

    fn close_branch(
        &mut self,
        name: &str,
    ) -> Box<dyn Task<Output = ActionResult>> {
        let mut tasks = task_vec();
        tasks.push(task(self, |command| {
            command.arg("branch").arg("-d").arg(name);
        }));
        tasks.push(task(self, |command| {
            command.arg("push").arg("-d").arg("origin").arg(name);
        }));
        serial(tasks, action_aggregator)
    }
}
