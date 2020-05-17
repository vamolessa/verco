use std::process::{Command, Stdio};

use crate::{
    action::{ActionTask, CommandTask},
    async_process::ChildOutput,
    select::Entry,
};

pub trait VersionControlActions: Send {
    fn executable_name(&self) -> &'static str;
    fn current_dir(&self) -> &str;

    fn command(&self) -> Command {
        let mut command = Command::new(self.executable_name());
        command.current_dir(self.current_dir());
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command
    }

    /// Sets the root of the current repository
    fn set_root(&mut self) -> Result<(), String>;
    /// Get the root of the current repository
    fn get_root(&self) -> &str;

    fn get_current_changed_files(&mut self) -> Result<Vec<Entry>, String>;
    fn get_revision_changed_files(
        &mut self,
        target: &str,
    ) -> Result<Vec<Entry>, String>;

    fn version(&mut self) -> Result<String, String>;

    fn status(&mut self) -> Box<dyn ActionTask>;
    /// Shows the header and all diffs for the current revision
    fn current_export(&mut self) -> Box<dyn ActionTask>;
    fn log(&mut self, count: usize) -> Box<dyn ActionTask>;

    fn current_diff_all(&mut self) -> Box<dyn ActionTask>;
    fn current_diff_selected(
        &mut self,
        entries: &Vec<Entry>,
    ) -> Box<dyn ActionTask>;

    fn revision_changes(&mut self, target: &str) -> Box<dyn ActionTask>;
    fn revision_diff_all(&mut self, target: &str) -> Box<dyn ActionTask>;
    fn revision_diff_selected(
        &mut self,
        target: &str,
        entries: &Vec<Entry>,
    ) -> Box<dyn ActionTask>;

    fn commit_all(&mut self, message: &str) -> Box<dyn ActionTask>;
    fn commit_selected(
        &mut self,
        message: &str,
        entries: &Vec<Entry>,
    ) -> Box<dyn ActionTask>;
    fn revert_all(&mut self) -> Box<dyn ActionTask>;
    fn revert_selected(&mut self, entries: &Vec<Entry>) -> Box<dyn ActionTask>;
    fn update(&mut self, target: &str) -> Box<dyn ActionTask>;
    fn merge(&mut self, target: &str) -> Box<dyn ActionTask>;

    fn conflicts(&mut self) -> Box<dyn ActionTask>;
    fn take_other(&mut self) -> Box<dyn ActionTask>;
    fn take_local(&mut self) -> Box<dyn ActionTask>;

    fn fetch(&mut self) -> Box<dyn ActionTask>;
    fn pull(&mut self) -> Box<dyn ActionTask>;
    fn push(&mut self) -> Box<dyn ActionTask>;

    fn create_tag(&mut self, name: &str) -> Box<dyn ActionTask>;
    fn list_branches(&mut self) -> Box<dyn ActionTask>;
    fn create_branch(&mut self, name: &str) -> Box<dyn ActionTask>;
    fn close_branch(&mut self, name: &str) -> Box<dyn ActionTask>;
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
    match ChildOutput::from_raw_output(command.output()) {
        ChildOutput::Ok(output) => Ok(output),
        ChildOutput::Err(output) => Err(output),
    }
}
