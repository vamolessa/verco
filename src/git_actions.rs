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

    fn get_current_changed_files(&self) -> Result<Vec<Entry>, String> {
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

    fn version(&self) -> Result<String, String> {
        handle_command(self.command().arg("--version"))
    }

    fn status(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.args(&["-c", "color.status=always", "status"]);
        })
    }

    fn current_export(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.args(&["show", "--color"]);
        })
    }

    fn log(&self, count: usize) -> Box<dyn ActionTask> {
        task(self, |command| {
            let count_str = format!("-{}", count);
            //let template = "--format=format:%C(auto,yellow)%h %C(auto,blue)%>(10,trunc)%ad %C(auto,green)%<(10,trunc)%aN %C(auto)%d %C(auto,reset)%s";
            let template =
                "--format=format:%x1e%h%x1e%as%x1e%<(10,trunc)%aN%x1e%D%x1e%s";
            command
                .arg("log")
                .arg("--all")
                .arg("--decorate")
                .arg("--oneline")
                .arg("--graph")
                .arg(&count_str)
                //.arg("--color")
                .arg(template);
            //.arg("--date=short");
        })
    }

    fn current_diff_all(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.args(&["diff", "--color"]);
        })
    }

    fn current_diff_selected(
        &mut self,
        entries: &Vec<Entry>,
    ) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.arg("diff").arg("--color").arg("--");
            for e in entries.iter().filter(|e| e.selected) {
                command.arg(&e.filename);
            }
        })
    }

    fn revision_changes(&self, target: &str) -> Box<dyn ActionTask> {
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

    fn revision_diff_all(&self, target: &str) -> Box<dyn ActionTask> {
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
    ) -> Box<dyn ActionTask> {
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

    fn commit_all(&self, message: &str) -> Box<dyn ActionTask> {
        let mut tasks = task_vec();
        tasks.push(task(self, |command| {
            command.args(&["add", "--all"]);
        }));
        tasks.push(task(self, |command| {
            command.arg("commit").arg("-m").arg(message);
        }));
        serial(tasks)
    }

    fn commit_selected(
        &mut self,
        message: &str,
        entries: &Vec<Entry>,
    ) -> Box<dyn ActionTask> {
        let mut tasks = task_vec();
        for e in entries.iter().filter(|e| e.selected) {
            tasks.push(task(self, |command| {
                command.arg("add").arg("--").arg(&e.filename);
            }));
        }

        tasks.push(task(self, |command| {
            command.arg("commit").arg("-m").arg(message);
        }));
        serial(tasks)
    }

    fn revert_all(&self) -> Box<dyn ActionTask> {
        let mut tasks = task_vec();
        tasks.push(task(self, |command| {
            command.args(&["reset", "--hard"]);
        }));
        tasks.push(task(self, |command| {
            command.args(&["clean", "-df"]);
        }));
        serial(tasks)
    }

    fn revert_selected(&self, entries: &Vec<Entry>) -> Box<dyn ActionTask> {
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
        parallel(tasks)
    }

    fn update(&self, target: &str) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.arg("checkout").arg(target);
        })
    }

    fn merge(&self, target: &str) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.arg("merge").arg(target);
        })
    }

    fn conflicts(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.args(&["diff", "--name-only", "--diff-filter=U"]);
        })
    }

    fn take_other(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.args(&["checkout", ".", "--theirs"]);
        })
    }

    fn take_local(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.args(&["checkout", ".", "--ours"]);
        })
    }

    fn fetch(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.args(&["fetch", "--all"]);
        })
    }

    fn pull(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.args(&["pull", "--all"]);
        })
    }

    fn push(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.arg("push");
        })
    }

    fn create_tag(&self, name: &str) -> Box<dyn ActionTask> {
        let mut tasks = task_vec();
        tasks.push(task(self, |command| {
            command.arg("tag").arg(name).arg("-f");
        }));
        tasks.push(task(self, |command| {
            command.arg("push").arg("origin").arg(name);
        }));
        serial(tasks)
    }

    fn list_branches(&self) -> Box<dyn ActionTask> {
        task(self, |command| {
            command.args(&["branch", "--all", "--color"]);
        })
    }

    fn create_branch(&self, name: &str) -> Box<dyn ActionTask> {
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
        serial(tasks)
    }

    fn close_branch(&self, name: &str) -> Box<dyn ActionTask> {
        let mut tasks = task_vec();
        tasks.push(task(self, |command| {
            command.arg("branch").arg("-d").arg(name);
        }));
        tasks.push(task(self, |command| {
            command.arg("push").arg("-d").arg("origin").arg(name);
        }));
        serial(tasks)
    }
}
