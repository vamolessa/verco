use std::process::{Command, Stdio};

use crate::{
    action::{ActionTask, CommandTask},
    select::Entry,
};

pub trait VersionControlActions: Send {
    fn executable_name(&self) -> &'static str;
    fn current_dir(&self) -> &str;

    fn command(&self) -> Command {
        let mut command = Command::new(self.executable_name());
        command.current_dir(self.current_dir());
        command.stdin(Stdio::null());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command
    }

    /// Sets the root of the current repository
    fn set_root(&mut self) -> Result<(), String>;
    /// Get the root of the current repository
    fn get_root(&self) -> &str;

    fn get_current_changed_files(&self) -> Result<Vec<Entry>, String>;
    fn get_revision_changed_files(
        &self,
        target: &str,
    ) -> Result<Vec<Entry>, String>;

    fn version(&self) -> Result<String, String>;

    fn status(&self) -> Box<dyn ActionTask>;
    /// Shows the header and all diffs for the current revision
    fn current_export(&self) -> Box<dyn ActionTask>;
    fn log(&self, count: usize) -> Box<dyn ActionTask>;

    fn current_diff_all(&self) -> Box<dyn ActionTask>;
    fn current_diff_selected(
        &self,
        entries: &Vec<Entry>,
    ) -> Box<dyn ActionTask>;

    fn revision_changes(&self, target: &str) -> Box<dyn ActionTask>;
    fn revision_diff_all(&self, target: &str) -> Box<dyn ActionTask>;
    fn revision_diff_selected(
        &self,
        target: &str,
        entries: &Vec<Entry>,
    ) -> Box<dyn ActionTask>;

    fn commit_all(&self, message: &str) -> Box<dyn ActionTask>;
    fn commit_selected(
        &self,
        message: &str,
        entries: &Vec<Entry>,
    ) -> Box<dyn ActionTask>;
    fn revert_all(&self) -> Box<dyn ActionTask>;
    fn revert_selected(&self, entries: &Vec<Entry>) -> Box<dyn ActionTask>;
    fn update(&self, target: &str) -> Box<dyn ActionTask>;
    fn merge(&self, target: &str) -> Box<dyn ActionTask>;

    fn conflicts(&self) -> Box<dyn ActionTask>;
    fn take_other(&self) -> Box<dyn ActionTask>;
    fn take_local(&self) -> Box<dyn ActionTask>;

    fn fetch(&self) -> Box<dyn ActionTask>;
    fn pull(&self) -> Box<dyn ActionTask>;
    fn push(&self) -> Box<dyn ActionTask>;

    fn create_tag(&self, name: &str) -> Box<dyn ActionTask>;
    fn list_branches(&self) -> Box<dyn ActionTask>;
    fn create_branch(&self, name: &str) -> Box<dyn ActionTask>;
    fn close_branch(&self, name: &str) -> Box<dyn ActionTask>;
}

pub fn task<F>(
    version_control: &dyn VersionControlActions,
    builder: F,
) -> Box<dyn ActionTask>
where
    F: FnOnce(&mut Command),
{
    let mut command = version_control.command();
    (builder)(&mut command);
    Box::new(CommandTask::Waiting(command))
}

pub fn handle_command(command: &mut Command) -> Result<String, String> {
    match command.output() {
        Ok(output) => {
            if output.status.success() {
                String::from_utf8(output.stdout).map_err(|e| e.to_string())
            } else {
                String::from_utf8(output.stderr)
                    .map_err(|e| e.to_string())
                    .and_then(|o| Err(o))
            }
        }
        Err(error) => Err(error.to_string()),
    }
}
