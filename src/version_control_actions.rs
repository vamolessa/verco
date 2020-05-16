use std::process::{Command, Stdio};

use crate::{
    application::{get_process_output, ActionResult, ActionTask},
    select::Entry,
    worker::Task,
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

    fn status(&mut self) -> Box<dyn Task<Output = ActionResult>>;
    /// Shows the header and all diffs for the current revision
    fn current_export(&mut self) -> Box<dyn Task<Output = ActionResult>>;
    fn log(&mut self, count: usize) -> Box<dyn Task<Output = ActionResult>>;

    fn current_diff_all(&mut self) -> Box<dyn Task<Output = ActionResult>>;
    fn current_diff_selected(
        &mut self,
        entries: &Vec<Entry>,
    ) -> Box<dyn Task<Output = ActionResult>>;

    fn revision_changes(
        &mut self,
        target: &str,
    ) -> Box<dyn Task<Output = ActionResult>>;
    fn revision_diff_all(
        &mut self,
        target: &str,
    ) -> Box<dyn Task<Output = ActionResult>>;
    fn revision_diff_selected(
        &mut self,
        target: &str,
        entries: &Vec<Entry>,
    ) -> Box<dyn Task<Output = ActionResult>>;

    fn commit_all(
        &mut self,
        message: &str,
    ) -> Box<dyn Task<Output = ActionResult>>;
    fn commit_selected(
        &mut self,
        message: &str,
        entries: &Vec<Entry>,
    ) -> Box<dyn Task<Output = ActionResult>>;
    fn revert_all(&mut self) -> Box<dyn Task<Output = ActionResult>>;
    fn revert_selected(
        &mut self,
        entries: &Vec<Entry>,
    ) -> Box<dyn Task<Output = ActionResult>>;
    fn update(&mut self, target: &str) -> Box<dyn Task<Output = ActionResult>>;
    fn merge(&mut self, target: &str) -> Box<dyn Task<Output = ActionResult>>;

    fn conflicts(&mut self) -> Box<dyn Task<Output = ActionResult>>;
    fn take_other(&mut self) -> Box<dyn Task<Output = ActionResult>>;
    fn take_local(&mut self) -> Box<dyn Task<Output = ActionResult>>;

    fn fetch(&mut self) -> Box<dyn Task<Output = ActionResult>>;
    fn pull(&mut self) -> Box<dyn Task<Output = ActionResult>>;
    fn push(&mut self) -> Box<dyn Task<Output = ActionResult>>;

    fn create_tag(
        &mut self,
        name: &str,
    ) -> Box<dyn Task<Output = ActionResult>>;
    fn list_branches(&mut self) -> Box<dyn Task<Output = ActionResult>>;
    fn create_branch(
        &mut self,
        name: &str,
    ) -> Box<dyn Task<Output = ActionResult>>;
    fn close_branch(
        &mut self,
        name: &str,
    ) -> Box<dyn Task<Output = ActionResult>>;
}

pub fn task<F>(
    version_control: &dyn VersionControlActions,
    builder: F,
) -> Box<dyn Task<Output = ActionResult>>
where
    F: FnOnce(&mut Command),
{
    let mut command = version_control.command();
    (builder)(&mut command);
    Box::new(ActionTask::Waiting(command))
}

pub fn handle_command(command: &mut Command) -> Result<String, String> {
    match command.spawn() {
        Ok(mut child) => get_process_output(&mut child),
        Err(e) => Err(e.to_string()),
    }
}
