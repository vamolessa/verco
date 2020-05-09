use std::process::Command;

use crate::{
    revision_shortcut::RevisionShortcut,
    select::{Entry, State},
    version_control_actions::{handle_command, VcsType, VersionControlActions},
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
    pub revision_shortcut: RevisionShortcut,
}

impl HgActions {
    fn command(&self) -> Command {
        let mut command = Command::new("hg");
        command.current_dir(&self.current_dir[..]);
        command
    }

    /// Disables user customizations for internal invocations
    fn plain_command(&self) -> Command {
        let mut command = self.command();
        command.env("HGPLAIN", "");
        command
    }
}

impl<'a> VersionControlActions for HgActions {
    fn get_type(&self) -> VcsType {
        VcsType::Hg
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
        let target = self.revision_shortcut.get_hash(target).unwrap_or(target);
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

    fn status(&mut self) -> Result<String, String> {
        let mut output = String::new();

        output.push_str(
            &handle_command(
                self.command().args(&["summary", "--color", "always"]),
            )?[..],
        );
        output.push_str("\n");
        output.push_str(
            &handle_command(
                self.command().args(&["status", "--color", "always"]),
            )?[..],
        );

        Ok(output)
    }

    fn current_export(&mut self) -> Result<String, String> {
        handle_command(self.command().args(&["export", "--color", "always"]))
    }

    fn log(&mut self, count: u32) -> Result<String, String> {
        let count_str = format!("{}", count);

        let hashes_output = handle_command(
            self.plain_command()
                .arg("log")
                .arg("--template")
                .arg("{node|short} ")
                .arg("-l")
                .arg(&count_str),
        )?;
        let hashes: Vec<_> = hashes_output
            .split_whitespace()
            .take(RevisionShortcut::max())
            .map(String::from)
            .collect();
        self.revision_shortcut.update_hashes(hashes);

        let template = "{label('green', if(topics, '[{topics}]'))} {label(ifeq(phase, 'secret', 'yellow', ifeq(phase, 'draft', 'yellow', 'red')), node|short)}{ifeq(branch, 'default', '', label('green', ' ({branch})'))}{bookmarks % ' {bookmark}{ifeq(bookmark, active, '*')}{bookmark}'}{label('yellow', tags % ' {tag}')} {label('magenta', author|person)} {desc|firstline|strip}";
        let mut output = handle_command(
            self.command()
                .arg("log")
                .arg("--graph")
                .arg("--template")
                .arg(template)
                .arg("-l")
                .arg(&count_str)
                .arg("--color")
                .arg("always"),
        )?;

        self.revision_shortcut.replace_occurrences(&mut output);
        Ok(output)
    }
    /// Only works if the user has the `topic` extension enabled
    fn current_stack(&mut self) -> Result<String, String> {
        handle_command(self.command().args(&["stack", "--color", "always"]))
    }

    fn current_diff_all(&mut self) -> Result<String, String> {
        handle_command(self.command().arg("diff").arg("--color").arg("always"))
    }

    fn current_diff_selected(
        &mut self,
        entries: &Vec<Entry>,
    ) -> Result<String, String> {
        let mut command = self.command();
        command.arg("diff").arg("--color").arg("always").arg("--");

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
                .arg("status")
                .arg("--change")
                .arg(target)
                .arg("--color")
                .arg("always"),
        )
    }

    fn revision_diff_all(&mut self, target: &str) -> Result<String, String> {
        let target = self.revision_shortcut.get_hash(target).unwrap_or(target);
        handle_command(
            self.command()
                .arg("diff")
                .arg("--change")
                .arg(target)
                .arg("--color")
                .arg("always"),
        )
    }

    fn revision_diff_selected(
        &mut self,
        target: &str,
        entries: &Vec<Entry>,
    ) -> Result<String, String> {
        let target = self.revision_shortcut.get_hash(target).unwrap_or(target);

        let mut command = self.command();
        command
            .arg("diff")
            .arg("--change")
            .arg(target)
            .arg("--color")
            .arg("always")
            .arg("--");

        for e in entries.iter() {
            if e.selected {
                command.arg(&e.filename);
            }
        }

        handle_command(&mut command)
    }

    fn commit_all(&mut self, message: &str) -> Result<String, String> {
        handle_command(
            self.command()
                .arg("commit")
                .arg("--addremove")
                .arg("-m")
                .arg(message)
                .arg("--color")
                .arg("always"),
        )
    }

    fn commit_selected(
        &mut self,
        message: &str,
        entries: &Vec<Entry>,
    ) -> Result<String, String> {
        let mut cmd = self.command();
        cmd.arg("commit");

        for e in entries.iter() {
            if e.selected {
                match e.state {
                    State::Missing | State::Deleted => {
                        handle_command(
                            self.command().arg("remove").arg(&e.filename),
                        )?;
                    }
                    State::Untracked => {
                        handle_command(
                            self.command().arg("add").arg(&e.filename),
                        )?;
                    }
                    _ => (),
                }

                cmd.arg(&e.filename);
            }
        }

        handle_command(cmd.arg("-m").arg(message).arg("--color").arg("always"))
    }

    fn revert_all(&mut self) -> Result<String, String> {
        let mut output = String::new();

        output.push_str(&handle_command(self.command().args(&["revert", "-C", "--all"]))?[..]);
        output.push_str("\n");
        output.push_str(&handle_command(self.command().args(&["purge"]))?[..]);

        Ok(output)
    }

    fn revert_selected(
        &mut self,
        entries: &Vec<Entry>,
    ) -> Result<String, String> {
        let mut output = String::new();

        let mut cmd = self.command();
        cmd.arg("revert").arg("-C").arg("--color").arg("always");

        let mut has_revert_file = false;

        for e in entries.iter() {
            if !e.selected {
                continue;
            }

            match e.state {
                State::Untracked => {
                    output.push_str(
                        &handle_command(
                            self.command().arg("purge").arg(&e.filename),
                        )?[..],
                    );
                }
                _ => {
                    has_revert_file = true;
                    cmd.arg(&e.filename);
                }
            }
        }

        if has_revert_file {
            output.push_str(&handle_command(&mut cmd)?[..]);
        }

        Ok(output)
    }

    fn update(&mut self, target: &str) -> Result<String, String> {
        let target = self.revision_shortcut.get_hash(target).unwrap_or(target);
        handle_command(self.command().arg("update").arg(target))
    }

    fn merge(&mut self, target: &str) -> Result<String, String> {
        let target = self.revision_shortcut.get_hash(target).unwrap_or(target);
        handle_command(self.command().arg("merge").arg(target))
    }

    fn conflicts(&mut self) -> Result<String, String> {
        handle_command(
            self.command().args(&["resolve", "-l", "--color", "always"]),
        )
    }

    fn take_other(&mut self) -> Result<String, String> {
        handle_command(self.command().args(&[
            "resolve",
            "-a",
            "-t",
            "internal:other",
        ]))
    }

    fn take_local(&mut self) -> Result<String, String> {
        handle_command(self.command().args(&[
            "resolve",
            "-a",
            "-t",
            "internal:local",
        ]))
    }

    fn fetch(&mut self) -> Result<String, String> {
        self.pull()
    }

    fn pull(&mut self) -> Result<String, String> {
        handle_command(self.command().arg("pull"))
    }

    fn push(&mut self) -> Result<String, String> {
        handle_command(self.command().args(&["push", "--new-branch"]))
    }

    fn create_tag(&mut self, name: &str) -> Result<String, String> {
        handle_command(self.command().arg("tag").arg(name).arg("-f"))
    }

    fn list_branches(&mut self) -> Result<String, String> {
        handle_command(self.command().args(&["branches", "--color", "always"]))
    }

    fn create_branch(&mut self, name: &str) -> Result<String, String> {
        handle_command(self.command().arg("branch").arg(name))
    }

    fn close_branch(&mut self, name: &str) -> Result<String, String> {
        let changeset =
            handle_command(self.command().args(&["identify", "--num"]))?;
        self.update(name)?;

        let mut output = String::new();
        output.push_str(
            &handle_command(self.command().args(&[
                "commit",
                "-m",
                "\"close branch\"",
                "--close-branch",
            ]))?[..],
        );
        output.push_str("\n");
        output.push_str(&self.update(changeset.trim())?[..]);

        Ok(output)
    }
}
