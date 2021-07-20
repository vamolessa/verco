use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use crate::{application::ProcessTag, platform::PlatformRequest};

pub struct Context<'a> {
    root: &'a Path,
    platform_requests: &'a mut Vec<PlatformRequest>,
}
impl<'a> Context<'a> {
    pub fn new(
        root: &'a Path,
        platform_requests: &'a mut Vec<PlatformRequest>,
    ) -> Self {
        Self {
            root,
            platform_requests,
        }
    }

    pub fn spawn(&mut self, tag: ProcessTag, mut command: Command) {
        command.current_dir(self.root);
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::null());

        self.platform_requests.push(PlatformRequest::SpawnProcess {
            tag,
            command,
            buf_len: 4 * 1024,
        });
    }
}

pub mod git;

pub trait Backend {
    fn name(&self) -> &str;

    //fn get_changed_files_workspace(&mut self, ctx: &mut Context);
    //fn get_changed_files_revision(&mut self, ctx: &mut Context, revision: &str);
}

/*
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
*/

pub fn get_command_output(command_name: &str, args: &[&str]) -> Option<String> {
    let mut command = Command::new(command_name);
    command.args(args);
    command.stdin(Stdio::null());
    command.stdout(Stdio::piped());
    command.stderr(Stdio::null());
    let child = command.spawn().ok()?;
    let output = child.wait_with_output().ok()?;
    let output = String::from_utf8_lossy(&output.stdout);
    Some(output.into())
}

pub fn backend_from_current_repository() -> Option<(PathBuf, Box<dyn Backend>)>
{
    if let Some((root, git)) = git::Git::try_new() {
        Some((root, Box::new(git)))
    } else {
        None
    }
}

