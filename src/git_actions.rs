use std::process::Command;

use crate::revision_shortcut::RevisionShortcut;
use crate::select::{Entry, State};
use crate::version_control_actions::{
    handle_command, VcsType, VersionControlActions,
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
    pub revision_shortcut: RevisionShortcut,
}

impl GitActions {
    fn command(&self) -> Command {
        let mut command = Command::new("git");
        command.current_dir(&self.current_dir[..]);
        command
    }
}

impl VersionControlActions for GitActions {
    fn get_type(&self) -> VcsType {
        VcsType::Git
    }

    fn set_root(&mut self) -> Result<(), String> {
        let mut command = self.command();
        let dir =
            handle_command(command.args(&["rev-parse", "--show-toplevel"]))?;

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
        let target = self.revision_shortcut.get_hash(target).unwrap_or(target);

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

    fn status(&mut self) -> Result<String, String> {
        handle_command(self.command().args(&[
            "-c",
            "color.status=always",
            "status",
        ]))
    }

    fn current_export(&mut self) -> Result<String, String> {
        handle_command(self.command().args(&["show", "--color"]))
    }

    fn log(&mut self, count: u32) -> Result<String, String> {
        let count_str = format!("-{}", count);

        let hashes_output = handle_command(
            self.command()
                .arg("log")
                .arg("--all")
                .arg("--format=format:%h")
                .arg(&count_str),
        )?;
        let hashes: Vec<_> = hashes_output
            .split_whitespace()
            .take(RevisionShortcut::max())
            .map(String::from)
            .collect();
        self.revision_shortcut.update_hashes(hashes);

        let template = "--format=format:%C(auto,yellow)%h %C(auto,blue)%>(10,trunc)%ad %C(auto,green)%<(10,trunc)%aN %C(auto)%d %C(auto,reset)%s";
        let mut output = handle_command(
            self.command()
                .arg("log")
                .arg("--all")
                .arg("--decorate")
                .arg("--oneline")
                .arg("--graph")
                .arg(&count_str)
                .arg("--color")
                .arg(template)
                .arg("--date=short"),
        )?;

        self.revision_shortcut.replace_occurrences(&mut output);
        Ok(output)
    }

    fn current_diff_all(&mut self) -> Result<String, String> {
        handle_command(self.command().args(&["diff", "--color"]))
    }

    fn current_diff_selected(
        &mut self,
        entries: &Vec<Entry>,
    ) -> Result<String, String> {
        let mut command = self.command();
        command.arg("diff").arg("--color").arg("--");

        for e in entries.iter() {
            if e.selected {
                command.arg(&e.filename);
            }
        }

        handle_command(&mut command)
    }

    fn revision_changes(&mut self, target: &str) -> Result<String, String> {
        let target = self.revision_shortcut.get_hash(target).unwrap_or(target);
        handle_command(
            self.command()
                .arg("diff-tree")
                .arg("--no-commit-id")
                .arg("--name-status")
                .arg("-r")
                .arg(target)
                .arg("--color"),
        )
    }

    fn revision_diff_all(&mut self, target: &str) -> Result<String, String> {
        let target = self.revision_shortcut.get_hash(target).unwrap_or(target);
        let mut parents = String::from(target);
        parents.push_str("^@");

        handle_command(
            self.command()
                .arg("diff")
                .arg(parents)
                .arg(target)
                .arg("--color"),
        )
    }

    fn revision_diff_selected(
        &mut self,
        target: &str,
        entries: &Vec<Entry>,
    ) -> Result<String, String> {
        let target = self.revision_shortcut.get_hash(target).unwrap_or(target);
        let mut parents = String::from(target);
        parents.push_str("^@");

        let mut command = self.command();
        command
            .arg("diff")
            .arg("--color")
            .arg(parents)
            .arg(target)
            .arg("--");

        for e in entries.iter() {
            if e.selected {
                command.arg(&e.filename);
            }
        }

        handle_command(&mut command)
    }

    fn commit_all(&mut self, message: &str) -> Result<String, String> {
        handle_command(self.command().args(&["add", "--all"]))?;
        handle_command(self.command().arg("commit").arg("-m").arg(message))
    }

    fn commit_selected(
        &mut self,
        message: &str,
        entries: &Vec<Entry>,
    ) -> Result<String, String> {
        for e in entries.iter() {
            if e.selected {
                handle_command(
                    self.command().arg("add").arg("--").arg(&e.filename),
                )?;
            }
        }

        handle_command(self.command().arg("commit").arg("-m").arg(message))
    }

    fn revert_all(&mut self) -> Result<String, String> {
        let mut output = String::new();

        output.push_str(
            &handle_command(self.command().args(&["reset", "--hard"]))?[..],
        );
        output.push('\n');
        output.push_str(
            &handle_command(self.command().args(&["clean", "-df"]))?[..],
        );

        Ok(output)
    }

    fn revert_selected(
        &mut self,
        entries: &Vec<Entry>,
    ) -> Result<String, String> {
        let mut output = String::new();

        for e in entries.iter() {
            if !e.selected {
                continue;
            }

            match e.state {
                State::Untracked => {
                    handle_command(
                        self.command()
                            .arg("clean")
                            .arg("-f")
                            .arg("--")
                            .arg(&e.filename),
                    )?;
                }
                State::Added => {
                    handle_command(
                        self.command()
                            .arg("rm")
                            .arg("-f")
                            .arg("--")
                            .arg(&e.filename),
                    )?;
                }
                _ => {
                    let o = handle_command(
                        self.command()
                            .arg("checkout")
                            .arg("--")
                            .arg(&e.filename),
                    )?;
                    output.push_str(&o[..]);
                }
            }
        }

        Ok(output)
    }

    fn update(&mut self, target: &str) -> Result<String, String> {
        let target = self.revision_shortcut.get_hash(target).unwrap_or(target);
        handle_command(self.command().arg("checkout").arg(target))
    }

    fn merge(&mut self, target: &str) -> Result<String, String> {
        let target = self.revision_shortcut.get_hash(target).unwrap_or(target);
        handle_command(self.command().arg("merge").arg(target))
    }

    fn conflicts(&mut self) -> Result<String, String> {
        handle_command(self.command().args(&[
            "diff",
            "--name-only",
            "--diff-filter=U",
        ]))
    }

    fn take_other(&mut self) -> Result<String, String> {
        handle_command(self.command().args(&["checkout", ".", "--theirs"]))
    }

    fn take_local(&mut self) -> Result<String, String> {
        handle_command(self.command().args(&["checkout", ".", "--ours"]))
    }

    fn fetch(&mut self) -> Result<String, String> {
        handle_command(self.command().args(&["fetch", "--all"]))
    }

    fn pull(&mut self) -> Result<String, String> {
        handle_command(self.command().args(&["pull", "--all"]))
    }

    fn push(&mut self) -> Result<String, String> {
        handle_command(self.command().arg("push"))
    }

    fn create_tag(&mut self, name: &str) -> Result<String, String> {
        let mut output = String::new();

        output.push_str(
            &handle_command(self.command().arg("tag").arg(name).arg("-f"))?[..],
        );
        output.push_str(
            &handle_command(
                self.command().arg("push").arg("origin").arg(name),
            )?[..],
        );

        Ok(output)
    }

    fn list_branches(&mut self) -> Result<String, String> {
        handle_command(self.command().args(&["branch", "--all", "--color"]))
    }

    fn create_branch(&mut self, name: &str) -> Result<String, String> {
        let mut output = String::new();

        output.push_str(
            &handle_command(self.command().arg("branch").arg(name))?[..],
        );
        output.push('\n');
        output.push_str(&self.update(name)?[..]);
        output.push('\n');
        output.push_str(
            &handle_command(
                self.command()
                    .arg("push")
                    .arg("--set-upstream")
                    .arg("origin")
                    .arg(name),
            )?[..],
        );

        Ok(output)
    }

    fn close_branch(&mut self, name: &str) -> Result<String, String> {
        let mut output = String::new();

        output.push_str(
            &handle_command(self.command().arg("branch").arg("-d").arg(name))?
                [..],
        );
        output.push('\n');
        output.push_str(
            &handle_command(
                self.command().arg("push").arg("-d").arg("origin").arg(name),
            )?[..],
        );

        Ok(output)
    }
}
