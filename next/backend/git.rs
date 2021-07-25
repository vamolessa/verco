use std::{path::PathBuf, process::Command};

use crate::{
    backend::{get_command_output, Backend},
    platform::Context,
    promise::Task,
};

pub struct Git {
    //
}

impl Git {
    pub fn try_new() -> Option<(PathBuf, Self)> {
        let dir = get_command_output("git", &["rev-parse", "--show-toplevel"])?;
        let dir = dir.lines().next()?;
        let mut root = PathBuf::new();
        root.push(dir);
        Some((root, Self {}))
    }
}

impl Backend for Git {
    fn name(&self) -> &str {
        "git"
    }

    fn status(&self, ctx: &mut Context) -> Task<String> {
        let mut command = Command::new("git");
        command.arg("status");
        ctx.spawn(command).into()
    }
}

