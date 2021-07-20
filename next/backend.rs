use std::{
    path::PathBuf,
    process::{Command, Stdio},
};

pub mod git;

pub trait Backend {
    //
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

pub fn backend_from_current_repository(
) -> Option<(PathBuf, Box<dyn Backend>)> {
    if let Some((root, git)) = git::Git::try_new() {
        Some((root, Box::new(git)))
    } else {
        None
    }
}

