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

//use crate::application

pub mod git;

pub trait Backend {
    fn status(&mut self, ctx: &mut Context);
}

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

